//! ADC (Analog-to-Digital Converter) HAL trait
//!
//! Provides interface for reading analog voltage channels.

use crate::HalResult;

/// ADC HAL trait for reading analog voltage channels
///
/// This trait is object-safe and can be used with `dyn AdcHal`.
pub trait AdcHal {
    /// Read a single sample from the specified channel
    ///
    /// # Arguments
    /// * `channel` - ADC channel number (0-based)
    ///
    /// # Returns
    /// * `u16` - ADC sample value (0-4095 for 12-bit, 0-65535 for 16-bit)
    ///
    /// # Errors
    /// * `HalError::InvalidParam` - Channel out of range
    /// * `HalError::NotInitialized` - ADC not initialized
    fn read_channel(&mut self, channel: u8) -> HalResult<u16>;

    /// Read multiple samples from the specified channel
    ///
    /// # Arguments
    /// * `channel` - ADC channel number
    /// * `buffer` - Buffer to store samples
    ///
    /// # Returns
    /// Number of samples actually read
    fn read_buffer(&mut self, channel: u8, buffer: &mut [u16]) -> HalResult<usize> {
        let mut count = 0;
        for slot in buffer.iter_mut() {
            match self.read_channel(channel) {
                Ok(value) => {
                    *slot = value;
                    count += 1;
                }
                Err(_) => break,
            }
        }
        Ok(count)
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::HalError;

    struct MockAdc {
        initialized: bool,
        channel_values: [u16; 8],
    }

    impl MockAdc {
        fn new() -> Self {
            Self {
                initialized: true,
                channel_values: [100, 200, 300, 400, 500, 600, 700, 800],
            }
        }

        fn uninitialized() -> Self {
            Self {
                initialized: false,
                channel_values: [0; 8],
            }
        }
    }

    impl AdcHal for MockAdc {
        fn read_channel(&mut self, channel: u8) -> HalResult<u16> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            if channel >= 8 {
                return Err(HalError::InvalidParam);
            }
            Ok(self.channel_values[channel as usize])
        }
    }

    #[test]
    fn test_adc_read_valid_channel() {
        let mut adc = MockAdc::new();
        assert_eq!(adc.read_channel(0).unwrap(), 100);
        assert_eq!(adc.read_channel(3).unwrap(), 400);
        assert_eq!(adc.read_channel(7).unwrap(), 800);
    }

    #[test]
    fn test_adc_invalid_channel() {
        let mut adc = MockAdc::new();
        assert_eq!(adc.read_channel(8).unwrap_err(), HalError::InvalidParam);
        assert_eq!(adc.read_channel(255).unwrap_err(), HalError::InvalidParam);
    }

    #[test]
    fn test_adc_not_initialized() {
        let mut adc = MockAdc::uninitialized();
        assert_eq!(adc.read_channel(0).unwrap_err(), HalError::NotInitialized);
    }

    #[test]
    fn test_adc_read_buffer() {
        let mut adc = MockAdc::new();
        let mut buffer = [0u16; 5];
        let count = adc.read_buffer(2, &mut buffer).unwrap();
        assert_eq!(count, 5);
        assert_eq!(buffer, [300; 5]);
    }

    #[test]
    fn test_adc_read_buffer_partial() {
        let mut adc = MockAdc::uninitialized();
        let mut buffer = [0u16; 5];
        let count = adc.read_buffer(0, &mut buffer).unwrap();
        assert_eq!(count, 0); // Fails immediately
    }

    #[test]
    fn test_adc_object_safe() {
        // Test that trait is object-safe
        let mut adc: std::boxed::Box<dyn AdcHal> = std::boxed::Box::new(MockAdc::new());
        assert_eq!(adc.read_channel(0).unwrap(), 100);
    }
}
