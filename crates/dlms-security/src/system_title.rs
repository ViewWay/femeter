//! System Title handling
//!
//! System Title is an 8-byte identifier for DLMS devices.
//! Reference: IEC 62056-6-2 §8.3

#[cfg(feature = "std")]
use core::fmt;

/// System title length in bytes
pub const SYSTEM_TITLE_LEN: usize = 8;

/// System Title - 8-byte unique identifier for a DLMS device
///
/// Used as:
/// - Part of the nonce for AES-GCM encryption
/// - Device identification in security contexts
/// - Component in authentication calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SystemTitle {
    bytes: [u8; SYSTEM_TITLE_LEN],
}

impl SystemTitle {
    /// Create a new SystemTitle from a byte array
    #[inline]
    pub const fn new(bytes: [u8; SYSTEM_TITLE_LEN]) -> Self {
        Self { bytes }
    }

    /// Create a new SystemTitle from a slice
    ///
    /// Returns None if the slice is not exactly 8 bytes
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() == SYSTEM_TITLE_LEN {
            let mut bytes = [0u8; SYSTEM_TITLE_LEN];
            bytes.copy_from_slice(slice);
            Some(Self { bytes })
        } else {
            None
        }
    }

    /// Get the system title as a byte slice
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; SYSTEM_TITLE_LEN] {
        &self.bytes
    }

    /// Get the length (always 8)
    #[inline]
    pub const fn len(&self) -> usize {
        SYSTEM_TITLE_LEN
    }

    /// Check if empty (all zeros)
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bytes.iter().all(|&b| b == 0)
    }

    /// Convert to hex string representation
    ///
    /// Only available with std feature for Display formatting
    #[cfg(feature = "std")]
    pub fn to_hex_string(&self) -> alloc::string::String {
        use alloc::format;
        self.bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect()
    }

    /// Create from u64 value (big-endian encoding)
    #[inline]
    pub const fn from_u64(value: u64) -> Self {
        Self {
            bytes: [
                (value >> 56) as u8,
                (value >> 48) as u8,
                (value >> 40) as u8,
                (value >> 32) as u8,
                (value >> 24) as u8,
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
            ],
        }
    }

    /// Convert to u64 value (big-endian encoding)
    #[inline]
    pub const fn to_u64(&self) -> u64 {
        ((self.bytes[0] as u64) << 56)
            | ((self.bytes[1] as u64) << 48)
            | ((self.bytes[2] as u64) << 40)
            | ((self.bytes[3] as u64) << 32)
            | ((self.bytes[4] as u64) << 24)
            | ((self.bytes[5] as u64) << 16)
            | ((self.bytes[6] as u64) << 8)
            | (self.bytes[7] as u64)
    }
}

impl Default for SystemTitle {
    fn default() -> Self {
        Self { bytes: [0u8; SYSTEM_TITLE_LEN] }
    }
}

impl From<[u8; SYSTEM_TITLE_LEN]> for SystemTitle {
    fn from(bytes: [u8; SYSTEM_TITLE_LEN]) -> Self {
        Self::new(bytes)
    }
}

impl From<&[u8; SYSTEM_TITLE_LEN]> for SystemTitle {
    fn from(bytes: &[u8; SYSTEM_TITLE_LEN]) -> Self {
        Self::new(*bytes)
    }
}

impl From<SystemTitle> for [u8; SYSTEM_TITLE_LEN] {
    fn from(st: SystemTitle) -> Self {
        st.bytes
    }
}

impl AsRef<[u8]> for SystemTitle {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(feature = "std")]
impl fmt::Display for SystemTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3],
            self.bytes[4], self.bytes[5], self.bytes[6], self.bytes[7])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_title_creation() {
        let bytes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let st = SystemTitle::new(bytes);
        assert_eq!(st.as_bytes(), &bytes);
    }

    #[test]
    fn test_system_title_from_slice() {
        let valid_slice = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let st = SystemTitle::from_slice(valid_slice);
        assert!(st.is_some());

        let short_slice = &[0x01, 0x02, 0x03];
        let st = SystemTitle::from_slice(short_slice);
        assert!(st.is_none());
    }

    #[test]
    fn test_system_title_default() {
        let st = SystemTitle::default();
        assert!(st.is_empty());
        assert_eq!(st.as_bytes(), &[0u8; 8]);
    }

    #[test]
    fn test_system_title_from_u64() {
        let value = 0x0102030405060708u64;
        let st = SystemTitle::from_u64(value);
        assert_eq!(st.as_bytes(), &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    }

    #[test]
    fn test_system_title_to_u64() {
        let bytes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let st = SystemTitle::new(bytes);
        assert_eq!(st.to_u64(), 0x0102030405060708u64);
    }

    #[test]
    fn test_system_title_roundtrip_u64() {
        let value = 0xDEADBEEFCAFEBABEu64;
        let st = SystemTitle::from_u64(value);
        assert_eq!(st.to_u64(), value);
    }

    #[test]
    fn test_system_title_from_array() {
        let bytes: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];
        let st: SystemTitle = bytes.into();
        assert_eq!(st.as_bytes(), &bytes);
    }

    #[test]
    fn test_system_title_into_array() {
        let bytes: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];
        let st = SystemTitle::new(bytes);
        let into_bytes: [u8; 8] = st.into();
        assert_eq!(into_bytes, bytes);
    }
}
