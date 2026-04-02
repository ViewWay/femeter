/* ================================================================== */
/*                                                                    */
/*  storage.rs — 外挂 Flash 存储管理 (W25Q64)                          */
/*                                                                    */
/*  8MB Flash, 分区管理:                                                */
/*    0x000000 - 0x00FFFF: 参数区 (64KB, 系统参数/校准数据)             */
/*    0x010000 - 0x07FFFF: 电能数据区 (448KB, 每日一条记录)             */
/*    0x080000 - 0x0FFFFF: 事件日志区 (512KB)                          */
/*    0x100000 - 0x1FFFFF: 负荷曲线区 (1MB, 间隔记录)                   */
/*    0x200000 - 0x3FFFFF: OTA 升级区 (2MB)                            */
/*    0x400000 - 0x7FFFFF: 保留 (4MB)                                  */
/*                                                                    */
/*  SPI 模式: 0 (CPOL=0, CPHA=0), 最高 80MHz                          */
/*                                                                    */
/*  特性: 磨损均衡、掉电安全(CRC)、负荷曲线冻结、事件日志持久化          */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use core::ops::Range;

/* ── Flash 分区定义 ── */

/// Flash 分区描述
#[derive(Clone, Copy, Debug)]
pub struct FlashPartition {
    /// 分区名称
    pub name: &'static str,
    /// 地址范围
    pub range: Range<u32>,
    /// 扇区大小
    pub sector_size: u32,
}

/// W25Q64 分区表
pub const PARTITIONS: &[FlashPartition] = &[
    FlashPartition { name: "params",   range: 0x000000..0x010000, sector_size: 4096 },   // 64KB
    FlashPartition { name: "energy",   range: 0x010000..0x080000, sector_size: 4096 },   // 448KB
    FlashPartition { name: "events",   range: 0x080000..0x100000, sector_size: 4096 },   // 512KB
    FlashPartition { name: "load",     range: 0x100000..0x200000, sector_size: 4096 },   // 1MB
    FlashPartition { name: "ota",      range: 0x200000..0x400000, sector_size: 4096 },   // 2MB
    FlashPartition { name: "reserved", range: 0x400000..0x800000, sector_size: 4096 },   // 4MB
];

/// 按 name 查找分区
pub fn find_partition(name: &str) -> Option<&'static FlashPartition> {
    PARTITIONS.iter().find(|p| p.name == name)
}

/* ── W25Q64 SPI 指令 ── */

pub mod cmd {
    pub const WRITE_ENABLE:   u8 = 0x06;
    pub const WRITE_DISABLE:  u8 = 0x04;
    pub const READ_STATUS:    u8 = 0x05;
    pub const WRITE_STATUS:   u8 = 0x01;
    pub const READ_DATA:      u8 = 0x03;
    pub const FAST_READ:      u8 = 0x0B;
    pub const PAGE_PROGRAM:   u8 = 0x02;
    pub const SECTOR_ERASE:   u8 = 0x20;
    pub const BLOCK_ERASE_32: u8 = 0x52;
    pub const BLOCK_ERASE_64: u8 = 0xD8;
    pub const CHIP_ERASE:     u8 = 0xC7;
    pub const READ_ID:        u8 = 0x9F;
    pub const READ_UID:       u8 = 0x4B;
    pub const POWER_DOWN:     u8 = 0xB9;
    pub const RELEASE_PD:     u8 = 0xAB;
}

/* ── SPI 传输 trait ── */

/// Flash SPI 传输接口 (由 board.rs 实现)
pub trait FlashSpi {
    /// SPI 读写 (发送 tx, 同时接收 rx)
    fn spi_transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), ()>;
    /// 选中 Flash (CS 拉低)
    fn cs_low(&mut self);
    /// 释放 Flash (CS 拉高)
    fn cs_high(&mut self);
    /// 延时 (ms)
    fn delay_ms(&mut self, ms: u32);
}

/* ── 状态寄存器位 ── */

const STATUS_BUSY: u8 = 0x01;
const STATUS_WEL:  u8 = 0x02;

/* ── 存储错误 ── */

/// Flash 操作错误
#[derive(Clone, Copy, Debug)]
pub enum FlashError {
    /// SPI 通信失败
    SpiError,
    /// CRC 校验失败
    CrcError,
    /// 地址越界
    OutOfBounds,
    /// 写入空间不足
    NoSpace,
    /// 未初始化
    NotInitialized,
    /// 数据无效
    InvalidData,
}

/* ── 数据记录格式 ── */

/// 负荷曲线记录头（每条记录 8 字节头 + N 字节数据）
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct LoadProfileHeader {
    /// 时间戳（秒，从 2000-01-01 起）
    pub timestamp: u32,
    /// 记录间隔（分钟）：15 或 60
    pub interval_min: u8,
    /// 通道数
    pub channels: u8,
    /// CRC16
    pub crc: u16,
}

/// 事件日志 Flash 记录头（8 字节）
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct EventFlashHeader {
    /// 魔数 "EVT1"
    pub magic: u32,
    /// 循环写入位置
    pub write_pos: u32,
}

/// 电能冻结记录
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct EnergyFreezeRecord {
    /// 冻结时间戳
    pub timestamp: u32,
    /// 正向有功电能 (0.01 kWh)
    pub active_import: u64,
    /// 反向有功电能 (0.01 kWh)
    pub active_export: u64,
    /// 正向无功电能 (0.01 kvarh)
    pub reactive_import: u64,
    /// 反向无功电能 (0.01 kvarh)
    pub reactive_export: u64,
    /// A 相正向有功
    pub active_import_a: u64,
    /// B 相正向有功
    pub active_import_b: u64,
    /// C 相正向有功
    pub active_import_c: u64,
    /// 最大需量 (W)
    pub max_demand: u32,
    /// CRC32
    pub crc: u32,
}

/// 系统参数存储记录
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ParamRecord {
    /// 参数类型标识
    pub param_type: u16,
    /// 参数版本
    pub version: u16,
    /// 参数数据 (最大 4088 字节，对齐到 4KB 扇区减去头和 CRC)
    pub data: [u8; 4088],
    /// CRC32
    pub crc: u32,
}

/* ── W25Q64 驱动 ── */

/// W25Q64 SPI Flash 驱动
///
/// 提供 Flash 底层读写、擦除操作，以及高层分区管理、
/// 磨损均衡、CRC 校验等功能。
pub struct W25Q64<SPI: FlashSpi> {
    spi: SPI,
    /// 是否已初始化
    initialized: bool,
}

impl<SPI: FlashSpi> W25Q64<SPI> {
    /// 创建 W25Q64 驱动实例
    pub fn new(spi: SPI) -> Self {
        Self { spi, initialized: false }
    }

    /// 初始化: 读 JEDEC ID 验证
    ///
    /// 返回 JEDEC ID，成功时 `initialized` 标志置位。
    pub fn init(&mut self) -> Result<u32, FlashError> {
        let id = self.read_jedec_id().map_err(|_| FlashError::SpiError)?;
        if (id >> 16) == 0 {
            return Err(FlashError::NotInitialized);
        }
        self.initialized = true;
        Ok(id)
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 读取 JEDEC ID (3 bytes)
    pub fn read_jedec_id(&mut self) -> Result<u32, ()> {
        let mut rx = [0u8; 4];
        let tx = [cmd::READ_ID, 0, 0, 0];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut rx)?;
        self.spi.cs_high();
        Ok((rx[1] as u32) << 16 | (rx[2] as u32) << 8 | rx[3] as u32)
    }

    /// 读状态寄存器
    fn read_status(&mut self) -> Result<u8, ()> {
        let mut rx = [0u8; 2];
        let tx = [cmd::READ_STATUS, 0];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut rx)?;
        self.spi.cs_high();
        Ok(rx[1])
    }

    /// 等待写入完成
    fn wait_busy(&mut self) {
        while self.read_status().map_or(true, |s| s & STATUS_BUSY != 0) {
            self.spi.delay_ms(1);
        }
    }

    /// 写使能
    fn write_enable(&mut self) -> Result<(), ()> {
        self.wait_busy();
        let tx = [cmd::WRITE_ENABLE];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 1])?;
        self.spi.cs_high();
        Ok(())
    }

    /// 读数据
    pub fn read(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), ()> {
        let mut tx = [0u8; 5];
        tx[0] = cmd::READ_DATA;
        tx[1] = (addr >> 16) as u8;
        tx[2] = (addr >> 8) as u8;
        tx[3] = addr as u8;

        self.spi.cs_low();
        self.spi.spi_transfer(&tx[..4], &mut [0; 4])?;
        self.spi.spi_transfer(&[0; buf.len()], buf)?;
        self.spi.cs_high();
        Ok(())
    }

    /// 写数据 (page program, 最大 256 bytes per op)
    pub fn write(&mut self, addr: u32, data: &[u8]) -> Result<(), ()> {
        let mut offset = 0usize;
        while offset < data.len() {
            let chunk = &data[offset..core::cmp::min(offset + 256, data.len())];
            let page_offset = (addr as usize + offset) % 256;

            if page_offset + chunk.len() > 256 {
                let first_len = 256 - page_offset;
                self.write_page(addr + offset as u32, &chunk[..first_len])?;
                offset += first_len;
            } else {
                self.write_page(addr + offset as u32, chunk)?;
                offset += chunk.len();
            }
        }
        Ok(())
    }

    fn write_page(&mut self, addr: u32, data: &[u8]) -> Result<(), ()> {
        self.write_enable()?;
        let mut tx = [0u8; 4];
        tx[0] = cmd::PAGE_PROGRAM;
        tx[1] = (addr >> 16) as u8;
        tx[2] = (addr >> 8) as u8;
        tx[3] = addr as u8;

        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 4])?;
        self.spi.spi_transfer(data, &mut [0; data.len()])?;
        self.spi.cs_high();
        self.wait_busy();
        Ok(())
    }

    /// 扇区擦除 (4KB)
    pub fn sector_erase(&mut self, addr: u32) -> Result<(), ()> {
        self.write_enable()?;
        let mut tx = [0u8; 4];
        tx[0] = cmd::SECTOR_ERASE;
        tx[1] = (addr >> 16) as u8;
        tx[2] = (addr >> 8) as u8;
        tx[3] = addr as u8;
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 4])?;
        self.spi.cs_high();
        self.wait_busy();
        Ok(())
    }

    /// 进入深度功耗模式
    pub fn power_down(&mut self) -> Result<(), ()> {
        let tx = [cmd::POWER_DOWN];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 1])?;
        self.spi.cs_high();
        Ok(())
    }

    /// 唤醒
    pub fn wake_up(&mut self) -> Result<(), ()> {
        let tx = [cmd::RELEASE_PD];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 1])?;
        self.spi.cs_high();
        self.spi.delay_ms(3);
        Ok(())
    }

    /// 擦除 + 写入 (自动处理扇区擦除)
    pub fn erase_write(&mut self, addr: u32, data: &[u8]) -> Result<(), ()> {
        let sector_start = addr & !0x0FFF;
        self.sector_erase(sector_start)?;
        self.write(addr, data)
    }
}

/* ================================================================== */
/*  分区存储管理器                                                     */
/* ================================================================== */

/// 分区存储管理器
///
/// 提供分区级别的存储管理，包括：
/// - 循环写入（磨损均衡）
/// - CRC 校验（掉电安全）
/// - 负荷曲线存储
/// - 事件日志存储
/// - 电能冻结存储
pub struct PartitionStorage<SPI: FlashSpi> {
    flash: W25Q64<SPI>,
}

impl<SPI: FlashSpi> PartitionStorage<SPI> {
    /// 创建分区存储管理器
    pub fn new(flash: W25Q64<SPI>) -> Self {
        Self { flash }
    }

    /// 获取底层 Flash 驱动的可变引用
    pub fn flash_mut(&mut self) -> &mut W25Q64<SPI> {
        &mut self.flash
    }

    /// 初始化并验证 Flash
    pub fn init(&mut self) -> Result<u32, FlashError> {
        self.flash.init()
    }

    /// 读取分区数据
    ///
    /// `partition_name`: 分区名称
    /// `offset`: 分区内偏移
    /// `buf`: 读取缓冲区
    pub fn read_partition(
        &mut self,
        partition_name: &str,
        offset: u32,
        buf: &mut [u8],
    ) -> Result<(), FlashError> {
        let part = find_partition(partition_name).ok_or(FlashError::OutOfBounds)?;
        let addr = part.range.start + offset;
        if addr + buf.len() as u32 > part.range.end {
            return Err(FlashError::OutOfBounds);
        }
        self.flash.read(addr, buf).map_err(|_| FlashError::SpiError)
    }

    /// 写入分区数据（带 CRC 校验）
    ///
    /// 自动处理跨扇区擦除，写入后附加 CRC32 尾部。
    /// `data`: 要写入的数据
    /// `partition_name`: 分区名称
    /// `offset`: 分区内偏移
    /// 返回 CRC32 值。
    pub fn write_with_crc(
        &mut self,
        partition_name: &str,
        offset: u32,
        data: &[u8],
    ) -> Result<u32, FlashError> {
        let part = find_partition(partition_name).ok_or(FlashError::OutOfBounds)?;
        let addr = part.range.start + offset;
        let total_len = data.len() + 4; // data + CRC32

        if addr + total_len as u32 > part.range.end {
            return Err(FlashError::NoSpace);
        }

        let crc = crc32_calc(data);

        // 构建写入缓冲区：data + CRC32
        let mut write_buf = [0u8; 4096];
        if data.len() + 4 > write_buf.len() {
            return Err(FlashError::NoSpace);
        }
        write_buf[..data.len()].copy_from_slice(data);
        write_buf[data.len()..data.len() + 4].copy_from_slice(&crc.to_le_bytes());

        // 擦除涉及的扇区
        let start_sector = (addr & !0x0FFF);
        let end_sector = (addr + total_len as u32 - 1) & !0x0FFF;
        let mut sector = start_sector;
        while sector <= end_sector {
            self.flash.sector_erase(sector).map_err(|_| FlashError::SpiError)?;
            sector += 4096;
        }

        // 写入数据 + CRC
        self.flash.write(addr, &write_buf[..total_len])
            .map_err(|_| FlashError::SpiError)?;

        // 读回验证 CRC
        let mut verify_buf = [0u8; 4];
        self.flash.read(addr + data.len() as u32, &mut verify_buf)
            .map_err(|_| FlashError::SpiError)?;
        let stored_crc = u32::from_le_bytes(verify_buf);
        if stored_crc != crc {
            return Err(FlashError::CrcError);
        }

        Ok(crc)
    }

    /// 读取并验证 CRC
    ///
    /// `partition_name`: 分区名称
    /// `offset`: 分区内偏移
    /// `data_len`: 数据长度（不含 CRC）
    /// `buf`: 输出缓冲区（至少 data_len 字节）
    /// 返回 true 表示 CRC 校验通过。
    pub fn read_verify_crc(
        &mut self,
        partition_name: &str,
        offset: u32,
        data_len: usize,
        buf: &mut [u8],
    ) -> Result<bool, FlashError> {
        if buf.len() < data_len {
            return Err(FlashError::OutOfBounds);
        }

        // 读取数据 + CRC
        let mut full_buf = [0u8; 4096];
        let total_len = data_len + 4;
        if total_len > full_buf.len() {
            return Err(FlashError::OutOfBounds);
        }

        self.read_partition(partition_name, offset, &mut full_buf[..total_len])?;

        let stored_crc = u32::from_le_bytes(full_buf[data_len..total_len]);
        let calc_crc = crc32_calc(&full_buf[..data_len]);

        buf[..data_len].copy_from_slice(&full_buf[..data_len]);
        Ok(stored_crc == calc_crc)
    }

    /* ── 循环写入（磨损均衡） ── */

    /// 循环写入一条记录到指定分区
    ///
    /// 自动在分区内循环写入，实现简单的磨损均衡。
    /// `header_pos`: 当前写入位置（从 Flash 读取的），更新后返回新位置。
    /// `record`: 要写入的记录数据。
    pub fn circular_write(
        &mut self,
        partition_name: &str,
        header_pos: &mut u32,
        record: &[u8],
    ) -> Result<u32, FlashError> {
        let part = find_partition(partition_name).ok_or(FlashError::OutOfBounds)?;
        let record_size = record.len() as u32;
        let partition_size = part.range.end - part.range.start;

        // 对齐到记录大小
        let max_records = partition_size / record_size;

        // 计算写入地址
        let pos = *header_pos % max_records;
        let addr = part.range.start + pos * record_size;

        // 擦除目标扇区（如果需要）
        let sector_start = addr & !0x0FFF;
        let sector_end = (addr + record_size - 1) & !0x0FFF;

        // 检查是否需要擦除（读取第一个字节，如果不是 0xFF 则已写入）
        let mut first_byte = [0u8; 1];
        let _ = self.flash.read(addr, &mut first_byte);
        if first_byte[0] != 0xFF {
            // 需要擦除扇区
            let mut sector = sector_start;
            while sector <= sector_end {
                self.flash.sector_erase(sector)
                    .map_err(|_| FlashError::SpiError)?;
                sector += 4096;
            }
        }

        // 写入记录
        self.flash.write(addr, record)
            .map_err(|_| FlashError::SpiError)?;

        // 更新位置
        *header_pos = (*header_pos + 1) % max_records;

        Ok(*header_pos)
    }

    /// 从循环分区读取最新 N 条记录
    ///
    /// `partition_name`: 分区名称
    /// `header_pos`: 当前写入位置
    /// `record_size`: 每条记录大小
    /// `count`: 要读取的记录数
    /// `buf`: 输出缓冲区
    pub fn circular_read_latest(
        &mut self,
        partition_name: &str,
        header_pos: u32,
        record_size: u32,
        count: u32,
        buf: &mut [u8],
    ) -> Result<u32, FlashError> {
        let part = find_partition(partition_name).ok_or(FlashError::OutOfBounds)?;
        let max_records = (part.range.end - part.range.start) / record_size;
        let count = count.min(max_records).min(buf.len() as u32 / record_size);
        if count == 0 { return Ok(0); }

        let mut read_count = 0u32;
        let mut pos = (header_pos + max_records - 1) % max_records;

        for _ in 0..count {
            let addr = part.range.start + pos * record_size;
            let offset = (read_count * record_size) as usize;
            self.flash.read(addr, &mut buf[offset..offset + record_size as usize])
                .map_err(|_| FlashError::SpiError)?;

            // 检查是否为空记录（全 0xFF）
            let is_empty = buf[offset..offset + record_size as usize]
                .iter().all(|&b| b == 0xFF);
            if is_empty { break; }

            read_count += 1;
            pos = (pos + max_records - 1) % max_records;
        }

        // 反转顺序（最新的在前）
        let records = read_count as usize;
        for i in 0..records / 2 {
            let a = i * record_size as usize;
            let b = (records - 1 - i) * record_size as usize;
            buf.swap(a..a + record_size as usize, b..b + record_size as usize);
        }

        Ok(read_count)
    }

    /* ── 负荷曲线存储 ── */

    /// 写入一条负荷曲线记录
    ///
    /// `header`: 负荷曲线记录头
    /// `data`: 负荷数据（电压/电流/功率等）
    pub fn write_load_profile(
        &mut self,
        header: &LoadProfileHeader,
        data: &[u8],
    ) -> Result<(), FlashError> {
        let record_size = core::mem::size_of::<LoadProfileHeader>() + data.len();
        let mut buf = [0u8; 512];
        if record_size > buf.len() {
            return Err(FlashError::NoSpace);
        }

        // 构建记录
        buf[..8].copy_from_slice(&[
            (header.timestamp & 0xFF) as u8,
            ((header.timestamp >> 8) & 0xFF) as u8,
            ((header.timestamp >> 16) & 0xFF) as u8,
            ((header.timestamp >> 24) & 0xFF) as u8,
            header.interval_min,
            header.channels,
            0, 0, // CRC 占位
        ]);
        buf[8..8 + data.len()].copy_from_slice(data);

        // 计算 CRC（不含 CRC 字段本身）
        let crc = crc16_calc(&buf[..6]);
        buf[6] = (crc & 0xFF) as u8;
        buf[7] = ((crc >> 8) & 0xFF) as u8;

        // 写入到 load 分区
        self.read_partition("load", 0, &mut [0u8; 2])?;
        // TODO: 实现基于时间戳的寻址写入
        self.write_with_crc("load", 0, &buf[..record_size])?;

        Ok(())
    }

    /* ── 电能冻结存储 ── */

    /// 写入电能冻结记录（每日结算）
    pub fn write_energy_freeze(
        &mut self,
        record: &EnergyFreezeRecord,
    ) -> Result<(), FlashError> {
        let bytes: [u8; core::mem::size_of::<EnergyFreezeRecord>()] =
            unsafe { core::mem::transmute_copy(record) };

        // 附加 CRC
        let crc = crc32_calc(&bytes[..bytes.len() - 4]);

        self.write_with_crc("energy", 0, &bytes[..bytes.len() - 4])?;

        // 写入 CRC
        self.read_partition("energy", 0, &mut [])?;
        defmt::info!("电能冻结已存储, CRC={:08x}", crc);

        Ok(())
    }

    /* ── 事件日志存储 ── */

    /// 写入事件日志记录到 Flash
    pub fn write_event_log(
        &mut self,
        write_pos: &mut u32,
        event_bytes: &[u8],
    ) -> Result<(), FlashError> {
        self.circular_write("events", write_pos, event_bytes)
    }

    /// 读取事件日志记录
    pub fn read_event_log(
        &mut self,
        write_pos: u32,
        record_size: u32,
        count: u32,
        buf: &mut [u8],
    ) -> Result<u32, FlashError> {
        self.circular_read_latest("events", write_pos, record_size, count, buf)
    }
}

/* ================================================================== */
/*  CRC 计算                                                           */
/* ================================================================== */

/// CRC32 计算（多项式 0xEDB88320）
pub fn crc32_calc(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// CRC16-CCITT 计算（多项式 0x1021）
pub fn crc16_calc(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    !crc
}
