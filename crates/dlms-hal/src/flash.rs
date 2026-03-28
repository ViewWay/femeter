//! Flash (Non-volatile memory) HAL trait
//!
//! Provides interface for flash memory operations.

use crate::HalResult;

/// Flash HAL trait for non-volatile memory
///
/// This trait is object-safe and can be used with `dyn FlashHal`.
pub trait FlashHal {
    /// Read data from flash
    ///
    /// # Arguments
    /// * `address` - Flash memory address
    /// * `buffer` - Buffer to store read data
    ///
    /// # Returns
    /// Number of bytes read
    fn read(&mut self, address: u32, buffer: &mut [u8]) -> HalResult<usize>;

    /// Write data to flash (must be aligned to page boundary)
    ///
    /// # Arguments
    /// * `address` - Flash memory address
    /// * `data` - Data to write
    ///
    /// # Returns
    /// Number of bytes written
    fn write(&mut self, address: u32, data: &[u8]) -> HalResult<usize>;

    /// Erase a sector
    ///
    /// # Arguments
    /// * `sector` - Sector number
    fn erase_sector(&mut self, sector: u32) -> HalResult<()>;

    /// Erase a page
    ///
    /// # Arguments
    /// * `page` - Page number
    fn erase_page(&mut self, page: u32) -> HalResult<()>;

    /// Get total number of sectors
    fn sector_count(&self) -> u32;

    /// Get sector size in bytes
    fn sector_size(&self) -> u32;

    /// Get page size in bytes
    fn page_size(&self) -> u32;
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::HalError;
    use std::vec;

    const FLASH_SIZE: usize = 64 * 1024; // 64KB mock flash
    const SECTOR_SIZE: u32 = 4 * 1024; // 4KB sectors
    const PAGE_SIZE: u32 = 256; // 256 byte pages

    struct MockFlash {
        data: std::vec::Vec<u8>,
        initialized: bool,
    }

    impl MockFlash {
        fn new() -> Self {
            Self {
                data: std::vec![0xFF; FLASH_SIZE],
                initialized: true,
            }
        }

        fn check_bounds(&self, address: u32, len: usize) -> HalResult<()> {
            if address as usize + len > FLASH_SIZE {
                return Err(HalError::InvalidParam);
            }
            Ok(())
        }
    }

    impl FlashHal for MockFlash {
        fn read(&mut self, address: u32, buffer: &mut [u8]) -> HalResult<usize> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.check_bounds(address, buffer.len())?;

            let start = address as usize;
            let end = start + buffer.len();
            buffer.copy_from_slice(&self.data[start..end]);
            Ok(buffer.len())
        }

        fn write(&mut self, address: u32, data: &[u8]) -> HalResult<usize> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.check_bounds(address, data.len())?;

            let start = address as usize;
            let end = start + data.len();

            // Simulate flash write (can only change 1->0)
            for (i, &byte) in data.iter().enumerate() {
                let existing = self.data[start + i];
                self.data[start + i] = existing & byte; // AND for flash simulation
            }

            // Check if any write failed (0->1 not allowed)
            for (i, &byte) in data.iter().enumerate() {
                if self.data[start + i] != byte {
                    return Err(HalError::HardwareFault);
                }
            }

            Ok(data.len())
        }

        fn erase_sector(&mut self, sector: u32) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            let sector_count = FLASH_SIZE as u32 / SECTOR_SIZE;
            if sector >= sector_count {
                return Err(HalError::InvalidParam);
            }

            let start = (sector * SECTOR_SIZE) as usize;
            let end = start + SECTOR_SIZE as usize;
            for byte in &mut self.data[start..end] {
                *byte = 0xFF;
            }
            Ok(())
        }

        fn erase_page(&mut self, page: u32) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            let page_count = FLASH_SIZE as u32 / PAGE_SIZE;
            if page >= page_count {
                return Err(HalError::InvalidParam);
            }

            let start = (page * PAGE_SIZE) as usize;
            let end = start + PAGE_SIZE as usize;
            for byte in &mut self.data[start..end] {
                *byte = 0xFF;
            }
            Ok(())
        }

        fn sector_count(&self) -> u32 {
            FLASH_SIZE as u32 / SECTOR_SIZE
        }

        fn sector_size(&self) -> u32 {
            SECTOR_SIZE
        }

        fn page_size(&self) -> u32 {
            PAGE_SIZE
        }
    }

    #[test]
    fn test_flash_read() {
        let mut flash = MockFlash::new();
        let mut buffer = [0u8; 10];

        let count = flash.read(0, &mut buffer).unwrap();
        assert_eq!(count, 10);
        assert_eq!(buffer, [0xFF; 10]);
    }

    #[test]
    fn test_flash_write() {
        let mut flash = MockFlash::new();
        let data = std::vec::Vec::from([0x42, 0x55, 0xAA]);

        flash.write(100, &data).unwrap();

        let mut buffer = [0u8; 3];
        flash.read(100, &mut buffer).unwrap();
        assert_eq!(buffer, [0x42, 0x55, 0xAA]);
    }

    #[test]
    fn test_flash_erase_sector() {
        let mut flash = MockFlash::new();
        let data = std::vec::Vec::from([0x00u8; 4096]);

        flash.write(0, &data).unwrap();
        flash.erase_sector(0).unwrap();

        let mut buffer = [0u8; 10];
        flash.read(0, &mut buffer).unwrap();
        assert_eq!(buffer, [0xFF; 10]);
    }

    #[test]
    fn test_flash_erase_page() {
        let mut flash = MockFlash::new();
        let data = std::vec::Vec::from([0x55u8; 256]);

        flash.write(256, &data).unwrap();
        flash.erase_page(1).unwrap(); // Page 1 starts at 256

        let mut buffer = [0u8; 10];
        flash.read(256, &mut buffer).unwrap();
        assert_eq!(buffer, [0xFF; 10]);
    }

    #[test]
    fn test_flash_sector_count() {
        let flash = MockFlash::new();
        assert_eq!(flash.sector_count(), 16); // 64KB / 4KB
    }

    #[test]
    fn test_flash_sector_size() {
        let flash = MockFlash::new();
        assert_eq!(flash.sector_size(), 4096);
    }

    #[test]
    fn test_flash_page_size() {
        let flash = MockFlash::new();
        assert_eq!(flash.page_size(), 256);
    }

    #[test]
    fn test_flash_invalid_sector() {
        let mut flash = MockFlash::new();
        assert_eq!(
            flash.erase_sector(99).unwrap_err(),
            HalError::InvalidParam
        );
    }

    #[test]
    fn test_flash_out_of_bounds() {
        let mut flash = MockFlash::new();
        let mut buffer = [0u8; 1000];

        assert_eq!(
            flash.read(FLASH_SIZE as u32 - 100, &mut buffer).unwrap_err(),
            HalError::InvalidParam
        );
    }

    #[test]
    fn test_flash_object_safe() {
        let flash: std::boxed::Box<dyn FlashHal> = std::boxed::Box::new(MockFlash::new());
        assert_eq!(flash.sector_count(), 16);
        assert_eq!(flash.page_size(), 256);
    }
}
