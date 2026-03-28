//!
//! S-FSK PLC Interface Classes (6 ICs)
//!
//! S-FSK (Spread-Frequency Shift Keying) PLC interface classes per IEC 61334-4-32.

pub mod ic50_sfsk_phy_mac_setup;
pub mod ic51_sfsk_active_initiator;
pub mod ic52_sfsk_mac_sync_timeouts;
pub mod ic53_sfsk_mac_counters;
pub mod ic55_iec_llc_setup;
pub mod ic56_sfsk_reporting_system_list;

// Re-exports
pub use ic50_sfsk_phy_mac_setup::SfskPhyMacSetup;
pub use ic51_sfsk_active_initiator::SfskActiveInitiator;
pub use ic52_sfsk_mac_sync_timeouts::SfskMacSyncTimeouts;
pub use ic53_sfsk_mac_counters::SfskMacCounters;
pub use ic55_iec_llc_setup::IecLlcSetup;
pub use ic56_sfsk_reporting_system_list::SfskReportingSystemList;
