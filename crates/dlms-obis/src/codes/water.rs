//! Water OBIS codes (A=8,9)
//! Reference: Blue Book Part 1

use dlms_core::ObisCode;

/// Total cold water volume — m³
pub const TOTAL_WATER_VOLUME: ObisCode = ObisCode::new(8, 0, 1, 8, 0, 255);
/// Water flow rate — m³/h
pub const WATER_FLOW_RATE: ObisCode = ObisCode::new(8, 0, 1, 7, 0, 255);
/// Water temperature — °C
pub const WATER_TEMPERATURE: ObisCode = ObisCode::new(8, 0, 32, 7, 0, 255);
/// Water pressure — Pa
pub const WATER_PRESSURE: ObisCode = ObisCode::new(8, 0, 33, 7, 0, 255);

/// Hot water (A=9)
pub const TOTAL_HOT_WATER_VOLUME: ObisCode = ObisCode::new(9, 0, 1, 8, 0, 255);
pub const HOT_WATER_FLOW_RATE: ObisCode = ObisCode::new(9, 0, 1, 7, 0, 255);
pub const HOT_WATER_TEMPERATURE: ObisCode = ObisCode::new(9, 0, 32, 7, 0, 255);
