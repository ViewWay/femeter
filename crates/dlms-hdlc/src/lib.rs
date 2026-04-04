//! HDLC link layer for DLMS/COSEM
//!
//! Reference: Green Book Ed.9 §8.4, §10

//! HDLC link layer for DLMS/COSEM
//!
//! Reference: Green Book Ed.9 §8.4, §10
//!
//! # Module Overview
//!
//! - [`address`] — HDLC address encoding/decoding (extended addressing)
//! - [`config`] — HDLC configuration parameters for SNRM negotiation
//! - [`connection`] — HDLC connection state machine (client-side)
//! - [`control`] — Frame control field types (I, S, U frames)
//! - [`crc`] — CRC-16-CCITT calculation and verification
//! - [`frame`] — HDLC frame structure, encoding, decoding, byte stuffing
//! - [`llc`] — Logical Link Control sub-layer
//! - [`segment`] — Long frame segmentation and reassembly

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

/// HDLC address encoding/decoding (extended addressing)
pub mod address;
/// HDLC configuration parameters for SNRM negotiation
pub mod config;
/// HDLC connection state machine (client-side)
pub mod connection;
/// Frame control field types (I, S, U frames)
pub mod control;
/// CRC-16-CCITT calculation and verification
pub mod crc;
/// HDLC frame structure, encoding, decoding, byte stuffing
pub mod frame;
/// Logical Link Control sub-layer
pub mod llc;
/// Long frame segmentation and reassembly
pub mod segment;

pub use address::HdlcAddress;
pub use config::HdlcConfig;
pub use connection::{ConnectionState, HdlcConnection};
pub use control::{ControlField, FrameType};
pub use crc::crc16;
pub use frame::HdlcFrame;
