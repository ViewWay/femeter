//! A-XDR encoder/decoder for DLMS/COSEM data types
//!
//! Reference: Green Book Ed.9 §9.5

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod encoder;
pub mod decoder;
pub mod compact;
pub mod datetime_codec;

pub use encoder::AxdrEncoder;
pub use decoder::AxdrDecoder;
pub use compact::CompactArrayCodec;

use dlms_core::DlmsType;

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
