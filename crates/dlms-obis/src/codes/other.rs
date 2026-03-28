//! Other OBIS codes (A=10-14)
//! Reference: Blue Book Part 1

use dlms_core::ObisCode;

// A=10: Heat meter
pub const HEAT_ENERGY: ObisCode = ObisCode::new(10, 0, 1, 8, 0, 255);

// A=11: Heat/Cooling load
pub const COOLING_ENERGY: ObisCode = ObisCode::new(11, 0, 1, 8, 0, 255);

// A=12-14: Reserved / site-specific
