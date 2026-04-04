//! ASN.1 BER encoder/decoder for DLMS/COSEM
//!
//! Handles AARQ/AARE/RLRQ/RLRE PDUs and ConformanceBlock.
//!
//! # Module Overview
//!
//! - [`aare`] — Application Association Response Entity
//! - [`aarq`] — Application Association Request Entity
//! - [`ber`] — Basic Encoding Rules encoder/decoder
//! - [`conformance`] — Conformance block bit flags
//! - [`context`] — Application context name definitions
//! - [`initiate`] — Initiate request/response
//! - [`rlrq_rlre`] — Release request/response

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

/// Application Association Response Entity
pub mod aare;
/// Application Association Request Entity
pub mod aarq;
/// Basic Encoding Rules encoder/decoder
pub mod ber;
/// Conformance block bit flags
pub mod conformance;
/// Application context name definitions
pub mod context;
/// Initiate request/response
pub mod initiate;
/// Release request/response
pub mod rlrq_rlre;

pub use aare::{decode_aare, encode_aare, Aare};
pub use aarq::{decode_aarq, encode_aarq, Aarq};
pub use ber::{BerDecoder, BerEncoder, BerError, BerTag};
pub use conformance::ConformanceBlock;
pub use context::ApplicationContextName;
pub use initiate::{
    decode_initiate_request, decode_initiate_response, encode_initiate_request,
    encode_initiate_response, InitiateRequest, InitiateResponse,
};
pub use rlrq_rlre::{decode_rlre, encode_rlrq, Rlre, Rlrq};
