//! Firmware manager for image transfer and update
//!
//! This module provides:
//! - Firmware image transfer state machine
//! - Image verification and activation
//! - Rollback support
//! - Transfer progress tracking

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use dlms_core::{errors::CosemError, types::DlmsType};

/// Maximum firmware image size (4MB)
const MAX_IMAGE_SIZE: usize = 4 * 1024 * 1024;

/// Block size for image transfer (typically 1-4KB)
const DEFAULT_BLOCK_SIZE: usize = 2048;

/// Image transfer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransferState {
    /// Idle - no transfer in progress
    Idle = 0,
    /// Transfer initialized, waiting for first block
    Initialized = 1,
    /// Transfer in progress
    Transferring = 2,
    /// Transfer complete, waiting for verification
    Complete = 3,
    /// Verification in progress
    Verifying = 4,
    /// Image verified and ready to activate
    Verified = 5,
    /// Image activated
    Activated = 6,
    /// Transfer failed
    Failed = 7,
}

impl TransferState {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Idle),
            1 => Some(Self::Initialized),
            2 => Some(Self::Transferring),
            3 => Some(Self::Complete),
            4 => Some(Self::Verifying),
            5 => Some(Self::Verified),
            6 => Some(Self::Activated),
            7 => Some(Self::Failed),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Check if transfer is active
    pub fn is_active(self) -> bool {
        matches!(self, Self::Initialized | Self::Transferring)
    }

    /// Check if transfer can be started
    pub fn can_start(self) -> bool {
        self == Self::Idle || self == Self::Failed
    }
}

/// Firmware image information
#[derive(Debug, Clone, PartialEq)]
pub struct ImageInfo {
    /// Image version string
    pub version: Vec<u8>,
    /// Image size in bytes
    pub size: u32,
    /// Image checksum (CRC-32 or SHA-256)
    pub checksum: Vec<u8>,
    /// Image signature (for signed images)
    pub signature: Option<Vec<u8>>,
}

impl ImageInfo {
    /// Create new image info
    pub fn new(version: Vec<u8>, size: u32, checksum: Vec<u8>) -> Self {
        Self {
            version,
            size,
            checksum,
            signature: None,
        }
    }

    /// Set image signature
    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = Some(signature);
        self
    }
}

/// Firmware transfer statistics
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransferStats {
    /// Number of blocks received
    pub blocks_received: u32,
    /// Number of blocks with errors
    pub blocks_error: u32,
    /// Total bytes transferred
    pub bytes_transferred: u32,
    /// Transfer start time (seconds since boot)
    pub start_time: u32,
    /// Transfer end time
    pub end_time: u32,
}

impl TransferStats {
    /// Create zero stats
    pub const fn zero() -> Self {
        Self {
            blocks_received: 0,
            blocks_error: 0,
            bytes_transferred: 0,
            start_time: 0,
            end_time: 0,
        }
    }

    /// Calculate transfer duration in seconds
    pub fn duration(&self) -> u32 {
        if self.end_time > self.start_time {
            self.end_time - self.start_time
        } else {
            0
        }
    }
}

impl Default for TransferStats {
    fn default() -> Self {
        Self::zero()
    }
}

/// Firmware manager for image transfer
#[derive(Debug, PartialEq)]
pub struct FirmwareManager {
    /// Current transfer state
    state: TransferState,
    /// Image information
    image_info: Option<ImageInfo>,
    /// Received image data
    image_data: Vec<u8>,
    /// Block size for transfer
    block_size: usize,
    /// Number of blocks expected
    total_blocks: u32,
    /// Next expected block number
    next_block: u32,
    /// Transfer statistics
    stats: TransferStats,
    /// Active firmware version
    active_version: Vec<u8>,
    /// Pending firmware version (after activation)
    pending_version: Vec<u8>,
    /// Rollback available
    rollback_available: bool,
}

impl FirmwareManager {
    /// Create a new firmware manager
    pub fn new(active_version: Vec<u8>) -> Self {
        Self {
            state: TransferState::Idle,
            image_info: None,
            image_data: Vec::with_capacity(DEFAULT_BLOCK_SIZE * 100),
            block_size: DEFAULT_BLOCK_SIZE,
            total_blocks: 0,
            next_block: 0,
            stats: TransferStats::zero(),
            active_version,
            pending_version: Vec::new(),
            rollback_available: false,
        }
    }

    /// Get current transfer state
    pub fn state(&self) -> TransferState {
        self.state
    }

    /// Get active firmware version
    pub fn active_version(&self) -> &[u8] {
        &self.active_version
    }

    /// Get pending firmware version (if any)
    pub fn pending_version(&self) -> Option<&[u8]> {
        if self.pending_version.is_empty() {
            None
        } else {
            Some(&self.pending_version)
        }
    }

    /// Initialize a new firmware transfer
    pub fn init_transfer(
        &mut self,
        image_info: ImageInfo,
        current_time: u32,
    ) -> Result<(), CosemError> {
        if !self.state.can_start() {
            return Err(CosemError::AccessDenied);
        }

        if image_info.size as usize > MAX_IMAGE_SIZE {
            return Err(CosemError::NotImplemented);
        }

        self.state = TransferState::Initialized;
        self.image_info = Some(image_info.clone());
        self.total_blocks = ((image_info.size as usize + self.block_size - 1) / self.block_size) as u32;
        self.next_block = 0;
        self.image_data.clear();
        self.image_data.reserve(image_info.size as usize);
        self.stats = TransferStats {
            start_time: current_time,
            ..TransferStats::zero()
        };
        self.pending_version = image_info.version.clone();

        Ok(())
    }

    /// Receive a block of firmware data
    pub fn receive_block(
        &mut self,
        block_number: u32,
        data: &[u8],
        current_time: u32,
    ) -> Result<(), CosemError> {
        if !self.state.is_active() {
            return Err(CosemError::AccessDenied);
        }

        // Check block number
        if block_number != self.next_block {
            self.stats.blocks_error += 1;
            return Err(CosemError::InvalidParameter);
        }

        // Check data size
        if data.len() > self.block_size {
            return Err(CosemError::InvalidParameter);
        }

        // Store data
        self.image_data.extend_from_slice(data);
        self.stats.blocks_received += 1;
        self.stats.bytes_transferred += data.len() as u32;
        self.next_block += 1;

        // Check if transfer is complete
        if self.next_block >= self.total_blocks {
            self.state = TransferState::Complete;
            self.stats.end_time = current_time;
        } else {
            self.state = TransferState::Transferring;
        }

        Ok(())
    }

    /// Verify the received image
    pub fn verify_image(&mut self) -> Result<(), CosemError> {
        if self.state != TransferState::Complete {
            return Err(CosemError::AccessDenied);
        }

        self.state = TransferState::Verifying;

        // In real implementation, verify checksum and signature
        // For now, just check that we received data
        if let Some(info) = &self.image_info {
            if self.image_data.len() != info.size as usize {
                self.state = TransferState::Failed;
                return Err(CosemError::NotImplemented);
            }
        }

        self.state = TransferState::Verified;
        Ok(())
    }

    /// Activate the verified image
    pub fn activate_image(&mut self) -> Result<(), CosemError> {
        if self.state != TransferState::Verified {
            return Err(CosemError::AccessDenied);
        }

        // Save current version for rollback
        self.rollback_available = true;

        // Activate new version
        self.active_version = self.pending_version.clone();
        self.pending_version.clear();

        self.state = TransferState::Activated;
        Ok(())
    }

    /// Rollback to previous firmware version
    pub fn rollback(&mut self) -> Result<(), CosemError> {
        if !self.rollback_available {
            return Err(CosemError::NotImplemented);
        }

        // In real implementation, would swap firmware banks
        self.rollback_available = false;
        self.state = TransferState::Idle;

        Ok(())
    }

    /// Abort current transfer
    pub fn abort_transfer(&mut self) -> Result<(), CosemError> {
        if !self.state.is_active() && self.state != TransferState::Complete {
            return Err(CosemError::AccessDenied);
        }

        self.state = TransferState::Failed;
        self.image_data.clear();
        self.pending_version.clear();

        Ok(())
    }

    /// Get transfer progress (0-100%)
    pub fn progress(&self) -> u8 {
        if self.total_blocks == 0 {
            return 0;
        }
        let percent = (self.next_block * 100 / self.total_blocks) as u8;
        percent.min(100)
    }

    /// Get transfer statistics
    pub fn stats(&self) -> TransferStats {
        self.stats
    }

    /// Get image information
    pub fn image_info(&self) -> Option<&ImageInfo> {
        self.image_info.as_ref()
    }

    /// Check if rollback is available
    pub fn rollback_available(&self) -> bool {
        self.rollback_available
    }

    /// Set block size for transfers
    pub fn set_block_size(&mut self, size: usize) -> Result<(), CosemError> {
        if size == 0 || size > 8192 {
            return Err(CosemError::InvalidParameter);
        }
        if self.state.is_active() {
            return Err(CosemError::AccessDenied);
        }
        self.block_size = size;
        Ok(())
    }

    /// Get block size
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Reset to idle state
    pub fn reset(&mut self) {
        self.state = TransferState::Idle;
        self.image_info = None;
        self.image_data.clear();
        self.next_block = 0;
        self.pending_version.clear();
    }

    /// Get transfer info as DLMS structure
    pub fn transfer_info_dlms(&self) -> DlmsType {
        DlmsType::Structure(alloc::vec![
            DlmsType::UInt8(self.state.code()),
            DlmsType::UInt32(self.stats.bytes_transferred),
            DlmsType::UInt8(self.progress()),
            DlmsType::UInt32(self.total_blocks),
            DlmsType::UInt32(self.next_block),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_firmware_manager_new() {
        let manager = FirmwareManager::new(b"1.0.0".to_vec());
        assert_eq!(manager.state(), TransferState::Idle);
        assert_eq!(manager.active_version(), b"1.0.0");
    }

    #[test]
    fn test_init_transfer() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());
        let info = ImageInfo::new(b"2.0.0".to_vec(), 4096, vec![0xAA, 0xBB, 0xCC, 0xDD]);

        assert!(manager.init_transfer(info, 100).is_ok());
        assert_eq!(manager.state(), TransferState::Initialized);
        assert_eq!(manager.total_blocks, 2); // 4096 / 2048
    }

    #[test]
    fn test_receive_block() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());
        let info = ImageInfo::new(b"2.0.0".to_vec(), 4096, vec![0; 4]);

        manager.init_transfer(info, 100).unwrap();

        let block1 = vec![0xAA; 2048];
        assert!(manager.receive_block(0, &block1, 110).is_ok());
        assert_eq!(manager.state(), TransferState::Transferring);

        let block2 = vec![0xBB; 2048];
        assert!(manager.receive_block(1, &block2, 120).is_ok());
        assert_eq!(manager.state(), TransferState::Complete);
        assert_eq!(manager.progress(), 100);
    }

    #[test]
    fn test_wrong_block_number() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());
        let info = ImageInfo::new(b"2.0.0".to_vec(), 4096, vec![0; 4]);

        manager.init_transfer(info, 100).unwrap();

        let block = vec![0xAA; 2048];
        // Wrong block number
        assert!(manager.receive_block(5, &block, 110).is_err());
        assert_eq!(manager.stats.blocks_error, 1);
    }

    #[test]
    fn test_verify_image() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());
        let info = ImageInfo::new(b"2.0.0".to_vec(), 2048, vec![0; 4]);

        manager.init_transfer(info, 100).unwrap();

        let block = vec![0xAA; 2048];
        manager.receive_block(0, &block, 110).unwrap();

        assert!(manager.verify_image().is_ok());
        assert_eq!(manager.state(), TransferState::Verified);
    }

    #[test]
    fn test_activate_image() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());
        let info = ImageInfo::new(b"2.0.0".to_vec(), 2048, vec![0; 4]);

        manager.init_transfer(info, 100).unwrap();
        manager.receive_block(0, &vec![0xAA; 2048], 110).unwrap();
        manager.verify_image().unwrap();

        assert!(manager.activate_image().is_ok());
        assert_eq!(manager.state(), TransferState::Activated);
        assert_eq!(manager.active_version(), b"2.0.0");
        assert!(manager.rollback_available());
    }

    #[test]
    fn test_abort_transfer() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());
        let info = ImageInfo::new(b"2.0.0".to_vec(), 4096, vec![0; 4]);

        manager.init_transfer(info, 100).unwrap();
        assert!(manager.abort_transfer().is_ok());

        assert_eq!(manager.state(), TransferState::Failed);
    }

    #[test]
    fn test_progress() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());
        let info = ImageInfo::new(b"2.0.0".to_vec(), 8192, vec![0; 4]); // 4 blocks

        manager.init_transfer(info, 100).unwrap();
        assert_eq!(manager.progress(), 0);

        manager.receive_block(0, &vec![0; 2048], 110).unwrap();
        assert_eq!(manager.progress(), 25);

        manager.receive_block(1, &vec![0; 2048], 120).unwrap();
        assert_eq!(manager.progress(), 50);
    }

    #[test]
    fn test_transfer_state_conversion() {
        assert_eq!(TransferState::from_u8(0), Some(TransferState::Idle));
        assert_eq!(TransferState::from_u8(1), Some(TransferState::Initialized));
        assert_eq!(TransferState::from_u8(7), Some(TransferState::Failed));
        assert_eq!(TransferState::from_u8(99), None);

        assert!(TransferState::Idle.can_start());
        assert!(TransferState::Failed.can_start());
        assert!(!TransferState::Transferring.can_start());
    }

    #[test]
    fn test_set_block_size() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());

        assert!(manager.set_block_size(4096).is_ok());
        assert_eq!(manager.block_size(), 4096);

        // Invalid sizes
        assert!(manager.set_block_size(0).is_err());
        assert!(manager.set_block_size(10000).is_err());
    }

    #[test]
    fn test_transfer_stats() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());
        let info = ImageInfo::new(b"2.0.0".to_vec(), 4096, vec![0; 4]);

        manager.init_transfer(info, 100).unwrap();
        manager.receive_block(0, &vec![0; 2048], 110).unwrap();
        manager.receive_block(1, &vec![0; 2048], 120).unwrap();

        let stats = manager.stats();
        assert_eq!(stats.blocks_received, 2);
        assert_eq!(stats.bytes_transferred, 4096);
        assert_eq!(stats.duration(), 20);
    }

    #[test]
    fn test_rollback() {
        let mut manager = FirmwareManager::new(b"1.0.0".to_vec());
        let info = ImageInfo::new(b"2.0.0".to_vec(), 2048, vec![0; 4]);

        manager.init_transfer(info, 100).unwrap();
        manager.receive_block(0, &vec![0; 2048], 110).unwrap();
        manager.verify_image().unwrap();
        manager.activate_image().unwrap();

        assert!(manager.rollback_available());
        assert!(manager.rollback().is_ok());
        assert!(!manager.rollback_available());
    }
}
