//! Gas OBIS codes (A=7)
//! Reference: Blue Book Part 1

use dlms_core::ObisCode;

/// Total gas volume — m³
pub const TOTAL_GAS_VOLUME: ObisCode = ObisCode::new(7, 0, 1, 8, 0, 255);
/// Gas flow rate — m³/h
pub const GAS_FLOW_RATE: ObisCode = ObisCode::new(7, 0, 1, 7, 0, 255);
/// Gas temperature — °C
pub const GAS_TEMPERATURE: ObisCode = ObisCode::new(7, 0, 32, 7, 0, 255);
/// Gas pressure — Pa
pub const GAS_PRESSURE: ObisCode = ObisCode::new(7, 0, 33, 7, 0, 255);
/// Gas correction factor
pub const GAS_CORRECTION_FACTOR: ObisCode = ObisCode::new(7, 0, 2, 7, 0, 255);
