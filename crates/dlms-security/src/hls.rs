//! High Level Security (HLS)
//!
//! Implements HLS authentication using GMAC or SHA256.
//! Reference: IEC 62056-6-2 §8.5.2

use alloc::vec::Vec;
use dlms_core::errors::SecurityError;

use crate::{aes_gcm::compute_gmac, HLS_AUTH_VALUE_SIZE};

/// Compute the HLS authentication value
///
/// For HLS-GMAC, this computes AES-GMAC over the challenge.
/// For HLS-SHA256, this would compute HMAC-SHA256 (not yet implemented).
///
/// # Arguments
/// * `ctx` - Security context
/// * `challenge` - Random challenge from the meter
///
/// # Returns
/// 16-byte authentication value
pub fn compute_auth_value(
    ctx: &crate::SecurityContext,
    challenge: &[u8],
) -> Result<Vec<u8>, SecurityError> {
    match ctx.security_level() {
        crate::SecurityLevel::HlsGmac => {
            let auth_key = ctx.get_auth_key()?;
            let tag = compute_gmac(
                auth_key,
                ctx.system_title(),
                ctx.frame_counter(),
                challenge,
            )?;
            Ok(tag.to_vec())
        }
        crate::SecurityLevel::HlsSha256 => {
            // SHA256-based HLS would use HMAC-SHA256
            // This is a placeholder implementation
            compute_hmac_sha256_placeholder(ctx, challenge)
        }
        _ => Err(SecurityError::InvalidSecurityLevel(ctx.security_level() as u8)),
    }
}

/// Verify an HLS authentication value
///
/// # Arguments
/// * `ctx` - Security context
/// * `challenge` - The challenge that was sent
/// * `received` - The authentication value received from the meter
///
/// # Returns
/// Ok(()) if authentication succeeds, Err otherwise
pub fn verify_auth_value(
    ctx: &crate::SecurityContext,
    challenge: &[u8],
    received: &[u8],
) -> Result<(), SecurityError> {
    let computed = compute_auth_value(ctx, challenge)?;

    if received.len() != computed.len() {
        return Err(SecurityError::AuthenticationFailed);
    }

    // Constant-time comparison
    let mut result = 0u8;
    for (a, b) in computed.iter().zip(received.iter()) {
        result |= a ^ b;
    }

    if result == 0 {
        Ok(())
    } else {
        Err(SecurityError::AuthenticationFailed)
    }
}

/// Generate a random HLS challenge
///
/// In a real system, this would use a cryptographically secure RNG.
/// This is a deterministic placeholder for testing.
#[cfg(feature = "std")]
pub fn generate_challenge() -> [u8; 8] {
    // Simple deterministic challenge - in production use proper RNG
    [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
}

/// Placeholder for HMAC-SHA256 based authentication
///
/// This is a simplified implementation. In production, use a proper
/// crypto library like sha2 and hmac.
fn compute_hmac_sha256_placeholder(
    ctx: &crate::SecurityContext,
    challenge: &[u8],
) -> Result<Vec<u8>, SecurityError> {
    // This is NOT a proper HMAC-SHA256 implementation
    // It's a placeholder to allow the code to compile
    // Real implementation should use the hmac and sha2 crates

    let auth_key = ctx.get_auth_key()?;

    // Simple placeholder: XOR-fold key, challenge, system title, and counter
    let mut result = alloc::vec![0u8; HLS_AUTH_VALUE_SIZE];

    let mut hash: u32 = 0x67452301u32;

    // Mix in key
    for (i, &byte) in auth_key.iter().enumerate() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32).wrapping_add(i as u32);
    }

    // Mix in challenge
    for (i, &byte) in challenge.iter().enumerate() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32).wrapping_add(i as u32);
    }

    // Mix in system title
    for (i, &byte) in ctx.system_title().iter().enumerate() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32).wrapping_add(i as u32);
    }

    // Mix in counter
    hash = hash.wrapping_mul(31).wrapping_add(ctx.frame_counter());

    // Expand to 16 bytes
    for i in 0..HLS_AUTH_VALUE_SIZE {
        let mixed = hash.wrapping_mul((i as u32).wrapping_add(1)).wrapping_add(0x9E3779B9);
        result[i] = ((mixed >> (i % 4 * 8)) & 0xFF) as u8;
    }

    Ok(result)
}

/// Prepare an HLS authentication request
///
/// # Arguments
/// * `_ctx` - Security context (unused, kept for API compatibility)
///
/// # Returns
/// The challenge that should be sent to the meter
#[cfg(feature = "std")]
pub fn prepare_authentication(_ctx: &crate::SecurityContext) -> [u8; 8] {
    // The client typically generates a random challenge
    generate_challenge()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
        0x0E, 0x0F,
    ];
    const TEST_SYSTEM_TITLE: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    fn create_test_context() -> crate::SecurityContext {
        crate::SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(crate::SecurityLevel::HlsGmac)
            .with_auth_key(TEST_KEY)
            .with_frame_counter(0)
    }

    #[test]
    fn test_compute_auth_value_gmac() {
        let ctx = create_test_context();
        let challenge = b"challenge";

        let result = compute_auth_value(&ctx, challenge);
        assert!(result.is_ok());
        let auth_value = result.unwrap();
        assert_eq!(auth_value.len(), HLS_AUTH_VALUE_SIZE);
    }

    #[test]
    fn test_compute_auth_value_deterministic() {
        let ctx = create_test_context();
        let challenge = b"same challenge";

        let auth1 = compute_auth_value(&ctx, challenge).unwrap();
        let auth2 = compute_auth_value(&ctx, challenge).unwrap();

        assert_eq!(auth1, auth2);
    }

    #[test]
    fn test_compute_auth_value_different_challenges() {
        let ctx = create_test_context();

        let auth1 = compute_auth_value(&ctx, b"challenge1").unwrap();
        let auth2 = compute_auth_value(&ctx, b"challenge2").unwrap();

        assert_ne!(auth1, auth2);
    }

    #[test]
    fn test_verify_auth_value_success() {
        let ctx = create_test_context();
        let challenge = b"test challenge";

        let auth_value = compute_auth_value(&ctx, challenge).unwrap();
        let result = verify_auth_value(&ctx, challenge, &auth_value);

        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_auth_value_failure() {
        let ctx = create_test_context();
        let challenge = b"test challenge";

        let mut auth_value = compute_auth_value(&ctx, challenge).unwrap();
        auth_value[0] ^= 0xFF; // Tamper with the value

        let result = verify_auth_value(&ctx, challenge, &auth_value);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_auth_value_wrong_length() {
        let ctx = create_test_context();
        let challenge = b"test challenge";

        let wrong_length = [0u8; 8]; // Wrong length
        let result = verify_auth_value(&ctx, challenge, &wrong_length);

        assert!(result.is_err());
    }

    #[test]
    fn test_auth_value_depends_on_counter() {
        let mut ctx = create_test_context();
        let challenge = b"challenge";

        let auth1 = compute_auth_value(&ctx, challenge).unwrap();

        ctx.increment_counter().unwrap();
        let auth2 = compute_auth_value(&ctx, challenge).unwrap();

        assert_ne!(auth1, auth2);
    }

    #[test]
    fn test_auth_value_depends_on_key() {
        let ctx1 = crate::SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(crate::SecurityLevel::HlsGmac)
            .with_auth_key(TEST_KEY)
            .with_frame_counter(0);

        let different_key = [0xFFu8; 16];
        let ctx2 = crate::SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(crate::SecurityLevel::HlsGmac)
            .with_auth_key(different_key)
            .with_frame_counter(0);

        let challenge = b"challenge";

        let auth1 = compute_auth_value(&ctx1, challenge).unwrap();
        let auth2 = compute_auth_value(&ctx2, challenge).unwrap();

        assert_ne!(auth1, auth2);
    }

    #[test]
    fn test_compute_auth_value_wrong_security_level() {
        let ctx = crate::SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(crate::SecurityLevel::None);

        let result = compute_auth_value(&ctx, b"challenge");
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_auth_value_no_key() {
        let ctx = crate::SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(crate::SecurityLevel::HlsGmac);
        // No auth key set

        let result = compute_auth_value(&ctx, b"challenge");
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_auth_value_sha256_placeholder() {
        let ctx = crate::SecurityContext::new(TEST_SYSTEM_TITLE)
            .with_level(crate::SecurityLevel::HlsSha256)
            .with_auth_key(TEST_KEY)
            .with_frame_counter(0);

        let challenge = b"test challenge";
        let result = compute_auth_value(&ctx, challenge);

        // Should succeed (even though it's a placeholder)
        assert!(result.is_ok());
        let auth_value = result.unwrap();
        assert_eq!(auth_value.len(), HLS_AUTH_VALUE_SIZE);
    }
}
