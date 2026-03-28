//! LLC (Logical Link Control) sublayer
//!
//! Reference: Green Book Ed.9 §8.4.1
//! LLC adds 3-byte header before HDLC information field:
//! - Server → Client: 0xE6 E7 00
//! - Client → Server: 0xE6 E6 00 (or 0xE6 E7 00 for management)

extern crate alloc;
use alloc::vec::Vec;

/// LLC header for server → client
pub const LLC_SERVER_TO_CLIENT: [u8; 3] = [0xE6, 0xE7, 0x00];
/// LLC header for client → server
pub const LLC_CLIENT_TO_SERVER: [u8; 3] = [0xE6, 0xE6, 0x00];

/// Add LLC header to payload
pub fn add_llc_header(client_to_server: bool, payload: &[u8]) -> Vec<u8> {
    let mut result = alloc::vec::Vec::with_capacity(3 + payload.len());
    if client_to_server {
        result.extend_from_slice(&LLC_CLIENT_TO_SERVER);
    } else {
        result.extend_from_slice(&LLC_SERVER_TO_CLIENT);
    }
    result.extend_from_slice(payload);
    result
}

/// Strip LLC header and return payload
pub fn strip_llc_header(data: &[u8]) -> Option<&[u8]> {
    if data.len() < 3 { return None; }
    // Validate LLC header
    if data[0] != 0xE6 { return None; }
    if data[1] != 0xE6 && data[1] != 0xE7 { return None; }
    Some(&data[3..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_strip_client() {
        let payload = [0x01, 0x02, 0x03];
        let with_llc = add_llc_header(true, &payload);
        assert_eq!(&with_llc[..3], &[0xE6, 0xE6, 0x00]);
        let stripped = strip_llc_header(&with_llc).unwrap();
        assert_eq!(stripped, payload);
    }

    #[test]
    fn test_add_strip_server() {
        let payload = [0xAA, 0xBB];
        let with_llc = add_llc_header(false, &payload);
        assert_eq!(&with_llc[..3], &[0xE6, 0xE7, 0x00]);
        let stripped = strip_llc_header(&with_llc).unwrap();
        assert_eq!(stripped, payload);
    }
}
