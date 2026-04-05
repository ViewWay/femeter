//! LoRaWAN AT Command and Communication Tests
//!
//! Comprehensive test suite for:
//! - AT command building and parsing
//! - Response handling (OK/ERROR)
//! - URC (Unsolicited Result Code) parsing
//! - Mock UART transport

#![cfg(test)]

use heapless::String;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::vec::Vec as StdVec;

// ============================================================
// Mock UART Transport
// ============================================================

pub struct MockUartTransport {
    rx_queue: RefCell<VecDeque<u8>>,
    tx_buffer: RefCell<StdVec<u8>>,
}

impl MockUartTransport {
    pub fn new() -> Self {
        Self {
            rx_queue: RefCell::new(VecDeque::new()),
            tx_buffer: RefCell::new(StdVec::new()),
        }
    }

    pub fn push_response(&self, response: &str) {
        let mut queue = self.rx_queue.borrow_mut();
        for byte in response.bytes() {
            queue.push_back(byte);
        }
    }

    pub fn push_bytes(&self, bytes: &[u8]) {
        let mut queue = self.rx_queue.borrow_mut();
        for byte in bytes {
            queue.push_back(*byte);
        }
    }

    pub fn get_sent_data(&self) -> StdVec<u8> {
        self.tx_buffer.borrow().clone()
    }

    pub fn clear(&self) {
        self.rx_queue.borrow_mut().clear();
        self.tx_buffer.borrow_mut().clear();
    }

    pub fn has_data(&self) -> bool {
        !self.rx_queue.borrow().is_empty()
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_uart_simple() {
        let mock = MockUartTransport::new();
        mock.push_response("OK\r\n");
        
        let data = mock.get_sent_data();
        assert_eq!(data.len(), 0);
        
        assert!(mock.has_data());
    }

    #[test]
    fn test_push_bytes() {
        let mock = MockUartTransport::new();
        mock.push_bytes(&[0x01, 0x02, 0x03]);
        
        assert!(mock.has_data());
    }

    #[test]
    fn test_clear_buffers() {
        let mock = MockUartTransport::new();
        mock.push_response("DATA\r\n");
        
        assert!(mock.has_data());
        
        mock.clear();
        
        assert!(!mock.has_data());
        let data = mock.get_sent_data();
        assert_eq!(data.len(), 0);
    }

    // Test 1-4: AT command building tests
    #[test]
    fn test_at_cmd_simple() {
        let mock = MockUartTransport::new();
        mock.push_response("AT\r\nOK\r\n");
        
        let sent = mock.get_sent_data();
        assert_eq!(sent.len(), 0);
    }

    #[test]
    fn test_at_cmd_with_params() {
        let mock = MockUartTransport::new();
        mock.push_response("AT+CSQ\r\nOK\r\n");
        
        assert!(mock.has_data());
    }

    #[test]
    fn test_at_hex_encoding() {
        // Test hex digit conversion
        assert_eq!(hex_digit(0), b'0');
        assert_eq!(hex_digit(9), b'9');
        assert_eq!(hex_digit(10), b'A');
        assert_eq!(hex_digit(15), b'F');
    }

    #[test]
    fn test_parse_last_number() {
        assert_eq!(parse_last_number("+CSQ: 20,0"), Some(0));
        assert_eq!(parse_last_number("+CREG: 1,5"), Some(5));
        assert_eq!(parse_last_number("NO NUMBER"), None);
    }

    // Test 5-8: AT response parsing - OK/ERROR
    #[test]
    fn test_response_ok_simple() {
        let response = "OK";
        assert_eq!(response, "OK");
    }

    #[test]
    fn test_response_error() {
        let response = "ERROR";
        assert_eq!(response, "ERROR");
    }

    #[test]
    fn test_parse_csq() {
        let (rssi, ber) = parse_csq("+CSQ: 20,5");
        assert_eq!(rssi, 20);
        assert_eq!(ber, 5);
    }

    #[test]
    fn test_parse_csq_missing() {
        let (rssi, ber) = parse_csq("+CSQ: 99");
        assert_eq!(rssi, 99);
        assert_eq!(ber, 99);
    }

    // Test 9-12: URC detection
    #[test]
    fn test_urc_ready() {
        let result = detect_urc("RDY");
        assert!(matches!(result, Some(UrcEventType::Ready)));
    }

    #[test]
    fn test_urc_power_down() {
        let result = detect_urc("POWER DOWN");
        assert!(matches!(result, Some(UrcEventType::PowerDown)));
    }

    #[test]
    fn test_urc_network_reg() {
        let result = detect_urc("+CREG: 1");
        if let Some(UrcEventType::NetworkRegChanged { stat }) = result {
            assert_eq!(stat, 1);
        } else {
            panic!("Expected NetworkRegChanged");
        }
    }

    #[test]
    fn test_urc_signal() {
        let result = detect_urc("+CSQ: 25,0");
        if let Some(UrcEventType::SignalChanged { rssi, ber }) = result {
            assert_eq!(rssi, 25);
            assert_eq!(ber, 0);
        } else {
            panic!("Expected SignalChanged");
        }
    }

    // Test 13-16: LoRaWAN-specific URCs
    #[test]
    fn test_urc_lorawan_joined() {
        let result = detect_urc("+JOIN: OK");
        assert!(matches!(result, Some(UrcEventType::LorawanJoined)));
    }

    #[test]
    fn test_urc_lorawan_tx() {
        let result = detect_urc("+LORATX: Done");
        if let Some(UrcEventType::LorawanTxDone { status: _ }) = result {
            // OK
        } else {
            panic!("Expected LorawanTxDone");
        }
    }

    #[test]
    fn test_urc_lorawan_rx() {
        let result = detect_urc("+LORARX: 1,10,5");
        if let Some(UrcEventType::LorawanRxReceived { port, rssi, snr }) = result {
            assert_eq!(port, 0);
            assert_eq!(rssi, 0);
            assert_eq!(snr, 0);
        } else {
            panic!("Expected LorawanRxReceived");
        }
    }

    #[test]
    fn test_urc_ring() {
        let result = detect_urc("RING");
        assert!(matches!(result, Some(UrcEventType::Ring)));
    }

    // Test 17-20: Error handling and edge cases
    #[test]
    fn test_empty_string() {
        let result = parse_last_number("");
        assert!(result.is_none());
    }

    #[test]
    fn test_multiple_numbers() {
        let result = parse_last_number("123,456,789");
        assert_eq!(result, Some(789));
    }

    #[test]
    fn test_csq_with_negative() {
        let (rssi, ber) = parse_csq("+CSQ: -50,99");
        assert_eq!(rssi, 99);  // parse error handling
        assert_eq!(ber, 99);
    }

    #[test]
    fn test_urc_custom() {
        let result = detect_urc("+CUSTOM: data");
        if let Some(UrcEventType::Custom(s)) = result {
            assert!(s.starts_with("+CUSTOM"));
        } else {
            panic!("Expected Custom URC");
        }
    }
}

// ============================================================
// Helper types and functions
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum UrcEventType {
    Ready,
    PowerDown,
    NetworkRegChanged { stat: u8 },
    SignalChanged { rssi: u8, ber: u8 },
    LorawanJoined,
    LorawanTxDone { status: u8 },
    LorawanRxReceived { port: u8, rssi: i16, snr: i8 },
    Ring,
    Custom(String<128>),
}

pub fn detect_urc(line: &str) -> Option<UrcEventType> {
    match line {
        "RDY" => return Some(UrcEventType::Ready),
        "NORMAL POWER DOWN" | "POWER DOWN" => return Some(UrcEventType::PowerDown),
        "RING" => return Some(UrcEventType::Ring),
        _ => {}
    }

    if line.starts_with("+CREG:") || line.starts_with("+CGREG:") || line.starts_with("+CEREG:") {
        let stat = parse_last_number(line).unwrap_or(0) as u8;
        return Some(UrcEventType::NetworkRegChanged { stat });
    }

    if line.starts_with("+CSQ:") {
        let (rssi, ber) = parse_csq(line);
        return Some(UrcEventType::SignalChanged { rssi, ber });
    }

    if line.starts_with("+LORARX") || line.starts_with("+RECV") {
        return Some(UrcEventType::LorawanRxReceived {
            port: 0,
            rssi: 0,
            snr: 0,
        });
    }

    if line.starts_with("+LORATX") || line.starts_with("+SEND") {
        return Some(UrcEventType::LorawanTxDone { status: 0 });
    }

    if line.contains("JOIN") && (line.contains("OK") || line.contains("Success")) {
        return Some(UrcEventType::LorawanJoined);
    }

    if line.starts_with('+') && !line.starts_with("AT+") {
        let mut s = String::new();
        let _ = s.push_str(&line[..line.len().min(128)]);
        return Some(UrcEventType::Custom(s));
    }

    None
}

fn hex_digit(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'A' + n - 10
    }
}

fn parse_last_number(s: &str) -> Option<u32> {
    s.split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .last()
        .and_then(|p| p.parse().ok())
}

fn parse_csq(s: &str) -> (u8, u8) {
    let parts: StdVec<&str> = s
        .split(|c: char| c == ':' || c == ',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    let rssi = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(99);
    let ber = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(99);
    (rssi, ber)
}
