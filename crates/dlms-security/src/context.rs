//! Security Context
//!
//! The SecurityContext holds all security-related state for DLMS communication,
//! including keys, counters, and security level settings.

use crate::{
    key, system_title::SystemTitle, KeySelection, SecurityControl, SecuritySuite,
    AUTH_KEY_SIZE, DEDICATED_KEY_SIZE, GLOBAL_KEY_SIZE,
};
use dlms_core::errors::SecurityError;

/// Security Level enumeration
///
/// Defines the security requirements for a DLMS connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum SecurityLevel {
    /// No security
    None = 0,
    /// Low Level Security (LLS) - password based
    Lls = 1,
    /// High Level Security (HLS) with GMAC
    HlsGmac = 2,
    /// High Level Security (HLS) with SHA256
    HlsSha256 = 3,
    /// AES-GCM-128 encryption
    AesGcm128 = 4,
}

impl SecurityLevel {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Lls),
            2 => Some(Self::HlsGmac),
            3 => Some(Self::HlsSha256),
            4 => Some(Self::AesGcm128),
            _ => None,
        }
    }

    /// Convert to u8
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Check if authentication is required
    pub fn requires_authentication(&self) -> bool {
        matches!(self, Self::Lls | Self::HlsGmac | Self::HlsSha256 | Self::AesGcm128)
    }

    /// Check if encryption is required
    pub fn requires_encryption(&self) -> bool {
        matches!(self, Self::AesGcm128)
    }
}

/// Security Context for DLMS communication
///
/// Holds cryptographic keys, counters, and security settings.
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SecurityContext {
    system_title: SystemTitle,
    security_level: SecurityLevel,
    frame_counter: u32,
    global_key: Option<[u8; GLOBAL_KEY_SIZE]>,
    dedicated_key: Option<[u8; DEDICATED_KEY_SIZE]>,
    auth_key: Option<[u8; AUTH_KEY_SIZE]>,
}

impl SecurityContext {
    /// Create a new SecurityContext with the given system title
    pub fn new(system_title: [u8; 8]) -> Self {
        Self {
            system_title: SystemTitle::new(system_title),
            security_level: SecurityLevel::None,
            frame_counter: 0,
            global_key: None,
            dedicated_key: None,
            auth_key: None,
        }
    }

    /// Get the system title
    pub fn system_title(&self) -> &[u8; 8] {
        self.system_title.as_bytes()
    }

    /// Set the system title
    pub fn with_system_title(mut self, system_title: [u8; 8]) -> Self {
        self.system_title = SystemTitle::new(system_title);
        self
    }

    /// Get the security level
    pub fn security_level(&self) -> SecurityLevel {
        self.security_level
    }

    /// Set the security level
    pub fn with_level(mut self, level: SecurityLevel) -> Self {
        self.security_level = level;
        self
    }

    /// Get the frame counter
    pub fn frame_counter(&self) -> u32 {
        self.frame_counter
    }

    /// Set the frame counter
    pub fn with_frame_counter(mut self, counter: u32) -> Self {
        self.frame_counter = counter;
        self
    }

    /// Increment the frame counter
    ///
    /// Returns an error if the counter would overflow
    pub fn increment_counter(&mut self) -> Result<(), SecurityError> {
        self.frame_counter = self.frame_counter.checked_add(1).ok_or(SecurityError::CounterOverflow)?;
        Ok(())
    }

    /// Get the global encryption key
    pub fn global_key(&self) -> Option<&[u8; GLOBAL_KEY_SIZE]> {
        self.global_key.as_ref()
    }

    /// Set the global encryption key
    pub fn with_global_key(mut self, key: [u8; GLOBAL_KEY_SIZE]) -> Self {
        self.global_key = Some(key);
        self
    }

    /// Get the dedicated key
    pub fn dedicated_key(&self) -> Option<&[u8; DEDICATED_KEY_SIZE]> {
        self.dedicated_key.as_ref()
    }

    /// Set the dedicated key
    pub fn with_dedicated_key(mut self, key: [u8; DEDICATED_KEY_SIZE]) -> Self {
        self.dedicated_key = Some(key);
        self
    }

    /// Get the authentication key
    pub fn auth_key(&self) -> Option<&[u8; AUTH_KEY_SIZE]> {
        self.auth_key.as_ref()
    }

    /// Set the authentication key
    pub fn with_auth_key(mut self, key: [u8; AUTH_KEY_SIZE]) -> Self {
        self.auth_key = Some(key);
        self
    }

    /// Get the appropriate key based on key selection
    pub fn get_key(&self, selection: KeySelection) -> Result<&[u8; 16], SecurityError> {
        match selection {
            KeySelection::Global => self.global_key.as_ref().ok_or(SecurityError::KeyNotFound),
            KeySelection::Dedicated => self.dedicated_key.as_ref().ok_or(SecurityError::KeyNotFound),
            KeySelection::Reserved(_) => Err(SecurityError::KeyNotFound),
        }
    }

    /// Get the authentication key
    pub fn get_auth_key(&self) -> Result<&[u8; 16], SecurityError> {
        self.auth_key.as_ref().ok_or(SecurityError::KeyNotFound)
    }

    /// Check if a key is set for the given selection
    pub fn has_key(&self, selection: KeySelection) -> bool {
        match selection {
            KeySelection::Global => self.global_key.is_some(),
            KeySelection::Dedicated => self.dedicated_key.is_some(),
            KeySelection::Reserved(_) => false,
        }
    }

    /// Create a security control byte based on current settings
    pub fn security_control(&self, key_selection: KeySelection) -> SecurityControl {
        let authenticated = self.security_level.requires_authentication();
        let encrypted = self.security_level.requires_encryption();

        SecurityControl::new(SecuritySuite::AesGcm128, authenticated, encrypted, key_selection)
    }

    /// Check if the context is properly configured for the security level
    pub fn is_valid(&self) -> bool {
        match self.security_level {
            SecurityLevel::None => true,
            SecurityLevel::Lls => true, // LLS only uses password, no keys
            SecurityLevel::HlsGmac | SecurityLevel::HlsSha256 => self.auth_key.is_some(),
            SecurityLevel::AesGcm128 => self.global_key.is_some() || self.dedicated_key.is_some(),
        }
    }

    /// Clear all sensitive data (keys) from the context
    pub fn clear_keys(&mut self) {
        if let Some(mut key) = self.global_key.take() {
            key::zero_key(&mut key);
        }
        if let Some(mut key) = self.dedicated_key.take() {
            key::zero_key(&mut key);
        }
        if let Some(mut key) = self.auth_key.take() {
            key::zero_key(&mut key);
        }
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self::new([0u8; 8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
        0x0E, 0x0F,
    ];

    #[test]
    fn test_security_level_roundtrip() {
        assert_eq!(SecurityLevel::from_u8(0), Some(SecurityLevel::None));
        assert_eq!(SecurityLevel::from_u8(1), Some(SecurityLevel::Lls));
        assert_eq!(SecurityLevel::from_u8(2), Some(SecurityLevel::HlsGmac));
        assert_eq!(SecurityLevel::from_u8(3), Some(SecurityLevel::HlsSha256));
        assert_eq!(SecurityLevel::from_u8(4), Some(SecurityLevel::AesGcm128));
        assert_eq!(SecurityLevel::from_u8(99), None);

        assert_eq!(SecurityLevel::None.as_u8(), 0);
        assert_eq!(SecurityLevel::Lls.as_u8(), 1);
        assert_eq!(SecurityLevel::HlsGmac.as_u8(), 2);
        assert_eq!(SecurityLevel::HlsSha256.as_u8(), 3);
        assert_eq!(SecurityLevel::AesGcm128.as_u8(), 4);
    }

    #[test]
    fn test_security_level_authentication() {
        assert!(!SecurityLevel::None.requires_authentication());
        assert!(SecurityLevel::Lls.requires_authentication());
        assert!(SecurityLevel::HlsGmac.requires_authentication());
        assert!(SecurityLevel::HlsSha256.requires_authentication());
        assert!(SecurityLevel::AesGcm128.requires_authentication());
    }

    #[test]
    fn test_security_level_encryption() {
        assert!(!SecurityLevel::None.requires_encryption());
        assert!(!SecurityLevel::Lls.requires_encryption());
        assert!(!SecurityLevel::HlsGmac.requires_encryption());
        assert!(!SecurityLevel::HlsSha256.requires_encryption());
        assert!(SecurityLevel::AesGcm128.requires_encryption());
    }

    #[test]
    fn test_context_creation() {
        let ctx = SecurityContext::new([1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(ctx.system_title(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(ctx.security_level(), SecurityLevel::None);
        assert_eq!(ctx.frame_counter(), 0);
    }

    #[test]
    fn test_context_builder() {
        let ctx = SecurityContext::new([0u8; 8])
            .with_level(SecurityLevel::AesGcm128)
            .with_global_key(TEST_KEY)
            .with_frame_counter(42);

        assert_eq!(ctx.security_level(), SecurityLevel::AesGcm128);
        assert_eq!(ctx.frame_counter(), 42);
        assert_eq!(ctx.global_key(), Some(&TEST_KEY));
    }

    #[test]
    fn test_increment_counter() {
        let mut ctx = SecurityContext::new([0u8; 8]).with_frame_counter(u32::MAX - 1);

        ctx.increment_counter().unwrap();
        assert_eq!(ctx.frame_counter(), u32::MAX);

        assert!(ctx.increment_counter().is_err());
    }

    #[test]
    fn test_get_key() {
        let ctx = SecurityContext::new([0u8; 8])
            .with_global_key(TEST_KEY);

        assert_eq!(ctx.get_key(KeySelection::Global), Ok(&TEST_KEY));
        assert!(ctx.get_key(KeySelection::Dedicated).is_err());
    }

    #[test]
    fn test_has_key() {
        let ctx = SecurityContext::new([0u8; 8])
            .with_global_key(TEST_KEY);

        assert!(ctx.has_key(KeySelection::Global));
        assert!(!ctx.has_key(KeySelection::Dedicated));
    }

    #[test]
    fn test_is_valid() {
        let mut ctx = SecurityContext::new([0u8; 8]);

        assert!(ctx.is_valid()); // No security is valid

        ctx = ctx.with_level(SecurityLevel::Lls);
        assert!(ctx.is_valid()); // LLS doesn't need keys

        ctx = ctx.with_level(SecurityLevel::HlsGmac);
        assert!(!ctx.is_valid()); // HLS needs auth key

        ctx = ctx.with_auth_key(TEST_KEY);
        assert!(ctx.is_valid());

        ctx = ctx.with_level(SecurityLevel::AesGcm128);
        ctx = ctx.with_auth_key([0u8; 16]); // Reset auth key
        assert!(!ctx.is_valid()); // AES needs encryption key

        ctx = ctx.with_global_key(TEST_KEY);
        assert!(ctx.is_valid());
    }

    #[test]
    fn test_clear_keys() {
        let mut ctx = SecurityContext::new([0u8; 8])
            .with_global_key(TEST_KEY)
            .with_dedicated_key(TEST_KEY)
            .with_auth_key(TEST_KEY);

        ctx.clear_keys();

        assert!(ctx.global_key().is_none());
        assert!(ctx.dedicated_key().is_none());
        assert!(ctx.auth_key().is_none());
    }

    #[test]
    fn test_security_control() {
        let ctx = SecurityContext::new([0u8; 8])
            .with_level(SecurityLevel::AesGcm128);

        let sc = ctx.security_control(KeySelection::Global);
        assert!(sc.is_authenticated());
        assert!(sc.is_encrypted());
        assert_eq!(sc.key_selection(), KeySelection::Global);
    }
}
