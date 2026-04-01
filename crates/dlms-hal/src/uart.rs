//! UART (Universal Asynchronous Receiver-Transmitter) HAL trait
//!
//! Provides interface for serial communication.

use crate::HalResult;

/// UART HAL trait for serial communication
///
/// This trait is object-safe and can be used with `dyn UartHal`.
pub trait UartHal {
    /// Read a single byte
    ///
    /// # Errors
    /// * `HalError::Timeout` - No data available
    fn read(&mut self) -> HalResult<u8>;

    /// Write a single byte
    fn write(&mut self, byte: u8) -> HalResult<()>;

    /// Read into buffer
    ///
    /// # Returns
    /// Number of bytes read
    fn read_buffer(&mut self, buffer: &mut [u8]) -> HalResult<usize> {
        let mut count = 0;
        for slot in buffer.iter_mut() {
            match self.read() {
                Ok(byte) => {
                    *slot = byte;
                    count += 1;
                }
                Err(_) => break,
            }
        }
        Ok(count)
    }

    /// Write from buffer
    ///
    /// # Returns
    /// Number of bytes written
    fn write_buffer(&mut self, buffer: &[u8]) -> HalResult<usize> {
        let mut count = 0;
        for byte in buffer {
            self.write(*byte)?;
            count += 1;
        }
        Ok(count)
    }

    /// Set baud rate
    ///
    /// # Arguments
    /// * `baud` - Baud rate (e.g., 9600, 115200)
    fn set_baud_rate(&mut self, baud: u32) -> HalResult<()>;

    /// Check if data is available for reading
    fn available(&mut self) -> HalResult<bool>;

    /// Flush transmit buffer
    fn flush(&mut self) -> HalResult<()> {
        Ok(())
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::HalError;
    use std::collections::VecDeque;
    use std::vec;
    use std::vec::Vec;

    struct MockUart {
        rx_buffer: VecDeque<u8>,
        tx_buffer: VecDeque<u8>,
        baud_rate: u32,
        initialized: bool,
    }

    impl MockUart {
        fn new() -> Self {
            Self {
                rx_buffer: VecDeque::new(),
                tx_buffer: VecDeque::new(),
                baud_rate: 115200,
                initialized: true,
            }
        }

        fn with_data(data: &[u8]) -> Self {
            let mut uart = Self::new();
            for &byte in data {
                uart.rx_buffer.push_back(byte);
            }
            uart
        }

        fn take_tx(&mut self) -> Vec<u8> {
            self.tx_buffer.drain(..).collect()
        }
    }

    impl UartHal for MockUart {
        fn read(&mut self) -> HalResult<u8> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.rx_buffer.pop_front().ok_or(HalError::Timeout)
        }

        fn write(&mut self, byte: u8) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.tx_buffer.push_back(byte);
            Ok(())
        }

        fn set_baud_rate(&mut self, baud: u32) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.baud_rate = baud;
            Ok(())
        }

        fn available(&mut self) -> HalResult<bool> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            Ok(!self.rx_buffer.is_empty())
        }
    }

    #[test]
    fn test_uart_read_write() {
        let mut uart = MockUart::with_data(&[0x41, 0x42, 0x43]);

        assert!(uart.available().unwrap());
        assert_eq!(uart.read().unwrap(), 0x41);
        assert_eq!(uart.read().unwrap(), 0x42);
        assert_eq!(uart.read().unwrap(), 0x43);
        assert!(!uart.available().unwrap());
    }

    #[test]
    fn test_uart_read_timeout() {
        let mut uart = MockUart::new();
        assert_eq!(uart.read().unwrap_err(), HalError::Timeout);
    }

    #[test]
    fn test_uart_write() {
        let mut uart = MockUart::new();
        uart.write(0x48).unwrap();
        uart.write(0x65).unwrap();
        uart.write(0x6C).unwrap();
        uart.write(0x6C).unwrap();
        uart.write(0x6F).unwrap();

        assert_eq!(uart.take_tx(), vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]);
    }

    #[test]
    fn test_uart_set_baud_rate() {
        let mut uart = MockUart::new();
        uart.set_baud_rate(9600).unwrap();
        uart.set_baud_rate(115200).unwrap();
    }

    #[test]
    fn test_uart_read_buffer() {
        let mut uart = MockUart::with_data(&[1, 2, 3, 4, 5]);
        let mut buffer = [0u8; 10];

        let count = uart.read_buffer(&mut buffer).unwrap();
        assert_eq!(count, 5);
        assert_eq!(&buffer[..5], &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_uart_write_buffer() {
        let mut uart = MockUart::new();
        let data = b"Hello";

        let count = uart.write_buffer(data).unwrap();
        assert_eq!(count, 5);
        assert_eq!(uart.take_tx(), b"Hello".to_vec());
    }

    #[test]
    fn test_uart_flush() {
        let mut uart = MockUart::new();
        uart.flush().unwrap(); // Should work even if no-op
    }

    #[test]
    fn test_uart_object_safe() {
        let mut uart: std::boxed::Box<dyn UartHal> = std::boxed::Box::new(MockUart::new());
        uart.write(0x55).unwrap();
        uart.set_baud_rate(9600).unwrap();
    }
}
