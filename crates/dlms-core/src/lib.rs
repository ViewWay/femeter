//! DLMS/COSEM core data types and error definitions
//!
//! This crate provides the foundational types shared by all other FeMeter crates:
//! - COSEM data type enum (A-XDR mapped)
//! - OBIS code structure
//! - Physical unit enumeration
//! - Date/time types
//! - Error types for all protocol layers
//! - Core traits for COSEM interface classes
//!
//! # Module Overview
//!
//! - [`access`] — Attribute and method access mode definitions
//! - [`datetime`] — Special date/time constants
//! - [`errors`] — Error types for all DLMS protocol layers
//! - [`obis`] — OBIS code structure and common constants
//! - [`traits`] — COSEM class trait definitions
//! - [`types`] — COSEM data types (DlmsType enum, date/time structs)
//! - [`units`] — Physical unit enumeration

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

/// Attribute and method access mode definitions
pub mod access;
/// Special date/time constants
pub mod datetime;
/// Error types for all DLMS protocol layers
pub mod errors;
/// OBIS code structure and common constants
pub mod obis;
/// COSEM class trait definitions
pub mod traits;
/// COSEM data types (DlmsType enum, date/time structs)
pub mod types;
/// Physical unit enumeration
pub mod units;

pub use access::AccessMode;
pub use datetime::*;
pub use errors::*;
pub use obis::ObisCode;
pub use traits::CosemClass;
pub use types::DlmsType;
pub use units::Unit;
