//! Selective access descriptor for Get operations
//!
//! Reference: IEC 62056-53 §8.4.3

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::vec::Vec;
use alloc::vec;
use dlms_core::DlmsType;
use crate::types::{ApduError, SELECTIVE_ACCESS_BY_RANGE, SELECTIVE_ACCESS_BY_ENTRY};

/// Selective access descriptor for reading partial data from arrays/profiles
#[derive(Debug, Clone, PartialEq)]
pub enum SelectiveAccess {
    /// Access by range (from index, to index)
    ByRange { from: u32, to: u32 },
    /// Access by specific entry indices
    ByEntry { indices: Vec<u32> },
}

impl SelectiveAccess {
    /// Create a range-based selective access
    pub fn range(from: u32, to: u32) -> Self {
        Self::ByRange { from, to }
    }

    /// Create an entry-based selective access
    pub fn entry(indices: Vec<u32>) -> Self {
        Self::ByEntry { indices }
    }

    /// Check if the range is valid (from <= to)
    pub fn is_valid(&self) -> bool {
        match self {
            Self::ByRange { from, to } => from <= to,
            Self::ByEntry { indices } => !indices.is_empty(),
        }
    }

    /// Get the access type code
    pub fn access_type(&self) -> u8 {
        match self {
            Self::ByRange { .. } => SELECTIVE_ACCESS_BY_RANGE,
            Self::ByEntry { .. } => SELECTIVE_ACCESS_BY_ENTRY,
        }
    }

    /// Calculate the encoded size
    pub fn encoded_size(&self) -> usize {
        match self {
            Self::ByRange { .. } => {
                // access selector(1) + from(4) + to(4) + parameter count(1) + 2 entries(8) = ~18 bytes
                // Actually the structure is:
                // access_selector(1) + access_parameters:
                //   - for range: from(4) + to(4) + entry_count_encoded(1) + 2*entry_count(1 each)
                1 + 4 + 4 + 1 + 2 // simplified
            }
            Self::ByEntry { indices } => {
                // access_selector(1) + entry_count(1) + indices(4 each)
                1 + 1 + indices.len() * 4
            }
        }
    }

    /// Encode the selective access descriptor to bytes using A-XDR encoding
    pub fn encode(&self, buf: &mut Vec<u8>) -> Result<(), ApduError> {
        buf.push(self.access_type());

        match self {
            Self::ByRange { from, to } => {
                // Encode from as uint32
                buf.extend_from_slice(&from.to_be_bytes());
                // Encode to as uint32
                buf.extend_from_slice(&to.to_be_bytes());

                // Encode the number of entries (always 2 for range: from, to)
                // In DLMS, range access parameters are encoded as:
                // - from (uint32)
                // - to (uint32)
                // - number of access parameters (uint8) = 2
                // - access parameter 1 (uint8) = entry number for from
                // - access parameter 2 (uint8) = entry number for to
                buf.push(2); // parameter count
                buf.push(1); // from entry number
                buf.push(2); // to entry number
            }
            Self::ByEntry { indices } => {
                // Number of indices
                let count = indices.len();
                if count > 255 {
                    return Err(ApduError::InvalidLength);
                }
                buf.push(count as u8);

                // Each index as uint32
                for &idx in indices {
                    buf.extend_from_slice(&idx.to_be_bytes());
                }
            }
        }

        Ok(())
    }

    /// Decode selective access from bytes
    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        if data.is_empty() {
            return Err(ApduError::TooShort);
        }

        let access_type = data[0];
        let mut pos = 1;

        match access_type {
            SELECTIVE_ACCESS_BY_RANGE => {
                // Need: from(4) + to(4) + param_count(1) + params(2)
                if data.len() < 11 {
                    return Err(ApduError::TooShort);
                }

                let from = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                pos += 4;
                let to = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                pos += 4;
                let _param_count = data[pos]; // should be 2

                Ok(Self::ByRange { from, to })
            }
            SELECTIVE_ACCESS_BY_ENTRY => {
                if data.len() < 2 {
                    return Err(ApduError::TooShort);
                }

                let count = data[pos] as usize;
                pos += 1;

                if data.len() < pos + count * 4 {
                    return Err(ApduError::TooShort);
                }

                let mut indices = Vec::with_capacity(count);
                for _ in 0..count {
                    let idx = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                    pos += 4;
                    indices.push(idx);
                }

                Ok(Self::ByEntry { indices })
            }
            _ => Err(ApduError::InvalidData),
        }
    }
}

/// Apply selective access to an array data, returning the selected elements
pub fn apply_selective_access(data: &[DlmsType], selective: &SelectiveAccess) -> Result<Vec<DlmsType>, ApduError> {
    let result = match selective {
        SelectiveAccess::ByRange { from, to } => {
            let from_idx = *from as usize;
            let to_idx = *to as usize;

            if from_idx > to_idx || to_idx >= data.len() {
                return Err(ApduError::InvalidRange);
            }

            data[from_idx..=to_idx].to_vec()
        }
        SelectiveAccess::ByEntry { indices } => {
            let mut result = Vec::with_capacity(indices.len());
            for &idx in indices {
                let idx_usize = idx as usize;
                if idx_usize >= data.len() {
                    return Err(ApduError::InvalidRange);
                }
                result.push(data[idx_usize].clone());
            }
            result
        }
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selective_range_encode_decode() {
        let range = SelectiveAccess::range(0, 9);

        let mut encoded = Vec::new();
        range.encode(&mut encoded).unwrap();

        let decoded = SelectiveAccess::decode(&encoded).unwrap();
        assert_eq!(range, decoded);
    }

    #[test]
    fn test_selective_entry_encode_decode() {
        let entry = SelectiveAccess::entry(vec![0, 5, 10]);

        let mut encoded = Vec::new();
        entry.encode(&mut encoded).unwrap();

        let decoded = SelectiveAccess::decode(&encoded).unwrap();
        assert_eq!(entry, decoded);
    }

    #[test]
    fn test_selective_range_valid() {
        assert!(SelectiveAccess::range(0, 10).is_valid());
        assert!(!SelectiveAccess::range(10, 0).is_valid());
    }

    #[test]
    fn test_selective_entry_valid() {
        assert!(SelectiveAccess::entry(vec![1, 2, 3]).is_valid());
        assert!(!SelectiveAccess::entry(vec![]).is_valid());
    }

    #[test]
    fn test_apply_selective_range() {
        let data = vec![
            DlmsType::from_u8(0),
            DlmsType::from_u8(1),
            DlmsType::from_u8(2),
            DlmsType::from_u8(3),
            DlmsType::from_u8(4),
        ];

        let selective = SelectiveAccess::range(1, 3);
        let result = apply_selective_access(&data, &selective).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], DlmsType::from_u8(1));
        assert_eq!(result[1], DlmsType::from_u8(2));
        assert_eq!(result[2], DlmsType::from_u8(3));
    }

    #[test]
    fn test_apply_selective_entry() {
        let data = vec![
            DlmsType::from_u8(0),
            DlmsType::from_u8(1),
            DlmsType::from_u8(2),
            DlmsType::from_u8(3),
            DlmsType::from_u8(4),
        ];

        let selective = SelectiveAccess::entry(vec![0, 2, 4]);
        let result = apply_selective_access(&data, &selective).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], DlmsType::from_u8(0));
        assert_eq!(result[1], DlmsType::from_u8(2));
        assert_eq!(result[2], DlmsType::from_u8(4));
    }

    #[test]
    fn test_apply_selective_invalid_range() {
        let data = vec![DlmsType::from_u8(0), DlmsType::from_u8(1)];

        let selective = SelectiveAccess::range(0, 10);
        assert!(apply_selective_access(&data, &selective).is_err());
    }
}
