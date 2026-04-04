//! OBIS code definitions and lookup utilities
//!
//! Reference: Blue Book Part 1 (DLMS UA 1000-1 Ed.16)
//!
//! # Module Overview
//!
//! - [`codes`] — Common OBIS code constants for metering
//! - [`lookup`] — OBIS group classification and description lookup
//! - [`parser`] — Parse "A.B.C.D.E.F" string format

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

/// Common OBIS code constants for metering
pub mod codes;
/// OBIS group classification and description lookup
pub mod lookup;
/// Parse "A.B.C.D.E.F" string format
pub mod parser;

pub use lookup::{obis_description, obis_group, ObisGroup};
