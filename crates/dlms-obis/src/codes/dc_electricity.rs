//! DC Electricity OBIS codes (A=2)
//! Reference: Blue Book Part 1

use dlms_core::ObisCode;

/// Total DC active energy import
pub const TOTAL_DC_ACTIVE_ENERGY_IMPORT: ObisCode = ObisCode::new(2, 0, 1, 8, 0, 255);
/// Total DC active energy export
pub const TOTAL_DC_ACTIVE_ENERGY_EXPORT: ObisCode = ObisCode::new(2, 0, 2, 8, 0, 255);

/// DC voltage
pub const DC_VOLTAGE: ObisCode = ObisCode::new(2, 0, 32, 7, 0, 255);
/// DC current
pub const DC_CURRENT: ObisCode = ObisCode::new(2, 0, 31, 7, 0, 255);
/// DC power
pub const DC_POWER: ObisCode = ObisCode::new(2, 0, 1, 7, 0, 255);
