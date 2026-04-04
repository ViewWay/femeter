//! HDLC frame structure and parsing
//!
//! Reference: Green Book Ed.9 §8.4

use crate::address::{decode_address, HdlcAddress};
use crate::control::{ControlField, FrameType};
use crate::crc::crc16;
use alloc::vec;
use alloc::vec::Vec;
use dlms_core::errors::HdlcError;

/// HDLC flag byte
pub const HDLC_FLAG: u8 = 0x7E;
/// HDLC escape byte
pub const HDLC_ESCAPE: u8 = 0x7D;
/// HDLC escape XOR mask
pub const HDLC_ESCAPE_MASK: u8 = 0x20;

/// Maximum frame size (default)
pub const DEFAULT_MAX_FRAME_SIZE: usize = 128;

/// HDLC frame
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HdlcFrame {
    pub address: HdlcAddress,
    pub control: ControlField,
    /// HCS (Header Check Sequence) — CRC of address + control
    pub hcs: u16,
    /// Information field (payload)
    pub information: Vec<u8>,
    /// FCS (Frame Check Sequence) — CRC of entire frame
    pub fcs: u16,
}

impl HdlcFrame {
    /// Create a new HDLC frame
    pub fn new(address: HdlcAddress, control: ControlField, information: Vec<u8>) -> Self {
        let hcs = 0; // calculated during encoding
        let fcs = 0; // calculated during encoding
        Self {
            address,
            control,
            hcs,
            fcs,
            information,
        }
    }

    /// Check if this is an I-frame (carries data)
    pub fn is_information(&self) -> bool {
        self.control.frame_type == FrameType::I
    }

    /// Check if this frame has the poll bit set
    pub fn is_poll(&self) -> bool {
        self.control.poll_final
    }

    /// Encode the complete frame to bytes (with flags and byte stuffing)
    pub fn encode(&mut self) -> Vec<u8> {
        let mut raw = Vec::new();

        // Encode address
        let addr_bytes = crate::address::encode_address(&self.address);
        raw.extend_from_slice(&addr_bytes);

        // Encode control
        raw.push(self.control.encode());

        // Calculate HCS (CRC of address + control) — big-endian
        self.hcs = crc16(&raw);
        raw.push((self.hcs >> 8) as u8);
        raw.push((self.hcs & 0xFF) as u8);

        // Add information field (if present)
        if !self.information.is_empty() {
            raw.extend_from_slice(&self.information);
        }

        // Calculate FCS (CRC of everything so far) — big-endian
        self.fcs = crc16(&raw);
        raw.push((self.fcs >> 8) as u8);
        raw.push((self.fcs & 0xFF) as u8);

        // Byte stuffing: escape 0x7E → 0x7D 0x5E, 0x7D → 0x7D 0x5D
        let mut stuffed = Vec::new();
        for &b in &raw {
            match b {
                HDLC_FLAG => {
                    stuffed.push(HDLC_ESCAPE);
                    stuffed.push(0x5E);
                }
                HDLC_ESCAPE => {
                    stuffed.push(HDLC_ESCAPE);
                    stuffed.push(0x5D);
                }
                _ => stuffed.push(b),
            }
        }
        let mut result = vec![HDLC_FLAG];
        result.extend_from_slice(&stuffed);
        result.push(HDLC_FLAG);
        result
    }

    /// Decode a frame from bytes (with flags and byte unstuffing)
    pub fn decode(data: &[u8]) -> Result<Self, HdlcError> {
        // Remove flags
        if data.len() < 4 {
            return Err(HdlcError::InvalidFrameFormat);
        }
        if data[0] != HDLC_FLAG || data[data.len() - 1] != HDLC_FLAG {
            return Err(HdlcError::InvalidFlag);
        }

        // Remove byte stuffing: 0x7D 0x5E → 0x7E, 0x7D 0x5D → 0x7D
        let mut raw = Vec::new();
        let mut i = 1; // skip opening flag
        while i < data.len() - 1 {
            if data[i] == HDLC_FLAG {
                break;
            } else if data[i] == HDLC_ESCAPE {
                if i + 1 >= data.len() - 1 {
                    return Err(HdlcError::InvalidFrameFormat);
                }
                match data[i + 1] {
                    0x5E => raw.push(HDLC_FLAG),
                    0x5D => raw.push(HDLC_ESCAPE),
                    _ => raw.push(data[i + 1] ^ 0x20),
                }
                i += 2;
            } else {
                raw.push(data[i]);
                i += 1;
            }
        }

        if raw.len() < 5 {
            return Err(HdlcError::InvalidFrameFormat);
        }

        // Verify FCS (big-endian)
        let payload_len = raw.len() - 2;
        let fcs = u16::from_be_bytes([raw[payload_len], raw[payload_len + 1]]);
        if crc16(&raw[..payload_len]) != fcs {
            return Err(HdlcError::CrcError);
        }

        // Parse address
        let (address, addr_consumed) = decode_address(&raw[..payload_len])?;

        // Parse control
        let ctrl_byte = raw[addr_consumed];
        let control = ControlField::decode(ctrl_byte);

        // Parse HCS (2 bytes after address + control) — big-endian
        let hcs_start = addr_consumed + 1;
        if hcs_start + 2 > payload_len {
            // No HCS for frames without information (like SNRM)
            // Information is empty
            return Ok(Self {
                address,
                control,
                hcs: 0,
                fcs,
                information: Vec::new(),
            });
        }
        let hcs = u16::from_be_bytes([raw[hcs_start], raw[hcs_start + 1]]);

        // Verify HCS
        let hcs_calc = crc16(&raw[..hcs_start]);
        if hcs_calc != hcs {
            return Err(HdlcError::CrcError);
        }

        // Information field (between HCS and FCS)
        let info_start = hcs_start + 2;
        let info_end = payload_len;
        let information = if info_start < info_end {
            raw[info_start..info_end].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            address,
            control,
            hcs,
            fcs,
            information,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::HdlcAddress;

    #[test]
    fn test_frame_encode_decode_ua() {
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::ua(true);
        let mut frame = HdlcFrame::new(addr, ctrl, Vec::new());
        let bytes = frame.encode();

        let decoded = HdlcFrame::decode(&bytes).unwrap();
        assert_eq!(decoded.address.client, 1);
        assert_eq!(decoded.control.frame_type, FrameType::UA);
        assert!(decoded.information.is_empty());
    }

    #[test]
    fn test_frame_with_data() {
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::information(0, 0, true);
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let mut frame = HdlcFrame::new(addr, ctrl, data);
        let bytes = frame.encode();

        let decoded = HdlcFrame::decode(&bytes).unwrap();
        assert_eq!(decoded.control.frame_type, FrameType::I);
        assert_eq!(decoded.information, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_byte_stuffing() {
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::information(0, 0, false);
        // Data containing 0x7E (flag) and 0x7D (escape)
        let data = vec![0x7E, 0x7D, 0x01];
        let mut frame = HdlcFrame::new(addr, ctrl, data);
        let bytes = frame.encode();

        // Should be longer due to stuffing
        assert!(bytes.len() > 10);

        let decoded = HdlcFrame::decode(&bytes).unwrap();
        assert_eq!(decoded.information, vec![0x7E, 0x7D, 0x01]);
    }

    #[test]
    fn test_invalid_flag() {
        let data = [0x00, 0x01, 0x02]; // no flags
        assert!(HdlcFrame::decode(&data).is_err());
    }

    #[test]
    fn test_crc_error() {
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::ua(true);
        let mut frame = HdlcFrame::new(addr, ctrl, Vec::new());
        let mut bytes = frame.encode();
        // Corrupt a byte in the middle
        if bytes.len() > 4 {
            bytes[2] ^= 0xFF;
        }
        assert!(HdlcFrame::decode(&bytes).is_err());
    }

    // ============================================================
    // Phase C — Boundary Tests
    // ============================================================

    #[test]
    fn test_max_frame_size() {
        // Test with maximum information field (128 bytes)
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::information(0, 0, true);
        let data = vec![0xAA; 128];
        let mut frame = HdlcFrame::new(addr, ctrl, data.clone());
        let bytes = frame.encode();
        let decoded = HdlcFrame::decode(&bytes).unwrap();
        assert_eq!(decoded.information.len(), 128);
        assert_eq!(decoded.information, data);
    }

    #[test]
    fn test_large_frame_size() {
        // Test with 200-byte payload (beyond default max)
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::information(0, 0, false);
        let data = vec![0xBB; 200];
        let mut frame = HdlcFrame::new(addr, ctrl, data.clone());
        let bytes = frame.encode();
        let decoded = HdlcFrame::decode(&bytes).unwrap();
        assert_eq!(decoded.information.len(), 200);
        assert_eq!(decoded.information, data);
    }

    #[test]
    fn test_empty_information() {
        // I-frame with empty information field
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::information(0, 0, false);
        let mut frame = HdlcFrame::new(addr, ctrl, Vec::new());
        let bytes = frame.encode();
        let decoded = HdlcFrame::decode(&bytes).unwrap();
        assert!(decoded.information.is_empty());
        assert_eq!(decoded.control.frame_type, FrameType::I);
    }

    #[test]
    fn test_sequence_numbers_wrap() {
        // Test sequence number wrapping (modulo 8)
        for seq in 0..8 {
            let ctrl = ControlField::information(seq, seq, false);
            let byte = ctrl.encode();
            let decoded = ControlField::decode(byte);
            assert_eq!(decoded.send_seq, seq);
            assert_eq!(decoded.recv_seq, seq);
        }
    }

    #[test]
    fn test_all_control_types() {
        let addr = HdlcAddress::new(1, 1, 0);
        let types = [
            ControlField::information(0, 0, true),
            ControlField::rr(0, true),
            ControlField::rnr(0, false),
            ControlField::snrm(true),
            ControlField::ua(true),
            ControlField::disc(true),
            ControlField::dm(false),
        ];
        for ctrl in &types {
            let mut frame = HdlcFrame::new(addr, *ctrl, Vec::new());
            let bytes = frame.encode();
            let decoded = HdlcFrame::decode(&bytes).unwrap();
            assert_eq!(decoded.control.frame_type, ctrl.frame_type);
        }
    }

    #[test]
    fn test_multiple_flags_between_frames() {
        // Multiple flag bytes between frames should not cause issues
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::ua(true);
        let mut frame = HdlcFrame::new(addr, ctrl, Vec::new());
        let mut bytes = frame.encode();
        // Insert extra flags
        let original = bytes.clone();
        bytes.insert(1, HDLC_FLAG);
        bytes.insert(2, HDLC_FLAG);
        // Decode should find the first valid frame
        // Find end flag after opening flag
        let end = bytes.iter().position(|&b| b == HDLC_FLAG).unwrap() + 1;
        let next_end = bytes[end..].iter().position(|&b| b == HDLC_FLAG);
        if let Some(offset) = next_end {
            let frame_bytes = &bytes[..end + offset + 1];
            // May fail due to extra flags, but shouldn't panic
            let _ = HdlcFrame::decode(frame_bytes);
        }
        // Verify original still works
        let decoded = HdlcFrame::decode(&original).unwrap();
        assert_eq!(decoded.control.frame_type, FrameType::UA);
    }

    #[test]
    fn test_data_with_all_special_bytes() {
        // Test frame with all bytes that need escaping: 0x7E and 0x7D
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::information(0, 0, false);
        let data = vec![0x7E, 0x7D, 0x7E, 0x7D, 0x00, 0xFF, 0x7E, 0x7D];
        let mut frame = HdlcFrame::new(addr, ctrl, data.clone());
        let bytes = frame.encode();
        let decoded = HdlcFrame::decode(&bytes).unwrap();
        assert_eq!(decoded.information, data);
    }

    #[test]
    fn test_too_short_frame() {
        assert!(HdlcFrame::decode(&[0x7E, 0x7E]).is_err());
        assert!(HdlcFrame::decode(&[0x7E]).is_err());
        assert!(HdlcFrame::decode(&[]).is_err());
    }

    #[test]
    fn test_missing_closing_flag() {
        let addr = HdlcAddress::new(1, 1, 0);
        let ctrl = ControlField::ua(true);
        let mut frame = HdlcFrame::new(addr, ctrl, Vec::new());
        let mut bytes = frame.encode();
        // Remove closing flag
        bytes.pop();
        assert!(HdlcFrame::decode(&bytes).is_err());
    }

    #[test]
    fn test_invalid_escape_sequence() {
        let data = [HDLC_FLAG, 0x03, 0x63, HDLC_ESCAPE, HDLC_FLAG];
        assert!(HdlcFrame::decode(&data).is_err());
    }

    #[test]
    fn test_broadcast_address() {
        // Client=0 is broadcast
        let addr = HdlcAddress::new(0, 0xFFFF, 0);
        let ctrl = ControlField::snrm(true);
        let mut frame = HdlcFrame::new(addr, ctrl, Vec::new());
        let bytes = frame.encode();
        let decoded = HdlcFrame::decode(&bytes).unwrap();
        assert_eq!(decoded.address.client, 0);
    }

    #[test]
    fn test_poll_final_bit_variants() {
        // Test all combinations of poll/final bit
        let addr = HdlcAddress::new(1, 1, 0);
        for pf in [true, false] {
            let ctrl = ControlField::information(0, 0, pf);
            let byte = ctrl.encode();
            let decoded = ControlField::decode(byte);
            assert_eq!(decoded.poll_final, pf);

            let ctrl = ControlField::rr(0, pf);
            let byte = ctrl.encode();
            let decoded = ControlField::decode(byte);
            assert_eq!(decoded.poll_final, pf);

            let ctrl = ControlField::ua(pf);
            let byte = ctrl.encode();
            let decoded = ControlField::decode(byte);
            assert_eq!(decoded.poll_final, pf);
        }
    }
}
