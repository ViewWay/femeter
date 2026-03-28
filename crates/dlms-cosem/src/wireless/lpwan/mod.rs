//!
//! LPWAN Interface Classes (4 ICs)
//!
//! LPWAN (Low-Power Wide Area Network) interface classes per IEC 62056-6-2.

pub mod ic126_schc_lpwan_setup;
pub mod ic127_schc_lpwan_diagnostic;
pub mod ic128_lorawan_setup;
pub mod ic129_lorawan_diagnostic;

// Re-exports
pub use ic126_schc_lpwan_setup::SchcLpwanSetup;
pub use ic127_schc_lpwan_diagnostic::SchcLpwanDiagnostic;
pub use ic128_lorawan_setup::LorawanSetup;
pub use ic129_lorawan_diagnostic::LorawanDiagnostic;
