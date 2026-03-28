//! DLMS/COSEM Security Layer
//!
//! Implements security mechanisms for DLMS/COSEM smart meter communication:
//! - LLS (Low Level Security): Simple password-based authentication
//! - HLS (High Level Security): GMAC/SHA256-based authentication
//! - AES-GCM-128: Encryption and authentication
//!
//! Reference: IEC 62056-6-2 (Green Book) §8 Security

#![no_std]

extern crate alloc;

mod aes_gcm;
mod control;
mod context;
mod hls;
mod key;
mod lls;
mod system_title;

// Re-exports
pub use aes_gcm::{decrypt, encrypt};
pub use control::{
    KeySelection, SecurityControl, SecurityControlInfo, SecuritySuite, AUTHENTICATED,
    ENCRYPTED, SECURITY_SUITE_MASK,
};
pub use context::{SecurityContext, SecurityLevel};
pub use hls::compute_auth_value;
pub use key::generate_key;
pub use lls::verify_password;
pub use system_title::{SystemTitle, SYSTEM_TITLE_LEN};

/// Maximum security control field size
pub const SECURITY_CONTROL_SIZE: usize = 1;

/// Default authentication tag size for AES-GCM (16 bytes)
pub const TAG_SIZE: usize = 16;

/// Nonce size for AES-GCM (12 bytes = system_title + 4 byte counter)
pub const NONCE_SIZE: usize = 12;

/// Global encryption key size (128-bit)
pub const GLOBAL_KEY_SIZE: usize = 16;

/// Dedicated key size (128-bit)
pub const DEDICATED_KEY_SIZE: usize = 16;

/// Authentication key size (128-bit)
pub const AUTH_KEY_SIZE: usize = 16;

/// Low-level security password size (max)
pub const LLS_PASSWORD_MAX_SIZE: usize = 64;

/// HLS challenge size
pub const HLS_CHALLENGE_SIZE: usize = 8;

/// HLS authentication value size
pub const HLS_AUTH_VALUE_SIZE: usize = 16;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Test vectors - known values for validation
    const TEST_SYSTEM_TITLE: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    const TEST_GLOBAL_KEY: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
        0x0E, 0x0F,
    ];
    const TEST_DEDICATED_KEY: [u8; 16] = [
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D,
        0x1E, 0x1F,
    ];
    const TEST_AUTH_KEY: [u8; 16] = [
        0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D,
        0x2E, 0x2F,
    ];

    fn create_test_context() -> SecurityContext {
        SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(SecurityLevel::AesGcm128)
            .with_global_key(TEST_GLOBAL_KEY)
            .with_dedicated_key(TEST_DEDICATED_KEY)
            .with_auth_key(TEST_AUTH_KEY)
            .with_frame_counter(0)
    }

    #[test]
    fn test_system_title_creation() {
        let st = SystemTitle::new(TEST_SYSTEM_TITLE);
        assert_eq!(st.as_bytes(), &TEST_SYSTEM_TITLE);
        assert_eq!(st.len(), SYSTEM_TITLE_LEN);

        let from_array: SystemTitle = TEST_SYSTEM_TITLE.into();
        assert_eq!(from_array.as_bytes(), &TEST_SYSTEM_TITLE);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_system_title_display() {
        let st = SystemTitle::new(TEST_SYSTEM_TITLE);
        assert_eq!(st.to_hex_string(), "0102030405060708");

        let empty = SystemTitle::new([0u8; 8]);
        assert_eq!(empty.to_hex_string(), "0000000000000000");
    }

    #[test]
    fn test_key_generation() {
        let key1 = generate_key(0x01);
        let key2 = generate_key(0x01);
        assert_eq!(key1, key2);

        let key3 = generate_key(0x02);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_key_from_slice() {
        let slice: &[u8] = &[0x00; 16];
        let key = key::from_slice(slice).unwrap();
        assert_eq!(key, [0x00; 16]);

        let short_slice: &[u8] = &[0x00; 8];
        assert!(key::from_slice(short_slice).is_err());
    }

    #[test]
    fn test_security_control_parsing() {
        // Test basic parsing
        let info = crate::control::parse_security_control(0b00000000);
        assert_eq!(info.suite(), SecuritySuite::AesGcm128);
        assert!(!info.is_authenticated());
        assert!(!info.is_encrypted());
        assert_eq!(info.key_selection(), KeySelection::Global);

        // Test with authentication
        let info = crate::control::parse_security_control(0b00100000);
        assert!(info.is_authenticated());
        assert!(!info.is_encrypted());

        // Test with encryption
        let info = crate::control::parse_security_control(0b00010000);
        assert!(!info.is_authenticated());
        assert!(info.is_encrypted());

        // Test with both
        let info = crate::control::parse_security_control(0b00110000);
        assert!(info.is_authenticated());
        assert!(info.is_encrypted());

        // Test dedicated key
        let info = crate::control::parse_security_control(0b00110001);
        assert_eq!(info.key_selection(), KeySelection::Dedicated);
    }

    #[test]
    fn test_security_control_creation() {
        let sc = SecurityControl::new(SecuritySuite::AesGcm128, true, true, KeySelection::Global);
        assert_eq!(sc.as_byte(), 0b00110000);

        let sc = SecurityControl::new(SecuritySuite::AesGcm128, false, false, KeySelection::Dedicated);
        assert_eq!(sc.as_byte(), 0b00000001);
    }

    #[test]
    fn test_security_context_creation() {
        let ctx = SecurityContext::new(TEST_SYSTEM_TITLE);
        assert_eq!(ctx.system_title(), &TEST_SYSTEM_TITLE);
        assert_eq!(ctx.security_level(), SecurityLevel::None);
        assert_eq!(ctx.frame_counter(), 0);
        // Default context has no keys set
        assert!(ctx.global_key().is_none());
        assert!(ctx.dedicated_key().is_none());
        assert!(ctx.auth_key().is_none());
    }

    #[test]
    fn test_security_context_builder() {
        let ctx = SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(SecurityLevel::HlsGmac)
            .with_global_key(TEST_GLOBAL_KEY)
            .with_dedicated_key(TEST_DEDICATED_KEY)
            .with_auth_key(TEST_AUTH_KEY)
            .with_frame_counter(42);

        assert_eq!(ctx.security_level(), SecurityLevel::HlsGmac);
        assert_eq!(ctx.frame_counter(), 42);
        assert_eq!(ctx.global_key(), Some(&TEST_GLOBAL_KEY));
        assert_eq!(ctx.dedicated_key(), Some(&TEST_DEDICATED_KEY));
        assert_eq!(ctx.auth_key(), Some(&TEST_AUTH_KEY));
    }

    #[test]
    fn test_security_context_increment_counter() {
        let mut ctx = create_test_context();
        assert_eq!(ctx.frame_counter(), 0);

        ctx.increment_counter().unwrap();
        assert_eq!(ctx.frame_counter(), 1);

        ctx.increment_counter().unwrap();
        assert_eq!(ctx.frame_counter(), 2);
    }

    #[test]
    fn test_security_context_get_key() {
        let ctx = create_test_context();

        // Test global key selection
        let key = ctx.get_key(KeySelection::Global).unwrap();
        assert_eq!(key, &TEST_GLOBAL_KEY);

        // Test dedicated key selection
        let key = ctx.get_key(KeySelection::Dedicated).unwrap();
        assert_eq!(key, &TEST_DEDICATED_KEY);

        // Test missing dedicated key
        let ctx_no_dedicated = SecurityContext::new(TEST_SYSTEM_TITLE).with_global_key(TEST_GLOBAL_KEY);
        assert!(ctx_no_dedicated.get_key(KeySelection::Dedicated).is_err());
    }

    #[test]
    fn test_lls_password_verification() {
        // Valid password
        let valid_password: [u8; 8] = [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38]; // "12345678"
        let stored_password_hash = lls::hash_password(&valid_password);

        assert!(verify_password(&valid_password, &stored_password_hash).is_ok());

        // Invalid password
        let invalid_password: [u8; 8] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        assert!(verify_password(&invalid_password, &stored_password_hash).is_err());
    }

    #[test]
    fn test_lls_empty_password() {
        let empty_password = &[0u8; 0];
        let hash = lls::hash_password(empty_password);
        assert!(verify_password(empty_password, &hash).is_ok());
    }

    #[test]
    fn test_hls_gmac_auth() {
        let ctx = SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(SecurityLevel::HlsGmac)
            .with_auth_key(TEST_AUTH_KEY)
            .with_frame_counter(0);
        let challenge: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];

        let auth_value = compute_auth_value(&ctx, &challenge).unwrap();

        // Verify auth value is correct length
        assert_eq!(auth_value.len(), HLS_AUTH_VALUE_SIZE);

        // Same input should produce same auth value
        let auth_value2 = compute_auth_value(&ctx, &challenge).unwrap();
        assert_eq!(auth_value, auth_value2);

        // Different challenge should produce different auth value
        let challenge2: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let auth_value3 = compute_auth_value(&ctx, &challenge2).unwrap();
        assert_ne!(auth_value, auth_value3);
    }

    #[test]
    fn test_hls_with_different_counters() {
        let mut ctx = SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(SecurityLevel::HlsGmac)
            .with_auth_key(TEST_AUTH_KEY)
            .with_frame_counter(0);
        let challenge: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];

        let auth1 = compute_auth_value(&ctx, &challenge).unwrap();

        ctx.increment_counter().unwrap();
        let auth2 = compute_auth_value(&ctx, &challenge).unwrap();

        // Different counters should produce different auth values
        assert_ne!(auth1, auth2);
    }

    #[test]
    fn test_aes_gcm_roundtrip() {
        let ctx = create_test_context();
        let plaintext = b"Hello, DLMS World!".to_vec();
        let associated_data = b"additional data";

        // Encrypt
        let ciphertext = encrypt(&ctx, &plaintext, Some(associated_data)).unwrap();

        // Ciphertext should be larger (plaintext + tag)
        assert!(ciphertext.len() >= plaintext.len());

        // Decrypt
        let decrypted = decrypt(&ctx, &ciphertext, Some(associated_data)).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aes_gcm_without_aad() {
        let ctx = create_test_context();
        let plaintext = b"No additional data".to_vec();

        let ciphertext = encrypt(&ctx, &plaintext, None).unwrap();
        let decrypted = decrypt(&ctx, &ciphertext, None).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aes_gcm_different_counters() {
        let mut ctx = create_test_context();
        let plaintext: alloc::vec::Vec<u8> = b"Counter test".to_vec();

        let ciphertext1 = encrypt(&ctx, &plaintext, None).unwrap();

        ctx.increment_counter().unwrap();
        let ciphertext2 = encrypt(&ctx, &plaintext, None).unwrap();

        // Different counters should produce different ciphertexts
        assert_ne!(ciphertext1, ciphertext2);
    }

    #[test]
    fn test_aes_gcm_modified_ciphertext_fails() {
        let ctx = create_test_context();
        let plaintext = b"Tamper test".to_vec();

        let mut ciphertext = encrypt(&ctx, &plaintext, None).unwrap();

        // Tamper with ciphertext
        if let Some(byte) = ciphertext.get_mut(0) {
            *byte ^= 0xFF;
        }

        // Decryption should fail
        assert!(decrypt(&ctx, &ciphertext, None).is_err());
    }

    #[test]
    fn test_aes_gcm_wrong_aad_fails() {
        let ctx = create_test_context();
        let plaintext = b"AAD test".to_vec();

        let aad1 = b"correct AAD";
        let aad2 = b"wrong AAD";

        let ciphertext = encrypt(&ctx, &plaintext, Some(aad1)).unwrap();

        // Decryption with wrong AAD should fail
        assert!(decrypt(&ctx, &ciphertext, Some(aad2)).is_err());
    }

    #[test]
    fn test_security_level_display() {
        assert_eq!(SecurityLevel::None.as_u8(), 0);
        assert_eq!(SecurityLevel::Lls.as_u8(), 1);
        assert_eq!(SecurityLevel::HlsGmac.as_u8(), 2);
        assert_eq!(SecurityLevel::HlsSha256.as_u8(), 3);
        assert_eq!(SecurityLevel::AesGcm128.as_u8(), 4);

        assert_eq!(SecurityLevel::from_u8(0), Some(SecurityLevel::None));
        assert_eq!(SecurityLevel::from_u8(1), Some(SecurityLevel::Lls));
        assert_eq!(SecurityLevel::from_u8(2), Some(SecurityLevel::HlsGmac));
        assert_eq!(SecurityLevel::from_u8(3), Some(SecurityLevel::HlsSha256));
        assert_eq!(SecurityLevel::from_u8(4), Some(SecurityLevel::AesGcm128));
        assert_eq!(SecurityLevel::from_u8(99), None);
    }

    #[test]
    fn test_nonce_generation() {
        let ctx = create_test_context();
        let nonce = aes_gcm::build_nonce(ctx.system_title(), ctx.frame_counter());

        assert_eq!(nonce.len(), NONCE_SIZE);
        assert_eq!(&nonce[..8], &TEST_SYSTEM_TITLE);
        assert_eq!(&nonce[8..], &[0, 0, 0, 0]); // counter = 0

        let ctx2 = create_test_context().with_frame_counter(0x12345678);
        let nonce2 = aes_gcm::build_nonce(ctx2.system_title(), ctx2.frame_counter());

        assert_eq!(&nonce2[..8], &TEST_SYSTEM_TITLE);
        assert_eq!(&nonce2[8..], &[0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_security_control_roundtrip() {
        for byte in 0u8..=255 {
            let info = crate::control::parse_security_control(byte);
            // Bit 3 (reserved) should be zero in output
            let expected = byte & 0b11110111;
            assert_eq!(info.as_byte(), expected);
        }
    }

    #[test]
    fn test_empty_plaintext() {
        let ctx = create_test_context();
        let plaintext: alloc::vec::Vec<u8> = alloc::vec![];

        let ciphertext = encrypt(&ctx, &plaintext, None).unwrap();
        let decrypted = decrypt(&ctx, &ciphertext, None).unwrap();

        assert_eq!(decrypted, plaintext);
        // Even empty plaintext produces ciphertext + tag
        assert!(ciphertext.len() >= TAG_SIZE);
    }

    #[test]
    fn test_large_plaintext() {
        let ctx = create_test_context();
        let plaintext: alloc::vec::Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        let ciphertext = encrypt(&ctx, &plaintext, None).unwrap();
        let decrypted = decrypt(&ctx, &ciphertext, None).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_context_with_dedicated_key_encryption() {
        let ctx = SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(SecurityLevel::AesGcm128)
            .with_dedicated_key(TEST_DEDICATED_KEY)
            .with_frame_counter(0);

        let plaintext: alloc::vec::Vec<u8> = b"Dedicated key test".to_vec();

        // Encrypt with dedicated key
        let ciphertext = aes_gcm::encrypt_with_key(
            ctx.get_key(KeySelection::Dedicated).unwrap(),
            ctx.system_title(),
            ctx.frame_counter(),
            &plaintext,
            None,
        )
        .unwrap();

        let decrypted = aes_gcm::decrypt_with_key(
            ctx.get_key(KeySelection::Dedicated).unwrap(),
            ctx.system_title(),
            ctx.frame_counter(),
            &ciphertext,
            None,
        )
        .unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
