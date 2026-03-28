//!
//! HS-PLC Interface Classes (4 ICs)
//!
//! HS-PLC interface classes per IEC 62056-6-2.

pub mod ic140_hs_mac_setup;
pub mod ic141_hs_cpas_setup;
pub mod ic142_hs_ip_ssas_setup;
pub mod ic143_hs_hdlc_ssas_setup;

// Re-exports
pub use ic140_hs_mac_setup::HsMacSetup;
pub use ic141_hs_cpas_setup::HsCpasSetup;
pub use ic142_hs_ip_ssas_setup::HsIpSsasSetup;
pub use ic143_hs_hdlc_ssas_setup::HsHdlcSsasSetup;
