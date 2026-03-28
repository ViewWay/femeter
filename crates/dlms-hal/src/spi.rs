//! SPI (Serial Peripheral Interface) HAL trait
//!
//! Provides interface for SPI communication.

use crate::{HalError, HalResult};

/// SPI HAL trait for SPI communication
///
/// This trait is object-safe and can be used with `dyn SpiHal`.
pub trait SpiHal {
    /// Transfer data (simultaneous write and read)
    ///
    /// # Arguments
    /// * `data` - Data to write, will be replaced with read data
    ///
    /// # Returns
    /// Number of bytes transferred
    fn transfer(&mut self, data: &mut [u8]) -> HalResult<usize>;

    /// Write data without reading
    ///
    /// # Returns
    /// Number of bytes written
    fn write(&mut self, data: &[u8]) -> HalResult<usize> {
        let mut buffer = [0u8; 256];
        if data.len() <= 256 {
            let len = data.len();
            buffer[..len].copy_from_slice(data);
            return self.transfer(&mut buffer[..len]);
        }

        // For larger writes, we'd need heap allocation but that's not available in no_std
        // Fall back to transfer-in-place for the caller
        Err(HalError::InvalidParam)
    }

    /// Read data (writes zeros)
    ///
    /// # Returns
    /// Number of bytes read
    fn read(&mut self, buffer: &mut [u8]) -> HalResult<usize> {
        // Fill with zeros for clock generation
        for byte in buffer.iter_mut() {
            *byte = 0;
        }
        self.transfer(buffer)
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use std::vec;

    struct MockSpi {
        response_data: std::vec::Vec<u8>,
        tx_data: std::vec::Vec<u8>,
        initialized: bool,
    }

    impl MockSpi {
        fn new() -> Self {
            Self {
                response_data: std::vec![0xFF; 256],
                tx_data: std::vec::Vec::new(),
                initialized: true,
            }
        }

        fn with_response(data: &[u8]) -> Self {
            let mut spi = Self::new();
            spi.response_data = data.to_vec();
            spi
        }

        fn take_tx(&mut self) -> std::vec::Vec<u8> {
            self.tx_data.drain(..).collect()
        }
    }

    impl SpiHal for MockSpi {
        fn transfer(&mut self, data: &mut [u8]) -> HalResult<usize> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }

            let len = data.len();
            for (i, byte) in data.iter_mut().enumerate() {
                self.tx_data.push(*byte);
                // Echo back with bit flip for testing
                *byte = self.response_data.get(i).copied().unwrap_or(!*byte);
            }
            Ok(len)
        }

        fn write(&mut self, data: &[u8]) -> HalResult<usize> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.tx_data.extend_from_slice(data);
            Ok(data.len())
        }
    }

    #[test]
    fn test_spi_transfer() {
        let mut spi = MockSpi::with_response(&[0xAA, 0xBB, 0xCC]);
        let mut data = std::vec![0x01, 0x02, 0x03];

        let count = spi.transfer(&mut data).unwrap();
        assert_eq!(count, 3);
        assert_eq!(data, [0xAA, 0xBB, 0xCC]);
        assert_eq!(spi.take_tx(), std::vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_spi_write() {
        let mut spi = MockSpi::new();
        let data = std::vec![0x11, 0x22, 0x33];

        let count = spi.write(&data).unwrap();
        assert_eq!(count, 3);
        assert_eq!(spi.take_tx(), std::vec![0x11, 0x22, 0x33]);
    }

    #[test]
    fn test_spi_read() {
        let mut spi = MockSpi::with_response(&[0x12, 0x34, 0x56]);
        let mut buffer = [0u8; 3];

        let count = spi.read(&mut buffer).unwrap();
        assert_eq!(count, 3);
        assert_eq!(buffer, [0x12, 0x34, 0x56]);
    }

    #[test]
    fn test_spi_not_initialized() {
        let mut spi = MockSpi {
            response_data: vec![],
            tx_data: vec![],
            initialized: false,
        };

        let mut data = vec![0x01];
        assert_eq!(
            spi.transfer(&mut data).unwrap_err(),
            HalError::NotInitialized
        );
    }

    #[test]
    fn test_spi_empty_transfer() {
        let mut spi = MockSpi::new();
        let mut data = std::vec::Vec::new();

        let count = spi.transfer(&mut data).unwrap();
        assert_eq!(count, 0);
        assert!(spi.take_tx().is_empty());
    }

    #[test]
    fn test_spi_object_safe() {
        let mut spi: std::boxed::Box<dyn SpiHal> = std::boxed::Box::new(MockSpi::new());
        let mut data = [0x55u8; 1];
        spi.transfer(&mut data).unwrap();
    }
}
