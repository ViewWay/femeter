//! HDLC link layer for DLMS/COSEM
//!
//! Reference: Green Book Ed.9 §8.4, §10

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod frame;
pub mod address;
pub mod control;
pub mod crc;
pub mod llc;
pub mod segment;
pub mod connection;
pub mod config;

pub use frame::HdlcFrame;
pub use address::HdlcAddress;
pub use control::{ControlField, FrameType};
pub use crc::crc16;
pub use config::HdlcConfig;
pub use connection::{HdlcConnection, ConnectionState};
