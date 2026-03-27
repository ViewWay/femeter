//! DLMS/COSEM core data types and error definitions
//!
//! This crate provides the foundational types shared by all other FeMeter crates:
//! - COSEM data type enum (A-XDR mapped)
//! - OBIS code structure
//! - Physical unit enumeration
//! - Date/time types
//! - Error types for all protocol layers
//! - Core traits for COSEM interface classes

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod types;
pub mod obis;
pub mod units;
pub mod errors;
pub mod datetime;
pub mod traits;
pub mod access;

pub use types::DlmsType;
pub use obis::ObisCode;
pub use units::Unit;
pub use errors::*;
pub use datetime::*;
pub use traits::CosemClass;
pub use access::AccessMode;
