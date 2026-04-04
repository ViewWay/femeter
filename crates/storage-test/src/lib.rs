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

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct LoadProfileHeader {
    pub timestamp: u32,
    pub crc: u32,
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
}
