//! OBIS code definitions and lookup utilities
//!
//! Reference: Blue Book Part 1 (DLMS UA 1000-1 Ed.16)

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod codes;
pub mod lookup;
pub mod parser;

pub use lookup::{obis_description, obis_group, ObisGroup};
