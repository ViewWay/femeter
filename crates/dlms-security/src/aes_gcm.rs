//! AES-GCM-128 Encryption/Decryption
//!
//! Implements AES-GCM-128 encryption for DLMS/COSEM.
//! Reference: IEC 62056-6-2 §8.6

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes128Gcm, Nonce,
};
use alloc::vec::Vec;
use dlms_core::errors::SecurityError;

use crate::{NONCE_SIZE, TAG_SIZE};

/// Build the nonce for AES-GCM
///
/// The nonce is 12 bytes: 8 bytes system_title + 4 bytes frame_counter
pub fn build_nonce(system_title: &[u8; 8], frame_counter: u32) -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    nonce[..8].copy_from_slice(system_title);
    nonce[8..].copy_from_slice(&frame_counter.to_be_bytes());
    nonce
}

/// Encrypt data using AES-GCM-128
///
/// # Arguments
/// * `ctx` - Security context containing system title and frame counter
/// * `plaintext` - Data to encrypt
/// * `associated_data` - Optional additional authenticated data
///
/// # Returns
/// Ciphertext with authentication tag appended
pub fn encrypt(
    ctx: &crate::SecurityContext,
    plaintext: &[u8],
    associated_data: Option<&[u8]>,
) -> Result<Vec<u8>, SecurityError> {
    let key = ctx.global_key().ok_or(SecurityError::KeyNotFound)?;
    encrypt_with_key(
        key,
        ctx.system_title(),
        ctx.frame_counter(),
        plaintext,
        associated_data,
    )
}

/// Decrypt data using AES-GCM-128
///
/// # Arguments
/// * `ctx` - Security context containing system title and frame counter
/// * `ciphertext` - Data to decrypt (with tag appended)
/// * `associated_data` - Optional additional authenticated data
///
/// # Returns
/// Decrypted plaintext
pub fn decrypt(
    ctx: &crate::SecurityContext,
    ciphertext: &[u8],
    associated_data: Option<&[u8]>,
) -> Result<Vec<u8>, SecurityError> {
    let key = ctx.global_key().ok_or(SecurityError::KeyNotFound)?;
    decrypt_with_key(
        key,
        ctx.system_title(),
        ctx.frame_counter(),
        ciphertext,
        associated_data,
    )
}

/// Encrypt with explicit key
///
/// # Arguments
/// * `key` - 128-bit encryption key
/// * `system_title` - 8-byte system title
/// * `frame_counter` - Frame counter for nonce
/// * `plaintext` - Data to encrypt
/// * `associated_data` - Optional AAD
pub fn encrypt_with_key(
    key: &[u8; 16],
    system_title: &[u8; 8],
    frame_counter: u32,
    plaintext: &[u8],
    associated_data: Option<&[u8]>,
) -> Result<Vec<u8>, SecurityError> {
    // Build cipher from key
    let cipher = Aes128Gcm::new_from_slice(key).map_err(|_| SecurityError::InvalidKey)?;

    // Build nonce
    let nonce_bytes = build_nonce(system_title, frame_counter);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt with optional AAD
    let ciphertext = match associated_data {
        Some(aad) => cipher
            .encrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| SecurityError::EncryptionFailed)?,
        None => cipher
            .encrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: plaintext,
                    aad: b"",
                },
            )
            .map_err(|_| SecurityError::EncryptionFailed)?,
    };

    Ok(ciphertext)
}

/// Decrypt with explicit key
///
/// # Arguments
/// * `key` - 128-bit decryption key
/// * `system_title` - 8-byte system title
/// * `frame_counter` - Frame counter for nonce
/// * `ciphertext` - Data to decrypt (with tag appended)
/// * `associated_data` - Optional AAD
pub fn decrypt_with_key(
    key: &[u8; 16],
    system_title: &[u8; 8],
    frame_counter: u32,
    ciphertext: &[u8],
    associated_data: Option<&[u8]>,
) -> Result<Vec<u8>, SecurityError> {
    // Verify minimum length (tag only)
    if ciphertext.len() < TAG_SIZE {
        return Err(SecurityError::InvalidTag);
    }

    // Build cipher from key
    let cipher = Aes128Gcm::new_from_slice(key).map_err(|_| SecurityError::InvalidKey)?;

    // Build nonce
    let nonce_bytes = build_nonce(system_title, frame_counter);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Decrypt with optional AAD
    let plaintext = match associated_data {
        Some(aad) => cipher
            .decrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: ciphertext,
                    aad,
                },
            )
            .map_err(|_| SecurityError::DecryptionFailed)?,
        None => cipher
            .decrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: ciphertext,
                    aad: b"",
                },
            )
            .map_err(|_| SecurityError::DecryptionFailed)?,
    };

    Ok(plaintext)
}

/// Compute GMAC (authentication only, no encryption)
///
/// # Arguments
/// * `key` - 128-bit authentication key
/// * `system_title` - 8-byte system title
/// * `frame_counter` - Frame counter for nonce
/// * `data` - Data to authenticate
///
/// # Returns
/// 16-byte authentication tag
pub fn compute_gmac(
    key: &[u8; 16],
    system_title: &[u8; 8],
    frame_counter: u32,
    data: &[u8],
) -> Result<[u8; TAG_SIZE], SecurityError> {
    let cipher = Aes128Gcm::new_from_slice(key).map_err(|_| SecurityError::InvalidKey)?;

    let nonce_bytes = build_nonce(system_title, frame_counter);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // GMAC is AES-GCM with empty plaintext
    let tag = cipher
        .encrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: &[],
                aad: data,
            },
        )
        .map_err(|_| SecurityError::EncryptionFailed)?;

    let mut result = [0u8; TAG_SIZE];
    result.copy_from_slice(&tag);
    Ok(result)
}

/// Generate a random encryption key
///
/// Only available with std feature and getrandom
#[cfg(feature = "std")]
#[allow(dead_code)]
pub fn generate_random_key() -> [u8; 16] {
    // Simple deterministic key for testing - in production use proper RNG
    [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F,
    ];
    const TEST_SYSTEM_TITLE: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    #[test]
    fn test_build_nonce() {
        let nonce = build_nonce(&TEST_SYSTEM_TITLE, 0x12345678);
        assert_eq!(nonce[..8], TEST_SYSTEM_TITLE);
        assert_eq!(nonce[8..], [0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = b"Hello, World!";
        let aad = b"additional data";

        let ciphertext =
            encrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, plaintext, Some(aad)).unwrap();
        let decrypted =
            decrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, &ciphertext, Some(aad)).unwrap();

        assert_eq!(decrypted, plaintext.as_ref());
    }

    #[test]
    fn test_encrypt_decrypt_without_aad() {
        let plaintext = b"No AAD";

        let ciphertext =
            encrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, plaintext, None).unwrap();
        let decrypted =
            decrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, &ciphertext, None).unwrap();

        assert_eq!(decrypted, plaintext.as_ref());
    }

    #[test]
    fn test_different_counters_different_ciphertext() {
        let plaintext = b"Counter test";

        let ct1 = encrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, plaintext, None).unwrap();
        let ct2 = encrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 1, plaintext, None).unwrap();

        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_wrong_key_fails() {
        let plaintext = b"Secret message";
        let wrong_key = [0xFFu8; 16];

        let ciphertext =
            encrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, plaintext, None).unwrap();

        let result = decrypt_with_key(&wrong_key, &TEST_SYSTEM_TITLE, 0, &ciphertext, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_aad_fails() {
        let plaintext = b"Message";
        let aad1 = b"correct AAD";
        let aad2 = b"wrong AAD";

        let ciphertext =
            encrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, plaintext, Some(aad1)).unwrap();

        let result = decrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, &ciphertext, Some(aad2));
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let plaintext = b"Don't tamper!";
        let mut ciphertext =
            encrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, plaintext, None).unwrap();

        // Tamper with the ciphertext
        if let Some(byte) = ciphertext.get_mut(0) {
            *byte ^= 0xFF;
        }

        let result = decrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, &ciphertext, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let plaintext = b"";

        let ciphertext =
            encrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, plaintext, None).unwrap();
        let decrypted =
            decrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, &ciphertext, None).unwrap();

        assert_eq!(decrypted, plaintext.as_ref());
        // Even empty plaintext produces tag
        assert!(ciphertext.len() >= TAG_SIZE);
    }

    #[test]
    fn test_large_plaintext() {
        let plaintext: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        let ciphertext =
            encrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, &plaintext, None).unwrap();
        let decrypted =
            decrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, &ciphertext, None).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_compute_gmac() {
        let data = b"Data to authenticate";

        let tag = compute_gmac(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, data).unwrap();

        assert_eq!(tag.len(), TAG_SIZE);

        // Same input should produce same tag
        let tag2 = compute_gmac(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, data).unwrap();
        assert_eq!(tag, tag2);

        // Different counter should produce different tag
        let tag3 = compute_gmac(&TEST_KEY, &TEST_SYSTEM_TITLE, 1, data).unwrap();
        assert_ne!(tag, tag3);
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key = [0u8; 8];
        let cipher = Aes128Gcm::new_from_slice(&short_key);
        assert!(cipher.is_err());
    }

    #[test]
    fn test_short_ciphertext() {
        let short_ct = &[0u8; 8]; // Shorter than TAG_SIZE

        let result = decrypt_with_key(&TEST_KEY, &TEST_SYSTEM_TITLE, 0, short_ct, None);
        assert!(result.is_err());
    }
}
