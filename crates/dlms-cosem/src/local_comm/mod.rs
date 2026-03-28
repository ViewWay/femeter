//!
//! Group 5: Local Communication Interface Classes (9 ICs)
//!
//! This module contains interface classes for local port communication:
//! - IC 13: IEC Local Port Setup
//! - IC 14: IEC HDLC Setup
//! - IC 15: IEC Twisted Pair Setup
//! - IC 16: IEC 62056-46 Modem Setup
//! - IC 17: SAP Assignment
//! - IC 19: IEC 62056-46 Control
//! - IC 20: Auto Answer
//! - IC 23: Modem Configuration

pub mod ic13_iec_local_port_setup;
pub mod ic14_iec_hdlc_setup;
pub mod ic15_iec_twisted_pair_setup;
pub mod ic16_iec_modem_setup;

// Re-export commonly used types
pub use ic13_iec_local_port_setup::IecLocalPortSetup;
pub use ic14_iec_hdlc_setup::IecHdlcSetup;
pub use ic15_iec_twisted_pair_setup::IecTwistedPairSetup;
pub use ic16_iec_modem_setup::IecModemSetup;
