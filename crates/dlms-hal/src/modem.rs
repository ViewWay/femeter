//! Modem HAL trait
//!
//! Provides interface for modem/communication module control.

use crate::{HalError, HalResult};

/// Modem HAL trait for modem control
///
/// This trait is object-safe and can be used with `dyn ModemHal`.
///
/// # Note
/// Uses buffer-based API for no_std compatibility.
pub trait ModemHal {
    /// Send AT command to modem
    ///
    /// # Arguments
    /// * `command` - AT command (without AT prefix, will be added)
    /// * `buffer` - Buffer to store response
    ///
    /// # Returns
    /// Number of bytes written to buffer
    fn send_at_command(&mut self, command: &str, buffer: &mut [u8]) -> HalResult<usize>;

    /// Check if modem is connected to network
    fn is_connected(&mut self) -> HalResult<bool>;

    /// Connect to network
    fn connect(&mut self) -> HalResult<()>;

    /// Disconnect from network
    fn disconnect(&mut self) -> HalResult<()>;

    /// Get signal strength (0-31, 99=unknown)
    fn get_signal_strength(&mut self) -> HalResult<u8> {
        Ok(99)
    }

    /// Get IMEI into buffer
    fn get_imei(&mut self, buffer: &mut [u8]) -> HalResult<usize> {
        let _ = buffer;
        Ok(0)
    }

    /// Send SMS
    ///
    /// # Arguments
    /// * `number` - Phone number
    /// * `message` - SMS content
    fn send_sms(&mut self, number: &str, message: &str) -> HalResult<()> {
        let _ = (number, message);
        Err(HalError::NotImplemented)
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use std::string::String;

    #[derive(Debug)]
    enum ConnectionState {
        Disconnected,
        Connecting,
        Connected,
    }

    struct MockModem {
        state: ConnectionState,
        signal: u8,
        imei: String,
        initialized: bool,
    }

    impl MockModem {
        fn new() -> Self {
            Self {
                state: ConnectionState::Disconnected,
                signal: 20,
                imei: String::from("123456789012345"),
                initialized: true,
            }
        }

        fn connected() -> Self {
            Self {
                state: ConnectionState::Connected,
                signal: 25,
                imei: String::from("123456789012345"),
                initialized: true,
            }
        }
    }

    impl ModemHal for MockModem {
        fn send_at_command(&mut self, command: &str, buffer: &mut [u8]) -> HalResult<usize> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }

            // Simple AT command mock
            let cmd = command.to_uppercase();
            let response = if cmd.contains("CSQ") {
                // Signal strength query
                std::format!("+CSQ: {},99\r\nOK", self.signal)
            } else if cmd.contains("CGSN") {
                // IMEI query
                std::format!("{}\r\nOK", self.imei)
            } else if cmd.contains("CIMI") {
                // IMSI query
                String::from("310260123456789\r\nOK")
            } else {
                String::from("OK")
            };

            let bytes = response.as_bytes();
            let len = buffer.len().min(bytes.len());
            buffer[..len].copy_from_slice(&bytes[..len]);
            Ok(len)
        }

        fn is_connected(&mut self) -> HalResult<bool> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            Ok(matches!(self.state, ConnectionState::Connected))
        }

        fn connect(&mut self) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.state = ConnectionState::Connected;
            Ok(())
        }

        fn disconnect(&mut self) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.state = ConnectionState::Disconnected;
            Ok(())
        }

        fn get_signal_strength(&mut self) -> HalResult<u8> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            Ok(self.signal)
        }

        fn get_imei(&mut self, buffer: &mut [u8]) -> HalResult<usize> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            let imei_bytes = self.imei.as_bytes();
            let len = buffer.len().min(imei_bytes.len());
            buffer[..len].copy_from_slice(&imei_bytes[..len]);
            Ok(len)
        }
    }

    #[test]
    fn test_modem_send_at_command() {
        let mut modem = MockModem::new();
        let mut buffer = [0u8; 64];
        let len = modem.send_at_command("AT", &mut buffer).unwrap();
        assert_eq!(String::from_utf8_lossy(&buffer[..len]), "OK");
    }

    #[test]
    fn test_modem_at_csq() {
        let mut modem = MockModem::new();
        let mut buffer = [0u8; 64];
        let len = modem.send_at_command("AT+CSQ", &mut buffer).unwrap();
        let response_str = String::from_utf8_lossy(&buffer[..len]);
        assert!(response_str.contains("+CSQ:"));
        assert!(response_str.contains("OK"));
    }

    #[test]
    fn test_modem_at_cgsn() {
        let mut modem = MockModem::new();
        let mut buffer = [0u8; 64];
        let len = modem.send_at_command("AT+CGSN", &mut buffer).unwrap();
        let response_str = String::from_utf8_lossy(&buffer[..len]);
        assert!(response_str.contains("123456789012345"));
    }

    #[test]
    fn test_modem_connect() {
        let mut modem = MockModem::new();
        assert!(!modem.is_connected().unwrap());

        modem.connect().unwrap();
        assert!(modem.is_connected().unwrap());
    }

    #[test]
    fn test_modem_disconnect() {
        let mut modem = MockModem::connected();
        assert!(modem.is_connected().unwrap());

        modem.disconnect().unwrap();
        assert!(!modem.is_connected().unwrap());
    }

    #[test]
    fn test_modem_signal_strength() {
        let mut modem = MockModem::new();
        assert_eq!(modem.get_signal_strength().unwrap(), 20);
    }

    #[test]
    fn test_modem_get_imei() {
        let mut modem = MockModem::new();
        let mut buffer = [0u8; 20];
        let len = modem.get_imei(&mut buffer).unwrap();
        assert_eq!(len, 15);
        assert_eq!(
            String::from_utf8_lossy(&buffer[..len]),
            "123456789012345"
        );
    }

    #[test]
    fn test_modem_not_initialized() {
        let mut modem = MockModem {
            state: ConnectionState::Disconnected,
            signal: 0,
            imei: String::new(),
            initialized: false,
        };

        assert_eq!(
            modem.is_connected().unwrap_err(),
            HalError::NotInitialized
        );
        assert_eq!(modem.connect().unwrap_err(), HalError::NotInitialized);
    }

    #[test]
    fn test_modem_send_sms_not_implemented() {
        let mut modem = MockModem::new();
        let result = modem.send_sms("1234567890", "Test");
        assert_eq!(result.unwrap_err(), HalError::NotImplemented);
    }

    #[test]
    fn test_modem_object_safe() {
        let mut modem: std::boxed::Box<dyn ModemHal> =
            std::boxed::Box::new(MockModem::new());
        let mut buffer = [0u8; 64];
        let len = modem.send_at_command("AT", &mut buffer).unwrap();
        assert!(len > 0);
    }
}
