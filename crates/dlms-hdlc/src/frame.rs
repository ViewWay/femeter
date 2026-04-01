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

        // Calculate HCS (CRC of address + control)
        self.hcs = crc16(&raw);
        raw.push((self.hcs & 0xFF) as u8);
        raw.push((self.hcs >> 8) as u8);

        // Add information field (if present)
        if !self.information.is_empty() {
            raw.extend_from_slice(&self.information);
        }

        // Calculate FCS (CRC of everything so far)
        self.fcs = crc16(&raw);
        raw.push((self.fcs & 0xFF) as u8);
        raw.push((self.fcs >> 8) as u8);

        // Apply byte stuffing and add flags
        let mut result = vec![HDLC_FLAG];
        for &b in &raw {
            if b == HDLC_FLAG || b == HDLC_ESCAPE {
                result.push(HDLC_ESCAPE);
                result.push(b ^ HDLC_ESCAPE_MASK);
            } else {
                result.push(b);
            }
        }
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

        // Remove byte stuffing
        let mut raw = Vec::new();
        let mut i = 1; // skip opening flag
        while i < data.len() - 1 {
            // skip closing flag
            if data[i] == HDLC_ESCAPE {
                if i + 1 >= data.len() - 1 {
                    return Err(HdlcError::InvalidFrameFormat);
                }
                raw.push(data[i + 1] ^ HDLC_ESCAPE_MASK);
                i += 2;
            } else if data[i] == HDLC_FLAG {
                break; // End of frame
            } else {
                raw.push(data[i]);
                i += 1;
            }
        }

        if raw.len() < 5 {
            return Err(HdlcError::InvalidFrameFormat);
        }

        // Verify FCS
        let payload_len = raw.len() - 2;
        let fcs = u16::from_le_bytes([raw[payload_len], raw[payload_len + 1]]);
        if crc16(&raw[..payload_len]) != fcs {
            return Err(HdlcError::CrcError);
        }

        // Parse address
        let (address, addr_consumed) = decode_address(&raw[..payload_len])?;

        // Parse control
        let ctrl_byte = raw[addr_consumed];
        let control = ControlField::decode(ctrl_byte);

        // Parse HCS (2 bytes after address + control)
        let hcs_start = addr_consumed + 1;
        if hcs_start + 2 > payload_len {
            return Err(HdlcError::InvalidFrameFormat);
        }
        let hcs = u16::from_le_bytes([raw[hcs_start], raw[hcs_start + 1]]);

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
}
