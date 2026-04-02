/* ================================================================== */
/*                                                                    */
/*  ota.rs — 双 Bank OTA 固件升级管理                                   */
/*                                                                    */
/*  Flash 布局 (FM33A068EV, 512KB):                                   */
/*    0x000000 - 0x000FFF: Bootloader (4KB)                           */
/*    0x001000 - 0x03FFFF: App Bank 1 (252KB)                         */
/*    0x040000 - 0x07FFFF: App Bank 2 (256KB)                         */
/*    0x080000 - 0x0FFFFF: OTA 数据区 (512KB)                         */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use core::mem::size_of;

/* ── Flash 地址定义 ── */

pub mod addr {
    pub const BOOT_START: u32 = 0x0000_0000;
    pub const BOOT_SIZE: u32 = 0x0000_1000;
    pub const APP1_START: u32 = 0x0000_1000;
    pub const APP1_SIZE: u32 = 0x0003_F000; // 252KB
    pub const APP2_START: u32 = 0x0004_0000;
    pub const APP2_SIZE: u32 = 0x0004_0000; // 256KB
    pub const OTA_START: u32 = 0x0008_0000;
    pub const OTA_SIZE: u32 = 0x0008_0000; // 512KB
    pub const APP_MAX_SIZE: u32 = APP1_SIZE;
    pub const UPGRADE_INFO_ADDR: u32 = OTA_START + OTA_SIZE - 0x1000;
}

/* ── 固件版本 ── */

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

    pub fn as_u32(&self) -> u32 {
        (self.major as u32) << 24 | (self.minor as u32) << 16
            | (self.patch as u32) << 8 | (self.build as u32)
    }

    pub fn to_str(&self) -> [u8; 16] {
        let mut s = [0u8; 16];
        s[0] = b'v';
        let v2s = |v: u8| -> [u8; 2] { [b'0' + (v / 10), b'0' + (v % 10)] };
        let mj = v2s(self.major);
        s[1] = mj[0]; s[2] = mj[1];
        s[3] = b'.';
        let mn = v2s(self.minor);
        s[4] = mn[0]; s[5] = mn[1];
        s[6] = b'.';
        let pa = v2s(self.patch);
        s[7] = pa[0]; s[8] = pa[1];
        s[9] = b'.';
        s[10] = b'0' + (self.build / 100);
        s[11] = b'0' + ((self.build / 10) % 10);
        s[12] = b'0' + (self.build % 10);
        s
    }
}

/* ── 升级状态 ── */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OtaState {
    Idle = 0,
    Receiving = 1,
    Received = 2,
    Verified = 3,
    Installing = 4,
    Installed = 5,
    Failed = 6,
}

/* ── 固件镜像头 ── */

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FirmwareHeader {
    pub magic: u32,
    pub version: FirmwareVersion,
    pub firmware_size: u32,
    pub crc32: u32,
    pub target_bank: u8,
    pub flags: u8,
    pub timestamp: u32,
    pub prev_version: FirmwareVersion,
    pub reserved: [u8; 16],
}

impl FirmwareHeader {
    pub const MAGIC: u32 = 0x464D5441;
    pub const SIZE: usize = size_of::<Self>();

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC && self.firmware_size <= addr::APP_MAX_SIZE
    }
}

/* ── 升级记录 ── */

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct UpgradeRecord {
    pub from_version: FirmwareVersion,
    pub to_version: FirmwareVersion,
    pub timestamp: u32,
    pub result: u32,
    pub bank: u8,
    pub source: u8,
    pub reserved: [u8; 10],
}

/* ── 升级信息 ── */

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct UpgradeInfo {
    pub active_bank: u8,
    pub state: OtaState,
    pub received_bytes: u32,
    pub error_code: u32,
    pub history: [UpgradeRecord; 8],
}

/* ── Flash 操作 trait ── */

pub trait OtaFlash {
    fn flash_read(addr: u32, buf: &mut [u8]) -> Result<(), ()>;
    fn flash_write(addr: u32, data: &[u8]) -> Result<(), ()>;
    fn flash_erase_sector(addr: u32) -> Result<(), ()>;
    fn get_active_bank() -> u8;
    fn set_boot_bank(bank: u8) -> Result<(), ()>;
    fn system_reset() -> !;
}

/* ── OTA 管理器 ── */

/// 占位 OTA Flash 实现 — 后续替换为实际内部 Flash 驱动
pub struct InternalFlash;
impl OtaFlash for InternalFlash {
    fn flash_read(_addr: u32, _buf: &mut [u8]) -> Result<(), ()> { Ok(()) }
    fn flash_write(_addr: u32, _data: &[u8]) -> Result<(), ()> { Ok(()) }
    fn flash_erase_sector(_addr: u32) -> Result<(), ()> { Ok(()) }
    fn get_active_bank() -> u8 { 1 }
    fn set_boot_bank(_bank: u8) -> Result<(), ()> { Ok(()) }
    fn system_reset() -> ! { loop {} }
}

pub struct OtaManager<F: OtaFlash> {
    _flash: core::marker::PhantomData<F>,
    state: OtaState,
    received_bytes: u32,
    upgrade_info: UpgradeInfo,
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
                    timestamp: 0, result: 0, bank: 0, source: 0,
                    reserved: [0; 10],
                }; 8],
            },
            running_crc: 0,
        }
    }

    pub fn state(&self) -> OtaState { self.state }
    pub fn active_bank(&self) -> u8 { self.upgrade_info.active_bank }
    pub fn received_bytes(&self) -> u32 { self.received_bytes }

    pub fn upgrade_history(&self) -> &[UpgradeRecord] {
        &self.upgrade_info.history
    }

    pub fn start_receive(&mut self) -> Result<(), ()> {
        if self.state != OtaState::Idle { return Err(()); }
        self.state = OtaState::Receiving;
        self.received_bytes = 0;
        self.running_crc = 0;
        Ok(())
    }

    pub fn write_chunk(&mut self, offset: u32, data: &[u8]) -> Result<(), ()> {
        if self.state != OtaState::Receiving { return Err(()); }
        let write_addr = addr::OTA_START + offset;
        if write_addr + data.len() as u32 > addr::OTA_START + addr::OTA_SIZE {
            return Err(());
        }
        let sector_start = write_addr & !0x0FFF;
        let sector_end = (write_addr + data.len() as u32 - 1) & !0x0FFF;
        let mut addr = sector_start;
        while addr <= sector_end {
            F::flash_erase_sector(addr)?;
            addr += 0x1000;
        }
        F::flash_write(write_addr, data)?;
        for &byte in data {
            self.running_crc = crc32_update(self.running_crc, byte);
        }
        self.received_bytes = self.received_bytes.max(offset + data.len() as u32);
        Ok(())
    }

    pub fn finalize_and_install(&mut self, source: u8) -> Result<OtaState, OtaState> {
        if self.state != OtaState::Receiving { return Err(self.state); }
        self.state = OtaState::Received;

        let mut header_buf = [0u8; FirmwareHeader::SIZE];
        F::flash_read(addr::OTA_START, &mut header_buf).map_err(|_| {
            self.state = OtaState::Failed;
            self.upgrade_info.error_code = 1;
            self.state
        })?;
        let header: FirmwareHeader = unsafe { core::ptr::read(header_buf.as_ptr() as *const _) };

        if !header.is_valid() {
            self.state = OtaState::Failed;
            self.upgrade_info.error_code = 1;
            return Err(self.state);
        }
        if self.running_crc != header.crc32 {
            self.state = OtaState::Failed;
            self.upgrade_info.error_code = 2;
            return Err(self.state);
        }

        self.state = OtaState::Verified;
        let target_bank = if self.upgrade_info.active_bank == 1 { 2 } else { 1 };
        let target_addr = if target_bank == 1 { addr::APP1_START } else { addr::APP2_START };
        let target_size = if target_bank == 1 { addr::APP1_SIZE } else { addr::APP2_SIZE };

        self.state = OtaState::Installing;
        let mut erase_addr = target_addr;
        while erase_addr < target_addr + target_size {
            F::flash_erase_sector(erase_addr).map_err(|_| {
                self.state = OtaState::Failed;
                self.upgrade_info.error_code = 4;
                self.state
            })?;
            erase_addr += 0x1000;
        }

        let firmware_start = addr::OTA_START + FirmwareHeader::SIZE as u32;
        let firmware_end = firmware_start + header.firmware_size;
        let mut copy_addr = firmware_start;
        let mut buf = [0u8; 256];
        while copy_addr < firmware_end {
            let remaining = (firmware_end - copy_addr) as usize;
            let chunk_size = remaining.min(256);
            F::flash_read(copy_addr, &mut buf[..chunk_size]).map_err(|_| {
                self.state = OtaState::Failed;
                self.upgrade_info.error_code = 5;
                self.state
            })?;
            F::flash_write(target_addr + (copy_addr - firmware_start), &buf[..chunk_size]).map_err(|_| {
                self.state = OtaState::Failed;
                self.upgrade_info.error_code = 5;
                self.state
            })?;
            copy_addr += chunk_size as u32;
        }

        let record = UpgradeRecord {
            from_version: FirmwareVersion::new(0, 2, 0, 0),
            to_version: header.version,
            timestamp: 0,
            result: 0,
            bank: target_bank,
            source,
            reserved: [0; 10],
        };
        self.push_history(record);
        F::set_boot_bank(target_bank).map_err(|_| {
            self.state = OtaState::Failed;
            self.upgrade_info.error_code = 6;
            self.state
        })?;
        self.state = OtaState::Installed;
        self.upgrade_info.active_bank = target_bank;
        Ok(self.state)
    }

    pub fn rollback(&mut self) -> Result<(), ()> {
        let rollback_bank = if self.upgrade_info.active_bank == 1 { 2 } else { 1 };
        F::set_boot_bank(rollback_bank)?;
        self.upgrade_info.active_bank = rollback_bank;
        Ok(())
    }

    fn push_history(&mut self, record: UpgradeRecord) {
        for i in (1..8).rev() {
            self.upgrade_info.history[i] = self.upgrade_info.history[i - 1];
        }
        self.upgrade_info.history[0] = record;
    }
}

/* ── CRC32 ── */

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
        crc = (crc >> 4) ^ CRC32_TABLE[(crc as usize) & 0x0F];
        i += 1;
    }
    crc
}

pub fn crc32_calc(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc = crc32_update(crc, byte);
    }
    !crc
}

/* ================================================================== */
/*  单元测试                                                           */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    struct MockFlash;
    impl OtaFlash for MockFlash {
        fn flash_read(_a: u32, _b: &mut [u8]) -> Result<(), ()> { Ok(()) }
        fn flash_write(_a: u32, _b: &[u8]) -> Result<(), ()> { Ok(()) }
        fn flash_erase_sector(_a: u32) -> Result<(), ()> { Ok(()) }
        fn get_active_bank() -> u8 { 1 }
        fn set_boot_bank(_b: u8) -> Result<(), ()> { Ok(()) }
        fn system_reset() -> ! { loop {} }
    }

    #[test]
    fn test_version_parse() {
        let v = FirmwareVersion::new(1, 2, 3, 4);
        assert_eq!(v.major, 1);
        assert_eq!(v.as_u32(), 0x01020304);
    }

    #[test]
    fn test_version_compare() {
        assert!(FirmwareVersion::new(1, 3, 0, 0).as_u32() > FirmwareVersion::new(1, 2, 0, 0).as_u32());
        assert!(FirmwareVersion::new(2, 0, 0, 0).as_u32() > FirmwareVersion::new(1, 9, 9, 9).as_u32());
    }

    #[test]
    fn test_version_to_str() {
        let s = FirmwareVersion::new(1, 2, 3, 0).to_str();
        assert_eq!(&s[0..8], b"v01.02.0");
    }

    #[test]
    fn test_crc32_known() {
        // 半字节查表 CRC32 (非标准 ISO CRC-32)
        let crc = crc32_calc(b"123456789");
        assert_eq!(crc, 3412128017);
    }

    #[test]
    fn test_crc32_deterministic() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF];
        assert_eq!(crc32_calc(&data), crc32_calc(&data));
    }

    #[test]
    fn test_header_valid() {
        let h = FirmwareHeader {
            magic: FirmwareHeader::MAGIC,
            version: FirmwareVersion::new(1, 0, 0, 0),
            firmware_size: 100000,
            crc32: 0, target_bank: 1, flags: 0, timestamp: 0,
            prev_version: FirmwareVersion::new(0, 9, 0, 0),
            reserved: [0; 16],
        };
        assert!(h.is_valid());
    }

    #[test]
    fn test_header_bad_magic() {
        let h = FirmwareHeader {
            magic: 0xBAD00000,
            version: FirmwareVersion::new(1, 0, 0, 0),
            firmware_size: 100000,
            crc32: 0, target_bank: 1, flags: 0, timestamp: 0,
            prev_version: FirmwareVersion::new(0, 9, 0, 0),
            reserved: [0; 16],
        };
        assert!(!h.is_valid());
    }

    #[test]
    fn test_header_too_large() {
        let h = FirmwareHeader {
            magic: FirmwareHeader::MAGIC,
            version: FirmwareVersion::new(1, 0, 0, 0),
            firmware_size: addr::APP_MAX_SIZE + 1,
            crc32: 0, target_bank: 1, flags: 0, timestamp: 0,
            prev_version: FirmwareVersion::new(0, 9, 0, 0),
            reserved: [0; 16],
        };
        assert!(!h.is_valid());
    }

    #[test]
    fn test_ota_initial_state() {
        let mgr = OtaManager::<MockFlash>::new();
        assert_eq!(mgr.state(), OtaState::Idle);
        assert_eq!(mgr.active_bank(), 1);
    }

    #[test]
    fn test_ota_start_receive() {
        let mut mgr = OtaManager::<MockFlash>::new();
        assert!(mgr.start_receive().is_ok());
        assert_eq!(mgr.state(), OtaState::Receiving);
    }

    #[test]
    fn test_ota_double_start_fails() {
        let mut mgr = OtaManager::<MockFlash>::new();
        mgr.start_receive().unwrap();
        assert!(mgr.start_receive().is_err());
    }

    #[test]
    fn test_ota_write_chunk() {
        let mut mgr = OtaManager::<MockFlash>::new();
        mgr.start_receive().unwrap();
        mgr.write_chunk(0, &[0xAA; 100]).unwrap();
        assert_eq!(mgr.received_bytes(), 100);
    }

    #[test]
    fn test_ota_write_chunk_out_of_range() {
        let mut mgr = OtaManager::<MockFlash>::new();
        mgr.start_receive().unwrap();
        assert!(mgr.write_chunk(addr::OTA_SIZE - 10, &[0; 100]).is_err());
    }

    #[test]
    fn test_ota_write_chunk_wrong_state() {
        let mut mgr = OtaManager::<MockFlash>::new();
        assert!(mgr.write_chunk(0, &[0; 10]).is_err());
    }

    #[test]
    fn test_upgrade_history_size() {
        let mgr = OtaManager::<MockFlash>::new();
        assert_eq!(mgr.upgrade_history().len(), 8);
    }

    #[test]
    fn test_ota_state_values() {
        assert_eq!(OtaState::Idle as u8, 0);
        assert_eq!(OtaState::Failed as u8, 6);
    }
}
