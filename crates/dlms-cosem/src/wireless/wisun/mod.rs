//!
//! Wi-SUN Interface Classes (4 ICs)
//!
//! Wi-SUN interface classes per IEC 62056-6-2.

pub mod ic95_wisun_setup;
pub mod ic96_wisun_diagnostic;
pub mod ic97_rpl_diagnostic;
pub mod ic98_mpl_diagnostic;

// Re-exports
pub use ic95_wisun_setup::WisunSetup;
pub use ic96_wisun_diagnostic::WisunDiagnostic;
pub use ic97_rpl_diagnostic::RplDiagnostic;
pub use ic98_mpl_diagnostic::MplDiagnostic;
