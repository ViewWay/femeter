/* ================================================================== */
/*                                                                    */
/*  ota.rs — 双 Bank OTA 固件升级管理                                   */
/*                                                                    */
/*  Flash 布局 (FM33A068EV, 512KB):                                   */
/*    0x000000 - 0x000FFF: Bootloader (4KB)                           */
/*    0x001000 - 0x03FFFF: App Bank 1 (252KB, 0x3F000)               */
/*    0x040000 - 0x07FFFF: App Bank 2 (256KB)                        */
/*    0x080000 - 0x0FFFFF: OTA 数据区 (512KB, 存储升级固件)           */
/*    0x100000 - 0x1FFFFF: 保留 (512KB)                               */
/*                                                                    */
/*  升级流程:                                                          */
/*    1. 接收新固件到 OTA 数据区                                       */
/*    2. 校验 CRC32 + 版本号                                           */
/*    3. 擦除目标 Bank                                                 */
/*    4. 拷贝 OTA 数据到目标 Bank                                      */
/*    5. 设置启动标志 + 重启                                           */
/*    6. Bootloader 检测标志, 从新 Bank 启动                           */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use core::mem::size_of;

/* ── Flash 地址定义 ── */

pub mod addr {
    /// Bootloader 起始地址
    pub const BOOT_START: u32 = 0x0000_0000;
    /// Bootloader 大小
    pub const BOOT_SIZE: u32 = 0x0000_1000; // 4KB

    /// App Bank 1 起始地址
    pub const APP1_START: u32 = 0x0000_1000;
    /// App Bank 1 大小
    pub const APP1_SIZE: u32 = 0x0003_F000; // 252KB

    /// App Bank 2 起始地址
    pub const APP2_START: u32 = 0x0004_0000;
    /// App Bank 2 大小
    pub const APP2_SIZE: u32 = 0x0004_0000; // 256KB

    /// OTA 暂存区起始地址
    pub const OTA_START: u32 = 0x0008_0000;
    /// OTA 暂存区大小
    pub const OTA_SIZE: u32 = 0x0008_0000; // 512KB

    /// App 最大固件大小 (取两个 Bank 较小值)
    pub const APP_MAX_SIZE: u32 = APP1_SIZE; // 252KB

    /// 升级信息存储地址 (OTA 区末尾最后 4KB)
    pub const UPGRADE_INFO_ADDR: u32 = OTA_START + OTA_SIZE - 0x1000;
}

/* ── 固件版本 ── */

/// 固件版本 (MAJOR.MINOR.PATCH.BUILD)
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FirmwareVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub build: u8,
}

impl FirmwareVersion {
    pub const fn new(major: u8, minor: u8, patch: u8, build: u8) -> Self {
        Self { major, minor, patch, build }
    }

    /// 转为 u32 用于比较
    pub fn as_u32(&self) -> u32 {
        (self.major as u32) << 24 | (self.minor as u32) << 16
            | (self.patch as u32) << 8 | (self.build as u32)
    }

    /// 版本字符串 (用于显示, 最多 16 字节含结尾 0)
    pub fn to_str(&self) -> [u8; 16] {
        let s = [
            b'v',
            b'0' + (self.major / 10), b'0' + (self.major % 10), b'.',
            b'0' + (self.minor / 10), b'0' + (self.minor % 10), b'.',
            b'0' + (self.patch / 10), b'0' + (self.patch % 10), b'.',
            b'0' + (self.build / 100), b'0' + ((self.build / 10) % 10), b'0' + (self.build % 10),
            0, 0, 0, 0,
        ];
        s
    }
}

/// 当前固件版本 (编译时注入)
pub const CURRENT_VERSION: FirmwareVersion = FirmwareVersion::new(
    env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap_or(0),
    env!("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(2),
    env!("CARGO_PKG_VERSION_PATCH").parse().unwrap_or(0),
    0, // build number, 编译时可选
);

/* ── 升级状态 ── */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OtaState {
    /// 空闲, 无升级
    Idle = 0,
    /// 正在接收固件
    Receiving = 1,
    /// 接收完成, 等待验证
    Received = 2,
    /// 校验通过, 等待安装
    Verified = 3,
    /// 正在写入目标 Bank
    Installing = 4,
    /// 安装完成, 等待重启
    Installed = 5,
    /// 升级失败
    Failed = 6,
}

/* ── 升级信息头 (存储在 OTA 区末尾) ── */

/// 固件镜像头 (附加在固件 bin 开头)
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FirmwareHeader {
    /// 魔数 0x464D5441 ("FMTA")
    pub magic: u32,
    /// 固件版本
    pub version: FirmwareVersion,
    /// 固件大小 (不含 header, bytes)
    pub firmware_size: u32,
    /// CRC32 (不含 header)
    pub crc32: u32,
    /// 目标 Bank: 1 或 2
    pub target_bank: u8,
    /// 标志位: 0x01=需要安装
    pub flags: u8,
    /// 升级时间戳 (秒, 2000-01-01 起)
    pub timestamp: u32,
    /// 上一个版本
    pub prev_version: FirmwareVersion,
    /// 保留
    pub reserved: [u8; 16],
}

impl FirmwareHeader {
    pub const MAGIC: u32 = 0x464D5441;
    pub const SIZE: usize = size_of::<Self>(); // 应该是 40 bytes

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC && self.firmware_size <= addr::APP_MAX_SIZE
    }
}

/* ── 升级信息 (存储在固定地址) ── */

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct UpgradeInfo {
    /// 当前活动 Bank (1 或 2)
    pub active_bank: u8,
    /// 升级状态
    pub state: OtaState,
    /// 已接收字节数
    pub received_bytes: u32,
    /// 升级错误码 (0=无错误)
    pub error_code: u32,
    /// 升级历史记录 (最近 8 次)
    pub history: [UpgradeRecord; 8],
}

/* ── 升级记录 ── */

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct UpgradeRecord {
    /// 旧版本
    pub from_version: FirmwareVersion,
    /// 新版本
    pub to_version: FirmwareVersion,
    /// 升级时间戳
    pub timestamp: u32,
    /// 结果: 0=成功, 非0=失败错误码
    pub result: u32,
    /// 目标 Bank
    pub bank: u8,
    /// 升级来源: 0=RS485, 1=红外, 2=LoRaWAN, 3=蜂窝, 4=本地
    pub source: u8,
    /// 保留
    pub reserved: [u8; 10],
}

/* ── OTA 管理器 ── */

/// Flash 操作 trait (由 board.rs 实现)
pub trait OtaFlash {
    /// 读取 Flash
    fn flash_read(addr: u32, buf: &mut [u8]) -> Result<(), ()>;
    /// 写入 Flash (已擦除的区域)
    fn flash_write(addr: u32, data: &[u8]) -> Result<(), ()>;
    /// 擦除扇区 (4KB)
    fn flash_erase_sector(addr: u32) -> Result<(), ()>;
    /// 获取当前运行的 Bank
    fn get_active_bank() -> u8;
    /// 设置启动 Bank (写入标志位给 Bootloader)
    fn set_boot_bank(bank: u8) -> Result<(), ()>;
    /// 系统软复位
    fn system_reset() -> !;
}

/// OTA 管理器
pub struct OtaManager<F: OtaFlash> {
    _flash: core::marker::PhantomData<F>,
    state: OtaState,
    received_bytes: u32,
    upgrade_info: UpgradeInfo,
    /// 临时 CRC32 计算
    running_crc: u32,
}

impl<F: OtaFlash> OtaManager<F> {
    pub fn new() -> Self {
        Self {
            _flash: core::marker::PhantomData,
            state: OtaState::Idle,
            received_bytes: 0,
            upgrade_info: UpgradeInfo {
                active_bank: F::get_active_bank(),
                state: OtaState::Idle,
                received_bytes: 0,
                error_code: 0,
                history: [UpgradeRecord {
                    from_version: FirmwareVersion::new(0, 0, 0, 0),
                    to_version: FirmwareVersion::new(0, 0, 0, 0),
                    timestamp: 0,
                    result: 0,
                    bank: 0,
                    source: 0,
                    reserved: [0; 10],
                }; 8],
            },
            running_crc: 0,
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> OtaState {
        self.state
    }

    /// 获取当前活动 Bank
    pub fn active_bank(&self) -> u8 {
        self.upgrade_info.active_bank
    }

    /// 获取当前版本
    pub fn current_version(&self) -> FirmwareVersion {
        CURRENT_VERSION
    }

    /// 获取升级历史
    pub fn upgrade_history(&self) -> &[UpgradeRecord] {
        &self.upgrade_info.history
    }

    /// 开始接收新固件
    pub fn start_receive(&mut self) -> Result<(), ()> {
        if self.state != OtaState::Idle {
            return Err(());
        }
        self.state = OtaState::Receiving;
        self.received_bytes = 0;
        self.running_crc = 0;
        Ok(())
    }

    /// 写入固件数据块 (不含 header, 从偏移 0 开始)
    pub fn write_chunk(&mut self, offset: u32, data: &[u8]) -> Result<(), ()> {
        if self.state != OtaState::Receiving {
            return Err(());
        }

        let write_addr = addr::OTA_START + offset;
        if write_addr + data.len() as u32 > addr::OTA_START + addr::OTA_SIZE {
            return Err(());
        }

        // 如果跨扇区边界且该扇区未擦除, 需要先擦除
        let sector_start = write_addr & !0x0FFF;
        let sector_end = (write_addr + data.len() as u32 - 1) & !0x0FFF;
        let mut addr = sector_start;
        while addr <= sector_end {
            F::flash_erase_sector(addr)?;
            addr += 0x1000;
        }

        F::flash_write(write_addr, data)?;

        // 更新 CRC32 (simple CRC32)
        for &byte in data {
            self.running_crc = crc32_update(self.running_crc, byte);
        }

        self.received_bytes = self.received_bytes.max(offset + data.len() as u32);
        Ok(())
    }

    /// 完成接收, 验证并安装
    pub fn finalize_and_install(&mut self, source: u8) -> Result<(), OtaState> {
        if self.state != OtaState::Receiving {
            return Err(self.state);
        }

        self.state = OtaState::Received;

        // 读取 header
        let mut header_buf = [0u8; FirmwareHeader::SIZE];
        F::flash_read(addr::OTA_START, &mut header_buf)?;
        let header: FirmwareHeader = unsafe { core::ptr::read(header_buf.as_ptr() as *const _) };

        if !header.is_valid() {
            self.state = OtaState::Failed;
            self.upgrade_info.error_code = 1; // invalid header
            return Err(self.state);
        }

        // 校验 CRC32
        if self.running_crc != header.crc32 {
            self.state = OtaState::Failed;
            self.upgrade_info.error_code = 2; // CRC mismatch
            return Err(self.state);
        }

        // 校验版本号 (新版本必须 >= 当前版本)
        if header.version.as_u32() < CURRENT_VERSION.as_u32() {
            self.state = OtaState::Failed;
            self.upgrade_info.error_code = 3; // downgrade not allowed
            return Err(self.state);
        }

        self.state = OtaState::Verified;

        // 确定目标 Bank
        let target_bank = if self.upgrade_info.active_bank == 1 { 2 } else { 1 };
        let target_addr = if target_bank == 1 { addr::APP1_START } else { addr::APP2_START };
        let target_size = if target_bank == 1 { addr::APP1_SIZE } else { addr::APP2_SIZE };

        self.state = OtaState::Installing;

        // 擦除目标 Bank
        let mut erase_addr = target_addr;
        while erase_addr < target_addr + target_size {
            F::flash_erase_sector(erase_addr).map_err(|_| {
                self.state = OtaState::Failed;
                self.upgrade_info.error_code = 4; // erase failed
                self.state
            })?;
            erase_addr += 0x1000;
        }

        // 拷贝固件 (跳过 header)
        let firmware_start = addr::OTA_START + FirmwareHeader::SIZE as u32;
        let firmware_end = firmware_start + header.firmware_size;
        let mut copy_addr = firmware_start;
        let mut buf = [0u8; 256];
        while copy_addr < firmware_end {
            let remaining = (firmware_end - copy_addr) as usize;
            let chunk_size = remaining.min(256);
            F::flash_read(copy_addr, &mut buf[..chunk_size])?;
            F::flash_write(target_addr + (copy_addr - firmware_start), &buf[..chunk_size]).map_err(|_| {
                self.state = OtaState::Failed;
                self.upgrade_info.error_code = 5; // write failed
                self.state
            })?;
            copy_addr += chunk_size as u32;
        }

        // 记录升级历史
        let record = UpgradeRecord {
            from_version: CURRENT_VERSION,
            to_version: header.version,
            timestamp: 0, // TODO: 从 RTC 获取
            result: 0,    // 成功
            bank: target_bank,
            source,
            reserved: [0; 10],
        };
        self.push_history(record);

        // 设置启动 Bank
        F::set_boot_bank(target_bank)?;

        self.state = OtaState::Installed;
        self.upgrade_info.active_bank = target_bank;
        Ok(self.state)
    }

    /// 回滚到另一个 Bank (当前 Bank 出问题时)
    pub fn rollback(&mut self) -> Result<(), ()> {
        let rollback_bank = if self.upgrade_info.active_bank == 1 { 2 } else { 1 };
        F::set_boot_bank(rollback_bank)?;
        self.upgrade_info.active_bank = rollback_bank;
        Ok(())
    }

    /// 获取已接收字节数
    pub fn received_bytes(&self) -> u32 {
        self.received_bytes
    }

    /// 获取升级进度百分比
    pub fn progress_percent(&self) -> u8 {
        // 需要知道总大小，从 header 获取
        // 简化：返回 0，实际由调用者计算
        0
    }

    fn push_history(&mut self, record: UpgradeRecord) {
        // 移位, 丢弃最旧的
        for i in (1..8).rev() {
            self.upgrade_info.history[i] = self.upgrade_info.history[i - 1];
        }
        self.upgrade_info.history[0] = record;
    }
}

/* ── CRC32 计算 ── */

const CRC32_TABLE: [u32; 16] = [
    0x00000000, 0x1DB71064, 0x3B6E20C8, 0x26D930AC,
    0x76DC4190, 0x6B6B51F4, 0x4DB26158, 0x5005713C,
    0xEDB88320, 0xF00F9344, 0xD6D6A3E8, 0xCB61B38C,
    0x9B64C2B0, 0x86D3CE2D, 0xA00AE278, 0xBDBDF21C,
];

fn crc32_update(crc: u32, byte: u8) -> u32 {
    let mut crc = crc ^ (byte as u32);
    let mut i = 0;
    while i < 8 {
        let idx = (crc as usize) & 0x0F;
        crc = (crc >> 4) ^ CRC32_TABLE[idx];
        i += 1;
    }
    crc
}

/// 计算缓冲区的 CRC32
pub fn crc32_calc(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc = crc32_update(crc, byte);
    }
    !crc
}
