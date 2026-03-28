//! Security Control Field parsing and handling
//!
//! The Security Control Field is a single byte that controls security settings
//! for DLMS/COSEM communication.
//!
//! Format (bit 7 is MSB):
//! - Bit 7-6: Security Suite (00 = AES-GCM-128)
//! - Bit 5: Authentication (1 = authenticated)
//! - Bit 4: Encryption (1 = encrypted)
//! - Bit 3: Reserved (0)
//! - Bit 2-0: Key Selection (000 = global, 001 = dedicated, others reserved)

#[cfg(feature = "std")]
use core::fmt;

/// Mask for security suite bits (7-6)
pub const SECURITY_SUITE_MASK: u8 = 0b11000000;
/// Shift for security suite bits
pub const SECURITY_SUITE_SHIFT: u8 = 6;
/// Authentication flag bit
pub const AUTHENTICATED: u8 = 0b00100000;
/// Encryption flag bit
pub const ENCRYPTED: u8 = 0b00010000;
/// Mask for key selection bits (2-0)
pub const KEY_SELECTION_MASK: u8 = 0b00000111;

/// Security Suite enumeration
///
/// Defines the cryptographic algorithms used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum SecuritySuite {
    /// AES-GCM-128 (0)
    AesGcm128,
    /// Reserved for future use
    Reserved(u8),
}

impl SecuritySuite {
    /// Create from the 2-bit security suite field
    pub fn from_bits(value: u8) -> Self {
        match value {
            0 => Self::AesGcm128,
            v => Self::Reserved(v),
        }
    }

    /// Convert to the 2-bit value
    pub fn to_bits(&self) -> u8 {
        match self {
            Self::AesGcm128 => 0,
            Self::Reserved(v) => *v & 0b11,
        }
    }
}

/// Key Selection enumeration
///
/// Defines which key to use for encryption/authentication
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum KeySelection {
    /// Global key (unicast or broadcast)
    Global,
    /// Dedicated key (point-to-point)
    Dedicated,
    /// Reserved for future use
    Reserved(u8),
}

impl KeySelection {
    /// Create from the 3-bit key selection field
    pub fn from_bits(value: u8) -> Self {
        match value {
            0 => Self::Global,
            1 => Self::Dedicated,
            v => Self::Reserved(v),
        }
    }

    /// Convert to the 3-bit value
    pub fn to_bits(&self) -> u8 {
        match self {
            Self::Global => 0,
            Self::Dedicated => 1,
            Self::Reserved(v) => *v & 0b111,
        }
    }
}

/// Security Control Field
///
/// Represents the parsed security control byte
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SecurityControl {
    suite: SecuritySuite,
    authenticated: bool,
    encrypted: bool,
    key_selection: KeySelection,
}

impl SecurityControl {
    /// Create a new SecurityControl
    pub fn new(suite: SecuritySuite, authenticated: bool, encrypted: bool, key_selection: KeySelection) -> Self {
        Self {
            suite,
            authenticated,
            encrypted,
            key_selection,
        }
    }

    /// Get the security suite
    pub fn suite(&self) -> SecuritySuite {
        self.suite
    }

    /// Check if authentication is enabled
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Check if encryption is enabled
    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    /// Get the key selection
    pub fn key_selection(&self) -> KeySelection {
        self.key_selection
    }

    /// Convert to a single byte
    pub fn as_byte(&self) -> u8 {
        let mut byte = 0u8;

        // Security suite (bits 7-6)
        byte |= (self.suite.to_bits() << SECURITY_SUITE_SHIFT) & SECURITY_SUITE_MASK;

        // Authentication (bit 5)
        if self.authenticated {
            byte |= AUTHENTICATED;
        }

        // Encryption (bit 4)
        if self.encrypted {
            byte |= ENCRYPTED;
        }

        // Key selection (bits 2-0)
        byte |= self.key_selection.to_bits() & KEY_SELECTION_MASK;

        byte
    }

    /// Parse from a single byte
    pub fn from_byte(byte: u8) -> Self {
        let suite = SecuritySuite::from_bits((byte & SECURITY_SUITE_MASK) >> SECURITY_SUITE_SHIFT);
        let authenticated = (byte & AUTHENTICATED) != 0;
        let encrypted = (byte & ENCRYPTED) != 0;
        let key_selection = KeySelection::from_bits(byte & KEY_SELECTION_MASK);

        Self {
            suite,
            authenticated,
            encrypted,
            key_selection,
        }
    }
}

/// Security Control Info
///
/// Convenience wrapper around SecurityControl with additional methods
pub type SecurityControlInfo = SecurityControl;

/// Parse a security control byte
///
/// This is a convenience function for SecurityControl::from_byte
pub fn parse_security_control(byte: u8) -> SecurityControlInfo {
    SecurityControl::from_byte(byte)
}

#[cfg(feature = "std")]
impl fmt::Display for SecurityControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SecurityControl(suite={:?}, auth={}, enc={}, key={:?})",
            self.suite, self.authenticated, self.encrypted, self.key_selection
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_suite_roundtrip() {
        assert_eq!(SecuritySuite::from_bits(0), SecuritySuite::AesGcm128);
        assert_eq!(SecuritySuite::AesGcm128.to_bits(), 0);
        assert_eq!(SecuritySuite::from_bits(1), SecuritySuite::Reserved(1));
        assert_eq!(SecuritySuite::Reserved(2).to_bits(), 2);
    }

    #[test]
    fn test_key_selection_roundtrip() {
        assert_eq!(KeySelection::from_bits(0), KeySelection::Global);
        assert_eq!(KeySelection::Global.to_bits(), 0);
        assert_eq!(KeySelection::from_bits(1), KeySelection::Dedicated);
        assert_eq!(KeySelection::Dedicated.to_bits(), 1);
        assert_eq!(KeySelection::from_bits(2), KeySelection::Reserved(2));
    }

    #[test]
    fn test_security_control_creation() {
        let sc = SecurityControl::new(SecuritySuite::AesGcm128, true, true, KeySelection::Global);
        assert_eq!(sc.suite(), SecuritySuite::AesGcm128);
        assert!(sc.is_authenticated());
        assert!(sc.is_encrypted());
        assert_eq!(sc.key_selection(), KeySelection::Global);
    }

    #[test]
    fn test_security_control_as_byte() {
        let sc = SecurityControl::new(SecuritySuite::AesGcm128, true, true, KeySelection::Global);
        assert_eq!(sc.as_byte(), 0b00110000);

        let sc = SecurityControl::new(SecuritySuite::AesGcm128, false, false, KeySelection::Dedicated);
        assert_eq!(sc.as_byte(), 0b00000001);
    }

    #[test]
    fn test_security_control_from_byte() {
        let byte = 0b00110000;
        let sc = SecurityControl::from_byte(byte);
        assert_eq!(sc.suite(), SecuritySuite::AesGcm128);
        assert!(sc.is_authenticated());
        assert!(sc.is_encrypted());
        assert_eq!(sc.key_selection(), KeySelection::Global);
    }

    #[test]
    fn test_security_control_roundtrip() {
        for suite in [SecuritySuite::AesGcm128, SecuritySuite::Reserved(1)] {
            for auth in [false, true] {
                for enc in [false, true] {
                    for key in [KeySelection::Global, KeySelection::Dedicated] {
                        let sc = SecurityControl::new(suite, auth, enc, key);
                        let byte = sc.as_byte();
                        let sc2 = SecurityControl::from_byte(byte);
                        assert_eq!(sc, sc2);
                    }
                }
            }
        }
    }

    #[test]
    fn test_parse_security_control() {
        let info = parse_security_control(0b00110010);
        assert_eq!(info.suite(), SecuritySuite::AesGcm128);
        assert!(info.is_authenticated());
        assert!(info.is_encrypted());
        assert_eq!(info.key_selection(), KeySelection::Reserved(2));
    }

    #[test]
    fn test_all_byte_values_roundtrip() {
        for byte in 0u8..=255 {
            let sc = SecurityControl::from_byte(byte);
            // Bit 3 (reserved) should be zero in output
            let expected = byte & 0b11110111;
            assert_eq!(sc.as_byte(), expected, "Roundtrip failed for byte: {:#04x}", byte);
        }
    }

    #[test]
    fn test_reserved_bits_ignored() {
        // Bit 3 is reserved - should be zero in output
        let sc1 = SecurityControl::from_byte(0b00001000);
        let sc2 = SecurityControl::from_byte(0b00000000);
        // The reserved bit should be cleared in output
        assert_eq!(sc1.as_byte(), 0);
        assert_eq!(sc2.as_byte(), 0);
        // And both should be equal
        assert_eq!(sc1, sc2);
    }
}
