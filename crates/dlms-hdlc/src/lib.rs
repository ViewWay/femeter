//! HDLC link layer for DLMS/COSEM
//!
//! Reference: Green Book Ed.9 §8.4, §10

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod address;
pub mod config;
pub mod connection;
pub mod control;
pub mod crc;
pub mod frame;
pub mod llc;
pub mod segment;

pub use address::HdlcAddress;
pub use config::HdlcConfig;
pub use connection::{ConnectionState, HdlcConnection};
pub use control::{ControlField, FrameType};
pub use crc::crc16;
pub use frame::HdlcFrame;
