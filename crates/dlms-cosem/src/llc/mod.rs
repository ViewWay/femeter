//!
//! Group 8c: LLC (Logical Link Control) Interface Classes (3 ICs)
//!
//! This module contains interface classes for LLC per ISO/IEC 8802-2:
//! - IC 57: ISO/IEC 8802-2 LLC Type 1 Setup
//! - IC 58: ISO/IEC 8802-2 LLC Type 2 Setup
//! - IC 59: ISO/IEC 8802-2 LLC Type 3 Setup

pub mod ic57_llc_type1_setup;
pub mod ic58_llc_type2_setup;
pub mod ic59_llc_type3_setup;

// Re-export commonly used types
pub use ic57_llc_type1_setup::LlcType1Setup;
pub use ic58_llc_type2_setup::LlcType2Setup;
pub use ic59_llc_type3_setup::LlcType3Setup;
