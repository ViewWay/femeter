//! A-XDR encoder/decoder for DLMS/COSEM data types
//!
//! Reference: Green Book Ed.9 §9.5
//!
//! # Module Overview
//!
//! - [`compact`] — Compact array encoding
//! - [`datetime_codec`] — COSEM date/time/datetime encoding
//! - [`decoder`] — A-XDR stream decoder
//! - [`encoder`] — A-XDR stream encoder

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

/// Compact array encoding
pub mod compact;
/// COSEM date/time/datetime encoding
pub mod datetime_codec;
/// A-XDR stream decoder
pub mod decoder;
/// A-XDR stream encoder
pub mod encoder;

pub use compact::CompactArrayCodec;
pub use decoder::AxdrDecoder;
pub use encoder::AxdrEncoder;

/// A-XDR encode/decode error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxdrError {
    BufferOverflow,
    BufferTooShort,
    InvalidTag(u8),
    InvalidLength,
    UnexpectedEnd,
    InvalidData(&'static str),
    TypeMismatch,
}
