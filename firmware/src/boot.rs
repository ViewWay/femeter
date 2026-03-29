/* ================================================================== */
/*                                                                    */
/*  boot.rs — FeMeter Bootloader                                      */
/*                                                                    */
/*  功能:                                                              */
/*    1. 检查 OTA 区是否有有效固件                                      */
/*    2. 校验 CRC32                                                    */
/*    3. 搬运 OTA → Normal 区                                          */
/*    4. 校验搬运结果                                                   */
/*    5. 跳转到 Normal 区执行                                           */
/*    6. 失败则回滚旧固件                                              */
/*    7. LED 指示升级状态                                              */
/*                                                                    */
/*  Flash 分区:                                                        */
/*    Boot:   0x0000_0000 ~ 0x0000_3FFF (16KB)                        */
/*    Normal: 0x0000_4000 ~ 0x0002_3FFF (128KB)                       */
/*    OTA:    0x0002_4000 ~ 0x0004_3FFF (128KB)                       */
/*    Param:  0x0004_4000 ~ 0x0004_7FFF (16KB)                        */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use panic_halt as _;

/* ================================================================== */
/*  Flash 分区地址常量                                                  */
/* ================================================================== */

const FLASH_BASE: u32    = 0x0000_0000;
const BOOT_BASE: u32     = 0x0000_0000;
const BOOT_SIZE: u32     = 16 * 1024;       // 16KB
const NORMAL_BASE: u32   = 0x0000_4000;
const NORMAL_SIZE: u32   = 128 * 1024;      // 128KB
const OTA_BASE: u32      = 0x0002_4000;
const OTA_SIZE: u32      = 128 * 1024;      // 128KB
const PARAM_BASE: u32    = 0x0004_4000;
const PARAM_SIZE: u32    = 16 * 1024;       // 16KB

/// OTA 固件头 (存放在 OTA 区开头)
#[repr(C)]
struct OtaHeader {
    /// 魔数 0x4654_4D52 ("FTMR" = FeMeTeR)
    magic: u32,
    /// 固件大小 (字节, 不含 header)
    firmware_size: u32,
    /// 固件 CRC32
    firmware_crc: u32,
    /// 固件版本 (major.minor.patch)
    version: [u8; 4],
    /// 构建时间戳 (Unix)
    build_timestamp: u32,
    /// 物料组合哈希 (确保固件与硬件匹配)
    hardware_hash: u32,
    /// 预留
    _reserved: [u32; 2],
}

const OTA_MAGIC: u32 = 0x4654_4D52; // "FTMR"

/// 升级状态
#[derive(Clone, Copy, Debug)]
enum BootStatus {
    /// 正常启动, 无 OTA
    NormalBoot,
    /// 检测到 OTA, 正在搬运
    OtaCopying,
    /// OTA 完成, 跳转
    OtaDone,
    /// OTA 校验失败, 回滚
    OtaFailedRollback,
    /// 严重错误 (无法恢复)
    FatalError,
}

/* ================================================================== */
/*  CRC32 计算                                                         */
/* ================================================================== */

/// CRC32 (IEEE 802.3 多项式: 0xEDB88320)
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/* ================================================================== */
/*  Flash 操作 (FM33A0xxEV)                                            */
/* ================================================================== */

/// Flash 页大小: 512 字节
const FLASH_PAGE_SIZE: u32 = 512;
/// Flash 扇区大小: 2KB
const FLASH_SECTOR_SIZE: u32 = 2048;

/// 解锁 Flash (允许写入/擦除)
unsafe fn flash_unlock() {
    // FM33A0xxEV Flash 解锁序列
    // 实际地址需查 SVD, 这里用占位符
    let flash_key = 0x4000_1000 as *mut u32;
    core::ptr::write_volatile(flash_key, 0x5566_A5A5);
}

/// 锁定 Flash
unsafe fn flash_lock() {
    let flash_key = 0x4000_1000 as *mut u32;
    core::ptr::write_volatile(flash_key, 0x0);
}

/// 擦除 Flash 扇区 (2KB)
unsafe fn flash_erase_sector(sector_addr: u32) {
    flash_unlock();
    // FM33A0xxEV 扇区擦除序列
    let flash_ctrl = 0x4000_1004 as *mut u32;
    // 1. 设置扇区擦除位
    core::ptr::write_volatile(flash_ctrl, 0x02);
    // 2. 写目标地址触发擦除
    core::ptr::write_volatile(sector_addr as *mut u32, 0xFFFF_FFFF);
    // 3. 等待完成
    while core::ptr::read_volatile(flash_ctrl) & 0x01 != 0 {}
    flash_lock();
}

/// 写入 Flash 页 (512 字节, 需先擦除)
unsafe fn flash_write_page(page_addr: u32, data: &[u8]) {
    flash_unlock();
    let flash_ctrl = 0x4000_1004 as *mut u32;
    // 1. 设置页编程位
    core::ptr::write_volatile(flash_ctrl, 0x01);

    // 2. 写入数据 (32-bit 对齐)
    let dst = page_addr as *mut u32;
    let mut src_offset = 0;
    while src_offset < data.len() {
        let word = if src_offset + 4 <= data.len() {
            u32::from_le_bytes([
                data[src_offset],
                data[src_offset + 1],
                data[src_offset + 2],
                data[src_offset + 3],
            ])
        } else {
            let mut buf = [0xFFu8; 4];
            let remaining = data.len() - src_offset;
            buf[..remaining].copy_from_slice(&data[src_offset..]);
            u32::from_le_bytes(buf)
        };
        core::ptr::write_volatile(dst.add(src_offset / 4), word);
        src_offset += 4;
    }

    // 3. 等待完成
    while core::ptr::read_volatile(flash_ctrl) & 0x01 != 0 {}
    flash_lock();
}

/* ================================================================== */
/*  内存拷贝 (Flash → Flash, 通过 RAM 中转)                            */
/* ================================================================== */

/// 将 OTA 区的固件搬运到 Normal 区
///
/// 返回 true = 成功, false = 失败
unsafe fn copy_ota_to_normal(header: &OtaHeader) -> bool {
    let size = header.firmware_size as usize;
    if size > NORMAL_SIZE as usize {
        return false;
    }

    // 按扇区擦除 Normal 区
    let sectors_needed = (size + FLASH_SECTOR_SIZE as usize - 1) / FLASH_SECTOR_SIZE as usize;
    for i in 0..sectors_needed {
        flash_erase_sector(NORMAL_BASE + (i as u32) * FLASH_SECTOR_SIZE);
    }

    // 按页拷贝 (512 字节)
    // Flash 不能直接 Flash→Flash, 需要读取到 RAM 再写回
    let mut page_buf = [0u8; 512];
    let pages_needed = (size + 511) / 512;
    for i in 0..pages_needed {
        let src_addr = OTA_BASE + 32 + (i as u32) * FLASH_PAGE_SIZE; // +32 跳过 header
        let dst_addr = NORMAL_BASE + (i as u32) * FLASH_PAGE_SIZE;

        // 读取源页到 RAM
        let src_ptr = src_addr as *const u8;
        let copy_len = if i == pages_needed - 1 && size % 512 != 0 {
            size % 512
        } else {
            512
        };
        for j in 0..copy_len {
            page_buf[j] = core::ptr::read_volatile(src_ptr.add(j));
        }
        // 剩余填充 0xFF
        for j in copy_len..512 {
            page_buf[j] = 0xFF;
        }

        // 写入目标页
        flash_write_page(dst_addr, &page_buf);
    }

    // 校验: 计算 Normal 区 CRC32
    let normal_ptr = NORMAL_BASE as *const u8;
    let mut verify_data = core::slice::from_raw_parts(normal_ptr, size);
    let actual_crc = crc32(verify_data);

    actual_crc == header.firmware_crc
}

/* ================================================================== */
/*  跳转到 Normal 区                                                   */
/* ================================================================== */

/// 跳转到应用程序 (Normal 区)
unsafe fn jump_to_app() -> ! {
    // Normal 区向量表: 第一个字 = 栈顶, 第二个字 = Reset Handler
    let vt_base = NORMAL_BASE;
    let sp = core::ptr::read_volatile(vt_base as *const u32);
    let reset_handler = core::ptr::read_volatile((vt_base + 4) as *const u32);

    // 设置栈指针
    cortex_m::register::msp::write(sp);

    // 重定向 VTOR (FM33A0xxEV SCB_VTOR)
    let scb_vtor = 0xE000_ED08 as *mut u32;
    core::ptr::write_volatile(scb_vtor, vt_base);

    // 跳转
    let reset_fn: fn() -> ! = core::mem::transmute(reset_handler as *const ());
    reset_fn()
}

/* ================================================================== */
/*  LED 指示 (简单 GPIO 翻转)                                          */
/* ================================================================== */

/// LED 快闪 (升级中)
fn led_blink_fast() {
    // TODO: 实现 GPIO 翻转
    // 使用 PA8 或其他可用引脚
}

/// LED 常亮 (正常启动)
fn led_on() {
    // TODO
}

/// LED 慢闪 (错误)
fn led_blink_slow() {
    // TODO
}

/* ================================================================== */
/*  简单延时                                                           */
/* ================================================================== */

fn delay_ms(ms: u32) {
    // 假设 CPU 在 64MHz: 每毫秒约 64000 次循环
    let loops = ms * 64000 / 10;
    let mut i = 0;
    while i < loops {
        i += 1;
        core::hint::spin_loop();
    }
}

/* ================================================================== */
/*  主入口                                                              */
/* ================================================================== */

#[entry]
fn main() -> ! {
    let status = boot_sequence();

    match status {
        BootStatus::NormalBoot | BootStatus::OtaDone => {
            led_on();
            unsafe { jump_to_app() };
        }
        BootStatus::OtaFailedRollback => {
            // 回滚失败, 尝试用旧固件启动
            led_blink_slow();
            delay_ms(1000);
            unsafe { jump_to_app() };
        }
        BootStatus::OtaCopying => {
            // 不应该到这里
            led_blink_slow();
            loop {}
        }
        BootStatus::FatalError => {
            led_blink_slow();
            loop {}
        }
    }
}

/// Boot 主逻辑
fn boot_sequence() -> BootStatus {
    // 1. 读取 OTA header
    let ota_header = unsafe {
        let ptr = OTA_BASE as *const OtaHeader;
        &*ptr
    };

    // 2. 检查是否有有效 OTA
    if ota_header.magic != OTA_MAGIC {
        // 无 OTA, 正常启动
        return BootStatus::NormalBoot;
    }

    // 3. 验证 OTA 固件 CRC
    let firmware_size = ota_header.firmware_size as usize;
    if firmware_size == 0 || firmware_size > OTA_SIZE as usize {
        // OTA 无效, 清除 magic 并正常启动
        unsafe {
            flash_erase_sector(OTA_BASE);
        }
        return BootStatus::NormalBoot;
    }

    // 读取 OTA 固件数据计算 CRC
    let ota_firmware_ptr = unsafe { (OTA_BASE + 32) as *const u8 };
    let ota_firmware = unsafe { core::slice::from_raw_parts(ota_firmware_ptr, firmware_size) };
    let calc_crc = crc32(ota_firmware);

    if calc_crc != ota_header.firmware_crc {
        // CRC 校验失败, 清除 OTA
        unsafe {
            flash_erase_sector(OTA_BASE);
        }
        return BootStatus::NormalBoot;
    }

    // 4. 保存旧 Normal 区的 CRC (用于回滚)
    let old_normal_ptr = unsafe { NORMAL_BASE as *const u8 };
    // 注意: 旧固件可能不完整, 取前 firmware_size 字节做 CRC
    let old_normal_data = unsafe { core::slice::from_raw_parts(old_normal_ptr, firmware_size.min(NORMAL_SIZE as usize)) };
    let old_crc = crc32(old_normal_data);

    // 5. 搬运 OTA → Normal
    led_blink_fast();
    let success = unsafe { copy_ota_to_normal(ota_header) };

    if success {
        // 6. 清除 OTA header (标记升级完成)
        unsafe {
            flash_erase_sector(OTA_BASE);
        }
        BootStatus::OtaDone
    } else {
        // 7. 搬运失败, 尝试回滚
        //    这里简单处理: 如果旧 CRC 仍然匹配, 说明旧固件还在
        let current_normal = unsafe { core::slice::from_raw_parts(old_normal_ptr, firmware_size.min(NORMAL_SIZE as usize)) };
        let current_crc = crc32(current_normal);

        if current_crc == old_crc {
            // 旧固件完好, 可以启动
            unsafe { flash_erase_sector(OTA_BASE); }
            BootStatus::OtaFailedRollback
        } else {
            // 旧固件也坏了, 严重错误
            BootStatus::FatalError
        }
    }
}
