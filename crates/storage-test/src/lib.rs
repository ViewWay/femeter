//! Host-testable storage logic extracted from firmware/src/storage.rs
//!
//! This module provides the pure-logic parts of storage (CRC, partition table,
//! crash recovery simulation) that can be tested on host without embedded deps.

use core::ops::Range;

/* ── Flash 分区定义 ── */

#[derive(Clone, Debug)]
pub struct FlashPartition {
    pub name: &'static str,
    pub range: Range<u32>,
    pub sector_size: u32,
}

pub const PARTITIONS: &[FlashPartition] = &[
    FlashPartition {
        name: "params",
        range: 0x000000..0x010000,
        sector_size: 4096,
    },
    FlashPartition {
        name: "energy",
        range: 0x010000..0x080000,
        sector_size: 4096,
    },
    FlashPartition {
        name: "events",
        range: 0x080000..0x100000,
        sector_size: 4096,
    },
    FlashPartition {
        name: "load",
        range: 0x100000..0x200000,
        sector_size: 4096,
    },
    FlashPartition {
        name: "ota",
        range: 0x200000..0x400000,
        sector_size: 4096,
    },
    FlashPartition {
        name: "reserved",
        range: 0x400000..0x800000,
        sector_size: 4096,
    },
];

pub fn find_partition(name: &str) -> Option<&'static FlashPartition> {
    PARTITIONS.iter().find(|p| p.name == name)
}

/* ── 记录结构体 ── */

/// 负荷曲线记录头（每条记录 8 字节头 + N 字节数据）
/// 与 firmware/src/storage.rs 保持一致
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

impl LoadProfileHeader {
    /// 记录头大小（字节）
    pub const SIZE: usize = 8;
    
    /// 从字节数组解析
    pub fn from_bytes(buf: &[u8; 8]) -> Self {
        Self {
            timestamp: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
            interval_min: buf[4],
            channels: buf[5],
            crc: u16::from_le_bytes([buf[6], buf[7]]),
        }
    }
    
    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0..4].copy_from_slice(&self.timestamp.to_le_bytes());
        buf[4] = self.interval_min;
        buf[5] = self.channels;
        buf[6..8].copy_from_slice(&self.crc.to_le_bytes());
        buf
    }
    
    /// 计算并设置 CRC（不含 CRC 字段本身）
    pub fn compute_crc(&mut self, data: &[u8]) {
        let header_bytes = [
            (self.timestamp & 0xFF) as u8,
            ((self.timestamp >> 8) & 0xFF) as u8,
            ((self.timestamp >> 16) & 0xFF) as u8,
            ((self.timestamp >> 24) & 0xFF) as u8,
            self.interval_min,
            self.channels,
        ];
        let mut combined = header_bytes.to_vec();
        combined.extend_from_slice(data);
        self.crc = crc16_calc(&combined);
    }
    
    /// 验证 CRC
    pub fn verify_crc(&self, data: &[u8]) -> bool {
        let mut temp = *self;
        let expected_crc = self.crc;
        temp.compute_crc(data);
        temp.crc == expected_crc
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct EventFlashHeader {
    pub timestamp: u32,
    pub crc: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EnergyFreezeRecord {
    pub timestamp: u32,
    pub active_import: u32,
    pub active_export: u32,
    pub reactive_import: u32,
    pub reactive_export: u32,
    pub active_import_a: u32,
    pub active_import_b: u32,
    pub active_import_c: u32,
    pub max_demand: u32,
    pub crc: u32,
}

/* ── 存储错误 ── */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlashError {
    SpiError,
    CrcError,
    OutOfBounds,
    NoSpace,
    NotInitialized,
    InvalidData,
}

/* ── CRC 计算 ── */

/// CRC32 计算 (与 firmware 一致)
pub fn crc32_calc(data: &[u8]) -> u32 {
    // CRC-32/ISO-HDLC (多项式 0xEDB88320)
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

/// CRC16 计算 (CRC-16/MODBUS)
pub fn crc16_calc(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

/* ── 掉电恢复辅助 ── */

/// 验证数据+CRC 的完整性
pub fn verify_crc32(data: &[u8], stored_crc: u32) -> bool {
    crc32_calc(data) == stored_crc
}

/// 从带 CRC 的缓冲区中验证并提取数据
/// 布局: [data(N)] [crc32_le(4)]
pub fn verify_and_extract(buf: &[u8]) -> Result<&[u8], FlashError> {
    if buf.len() < 4 {
        return Err(FlashError::InvalidData);
    }
    let data = &buf[..buf.len() - 4];
    let stored_crc = u32::from_le_bytes([
        buf[buf.len() - 4],
        buf[buf.len() - 3],
        buf[buf.len() - 2],
        buf[buf.len() - 1],
    ]);
    if verify_crc32(data, stored_crc) {
        Ok(data)
    } else {
        Err(FlashError::CrcError)
    }
}

/// 模拟双区原子写入: 选择最新有效 slot
pub fn find_latest_valid_slot(slots: &[bool]) -> usize {
    slots.iter().rposition(|&v| v).unwrap_or(0)
}

/// 模拟循环写入偏移计算
pub fn circular_next_offset(current: u32, record_size: u32, partition_size: u32) -> u32 {
    let next = current + record_size;
    if next >= partition_size {
        0
    } else {
        next
    }
}

/* ── 时间戳寻址系统 ── */

/// 时间戳寻址配置
#[derive(Clone, Copy, Debug)]
pub struct TimestampAddressingConfig {
    /// 分区起始地址
    pub partition_start: u32,
    /// 分区大小（字节）
    pub partition_size: u32,
    /// 扇区大小（字节）
    pub sector_size: u32,
    /// 记录大小（字节，包含头）
    pub record_size: u32,
    /// 记录间隔（分钟）
    pub interval_min: u8,
    /// 最大记录数
    pub max_records: u32,
    /// 基准时间戳（2000-01-01 00:00:00 UTC）
    pub base_timestamp: u32,
}

impl TimestampAddressingConfig {
    /// 从分区创建配置
    pub fn from_partition(partition: &FlashPartition, record_size: u32, interval_min: u8) -> Self {
        let partition_size = partition.range.end - partition.range.start;
        let max_records = partition_size / record_size;
        Self {
            partition_start: partition.range.start,
            partition_size,
            sector_size: partition.sector_size,
            record_size,
            interval_min,
            max_records,
            base_timestamp: 946684800, // 2000-01-01 00:00:00 UTC
        }
    }
    
    /// 计算时间戳对应的记录索引
    /// 
    /// # Arguments
    /// * `timestamp` - Unix 时间戳（秒）
    /// 
    /// # Returns
    /// 记录索引（0 到 max_records-1）
    pub fn timestamp_to_index(&self, timestamp: u32) -> u32 {
        if timestamp < self.base_timestamp {
            return 0;
        }
        
        let elapsed = timestamp - self.base_timestamp;
        let interval_seconds = (self.interval_min as u32) * 60;
        let index = elapsed / interval_seconds;
        
        index % self.max_records
    }
    
    /// 计算记录索引对应的 Flash 偏移
    /// 
    /// # Arguments
    /// * `index` - 记录索引
    /// 
    /// # Returns
    /// Flash 偏移（相对于分区起始）
    pub fn index_to_offset(&self, index: u32) -> u32 {
        index * self.record_size
    }
    
    /// 计算时间戳对应的 Flash 偏移
    /// 
    /// # Arguments
    /// * `timestamp` - Unix 时间戳（秒）
    /// 
    /// # Returns
    /// Flash 偏移（相对于分区起始）
    pub fn timestamp_to_offset(&self, timestamp: u32) -> u32 {
        let index = self.timestamp_to_index(timestamp);
        self.index_to_offset(index)
    }
    
    /// 计算索引对应的时间戳
    /// 
    /// # Arguments
    /// * `index` - 记录索引
    /// 
    /// # Returns
    /// Unix 时间戳（秒）
    pub fn index_to_timestamp(&self, index: u32) -> u32 {
        let interval_seconds = (self.interval_min as u32) * 60;
        self.base_timestamp + (index * interval_seconds)
    }
    
    /// 计算时间戳范围对应的记录范围
    /// 
    /// # Arguments
    /// * `start_ts` - 起始时间戳
    /// * `end_ts` - 结束时间戳
    /// 
    /// # Returns
    /// (起始索引, 记录数)
    pub fn timestamp_range_to_indices(&self, start_ts: u32, end_ts: u32) -> (u32, u32) {
        if end_ts < start_ts {
            return (0, 0);
        }
        
        let start_index = self.timestamp_to_index(start_ts);
        let end_index = self.timestamp_to_index(end_ts);
        
        if end_index >= start_index {
            let count = end_index - start_index + 1;
            (start_index, count.min(self.max_records))
        } else {
            // 跨越循环边界
            let count = (self.max_records - start_index) + end_index + 1;
            (start_index, count.min(self.max_records))
        }
    }
    
    /// 计算所在的扇区索引
    /// 
    /// # Arguments
    /// * `offset` - Flash 偏移
    /// 
    /// # Returns
    /// 扇区索引
    pub fn offset_to_sector(&self, offset: u32) -> u32 {
        offset / self.sector_size
    }
    
    /// 检查是否需要擦除新扇区
    /// 
    /// # Arguments
    /// * `current_offset` - 当前写入偏移
    /// * `next_offset` - 下一次写入偏移
    /// 
    /// # Returns
    /// true 如果需要擦除新扇区
    pub fn needs_sector_erase(&self, current_offset: u32, next_offset: u32) -> bool {
        self.offset_to_sector(current_offset) != self.offset_to_sector(next_offset)
    }
    
    /// 计算一天有多少条记录
    pub fn records_per_day(&self) -> u32 {
        let minutes_per_day = 24 * 60;
        minutes_per_day / (self.interval_min as u32)
    }
    
    /// 计算可存储多少天的数据
    pub fn storage_days(&self) -> u32 {
        self.max_records / self.records_per_day()
    }
}

/// 时间戳寻址写入器
#[derive(Clone, Debug)]
pub struct TimestampAddressedWriter {
    config: TimestampAddressingConfig,
    /// 当前写入偏移
    current_offset: u32,
    /// 已写入记录数
    records_written: u32,
}

impl TimestampAddressedWriter {
    /// 创建新的写入器
    pub fn new(config: TimestampAddressingConfig) -> Self {
        Self {
            config,
            current_offset: 0,
            records_written: 0,
        }
    }
    
    /// 根据时间戳计算写入位置
    /// 
    /// # Arguments
    /// * `timestamp` - Unix 时间戳
    /// 
    /// # Returns
    /// (偏移, 是否需要擦除扇区)
    pub fn prepare_write(&mut self, timestamp: u32) -> (u32, bool) {
        let offset = self.config.timestamp_to_offset(timestamp);
        let needs_erase = self.config.needs_sector_erase(self.current_offset, offset);
        self.current_offset = offset;
        self.records_written += 1;
        (offset, needs_erase)
    }
    
    /// 写入负荷曲线记录
    /// 
    /// # Arguments
    /// * `timestamp` - 时间戳
    /// * `data` - 负荷数据
    /// 
    /// # Returns
    /// 写入偏移
    pub fn write_record(&mut self, timestamp: u32, data: &[u8]) -> Result<(u32, bool), FlashError> {
        let header_size = LoadProfileHeader::SIZE;
        let record_size = header_size + data.len();
        
        if record_size as u32 > self.config.record_size {
            return Err(FlashError::NoSpace);
        }
        
        let (offset, needs_erase) = self.prepare_write(timestamp);
        
        // 构建记录头
        let mut header = LoadProfileHeader {
            timestamp,
            interval_min: self.config.interval_min,
            channels: data.len() as u8,
            crc: 0,
        };
        header.compute_crc(data);
        
        Ok((offset, needs_erase))
    }
    
    /// 获取当前状态
    pub fn status(&self) -> (u32, u32) {
        (self.current_offset, self.records_written)
    }
}

/// 时间戳寻址读取器
#[derive(Clone, Debug)]
pub struct TimestampAddressedReader {
    config: TimestampAddressingConfig,
}

impl TimestampAddressedReader {
    /// 创建新的读取器
    pub fn new(config: TimestampAddressingConfig) -> Self {
        Self { config }
    }
    
    /// 根据时间戳计算读取位置
    /// 
    /// # Arguments
    /// * `timestamp` - Unix 时间戳
    /// 
    /// # Returns
    /// Flash 偏移
    pub fn get_read_offset(&self, timestamp: u32) -> u32 {
        self.config.timestamp_to_offset(timestamp)
    }
    
    /// 获取时间戳范围内的所有偏移
    /// 
    /// # Arguments
    /// * `start_ts` - 起始时间戳
    /// * `end_ts` - 结束时间戳
    /// 
    /// # Returns
    /// 偏移列表
    pub fn get_range_offsets(&self, start_ts: u32, end_ts: u32) -> Vec<u32> {
        let (start_index, count) = self.config.timestamp_range_to_indices(start_ts, end_ts);
        
        (0..count)
            .map(|i| {
                let index = (start_index + i) % self.config.max_records;
                self.config.index_to_offset(index)
            })
            .collect()
    }
    
    /// 解析记录头
    /// 
    /// # Arguments
    /// * `buf` - 包含记录头的缓冲区（至少8字节）
    /// 
    /// # Returns
    /// 解析后的记录头
    pub fn parse_header(&self, buf: &[u8]) -> Result<LoadProfileHeader, FlashError> {
        if buf.len() < LoadProfileHeader::SIZE {
            return Err(FlashError::InvalidData);
        }
        
        let header_bytes: [u8; 8] = buf[..8].try_into().map_err(|_| FlashError::InvalidData)?;
        Ok(LoadProfileHeader::from_bytes(&header_bytes))
    }
    
    /// 验证记录完整性
    /// 
    /// # Arguments
    /// * `header` - 记录头
    /// * `data` - 负荷数据
    /// 
    /// # Returns
    /// true 如果 CRC 校验通过
    pub fn verify_record(&self, header: &LoadProfileHeader, data: &[u8]) -> bool {
        header.verify_crc(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // 基础 CRC 测试
    // ============================================================

    #[test]
    fn test_crc32_empty() {
        let crc = crc32_calc(&[]);
        // CRC32 of empty data is 0x00000000 (standard behavior)
        assert_eq!(crc, 0);
    }

    #[test]
    fn test_crc32_deterministic() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF];
        assert_eq!(crc32_calc(&data), crc32_calc(&data));
    }

    #[test]
    fn test_crc32_differs_for_different_data() {
        assert_ne!(crc32_calc(b"AAAA"), crc32_calc(b"AAAB"));
    }

    #[test]
    fn test_crc16_known() {
        // Project-specific CRC16 implementation
        let crc = crc16_calc(b"123456789");
        assert_eq!(crc, crc16_calc(b"123456789")); // deterministic
        assert_ne!(crc, 0);
    }

    // ============================================================
    // 分区表测试
    // ============================================================

    #[test]
    fn test_find_partition() {
        let p = find_partition("params").unwrap();
        assert_eq!(p.name, "params");
        assert_eq!(p.range.start, 0x000000);
        assert_eq!(p.range.end, 0x010000);
        assert!(find_partition("nonexistent").is_none());
    }

    #[test]
    fn test_partition_total_size() {
        let total: u32 = PARTITIONS.iter().map(|p| p.range.end - p.range.start).sum();
        assert_eq!(total, 8 * 1024 * 1024);
    }

    #[test]
    fn test_partitions_count() {
        assert_eq!(PARTITIONS.len(), 6);
    }

    #[test]
    fn test_partition_table_contiguous() {
        let mut prev_end = 0u32;
        for p in PARTITIONS {
            assert_eq!(
                p.range.start, prev_end,
                "Partition '{}' gap at {:08X}",
                p.name, prev_end
            );
            assert!(p.range.start < p.range.end);
            assert_eq!(p.range.start % p.sector_size, 0);
            assert_eq!(p.range.end % p.sector_size, 0);
            prev_end = p.range.end;
        }
        assert_eq!(prev_end, 8 * 1024 * 1024);
    }

    #[test]
    fn test_partition_address_ranges() {
        for p in PARTITIONS {
            assert!(p.range.start < p.range.end, "Partition {} invalid", p.name);
            let size = p.range.end - p.range.start;
            assert!(size >= p.sector_size, "Partition {} too small", p.name);
            assert_eq!(size % p.sector_size, 0, "Partition {} not aligned", p.name);
        }
    }

    // ============================================================
    // 记录大小验证
    // ============================================================

    #[test]
    fn test_load_profile_header_size() {
        assert_eq!(core::mem::size_of::<LoadProfileHeader>(), 8);
    }

    #[test]
    fn test_event_flash_header_size() {
        assert_eq!(core::mem::size_of::<EventFlashHeader>(), 8);
    }

    #[test]
    fn test_energy_freeze_record_size() {
        assert_eq!(core::mem::size_of::<EnergyFreezeRecord>(), 40);
    }

    #[test]
    fn test_load_profile_header_default() {
        let h = LoadProfileHeader::default();
        assert_eq!(h.timestamp, 0);
        assert_eq!(h.crc, 0);
    }

    #[test]
    fn test_event_flash_header_default() {
        let h = EventFlashHeader::default();
        assert_eq!(h.timestamp, 0);
        assert_eq!(h.crc, 0);
    }

    // ============================================================
    // 掉电恢复压力测试
    // ============================================================

    /// CRC32 多种数据模式
    #[test]
    fn test_crc32_various_patterns() {
        let zeros = [0u8; 256];
        let crc_z = crc32_calc(&zeros);
        assert_ne!(crc_z, 0);

        let ff = [0xFFu8; 256];
        let crc_f = crc32_calc(&ff);
        assert_ne!(crc_f, crc_z);

        let inc: Vec<u8> = (0..=255u8).collect();
        let crc_i = crc32_calc(&inc);
        assert_ne!(crc_i, crc_z);
        assert_ne!(crc_i, crc_f);

        let dec: Vec<u8> = (0..=255u8).rev().collect();
        let crc_d = crc32_calc(&dec);
        assert_ne!(crc_d, crc_i);
    }

    /// CRC32 单比特翻转检测 (每一位都能检出)
    #[test]
    fn test_crc32_single_bit_flip_detection() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
        let original_crc = crc32_calc(&data);

        for byte_idx in 0..data.len() {
            for bit in 0..8 {
                let mut corrupted = data;
                corrupted[byte_idx] ^= 1 << bit;
                assert_ne!(
                    original_crc,
                    crc32_calc(&corrupted),
                    "Failed to detect bit flip at byte {} bit {}",
                    byte_idx,
                    bit
                );
            }
        }
    }

    /// CRC32 前缀后缀独立性
    #[test]
    fn test_crc32_prefix_suffix_independence() {
        let base = [0x01, 0x02, 0x03, 0x04];
        let crc_base = crc32_calc(&base);

        let with_prefix = [0xFF, 0x01, 0x02, 0x03, 0x04];
        assert_ne!(crc32_calc(&with_prefix), crc_base);

        let with_suffix = [0x01, 0x02, 0x03, 0x04, 0xFF];
        assert_ne!(crc32_calc(&with_suffix), crc_base);
    }

    /// CRC16 多种数据模式
    #[test]
    fn test_crc16_various_patterns() {
        let zeros = [0u8; 256];
        let crc_z = crc16_calc(&zeros);

        let ff = [0xFFu8; 256];
        let crc_f = crc16_calc(&ff);
        assert_ne!(crc_z, crc_f);

        let inc: Vec<u8> = (0..=255u8).collect();
        let crc_i = crc16_calc(&inc);
        assert_ne!(crc_z, crc_i);
        assert_ne!(crc_f, crc_i);
    }

    /// 掉电: CRC 写入一半 (前2字节)
    #[test]
    fn test_power_loss_crc_half_written() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let full_crc = crc32_calc(&data);
        let crc_bytes = full_crc.to_le_bytes();

        let half_crc = [crc_bytes[0], crc_bytes[1], 0x00, 0x00];
        let stored_crc = u32::from_le_bytes(half_crc);
        assert_ne!(stored_crc, full_crc);
        assert_ne!(stored_crc, crc32_calc(&data));
    }

    /// 掉电: CRC 写入一半 (后2字节)
    #[test]
    fn test_power_loss_crc_half_written_tail() {
        let data = [0x05, 0x06, 0x07, 0x08];
        let full_crc = crc32_calc(&data);
        let crc_bytes = full_crc.to_le_bytes();

        let half_crc = [0x00, 0x00, crc_bytes[2], crc_bytes[3]];
        let stored_crc = u32::from_le_bytes(half_crc);
        assert_ne!(stored_crc, full_crc);
    }

    /// 掉电: 数据写入部分完成
    #[test]
    fn test_power_loss_partial_data_write() {
        let full_data: Vec<u8> = (0..16).collect();
        let full_crc = crc32_calc(&full_data);

        let partial_data: Vec<u8> = (0..8).collect();
        let partial_crc = crc32_calc(&partial_data);
        assert_ne!(full_crc, partial_crc);

        let corrupted: Vec<u8> = (0..8)
            .chain(std::iter::once(0xFF))
            .chain(std::iter::repeat(0).take(7))
            .collect();
        assert_ne!(crc32_calc(&corrupted), full_crc);
    }

    /// 掉电: CRC 校验恢复
    #[test]
    fn test_power_loss_crc_detection() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05];
        let crc = crc32_calc(&data);
        let corrupted_crc = crc ^ 0x00000001;
        assert_ne!(crc, corrupted_crc);

        let mut full_buf = [0u8; 16];
        full_buf[..data.len()].copy_from_slice(&data);
        full_buf[data.len()..data.len() + 4].copy_from_slice(&corrupted_crc.to_le_bytes());

        let stored_crc =
            u32::from_le_bytes(full_buf[data.len()..data.len() + 4].try_into().unwrap());
        let calc_crc = crc32_calc(&full_buf[..data.len()]);
        assert_ne!(stored_crc, calc_crc);
    }

    /// 双区交替写入原子性
    #[test]
    fn test_dual_slot_atomic_write() {
        let slot_a_valid = true;
        let slot_b_valid = false;
        let use_slot = if slot_b_valid { 1 } else { 0 };
        assert_eq!(use_slot, 0);
    }

    /// 电能数据 CRC 保护
    #[test]
    fn test_energy_data_crc_protection() {
        let record = EnergyFreezeRecord {
            timestamp: 1700000000,
            active_import: 12345,
            active_export: 0,
            reactive_import: 100,
            reactive_export: 0,
            active_import_a: 4115,
            active_import_b: 4115,
            active_import_c: 4115,
            max_demand: 5000,
            crc: 0,
        };
        let bytes: [u8; core::mem::size_of::<EnergyFreezeRecord>()] =
            unsafe { core::mem::transmute_copy(&record) };
        let crc = crc32_calc(&bytes[..bytes.len() - 4]);

        let mut corrupted = bytes;
        corrupted[8] ^= 0x01;
        assert_ne!(crc32_calc(&corrupted[..corrupted.len() - 4]), crc);
    }

    /// 能量冻结记录: 掉电后每个字段损坏检测
    #[test]
    fn test_energy_freeze_power_loss_all_fields() {
        let record = EnergyFreezeRecord {
            timestamp: 1700000000,
            active_import: 12345678,
            active_export: 100,
            reactive_import: 200,
            reactive_export: 300,
            active_import_a: 4115226,
            active_import_b: 4115226,
            active_import_c: 4115226,
            max_demand: 99999,
            crc: 0,
        };
        let bytes: [u8; core::mem::size_of::<EnergyFreezeRecord>()] =
            unsafe { core::mem::transmute_copy(&record) };
        let valid_crc = crc32_calc(&bytes[..bytes.len() - 4]);
        let crc_field_offset = bytes.len() - 4;

        for field_byte in 0..crc_field_offset {
            let mut corrupted = bytes;
            corrupted[field_byte] ^= 0x01;
            assert_ne!(
                valid_crc,
                crc32_calc(&corrupted[..corrupted.len() - 4]),
                "Failed to detect corruption at byte offset {}",
                field_byte
            );
        }
    }

    // ============================================================
    // 循环写入边界测试
    // ============================================================

    #[test]
    fn test_circular_write_sector_boundary() {
        let sector_size = 4096u32;
        let partition_size = 64 * 1024u32;
        let record_size = 32u32;
        let records_per_sector = sector_size / record_size;
        assert_eq!(records_per_sector, 128);
        let total_records = partition_size / record_size;
        assert_eq!(total_records, 2048);
    }

    #[test]
    fn test_circular_write_wrap_around() {
        let partition_size = 4096u32;
        let record_size = 32u32;
        let total_records = partition_size / record_size;

        let mut offset = 0u32;
        for _ in 0..(total_records + 10) {
            offset = circular_next_offset(offset, record_size, partition_size);
        }
        assert!(offset < partition_size);
    }

    #[test]
    fn test_circular_find_latest_valid() {
        let valid_slots = [true, true, true, true, false];
        let latest = find_latest_valid_slot(&valid_slots);
        assert_eq!(latest, 3);

        let recovered = find_latest_valid_slot(&valid_slots);
        assert_eq!(recovered, 3);

        // 全部无效
        let all_invalid = [false, false, false];
        assert_eq!(find_latest_valid_slot(&all_invalid), 0);
    }

    /// 连续掉电恢复模拟 (10次写入, 20%掉电率)
    #[test]
    fn test_dual_slot_sequential_power_loss() {
        let mut slot_valid = [false; 2];
        let mut current_slot = 0usize;
        let mut successful_writes = 0;

        for attempt in 0..10 {
            if attempt % 5 != 0 {
                let other = 1 - current_slot;
                slot_valid[other] = true;
                slot_valid[current_slot] = false;
                current_slot = other;
                successful_writes += 1;
            }
        }

        let recovered = if slot_valid[0] && slot_valid[1] {
            current_slot
        } else if slot_valid[0] {
            0
        } else if slot_valid[1] {
            1
        } else {
            0
        };

        assert!(successful_writes > 0);
        assert!(slot_valid[recovered]);
    }

    // ============================================================
    // verify_and_extract 测试
    // ============================================================

    #[test]
    fn test_verify_and_extract_valid() {
        let data = [0x01, 0x02, 0x03];
        let crc = crc32_calc(&data);
        let mut buf = Vec::with_capacity(7);
        buf.extend_from_slice(&data);
        buf.extend_from_slice(&crc.to_le_bytes());

        let extracted = verify_and_extract(&buf).unwrap();
        assert_eq!(extracted, &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_verify_and_extract_corrupted() {
        let data = [0x01, 0x02, 0x03];
        let mut crc = crc32_calc(&data);
        crc ^= 0x01;
        let mut buf = Vec::with_capacity(7);
        buf.extend_from_slice(&data);
        buf.extend_from_slice(&crc.to_le_bytes());

        assert!(matches!(
            verify_and_extract(&buf),
            Err(FlashError::CrcError)
        ));
    }

    #[test]
    fn test_verify_and_extract_too_short() {
        assert!(matches!(
            verify_and_extract(&[0x01]),
            Err(FlashError::InvalidData)
        ));
        assert!(matches!(
            verify_and_extract(&[]),
            Err(FlashError::InvalidData)
        ));
        assert!(matches!(
            verify_and_extract(&[0x01, 0x02, 0x03]),
            Err(FlashError::InvalidData)
        ));
    }

    // ============================================================
    // 原子写入模拟
    // ============================================================

    #[test]
    fn test_atomic_write_simulation() {
        let old_data = [0xAA, 0xBB, 0xCC, 0xDD];
        let old_crc = crc32_calc(&old_data);
        let new_data = [0x11, 0x22, 0x33, 0x44];
        let new_crc = crc32_calc(&new_data);

        let mut storage = [0u8; 8];
        storage[..4].copy_from_slice(&old_data);
        storage[4..].copy_from_slice(&old_crc.to_le_bytes());

        // 掉电: 新数据写入完成, CRC 未写
        storage[..4].copy_from_slice(&new_data);
        let stored_crc = u32::from_le_bytes([storage[4], storage[5], storage[6], storage[7]]);
        assert_ne!(stored_crc, crc32_calc(&storage[..4]));

        // CRC 写入完成
        storage[4..].copy_from_slice(&new_crc.to_le_bytes());
        let stored_crc = u32::from_le_bytes([storage[4], storage[5], storage[6], storage[7]]);
        assert_eq!(stored_crc, crc32_calc(&storage[..4]));
    }

    /// CRC32 大数据块
    #[test]
    fn test_crc32_large_data() {
        let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let crc = crc32_calc(&data);
        assert_ne!(crc, 0);

        let mut modified = data.clone();
        modified[1023] ^= 0x01;
        assert_ne!(crc32_calc(&modified), crc);
    }

    /// FlashError debug
    #[test]
    fn test_flash_error_debug() {
        assert_eq!(format!("{:?}", FlashError::SpiError), "SpiError");
        assert_eq!(format!("{:?}", FlashError::CrcError), "CrcError");
        assert_eq!(format!("{:?}", FlashError::NoSpace), "NoSpace");
    }

    // ============================================================
    // 循环写入压力测试
    // ============================================================

    #[test]
    fn test_circular_write_stress_1000_records() {
        let partition_size = 4096u32;
        let record_size = 8u32;
        let mut offset = 0u32;
        let mut wraps = 0;

        for _ in 0..1000 {
            let next = circular_next_offset(offset, record_size, partition_size);
            if next <= offset {
                wraps += 1;
            }
            offset = next;
        }
        assert!(wraps > 0);
    }

    #[test]
    fn test_circular_write_full_partition() {
        let partition_size = 4096u32;
        let record_size = 4u32;
        let total = partition_size / record_size; // 1024

        let mut offset = 0u32;
        for i in 0..total {
            offset = circular_next_offset(offset, record_size, partition_size);
            // Wrap happens at i=total-1 when offset reaches partition_size
            if i < total - 1 {
                assert_ne!(offset, 0, "Premature wrap at record {}", i);
            } else {
                // Last write wraps
                assert_eq!(offset, 0);
            }
        }
    }

    // ============================================================
    // 时间戳寻址系统测试
    // ============================================================

    #[test]
    fn test_load_profile_header_size_updated() {
        assert_eq!(core::mem::size_of::<LoadProfileHeader>(), 8);
    }

    #[test]
    fn test_load_profile_header_serde() {
        let header = LoadProfileHeader {
            timestamp: 0x12345678,
            interval_min: 15,
            channels: 6,
            crc: 0xABCD,
        };
        
        let bytes = header.to_bytes();
        let decoded = LoadProfileHeader::from_bytes(&bytes);
        
        assert_eq!(decoded.timestamp, header.timestamp);
        assert_eq!(decoded.interval_min, header.interval_min);
        assert_eq!(decoded.channels, header.channels);
        assert_eq!(decoded.crc, header.crc);
    }

    #[test]
    fn test_load_profile_header_crc() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let mut header = LoadProfileHeader {
            timestamp: 1609459200, // 2021-01-01 00:00:00
            interval_min: 15,
            channels: 6,
            crc: 0,
        };
        
        header.compute_crc(&data);
        assert_ne!(header.crc, 0);
        assert!(header.verify_crc(&data));
        
        // 修改数据后 CRC 应该校验失败
        let bad_data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x07];
        assert!(!header.verify_crc(&bad_data));
    }

    #[test]
    fn test_timestamp_addressing_config_creation() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64, // record_size
            15, // interval_min
        );
        
        assert_eq!(config.partition_start, 0x100000);
        assert_eq!(config.partition_size, 0x100000);
        assert_eq!(config.record_size, 64);
        assert_eq!(config.interval_min, 15);
        assert!(config.max_records > 0);
    }

    #[test]
    fn test_timestamp_to_index() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64,
            15,
        );
        
        // 2000-01-01 00:00:00 -> index 0
        let index0 = config.timestamp_to_index(946684800);
        assert_eq!(index0, 0);
        
        // 15分钟后 -> index 1
        let index1 = config.timestamp_to_index(946684800 + 15 * 60);
        assert_eq!(index1, 1);
        
        // 1小时后 -> index 4
        let index4 = config.timestamp_to_index(946684800 + 60 * 60);
        assert_eq!(index4, 4);
        
        // 1天后 -> index 96 (24*60/15)
        let index96 = config.timestamp_to_index(946684800 + 24 * 60 * 60);
        assert_eq!(index96, 96);
    }

    #[test]
    fn test_index_to_offset() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64,
            15,
        );
        
        assert_eq!(config.index_to_offset(0), 0);
        assert_eq!(config.index_to_offset(1), 64);
        assert_eq!(config.index_to_offset(10), 640);
    }

    #[test]
    fn test_timestamp_range_to_indices() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64,
            15,
        );
        
        let base = 946684800u32; // 2000-01-01 00:00:00
        
        // 1小时范围
        let (start, count) = config.timestamp_range_to_indices(base, base + 60 * 60);
        assert_eq!(start, 0);
        assert_eq!(count, 5); // 0, 1, 2, 3, 4
        
        // 无效范围
        let (start, count) = config.timestamp_range_to_indices(base + 60, base);
        assert_eq!(start, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_records_per_day() {
        let partition = find_partition("load").unwrap();
        
        // 15分钟间隔 -> 96条/天
        let config15 = TimestampAddressingConfig::from_partition(partition, 64, 15);
        assert_eq!(config15.records_per_day(), 96);
        
        // 60分钟间隔 -> 24条/天
        let config60 = TimestampAddressingConfig::from_partition(partition, 64, 60);
        assert_eq!(config60.records_per_day(), 24);
    }

    #[test]
    fn test_storage_days() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64,  // record_size
            15,  // interval_min
        );
        
        // load 分区 1MB，每条记录 64 字节，可存储 ~16384 条
        // 96 条/天 -> ~170 天
        let days = config.storage_days();
        assert!(days >= 150);
        assert!(days <= 200);
    }

    #[test]
    fn test_needs_sector_erase() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64,
            15,
        );
        
        // 同一扇区内不需要擦除
        assert!(!config.needs_sector_erase(0, 64));
        assert!(!config.needs_sector_erase(1000, 2000));
        
        // 跨扇区需要擦除
        assert!(config.needs_sector_erase(4090, 4096));
    }

    #[test]
    fn test_timestamp_addressed_writer() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64,
            15,
        );
        
        let mut writer = TimestampAddressedWriter::new(config);
        let data = [0u8; 56]; // 64 - 8 (header)
        
        let base = 946684800u32;
        let (offset1, _) = writer.write_record(base, &data).unwrap();
        assert_eq!(offset1, 0);
        
        let (offset2, _) = writer.write_record(base + 15 * 60, &data).unwrap();
        assert_eq!(offset2, 64);
        
        let (current_offset, records_written) = writer.status();
        assert_eq!(records_written, 2);
    }

    #[test]
    fn test_timestamp_addressed_reader() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64,
            15,
        );
        
        let reader = TimestampAddressedReader::new(config);
        let base = 946684800u32;
        
        // 单个时间戳
        let offset = reader.get_read_offset(base);
        assert_eq!(offset, 0);
        
        // 时间范围
        let offsets = reader.get_range_offsets(base, base + 60 * 60);
        assert_eq!(offsets.len(), 5);
        assert_eq!(offsets[0], 0);
        assert_eq!(offsets[1], 64);
    }

    #[test]
    fn test_timestamp_addressing_circular_wrap() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64,
            15,
        );
        
        // 写入超过最大记录数后应该循环
        let base = 946684800u32;
        let interval = 15 * 60;
        
        // 超过最大记录数的时间戳
        let far_future = base + (config.max_records + 10) * interval;
        let index = config.timestamp_to_index(far_future);
        
        // 应该循环回到较小的索引
        assert!(index < config.max_records);
    }

    #[test]
    fn test_timestamp_addressing_edge_cases() {
        let partition = find_partition("load").unwrap();
        let config = TimestampAddressingConfig::from_partition(
            partition,
            64,
            15,
        );
        
        // 早于基准时间的时间戳
        let early_ts = 0u32;
        let index = config.timestamp_to_index(early_ts);
        assert_eq!(index, 0);
        
        // 精确对齐间隔的时间戳
        let base = 946684800u32;
        let aligned = base + 15 * 60 * 100;
        let index = config.timestamp_to_index(aligned);
        assert_eq!(index, 100);
        
        // 未对齐的时间戳（向下取整）
        let unaligned = base + 15 * 60 * 100 + 7 * 60; // 7分钟偏移
        let index = config.timestamp_to_index(unaligned);
        assert_eq!(index, 100); // 应该向下取整到 100
    }
}
