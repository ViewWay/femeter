//! Compact array codec
//!
//! Reference: Green Book Ed.9 §9.5

use crate::AxdrError;

/// Compact array element type descriptor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactArrayCodec;

impl CompactArrayCodec {
    /// Calculate the byte size of a compact array element given its type tag
    pub fn element_size(tag: u8) -> Option<usize> {
        match tag {
            0x0F => Some(1), // integer (int8)
            0x10 => Some(2), // long (int16)
            0x05 => Some(4), // double-long (int32)
            0x06 => Some(4), // double-long-unsigned (uint32)
            0x11 => Some(1), // unsigned (uint8)
            0x12 => Some(2), // long-unsigned (uint16)
            0x14 => Some(8), // long64 (int64)
            0x15 => Some(8), // long64-unsigned (uint64)
            0x16 => Some(1), // enum
            0x17 => Some(4), // float32
            0x18 => Some(8), // float64
            _ => None,
        }
    }

    /// Validate compact array data against element type and count
    pub fn validate(tag: u8, count: u32, data: &[u8]) -> Result<(), AxdrError> {
        let elem_size = Self::element_size(tag).ok_or(AxdrError::InvalidTag(tag))?;
        let expected = elem_size * (count as usize);
        if data.len() < expected {
            return Err(AxdrError::BufferTooShort);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_size() {
        assert_eq!(CompactArrayCodec::element_size(0x11), Some(1)); // uint8
        assert_eq!(CompactArrayCodec::element_size(0x06), Some(4)); // uint32
        assert_eq!(CompactArrayCodec::element_size(0x17), Some(4)); // float32
        assert_eq!(CompactArrayCodec::element_size(0xFF), None);
    }

    #[test]
    fn test_validate() {
        // 3 uint8 values = 3 bytes
        assert!(CompactArrayCodec::validate(0x11, 3, &[1, 2, 3]).is_ok());
        // Too few bytes
        assert!(CompactArrayCodec::validate(0x11, 3, &[1, 2]).is_err());
    }
}
