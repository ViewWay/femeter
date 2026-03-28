//! I2C (Inter-Integrated Circuit) HAL trait
//!
//! Provides interface for I2C communication.

use crate::HalResult;

/// I2C HAL trait for I2C communication
///
/// This trait is object-safe and can be used with `dyn I2cHal`.
pub trait I2cHal {
    /// Read from I2C device
    ///
    /// # Arguments
    /// * `address` - 7-bit I2C device address
    /// * `buffer` - Buffer to store received data
    ///
    /// # Returns
    /// Number of bytes read
    fn read(&mut self, address: u8, buffer: &mut [u8]) -> HalResult<usize>;

    /// Write to I2C device
    ///
    /// # Arguments
    /// * `address` - 7-bit I2C device address
    /// * `data` - Data to write
    ///
    /// # Returns
    /// Number of bytes written
    fn write(&mut self, address: u8, data: &[u8]) -> HalResult<usize>;

    /// Write then read from I2C device (repeated start)
    ///
    /// # Arguments
    /// * `address` - 7-bit I2C device address
    /// * `write_data` - Data to write
    /// * `read_buffer` - Buffer to store received data
    ///
    /// # Returns
    /// Number of bytes read
    fn write_read(
        &mut self,
        address: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> HalResult<usize> {
        self.write(address, write_data)?;
        self.read(address, read_buffer)
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::HalError;
    use std::collections::HashMap;
    use std::vec;
    use std::vec::Vec;

    struct MockI2cDevice {
        data: Vec<u8>,
    }

    struct MockI2c {
        devices: HashMap<u8, MockI2cDevice>,
        initialized: bool,
    }

    impl MockI2c {
        fn new() -> Self {
            Self {
                devices: HashMap::new(),
                initialized: true,
            }
        }

        fn add_device(&mut self, address: u8, data: &[u8]) {
            self.devices.insert(
                address,
                MockI2cDevice {
                    data: data.to_vec(),
                },
            );
        }
    }

    impl I2cHal for MockI2c {
        fn read(&mut self, address: u8, buffer: &mut [u8]) -> HalResult<usize> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            let device = self
                .devices
                .get_mut(&address)
                .ok_or(HalError::InvalidParam)?;

            let len = buffer.len().min(device.data.len());
            buffer[..len].copy_from_slice(&device.data[..len]);
            device.data.drain(..len);
            Ok(len)
        }

        fn write(&mut self, address: u8, data: &[u8]) -> HalResult<usize> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            if !self.devices.contains_key(&address) {
                return Err(HalError::InvalidParam);
            }
            // Just count bytes for mock
            Ok(data.len())
        }
    }

    #[test]
    fn test_i2c_read() {
        let mut i2c = MockI2c::new();
        i2c.add_device(0x50, &[0x01, 0x02, 0x03, 0x04]);

        let mut buffer = [0u8; 4];
        let count = i2c.read(0x50, &mut buffer).unwrap();

        assert_eq!(count, 4);
        assert_eq!(buffer, [0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_i2c_read_partial() {
        let mut i2c = MockI2c::new();
        i2c.add_device(0x50, &[0xAA, 0xBB]);

        let mut buffer = [0u8; 10];
        let count = i2c.read(0x50, &mut buffer).unwrap();

        assert_eq!(count, 2);
        assert_eq!(buffer[..2], [0xAA, 0xBB]);
    }

    #[test]
    fn test_i2c_write() {
        let mut i2c = MockI2c::new();
        i2c.add_device(0x50, &[]);

        let data = vec![0x10, 0x20, 0x30];
        let count = i2c.write(0x50, &data).unwrap();

        assert_eq!(count, 3);
    }

    #[test]
    fn test_i2c_invalid_device() {
        let mut i2c = MockI2c::new();

        let mut buffer = [0u8; 4];
        assert_eq!(
            i2c.read(0x99, &mut buffer).unwrap_err(),
            HalError::InvalidParam
        );

        assert_eq!(
            i2c.write(0x99, &[0x01]).unwrap_err(),
            HalError::InvalidParam
        );
    }

    #[test]
    fn test_i2c_write_read() {
        let mut i2c = MockI2c::new();
        i2c.add_device(0x50, &[0xAB, 0xCD]);

        let write_data = vec![0x00]; // Register address
        let mut read_buffer = [0u8; 2];

        let count = i2c.write_read(0x50, &write_data, &mut read_buffer).unwrap();

        assert_eq!(count, 2);
        assert_eq!(read_buffer, [0xAB, 0xCD]);
    }

    #[test]
    fn test_i2c_object_safe() {
        let mut i2c: std::boxed::Box<dyn I2cHal> = std::boxed::Box::new(MockI2c::new());
        let mut buffer = [0u8; 4];
        // Will fail with InvalidParam since no devices added
        assert!(i2c.read(0x50, &mut buffer).is_err());
    }
}
