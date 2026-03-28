//!
//! PRIME PLC Interface Classes (7 ICs)
//!
//! PRIME NB OFDM PLC interface classes per IEC 62056-6-2.

pub mod ic80_prime_llc_sscs_setup;
pub mod ic81_prime_phy_counters;
pub mod ic82_prime_mac_setup;
pub mod ic83_prime_mac_functional_params;
pub mod ic84_prime_mac_counters;
pub mod ic85_prime_mac_network_admin_data;
pub mod ic86_prime_app_identification;

// Re-exports
pub use ic80_prime_llc_sscs_setup::PrimeLlcSscsSetup;
pub use ic81_prime_phy_counters::PrimePhyCounters;
pub use ic82_prime_mac_setup::PrimeMacSetup;
pub use ic83_prime_mac_functional_params::PrimeMacFunctionalParams;
pub use ic84_prime_mac_counters::PrimeMacCounters;
pub use ic85_prime_mac_network_admin_data::PrimeMacNetworkAdminData;
pub use ic86_prime_app_identification::PrimeAppIdentification;
