//! Thermal energy OBIS codes (A=5,6)
//! Reference: Blue Book Part 1

use dlms_core::ObisCode;

/// Total thermal energy — J
pub const TOTAL_THERMAL_ENERGY: ObisCode = ObisCode::new(5, 0, 1, 8, 0, 255);
/// Thermal power — W
pub const THERMAL_POWER: ObisCode = ObisCode::new(5, 0, 1, 7, 0, 255);
/// Thermal inlet temperature — °C
pub const THERMAL_INLET_TEMPERATURE: ObisCode = ObisCode::new(5, 0, 32, 7, 0, 255);
/// Thermal outlet temperature — °C
pub const THERMAL_OUTLET_TEMPERATURE: ObisCode = ObisCode::new(5, 0, 52, 7, 0, 255);

/// Heat cost allocator (A=6)
pub const HCA_CURRENT_CONSUMPTION: ObisCode = ObisCode::new(6, 0, 1, 8, 0, 255);
