//! Key management utilities
//!
//! Helper functions for cryptographic key handling.

use dlms_core::errors::SecurityError;

/// Key size in bytes for AES-128
pub const KEY_SIZE: usize = 16;

/// Generate a deterministic key from a seed value
///
/// This is a simple key derivation for testing purposes.
/// In production, use proper key derivation functions.
pub fn generate_key(seed: u32) -> [u8; KEY_SIZE] {
    let mut key = [0u8; KEY_SIZE];
    let mut state: u32 = seed.wrapping_mul(31).wrapping_add(0xB);

    for (i, byte) in key.iter_mut().enumerate() {
        state = state.wrapping_mul(31).wrapping_add(0xB);
        *byte = ((state >> (i * 8 % 32)) & 0xFF) as u8;
    }

    key
}

/// Parse a key from a byte slice
///
/// Returns an error if the slice is not exactly 16 bytes
#[allow(dead_code)]
pub fn from_slice(slice: &[u8]) -> Result<[u8; KEY_SIZE], SecurityError> {
    if slice.len() == KEY_SIZE {
        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(slice);
        Ok(key)
    } else {
        Err(SecurityError::InvalidKey)
    }
}

/// Zero out a key in memory
///
/// Use this to securely clear keys from memory when no longer needed
#[allow(dead_code)]
pub fn zero_key(key: &mut [u8; KEY_SIZE]) {
    key.iter_mut().for_each(|b| *b = 0);
}

/// Compare two keys in constant time
///
/// Returns true if the keys are equal
#[allow(dead_code)]
pub fn constant_time_eq(a: &[u8; KEY_SIZE], b: &[u8; KEY_SIZE]) -> bool {
    let mut result = 0u8;
    for (ai, bi) in a.iter().zip(b.iter()) {
        result |= ai ^ bi;
    }
    result == 0
}

/// Check if a key is all zeros
#[allow(dead_code)]
pub fn is_zero_key(key: &[u8; KEY_SIZE]) -> bool {
    key.iter().all(|&b| b == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key_deterministic() {
        let key1 = generate_key(12345);
        let key2 = generate_key(12345);
        assert_eq!(key1, key2);

        let key3 = generate_key(54321);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_from_slice_valid() {
        let slice = &[
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F,
        ];
        let key = from_slice(slice).unwrap();
        assert_eq!(&key[..], slice);
    }

    #[test]
    fn test_from_slice_invalid_length() {
        let short = &[0x00, 0x01, 0x02];
        assert!(from_slice(short).is_err());

        // Test with a slice that's too long
        let long_array = [0u8; 32];
        assert!(from_slice(&long_array[..]).is_err());
    }

    #[test]
    fn test_zero_key() {
        let mut key = [0xFFu8; 16];
        zero_key(&mut key);
        assert!(is_zero_key(&key));
    }

    #[test]
    fn test_constant_time_eq() {
        let key1 = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ];
        let key2 = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ];
        let key3 = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x11,
        ];

        assert!(constant_time_eq(&key1, &key2));
        assert!(!constant_time_eq(&key1, &key3));
    }

    #[test]
    fn test_is_zero_key() {
        assert!(is_zero_key(&[0u8; 16]));
        assert!(!is_zero_key(&[0xFFu8; 16]));
        // Test with one byte non-zero
        let mut key = [0u8; 16];
        key[0] = 1;
        assert!(!is_zero_key(&key));
    }
}
