//! HDLC address encoding/decoding
//!
//! Reference: Green Book Ed.9 §8.4.2.2
//! Extended addressing: bit0=0 means more bytes follow, bit0=1 = last byte

use dlms_core::errors::HdlcError;
use alloc::vec::Vec;

/// HDLC address (client + server upper/lower)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdlcAddress {
    /// Client address (1 byte, typically 0x01-0x7F)
    pub client: u8,
    /// Server upper address (logical device, typically 0x0001)
    pub server_upper: u16,
    /// Server lower address (physical device, typically 0x00)
    pub server_lower: u16,
}

impl HdlcAddress {
    pub const fn new(client: u8, server_upper: u16, server_lower: u16) -> Self {
        Self { client, server_upper, server_lower }
    }

    /// Default meter address (client=1, logical device=1, physical=0)
    pub const fn default_meter() -> Self {
        Self::new(1, 1, 0)
    }
}

/// Encode HDLC address to bytes (extended addressing)
/// Returns encoded bytes with HDLC extension bits
pub fn encode_address(addr: &HdlcAddress) -> Vec<u8> {
    let mut bytes = Vec::new();

    // Client address (1 byte, bit0=1 = last byte)
    bytes.push((addr.client << 1) | 0x01);

    // Server address (upper + lower combined)
    let server = ((addr.server_upper as u32) << 16) | (addr.server_lower as u32);
    if server <= 0x7F {
        // 1 byte
        bytes.push(((server as u8) << 1) | 0x01);
    } else if server <= 0x3FFF {
        // 2 bytes
        bytes.push(((server >> 7) as u8) << 1); // bit0=0, more follows
        bytes.push(((server & 0x7F) as u8) << 1 | 0x01);
    } else {
        // 4 bytes
        bytes.push(((server >> 21) as u8) << 1);
        bytes.push((((server >> 14) & 0x7F) as u8) << 1);
        bytes.push((((server >> 7) & 0x7F) as u8) << 1);
        bytes.push(((server & 0x7F) as u8) << 1 | 0x01);
    }

    bytes
}

/// Decode HDLC address from bytes
/// Returns (address, bytes_consumed)
pub fn decode_address(data: &[u8]) -> Result<(HdlcAddress, usize), HdlcError> {
    if data.len() < 2 { return Err(HdlcError::AddressError); }

    // Client address (1 byte)
    let client = data[0] >> 1;
    let mut pos = 1;

    // Server address (variable length)
    let mut server: u32 = 0;
    let server_start = pos;
    loop {
        if pos >= data.len() { return Err(HdlcError::AddressError); }
        server = (server << 7) | ((data[pos] >> 1) as u32);
        if data[pos] & 0x01 != 0 {
            pos += 1;
            break;
        }
        pos += 1;
        if pos - server_start > 4 { return Err(HdlcError::AddressError); }
    }

    // Split server into upper (logical device) and lower (physical device)
    let server_upper = (server >> 16) as u16;
    let server_lower = (server & 0xFFFF) as u16;

    Ok((HdlcAddress { client, server_upper, server_lower }, pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_simple() {
        let addr = HdlcAddress::new(1, 1, 0);
        let encoded = encode_address(&addr);
        let (decoded, consumed) = decode_address(&encoded).unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded, addr);
    }

    #[test]
    fn test_client_address_encoding() {
        // Client=1 → byte = (1<<1)|1 = 0x03
        let addr = HdlcAddress::new(1, 0, 0);
        let encoded = encode_address(&addr);
        assert_eq!(encoded[0], 0x03);
    }

    #[test]
    fn test_server_address_1byte() {
        // Server=1 → byte = (1<<1)|1 = 0x03
        let addr = HdlcAddress::new(1, 0, 1);
        let encoded = encode_address(&addr);
        assert_eq!(encoded[1], 0x03);
    }

    #[test]
    fn test_roundtrip_various() {
        let addrs = [
            HdlcAddress::new(1, 1, 0),
            HdlcAddress::new(0x10, 0x0001, 0),
            HdlcAddress::new(1, 0, 0x1234),
        ];
        for addr in &addrs {
            let encoded = encode_address(addr);
            let (decoded, _) = decode_address(&encoded).unwrap();
            assert_eq!(decoded, *addr);
        }
    }
}
