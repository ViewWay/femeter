//!
//! GMAC (SM4-based) authentication for Chinese national standard compliance
//!
//! Implements GMAC (Galois Message Authentication Code) using SM4 block cipher,
//! as required by Chinese national standards for smart meter security.
//!
//! SM4 is a 128-bit block cipher defined in GB/T 32907-2016.
//! GMAC is a variant of GCM that only provides authentication (no encryption).
//!
//! This implementation uses a pure-Rust SM4 implementation suitable for no_std.
//!
//! Note: For production use, a hardware-accelerated SM4 implementation should
//! be preferred on embedded targets.

use dlms_core::errors::CosemError;

/// SM4 block size in bytes
const SM4_BLOCK_SIZE: usize = 16;

/// SM4 key size in bytes
const SM4_KEY_SIZE: usize = 16;

/// GMAC tag size in bytes
const GMAC_TAG_SIZE: usize = 16;

/// Number of SM4 rounds
const SM4_ROUNDS: usize = 32;

/// SM4 round constants (FK and CK)
const FK: [u32; 4] = [0xA3B1BAC6, 0x56AA3350, 0x677D9197, 0xB27022DC];
const CK: [u32; 32] = [
    0x00070E15, 0x1C232A31, 0x383F464D, 0x545B6269, 0x70777E85, 0x8C939AA1, 0xA8AFB6BD, 0xC4CBD2D9,
    0xE0E7EEF5, 0xFC030A11, 0x181F262D, 0x343B4249, 0x50575E65, 0x6C737A81, 0x888F969D, 0xA4ABB2B9,
    0xC0C7CED5, 0xDCE3EAF1, 0xF8FF060D, 0x141B2229, 0x30373E45, 0x4C535A61, 0x686F767D, 0x848B9299,
    0xA0A7AEB5, 0xBCC3CAD1, 0xD8DFE6ED, 0xF4FB0209, 0x10171E25, 0x2C333A41, 0x484F565D, 0x646B7279,
];

/// SM4 S-box (fixed lookup table)
const SBOX: [u8; 256] = [
    0xD6, 0x90, 0xE9, 0xFE, 0xCC, 0xE1, 0x3D, 0xB7, 0x16, 0xB6, 0x14, 0xC2, 0x28, 0xFB, 0x2C, 0x05,
    0x2B, 0x67, 0x9A, 0x76, 0x2A, 0xBE, 0x04, 0xC3, 0xAA, 0x44, 0x13, 0x26, 0x49, 0x86, 0x06, 0x99,
    0x9C, 0x42, 0x50, 0xF4, 0x91, 0xEF, 0x98, 0x7A, 0x33, 0x54, 0x0B, 0x43, 0xED, 0xCF, 0xAC, 0x62,
    0xE4, 0xB3, 0x1C, 0xA9, 0xC9, 0x08, 0xE8, 0x95, 0x80, 0xDF, 0x94, 0xFA, 0x75, 0x8F, 0x3F, 0xA6,
    0x47, 0x07, 0xA7, 0xFC, 0xF3, 0x73, 0x17, 0xBA, 0x83, 0x59, 0x3C, 0x19, 0xE6, 0x85, 0x4F, 0xA8,
    0x68, 0x6B, 0x81, 0xB2, 0x71, 0x64, 0xDA, 0x8B, 0xF8, 0xEB, 0x0F, 0x4B, 0x70, 0x56, 0x9D, 0x35,
    0x1E, 0x24, 0x0E, 0x5E, 0x63, 0x58, 0xD1, 0xA2, 0x25, 0x22, 0x7C, 0x3B, 0x01, 0x21, 0x78, 0x87,
    0xD4, 0x00, 0x46, 0x57, 0x9F, 0xD3, 0x27, 0x52, 0x4C, 0x36, 0x02, 0xE7, 0xA0, 0xC4, 0xC8, 0x9E,
    0xEA, 0xBF, 0x8A, 0xD2, 0x40, 0xC7, 0x38, 0xB5, 0xA3, 0xF7, 0xF2, 0xCE, 0xF9, 0x61, 0x15, 0xA1,
    0xE0, 0xAE, 0x5D, 0xA4, 0x9B, 0x34, 0x1A, 0x55, 0xAD, 0x93, 0x32, 0x30, 0xF5, 0x8C, 0xB1, 0xE3,
    0x1D, 0xF6, 0xE2, 0x2E, 0x82, 0x66, 0xCA, 0x60, 0xC0, 0x29, 0x23, 0xAB, 0x0D, 0x53, 0x4E, 0x6F,
    0xD5, 0xDB, 0x37, 0x45, 0xDE, 0xFD, 0x8E, 0x2F, 0x03, 0xFF, 0x6A, 0x72, 0x6D, 0x6C, 0x5B, 0x51,
    0x8D, 0x1B, 0xAF, 0x92, 0xBB, 0xDD, 0xBC, 0x7F, 0x11, 0xD9, 0x5C, 0x41, 0x1F, 0x10, 0x5A, 0xD8,
    0x0A, 0xC1, 0x31, 0x88, 0xA5, 0xCD, 0x7B, 0xBD, 0x2D, 0x74, 0xD0, 0x12, 0xB8, 0xE5, 0xB4, 0xB0,
    0x89, 0x69, 0x97, 0x4A, 0x0C, 0x96, 0x77, 0x7E, 0x65, 0xB9, 0xF1, 0x09, 0xC5, 0x6E, 0xC6, 0x84,
    0x18, 0xF0, 0x7D, 0xEC, 0x3A, 0xDC, 0x4D, 0x20, 0x79, 0xEE, 0x5F, 0x3E, 0xD7, 0xCB, 0x39, 0x48,
];

/// SM4 round key schedule
#[derive(Clone)]
pub struct Sm4KeySchedule {
    rk: [u32; SM4_ROUNDS],
}

impl Sm4KeySchedule {
    /// Expand a 16-byte key into 32 round keys
    pub fn new(key: &[u8; SM4_KEY_SIZE]) -> Self {
        let mk = u32s_from_bytes(key);
        let mut k = [0u32; 4];
        for i in 0..4 {
            k[i] = mk[i] ^ FK[i];
        }

        let mut rk = [0u32; 32];
        for i in 0..32 {
            rk[i] = k[0] ^ t1(k[1] ^ k[2] ^ k[3] ^ CK[i]);
            k[0] = k[1];
            k[1] = k[2];
            k[2] = k[3];
            k[3] = rk[i];
        }
        Self { rk }
    }

    /// Encrypt a single 16-byte block
    pub fn encrypt_block(&self, block: &[u8; SM4_BLOCK_SIZE]) -> [u8; SM4_BLOCK_SIZE] {
        let mut x = u32s_from_bytes(block);
        for i in 0..32 {
            let tmp = x[1] ^ x[2] ^ x[3] ^ self.rk[i];
            x[0] ^= t0(tmp);
            // Rotate left
            let tmp = x[0];
            x[0] = x[1];
            x[1] = x[2];
            x[2] = x[3];
            x[3] = tmp;
        }
        // Reverse order for output
        bytes_from_u32s(&[x[3], x[2], x[1], x[0]])
    }
}

/// Non-linear transformation τ (S-box substitution + linear transform L)
fn t0(a: u32) -> u32 {
    let b = sbox_sub(a);
    l(b)
}

/// Non-linear transformation for key expansion (uses L' instead of L)
fn t1(a: u32) -> u32 {
    let b = sbox_sub(a);
    l_prime(b)
}

/// S-box substitution on a 32-bit word
fn sbox_sub(a: u32) -> u32 {
    let bytes = a.to_be_bytes();
    let r = [
        SBOX[bytes[0] as usize],
        SBOX[bytes[1] as usize],
        SBOX[bytes[2] as usize],
        SBOX[bytes[3] as usize],
    ];
    u32::from_be_bytes(r)
}

/// Linear transform L
fn l(b: u32) -> u32 {
    b ^ b.rotate_left(2) ^ b.rotate_left(10) ^ b.rotate_left(18) ^ b.rotate_left(24)
}

/// Linear transform L' (for key expansion)
fn l_prime(b: u32) -> u32 {
    b ^ b.rotate_left(13) ^ b.rotate_left(23)
}

/// Convert 16 bytes to 4 big-endian u32s
fn u32s_from_bytes(bytes: &[u8; 16]) -> [u32; 4] {
    [
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
    ]
}

/// Convert 4 big-endian u32s to 16 bytes
fn bytes_from_u32s(words: &[u32; 4]) -> [u8; 16] {
    let a = words[0].to_be_bytes();
    let b = words[1].to_be_bytes();
    let c = words[2].to_be_bytes();
    let d = words[3].to_be_bytes();
    [
        a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3], d[0], d[1], d[2],
        d[3],
    ]
}

/// XOR two 16-byte blocks
fn xor_blocks(a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
    let mut result = [0u8; 16];
    for (i, (ai, bi)) in a.iter().zip(b).enumerate() {
        result[i] = ai ^ bi;
    }
    result
}

/// Increment a 16-byte counter (big-endian)
#[allow(dead_code)]
fn increment_counter(counter: &mut [u8; 16]) {
    for i in (0..16).rev() {
        counter[i] = counter[i].wrapping_add(1);
        if counter[i] != 0 {
            break;
        }
    }
}

/// GF(2^128) multiplication for GHASH
fn gf128_mul(x: &[u8; 16], y: &[u8; 16]) -> [u8; 16] {
    let mut z = [0u8; 16];
    let mut v = *y;

    for byte in x.iter() {
        for bit in 0..8 {
            if (*byte >> (7 - bit)) & 1 == 1 {
                z = xor_blocks(&z, &v);
            }
            let carry = (v[15] & 1) != 0;
            // Right shift V by 1
            for j in (1..16).rev() {
                v[j] = (v[j] >> 1) | (v[j - 1] << 7);
            }
            v[0] >>= 1;
            if carry {
                v[0] ^= 0xE1; // R = 11100001
            }
        }
    }
    z
}

/// Compute GMAC tag using SM4
///
/// Parameters:
/// - `key`: 16-byte SM4 key
/// - `iv`: 12-byte initialization vector (nonce)
/// - `data`: message to authenticate
///
/// Returns: 16-byte authentication tag
pub fn sm4_gmac(key: &[u8; SM4_KEY_SIZE], iv: &[u8; 8], data: &[u8]) -> [u8; GMAC_TAG_SIZE] {
    let schedule = Sm4KeySchedule::new(key);

    // Construct 12-byte IV: pad iv[0..8] with iv[8..12] = {0,0,0,1}
    let mut full_iv = [0u8; 12];
    full_iv[..8].copy_from_slice(iv);
    full_iv[8..12].copy_from_slice(&[0, 0, 0, 1]);

    // Compute H = E_K(0^128)
    let h_block = [0u8; 16];
    let h = schedule.encrypt_block(&h_block);

    // GHASH computation
    let mut y = [0u8; 16];

    // Pad data to multiple of 16 bytes
    let padded_len = if data.is_empty() {
        0
    } else {
        data.len().div_ceil(16) * 16
    };
    let mut padded = alloc::vec![0u8; padded_len];
    padded[..data.len()].copy_from_slice(data);

    for chunk in padded.chunks_exact(16) {
        let block: [u8; 16] = chunk.try_into().unwrap();
        y = xor_blocks(&y, &block);
        y = gf128_mul(&y, &h);
    }

    // Length block: [0;8] || len(data) in bits as 64-bit BE
    let len_bits = (data.len() as u64) * 8;
    let mut len_block = [0u8; 16];
    len_block[8..16].copy_from_slice(&len_bits.to_be_bytes());

    y = xor_blocks(&y, &len_block);
    y = gf128_mul(&y, &h);

    // Encrypt IV to get final tag
    let mut j0 = [0u8; 16];
    j0[..12].copy_from_slice(&full_iv);
    let encrypted_j0 = schedule.encrypt_block(&j0);

    xor_blocks(&encrypted_j0, &y)
}

/// Verify GMAC tag
///
/// Returns Ok(()) if the tag is valid, Err otherwise.
pub fn sm4_gmac_verify(
    key: &[u8; SM4_KEY_SIZE],
    iv: &[u8; 8],
    data: &[u8],
    expected_tag: &[u8; GMAC_TAG_SIZE],
) -> Result<(), CosemError> {
    let computed = sm4_gmac(key, iv, data);
    if computed == *expected_tag {
        Ok(())
    } else {
        Err(CosemError::AccessDenied)
    }
}

/// SM4-ECB encrypt (single block, for GCM compatibility)
pub fn sm4_encrypt_block(key: &[u8; SM4_KEY_SIZE], block: &[u8; 16]) -> [u8; 16] {
    Sm4KeySchedule::new(key).encrypt_block(block)
}

/// SM4-GCM encrypt
///
/// Full GCM mode: encryption + authentication
#[allow(dead_code)]
pub fn sm4_gcm_encrypt(
    key: &[u8; SM4_KEY_SIZE],
    iv: &[u8; 8],
    plaintext: &[u8],
    aad: &[u8],
) -> alloc::vec::Vec<u8> {
    let schedule = Sm4KeySchedule::new(key);

    // H = E_K(0^128)
    let h = schedule.encrypt_block(&[0u8; 16]);

    // J0 = IV || 0x00000001
    let mut j0 = [0u8; 16];
    j0[..8].copy_from_slice(iv);
    j0[8..12].copy_from_slice(&[0, 0, 0, 1]);

    // Encrypted J0 for final tag
    let e_j0 = schedule.encrypt_block(&j0);

    // Counter starts at J0 incremented
    let mut counter = j0;
    increment_counter(&mut counter);

    // Encrypt plaintext with CTR mode
    let mut ciphertext = alloc::vec![0u8; plaintext.len()];
    for (i, chunk) in plaintext.chunks(16).enumerate() {
        let enc_counter = schedule.encrypt_block(&counter);
        for (j, &byte) in chunk.iter().enumerate() {
            ciphertext[i * 16 + j] = byte ^ enc_counter[j];
        }
        increment_counter(&mut counter);
    }

    // GHASH over AAD then ciphertext
    let mut y = [0u8; 16];

    // Process AAD
    let aad_padded_len = if aad.is_empty() {
        0
    } else {
        aad.len().div_ceil(16) * 16
    };
    let mut aad_padded = alloc::vec![0u8; aad_padded_len];
    aad_padded[..aad.len()].copy_from_slice(aad);
    for chunk in aad_padded.chunks_exact(16) {
        let block: [u8; 16] = chunk.try_into().unwrap();
        y = xor_blocks(&y, &block);
        y = gf128_mul(&y, &h);
    }

    // Process ciphertext
    let ct_padded_len = if ciphertext.is_empty() {
        0
    } else {
        ciphertext.len().div_ceil(16) * 16
    };
    let mut ct_padded = alloc::vec![0u8; ct_padded_len];
    ct_padded[..ciphertext.len()].copy_from_slice(&ciphertext);
    for chunk in ct_padded.chunks_exact(16) {
        let block: [u8; 16] = chunk.try_into().unwrap();
        y = xor_blocks(&y, &block);
        y = gf128_mul(&y, &h);
    }

    // Length block: len(A) || len(C) in bits
    let mut len_block = [0u8; 16];
    len_block[..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
    len_block[8..].copy_from_slice(&((ciphertext.len() as u64) * 8).to_be_bytes());
    y = xor_blocks(&y, &len_block);
    y = gf128_mul(&y, &h);

    // Tag = E_K(J0) ^ GHASH
    let tag = xor_blocks(&e_j0, &y);

    // Append tag to ciphertext
    ciphertext.extend_from_slice(&tag);
    ciphertext
}

/// SM4-GCM decrypt and verify
#[allow(dead_code)]
pub fn sm4_gcm_decrypt(
    key: &[u8; SM4_KEY_SIZE],
    iv: &[u8; 8],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<alloc::vec::Vec<u8>, CosemError> {
    if ciphertext.len() < GMAC_TAG_SIZE {
        return Err(CosemError::AccessDenied);
    }

    let ct_len = ciphertext.len() - GMAC_TAG_SIZE;
    let ct = &ciphertext[..ct_len];
    let tag = &ciphertext[ct_len..];

    // Recompute tag
    let expected = sm4_gmac_with_aad(key, iv, ct, aad);

    // Compare tags (constant-time would be better but this is sufficient for now)
    if &expected[..] != tag {
        return Err(CosemError::AccessDenied);
    }

    // Decrypt with CTR
    let schedule = Sm4KeySchedule::new(key);
    let mut j0 = [0u8; 16];
    j0[..8].copy_from_slice(iv);
    j0[8..12].copy_from_slice(&[0, 0, 0, 1]);
    let mut counter = j0;
    increment_counter(&mut counter);

    let mut plaintext = alloc::vec![0u8; ct_len];
    for (i, chunk) in ct.chunks(16).enumerate() {
        let enc_counter = schedule.encrypt_block(&counter);
        for (j, &byte) in chunk.iter().enumerate() {
            plaintext[i * 16 + j] = byte ^ enc_counter[j];
        }
        increment_counter(&mut counter);
    }

    Ok(plaintext)
}

/// GMAC with separate AAD (used internally)
#[allow(dead_code)]
fn sm4_gmac_with_aad(
    key: &[u8; SM4_KEY_SIZE],
    iv: &[u8; 8],
    data: &[u8],
    aad: &[u8],
) -> [u8; GMAC_TAG_SIZE] {
    let schedule = Sm4KeySchedule::new(key);
    let h = schedule.encrypt_block(&[0u8; 16]);

    let mut j0 = [0u8; 16];
    j0[..8].copy_from_slice(iv);
    j0[8..12].copy_from_slice(&[0, 0, 0, 1]);
    let e_j0 = schedule.encrypt_block(&j0);

    let mut y = [0u8; 16];

    // Process AAD
    let aad_pad = aad.len().div_ceil(16) * 16;
    let mut buf = alloc::vec![0u8; aad_pad.max(1)];
    buf[..aad.len()].copy_from_slice(aad);
    for chunk in buf.chunks_exact(16) {
        let block: [u8; 16] = chunk.try_into().unwrap();
        y = xor_blocks(&y, &block);
        y = gf128_mul(&y, &h);
    }

    // Process data
    let data_pad = data.len().div_ceil(16) * 16;
    buf = alloc::vec![0u8; data_pad.max(1)];
    buf[..data.len()].copy_from_slice(data);
    for chunk in buf.chunks_exact(16) {
        let block: [u8; 16] = chunk.try_into().unwrap();
        y = xor_blocks(&y, &block);
        y = gf128_mul(&y, &h);
    }

    let mut len_block = [0u8; 16];
    len_block[..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
    len_block[8..].copy_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    y = xor_blocks(&y, &len_block);
    y = gf128_mul(&y, &h);

    xor_blocks(&e_j0, &y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];

    #[test]
    fn test_sbox() {
        // Known SM4 S-box test vectors
        assert_eq!(SBOX[0x00], 0xD6);
        assert_eq!(SBOX[0x01], 0x90);
        assert_eq!(SBOX[0xFF], 0x48);
    }

    #[test]
    fn test_sm4_key_expansion() {
        let schedule = Sm4KeySchedule::new(&TEST_KEY);
        // Just verify it doesn't panic and produces non-zero round keys
        assert_ne!(schedule.rk[0], 0);
        assert_ne!(schedule.rk[31], 0);
    }

    #[test]
    fn test_sm4_encrypt_block() {
        let schedule = Sm4KeySchedule::new(&TEST_KEY);
        let plaintext = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let ciphertext = schedule.encrypt_block(&plaintext);
        // Encrypting the same block twice should give same result
        let ciphertext2 = schedule.encrypt_block(&plaintext);
        assert_eq!(ciphertext, ciphertext2);
        // Should not equal plaintext (for this key)
        assert_ne!(ciphertext, plaintext);
    }

    #[test]
    fn test_gmac_roundtrip() {
        let iv = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let data = b"Hello, SM4 GMAC!";

        let tag = sm4_gmac(&TEST_KEY, &iv, data);
        assert_eq!(tag.len(), 16);
        assert!(sm4_gmac_verify(&TEST_KEY, &iv, data, &tag).is_ok());
    }

    #[test]
    fn test_gmac_wrong_data_fails() {
        let iv = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let tag = sm4_gmac(&TEST_KEY, &iv, b"correct data");
        assert!(sm4_gmac_verify(&TEST_KEY, &iv, b"wrong data", &tag).is_err());
    }

    #[test]
    fn test_gmac_empty_data() {
        let iv = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let tag = sm4_gmac(&TEST_KEY, &iv, b"");
        assert_eq!(tag.len(), 16);
        assert!(sm4_gmac_verify(&TEST_KEY, &iv, b"", &tag).is_ok());
    }

    #[test]
    fn test_gcm_encrypt_decrypt_roundtrip() {
        let iv = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let plaintext = b"Test SM4 GCM encryption";
        let aad = b"additional data";

        let ct = sm4_gcm_encrypt(&TEST_KEY, &iv, plaintext, aad);
        assert_eq!(ct.len(), plaintext.len() + 16); // ciphertext + tag

        let pt = sm4_gcm_decrypt(&TEST_KEY, &iv, &ct, aad).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_gcm_wrong_aad_fails() {
        let iv = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let ct = sm4_gcm_encrypt(&TEST_KEY, &iv, b"secret", b"correct aad");
        assert!(sm4_gcm_decrypt(&TEST_KEY, &iv, &ct, b"wrong aad").is_err());
    }

    #[test]
    fn test_gcm_empty_plaintext() {
        let iv = [0x00; 8];
        let ct = sm4_gcm_encrypt(&TEST_KEY, &iv, b"", b"");
        assert_eq!(ct.len(), 16); // just tag
        let pt = sm4_gcm_decrypt(&TEST_KEY, &iv, &ct, b"").unwrap();
        assert_eq!(pt, b"" as &[u8]);
    }

    #[test]
    fn test_xor_blocks() {
        let a = [0xFF; 16];
        let b = [0x00; 16];
        let c = xor_blocks(&a, &b);
        assert_eq!(c, a);

        let d = xor_blocks(&a, &a);
        assert_eq!(d, [0u8; 16]);
    }

    #[test]
    fn test_increment_counter() {
        let mut c = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        increment_counter(&mut c);
        assert_eq!(c[15], 1);

        let mut c2 = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF];
        increment_counter(&mut c2);
        assert_eq!(c2[13], 1);
        assert_eq!(c2[14], 0);
        assert_eq!(c2[15], 0);
    }
}
