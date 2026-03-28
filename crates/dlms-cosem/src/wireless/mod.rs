//!
//! Group 8b: Wireless Medium Interface Classes (17 ICs)
//!
//! This module contains interface classes for wireless communication:
//!
//! **ZigBee (5 ICs)**:
//! - IC 101: ZigBee SAS Startup
//! - IC 102: ZigBee SAS Join
//! - IC 103: ZigBee SAS APS Fragmentation
//! - IC 104: ZigBee Network Control
//! - IC 105: ZigBee Tunnel Setup
//!
//! **Wi-SUN (4 ICs)**:
//! - IC 95: Wi-SUN Setup
//! - IC 96: Wi-SUN Diagnostic
//! - IC 97: RPL Diagnostic
//! - IC 98: MPL Diagnostic
//!
//! **LPWAN (4 ICs)**:
//! - IC 126: SCHC-LPWAN Setup
//! - IC 127: SCHC-LPWAN Diagnostic
//! - IC 128: LoRaWAN Setup
//! - IC 129: LoRaWAN Diagnostic
//!
//! **ISO/IEC 14908 LON (4 ICs)**:
//! - IC 130: ISO/IEC 14908 Identification
//! - IC 131: ISO/IEC 14908 Protocol Setup
//! - IC 132: ISO/IEC 14908 Protocol Status
//! - IC 133: ISO/IEC 14908 Diagnostic

pub mod zigbee;
pub mod wisun;
pub mod lpwan;
pub mod lon;

// Re-export commonly used types
pub use zigbee::{
    ZigbeeSasStartup, ZigbeeSasJoin, ZigbeeSasApsFragmentation,
    ZigbeeNetworkControl, ZigbeeTunnelSetup,
};
pub use wisun::{
    WisunSetup, WisunDiagnostic, RplDiagnostic, MplDiagnostic,
};
pub use lpwan::{
    SchcLpwanSetup, SchcLpwanDiagnostic, LorawanSetup, LorawanDiagnostic,
};
pub use lon::{
    LonIdentification, LonProtocolSetup, LonProtocolStatus, LonDiagnostic,
};
