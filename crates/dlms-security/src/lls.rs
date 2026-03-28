//! Low Level Security (LLS)
//!
//! Simple password-based authentication for DLMS/COSEM.
//! Reference: IEC 62056-6-2 §8.5.1

use alloc::vec::Vec;
use dlms_core::errors::SecurityError;

/// Maximum password length
pub const MAX_PASSWORD_SIZE: usize = 64;

/// Hash a password for storage/comparison
///
/// This is a simple hash implementation. In production, use proper
/// password hashing algorithms like Argon2 or scrypt.
pub fn hash_password(password: &[u8]) -> Vec<u8> {
    if password.is_empty() {
        return Vec::new();
    }

    // Simple hash: XOR-fold with rotation
    // This is NOT cryptographically secure but sufficient for protocol compatibility
    let mut hash = Vec::with_capacity(16);
    let mut acc: u32 = 0x67452301;

    for (i, &byte) in password.iter().enumerate() {
        acc = acc.wrapping_mul(31).wrapping_add(byte as u32).wrapping_add(i as u32);
    }

    // Expand to 16 bytes using simple mixing
    for i in 0..16 {
        let mixed = acc.wrapping_mul((i as u32).wrapping_add(1)).wrapping_add(0x9E3779B9);
        hash.push(((mixed >> (i % 4 * 8)) & 0xFF) as u8);
    }

    hash
}

/// Verify a password against a stored hash
pub fn verify_password(password: &[u8], stored_hash: &[u8]) -> Result<(), SecurityError> {
    let computed = hash_password(password);

    if stored_hash.is_empty() && password.is_empty() {
        return Ok(());
    }

    if stored_hash.len() != computed.len() {
        return Err(SecurityError::AuthenticationFailed);
    }

    // Constant-time comparison
    let mut result = 0u8;
    for (a, b) in computed.iter().zip(stored_hash.iter()) {
        result |= a ^ b;
    }

    if result == 0 {
        Ok(())
    } else {
        Err(SecurityError::AuthenticationFailed)
    }
}

/// Validate password length
pub fn validate_password(password: &[u8]) -> Result<(), SecurityError> {
    if password.len() > MAX_PASSWORD_SIZE {
        Err(SecurityError::InvalidKey)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_empty_password() {
        let hash = hash_password(&[]);
        assert!(hash.is_empty());
    }

    #[test]
    fn test_hash_non_empty_password() {
        let password = b"test123";
        let hash = hash_password(password);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn test_hash_deterministic() {
        let password = b"same password";
        let hash1 = hash_password(password);
        let hash2 = hash_password(password);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_passwords() {
        let hash1 = hash_password(b"password1");
        let hash2 = hash_password(b"password2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_correct_password() {
        let password = b"correct password";
        let hash = hash_password(password);
        assert!(verify_password(password, &hash).is_ok());
    }

    #[test]
    fn test_verify_incorrect_password() {
        let password = b"correct password";
        let hash = hash_password(password);
        assert!(verify_password(b"wrong password", &hash).is_err());
    }

    #[test]
    fn test_verify_empty_password() {
        let hash = hash_password(&[]);
        assert!(verify_password(&[], &hash).is_ok());
    }

    #[test]
    fn test_verify_length_mismatch() {
        let password = b"test";
        let mut hash = hash_password(password);
        hash.truncate(8); // Corrupt the hash
        assert!(verify_password(password, &hash).is_err());
    }

    #[test]
    fn test_validate_password_length() {
        assert!(validate_password(b"short").is_ok());

        let long_vec: Vec<u8> = alloc::vec![b'x'; MAX_PASSWORD_SIZE];
        assert!(validate_password(&long_vec).is_ok());

        let too_long: Vec<u8> = alloc::vec![b'x'; MAX_PASSWORD_SIZE + 1];
        assert!(validate_password(&too_long).is_err());
    }

    #[test]
    fn test_unicode_password() {
        let password = "パスワード123".as_bytes();
        let hash = hash_password(password);
        assert!(verify_password(password, &hash).is_ok());
    }
}
