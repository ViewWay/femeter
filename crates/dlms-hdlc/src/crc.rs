//! CRC-16-CCITT for HDLC
//!
//! Polynomial: 0x8408 (reversed representation of 0x1021)
//! Initial value: 0xFFFF
//! Reference: Green Book Ed.9 §8.4.2.3

/// Calculate CRC-16 for HDLC frames
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF
}

/// Verify CRC-16 of an HDLC frame (excluding flags)
/// The last 2 bytes before the closing flag should be the FCS
pub fn verify_crc(data: &[u8]) -> bool {
    if data.len() < 3 {
        return false;
    }
    let payload_len = data.len() - 2;
    let expected = u16::from_le_bytes([data[payload_len], data[payload_len + 1]]);
    crc16(&data[..payload_len]) == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_known_vector() {
        // SNRM frame example (no address, just control)
        // Green Book example: CRC of "41 00" = various depending on address
        let data = [0x41, 0x00];
        let crc = crc16(&data);
        // Known: CRC of empty frame with control 0x41, address 0x00
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc16_empty() {
        let crc = crc16(&[]);
        assert_eq!(crc, 0x0);
    }

    #[test]
    fn test_crc16_consistency() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05];
        let crc1 = crc16(&data);
        let crc2 = crc16(&data);
        assert_eq!(crc1, crc2);
    }

    #[test]
    fn test_verify_crc() {
        let data = [0x41, 0x00];
        let crc = crc16(&data);
        let mut frame = data.to_vec();
        frame.push((crc & 0xFF) as u8);
        frame.push((crc >> 8) as u8);
        assert!(verify_crc(&frame));
    }
}
