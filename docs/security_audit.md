# Security Audit Report

**Date:** 2026-04-03  
**Scope:** All workspace crates (excluding femeter-firmware)  
**Status:** ✅ No critical issues found

## 1. Array Bounds Safety

### Summary
All crates use safe Rust patterns. Direct indexing is limited to:
- Fixed-size arrays with compile-time known bounds (e.g., `[u8; 16]` key operations)
- `chunks_exact()` which guarantees element count
- Test code (acceptable)

### Findings
- **dlms-security/src/sm4_gmac.rs**: `chunks_exact(16)` + `try_into().unwrap()` — safe due to `chunks_exact` contract. Added `expect()` for clarity.
- **dlms-core/src/obis.rs**: OBIS code parsing uses bounded indexing into `[u8; 6]` — safe by construction.
- **dlms-meter-app/src/**: Energy accumulators use `saturating_add` for overflow protection.

### Recommendation
No changes needed. All direct indexing is either compile-time verified or guarded.

## 2. Integer Overflow

### Summary
✅ Energy accumulation uses `saturating_add` / `saturating_sub` correctly.

### Key Locations
| File | Operation | Method |
|------|-----------|--------|
| `femeter-core/src/event_detect.rs` | Tick counters | `saturating_add` |
| `femeter-core/src/tamper_detection.rs` | CT short count | `saturating_add` |
| `dlms-meter-app/src/common.rs` | Energy registers | `saturating_add` |
| `dlms-security/src/key.rs` | Key derivation | `wrapping_mul/add` (intentional) |

### Recommendation
Current approach is correct. `wrapping_mul` in key derivation is intentional for deterministic generation.

## 3. Sensitive Data Handling

### Summary
✅ No sensitive data (keys, secrets, tokens) is logged or printed.

### Verification
- `rg 'dbg!|println!.*key|println!.*secret'` — zero matches in production code
- `dlms-security/src/key.rs` provides `zero_key()` for secure memory clearing
- `constant_time_eq()` is used for key comparison (timing-safe)
- No `#[derive(Debug)]` on key-containing types that could leak keys via logging

### Recommendation
- Consider adding `#[derive(Debug)]` with redacted output for key types if Debug is needed in future
- Ensure `zero_key()` is called on key drop (consider `Drop` impl)

## 4. Error Handling

### Summary
✅ Unified `FemeterError` enum created in `femeter-core` using `thiserror`.

### Error Categories Covered
- `InvalidParameter` — invalid input values
- `BufferOverflow` — insufficient capacity
- `OutOfRange` — value out of expected bounds
- `CalibrationError` — calibration failures
- `StorageError` — read/write/erase failures
- `CommunicationError` — timeout/CRC/frame errors
- `SecurityError` — authentication/encryption failures
- `InternalError` — consistency violations

### Changes Made
- Non-test `unwrap()` calls replaced with `expect("reason")` in:
  - `dlms-rtos/src/std_impl.rs` (RwLock poisoning)
  - `virtual-meter/src/*.rs` (Mutex poisoning)
  - `dlms-security/src/sm4_gmac.rs` (chunks_exact invariant)
  - `virtual-meter/src/tcp_server.rs` (address parsing)

## 5. Cryptographic Security

### Summary
- AES-128-GCM encryption/decryption implemented correctly
- SM4-GMAC for Chinese national standard compliance
- HLS (High Level Security) authentication with challenge-response
- Counter-based nonce generation (must ensure uniqueness in production)

### Recommendation
- Monitor counter rollover in production firmware
- Consider AES-256 upgrade path for future requirements

## 6. Dependencies

No known vulnerable dependencies at this audit date. Run `cargo audit` periodically.
