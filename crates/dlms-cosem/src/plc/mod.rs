//!
//! Group 8a: PLC (Power Line Communication) Interface Classes (25 ICs)
//!
//! This module contains interface classes for PLC communication:
//!
//! **S-FSK PLC (6 ICs)**:
//! - IC 50: S-FSK Phy&MAC Setup
//! - IC 51: S-FSK Active Initiator
//! - IC 52: S-FSK MAC Synchronization Timeouts
//! - IC 53: S-FSK MAC Counters
//! - IC 55: IEC 61334-4-32 LLC Setup
//! - IC 56: S-FSK Reporting System List
//!
//! **PRIME PLC (7 ICs)**:
//! - IC 80: 61334-4-32 LLC SSCS Setup
//! - IC 81: PRIME NB OFDM PLC Physical Layer Counters
//! - IC 82: PRIME NB OFDM PLC MAC Setup
//! - IC 83: PRIME NB OFDM PLC MAC Functional Parameters
//! - IC 84: PRIME NB OFDM PLC MAC Counters
//! - IC 85: PRIME NB OFDM PLC MAC Network Admin Data
//! - IC 86: PRIME NB OFDM PLC Application Identification
//!
//! **G3-PLC (6 ICs)**:
//! - IC 90: G3-PLC MAC Layer Counters
//! - IC 91: G3-PLC MAC Setup
//! - IC 92: G3-PLC 6LoWPAN Adaptation Layer Setup
//! - IC 160: G3-PLC Hybrid RF MAC Layer Counters
//! - IC 161: G3-PLC Hybrid RF MAC Setup
//! - IC 162: G3-PLC Hybrid 6LoWPAN Adaptation Layer Setup
//!
//! **HS-PLC (4 ICs)**:
//! - IC 140: HS-PLC ISO/IEC 12139-1 MAC Setup
//! - IC 141: HS-PLC ISO/IEC 12139-1 CPAS Setup
//! - IC 142: HS-PLC ISO/IEC 12139-1 IP SSAS Setup
//! - IC 143: HS-PLC ISO/IEC 12139-1 HDLC SSAS Setup

// S-FSK PLC
pub mod sfsk;

// PRIME PLC
pub mod prime;

// G3-PLC
pub mod g3;

// HS-PLC
pub mod hs;

// Re-export commonly used types
pub use sfsk::{
    SfskPhyMacSetup, SfskActiveInitiator, SfskMacSyncTimeouts,
    SfskMacCounters, IecLlcSetup, SfskReportingSystemList,
};
pub use prime::{
    PrimeLlcSscsSetup, PrimePhyCounters, PrimeMacSetup,
    PrimeMacFunctionalParams, PrimeMacCounters, PrimeMacNetworkAdminData,
    PrimeAppIdentification,
};
pub use g3::{
    G3MacCounters, G3MacSetup, G3SixlowpanSetup,
    G3HybridRfMacCounters, G3HybridRfMacSetup, G3HybridSixlowpanSetup,
};
pub use hs::{
    HsMacSetup, HsCpasSetup, HsIpSsasSetup, HsHdlcSsasSetup,
};
