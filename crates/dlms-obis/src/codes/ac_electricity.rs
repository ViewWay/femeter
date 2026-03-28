//! AC Electricity OBIS codes (A=1)
//! Reference: Blue Book Part 1 §2
//! This is the largest group with hundreds of standard codes.

use dlms_core::ObisCode;

// ============================================================
// Energy (C=1, D=8) — Cumulative energy registers
// ============================================================

/// Total active energy import (forward) — kWh
pub const TOTAL_ACTIVE_ENERGY_IMPORT: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 255);
/// Total active energy export (reverse) — kWh
pub const TOTAL_ACTIVE_ENERGY_EXPORT: ObisCode = ObisCode::new(1, 0, 2, 8, 0, 255);
/// Total reactive energy import — kvarh
pub const TOTAL_REACTIVE_ENERGY_IMPORT: ObisCode = ObisCode::new(1, 0, 3, 8, 0, 255);
/// Total reactive energy export — kvarh
pub const TOTAL_REACTIVE_ENERGY_EXPORT: ObisCode = ObisCode::new(1, 0, 4, 8, 0, 255);

/// Active energy import by tariff 1
pub const ACTIVE_ENERGY_IMPORT_TARIFF1: ObisCode = ObisCode::new(1, 0, 1, 8, 1, 255);
/// Active energy import by tariff 2
pub const ACTIVE_ENERGY_IMPORT_TARIFF2: ObisCode = ObisCode::new(1, 0, 1, 8, 2, 255);
/// Active energy import by tariff 3
pub const ACTIVE_ENERGY_IMPORT_TARIFF3: ObisCode = ObisCode::new(1, 0, 1, 8, 3, 255);
/// Active energy import by tariff 4
pub const ACTIVE_ENERGY_IMPORT_TARIFF4: ObisCode = ObisCode::new(1, 0, 1, 8, 4, 255);

/// Active energy export by tariff 1
pub const ACTIVE_ENERGY_EXPORT_TARIFF1: ObisCode = ObisCode::new(1, 0, 2, 8, 1, 255);
/// Active energy export by tariff 2
pub const ACTIVE_ENERGY_EXPORT_TARIFF2: ObisCode = ObisCode::new(1, 0, 2, 8, 2, 255);
/// Active energy export by tariff 3
pub const ACTIVE_ENERGY_EXPORT_TARIFF3: ObisCode = ObisCode::new(1, 0, 2, 8, 3, 255);
/// Active energy export by tariff 4
pub const ACTIVE_ENERGY_EXPORT_TARIFF4: ObisCode = ObisCode::new(1, 0, 2, 8, 4, 255);

/// Reactive energy import by tariff 1
pub const REACTIVE_ENERGY_IMPORT_TARIFF1: ObisCode = ObisCode::new(1, 0, 3, 8, 1, 255);
/// Reactive energy import by tariff 2
pub const REACTIVE_ENERGY_IMPORT_TARIFF2: ObisCode = ObisCode::new(1, 0, 3, 8, 2, 255);
/// Reactive energy import by tariff 3
pub const REACTIVE_ENERGY_IMPORT_TARIFF3: ObisCode = ObisCode::new(1, 0, 3, 8, 3, 255);
/// Reactive energy import by tariff 4
pub const REACTIVE_ENERGY_IMPORT_TARIFF4: ObisCode = ObisCode::new(1, 0, 3, 8, 4, 255);

/// Apparent energy import — kVAh
pub const APPARENT_ENERGY_IMPORT: ObisCode = ObisCode::new(1, 0, 5, 8, 0, 255);
/// Apparent energy export — kVAh
pub const APPARENT_ENERGY_EXPORT: ObisCode = ObisCode::new(1, 0, 6, 8, 0, 255);

// ============================================================
// Power (C=1, D=7) — Instantaneous power
// ============================================================

/// Total active power import — W
pub const TOTAL_ACTIVE_POWER_IMPORT: ObisCode = ObisCode::new(1, 0, 1, 7, 0, 255);
/// Total active power export — W
pub const TOTAL_ACTIVE_POWER_EXPORT: ObisCode = ObisCode::new(1, 0, 2, 7, 0, 255);
/// Total reactive power import — var
pub const TOTAL_REACTIVE_POWER_IMPORT: ObisCode = ObisCode::new(1, 0, 3, 7, 0, 255);
/// Total reactive power export — var
pub const TOTAL_REACTIVE_POWER_EXPORT: ObisCode = ObisCode::new(1, 0, 4, 7, 0, 255);

/// Active power L1 — W
pub const ACTIVE_POWER_L1: ObisCode = ObisCode::new(1, 0, 15, 7, 0, 255);
/// Active power L2 — W
pub const ACTIVE_POWER_L2: ObisCode = ObisCode::new(1, 0, 31, 7, 0, 255);
/// Active power L3 — W
pub const ACTIVE_POWER_L3: ObisCode = ObisCode::new(1, 0, 47, 7, 0, 255);

/// Reactive power L1 — var
pub const REACTIVE_POWER_L1: ObisCode = ObisCode::new(1, 0, 16, 7, 0, 255);
/// Reactive power L2 — var
pub const REACTIVE_POWER_L2: ObisCode = ObisCode::new(1, 0, 32, 7, 0, 255);
/// Reactive power L3 — var
pub const REACTIVE_POWER_L3: ObisCode = ObisCode::new(1, 0, 48, 7, 0, 255);

// ============================================================
// Voltage (C=32)
// ============================================================

/// Voltage L1 — V
pub const VOLTAGE_L1: ObisCode = ObisCode::new(1, 0, 32, 7, 0, 255);
/// Voltage L2 — V
pub const VOLTAGE_L2: ObisCode = ObisCode::new(1, 0, 52, 7, 0, 255);
/// Voltage L3 — V
pub const VOLTAGE_L3: ObisCode = ObisCode::new(1, 0, 72, 7, 0, 255);

/// Average voltage — V
pub const AVERAGE_VOLTAGE: ObisCode = ObisCode::new(1, 0, 12, 7, 0, 255);

// ============================================================
// Current (C=31)
// ============================================================

/// Current L1 — A
pub const CURRENT_L1: ObisCode = ObisCode::new(1, 0, 31, 7, 0, 255);
/// Current L2 — A
pub const CURRENT_L2: ObisCode = ObisCode::new(1, 0, 51, 7, 0, 255);
/// Current L3 — A
pub const CURRENT_L3: ObisCode = ObisCode::new(1, 0, 71, 7, 0, 255);

/// Average current — A
pub const AVERAGE_CURRENT: ObisCode = ObisCode::new(1, 0, 11, 7, 0, 255);

/// Neutral current — A
pub const NEUTRAL_CURRENT: ObisCode = ObisCode::new(1, 0, 91, 7, 0, 255);

// ============================================================
// Power factor (C=13)
// ============================================================

/// Total power factor
pub const TOTAL_POWER_FACTOR: ObisCode = ObisCode::new(1, 0, 13, 7, 0, 255);
/// Power factor L1
pub const POWER_FACTOR_L1: ObisCode = ObisCode::new(1, 0, 14, 7, 0, 255);
/// Power factor L2
pub const POWER_FACTOR_L2: ObisCode = ObisCode::new(1, 0, 33, 7, 0, 255);
/// Power factor L3
pub const POWER_FACTOR_L3: ObisCode = ObisCode::new(1, 0, 53, 7, 0, 255);

// ============================================================
// Frequency (C=14)
// ============================================================

/// Frequency — Hz
pub const FREQUENCY: ObisCode = ObisCode::new(1, 0, 14, 7, 0, 255);
/// Frequency L1
pub const FREQUENCY_L1: ObisCode = ObisCode::new(1, 0, 14, 4, 1, 255);

// ============================================================
// Phase angle (C=12)
// ============================================================

/// Phase angle L1 — deg
pub const PHASE_ANGLE_L1: ObisCode = ObisCode::new(1, 0, 12, 7, 0, 255);
/// Phase angle L2 — deg
pub const PHASE_ANGLE_L2: ObisCode = ObisCode::new(1, 0, 32, 7, 0, 255);
/// Phase angle L3 — deg
pub const PHASE_ANGLE_L3: ObisCode = ObisCode::new(1, 0, 52, 7, 0, 255);

/// Phase angle between L1 and L2
pub const PHASE_ANGLE_L1L2: ObisCode = ObisCode::new(1, 0, 22, 7, 0, 255);
/// Phase angle between L2 and L3
pub const PHASE_ANGLE_L2L3: ObisCode = ObisCode::new(1, 0, 42, 7, 0, 255);
/// Phase angle between L3 and L1
pub const PHASE_ANGLE_L3L1: ObisCode = ObisCode::new(1, 0, 62, 7, 0, 255);

// ============================================================
// Demand (C=1, D=6)
// ============================================================

/// Current demand — A
pub const CURRENT_DEMAND: ObisCode = ObisCode::new(1, 0, 1, 6, 0, 255);
/// Active power demand — W
pub const ACTIVE_POWER_DEMAND: ObisCode = ObisCode::new(1, 0, 15, 6, 0, 255);

// ============================================================
// THD (Total Harmonic Distortion)
// ============================================================

/// THD voltage L1 — %
pub const THD_VOLTAGE_L1: ObisCode = ObisCode::new(1, 0, 32, 5, 0, 255);
/// THD voltage L2 — %
pub const THD_VOLTAGE_L2: ObisCode = ObisCode::new(1, 0, 52, 5, 0, 255);
/// THD voltage L3 — %
pub const THD_VOLTAGE_L3: ObisCode = ObisCode::new(1, 0, 72, 5, 0, 255);
/// THD current L1 — %
pub const THD_CURRENT_L1: ObisCode = ObisCode::new(1, 0, 31, 5, 0, 255);
/// THD current L2 — %
pub const THD_CURRENT_L2: ObisCode = ObisCode::new(1, 0, 51, 5, 0, 255);
/// THD current L3 — %
pub const THD_CURRENT_L3: ObisCode = ObisCode::new(1, 0, 71, 5, 0, 255);

// ============================================================
// Load profile (C=1, D=7)
// ============================================================

/// Instantaneous active power L1
pub const INST_ACTIVE_POWER_L1: ObisCode = ObisCode::new(1, 0, 21, 7, 0, 255);
/// Instantaneous active power L2
pub const INST_ACTIVE_POWER_L2: ObisCode = ObisCode::new(1, 0, 41, 7, 0, 255);
/// Instantaneous active power L3
pub const INST_ACTIVE_POWER_L3: ObisCode = ObisCode::new(1, 0, 61, 7, 0, 255);

// ============================================================
// Apparent power
// ============================================================

/// Apparent power L1 — VA
pub const APPARENT_POWER_L1: ObisCode = ObisCode::new(1, 0, 17, 7, 0, 255);
/// Apparent power L2 — VA
pub const APPARENT_POWER_L2: ObisCode = ObisCode::new(1, 0, 37, 7, 0, 255);
/// Apparent power L3 — VA
pub const APPARENT_POWER_L3: ObisCode = ObisCode::new(1, 0, 57, 7, 0, 255);

// ============================================================
// Temperature
// ============================================================

/// Internal temperature — °C
pub const INTERNAL_TEMPERATURE: ObisCode = ObisCode::new(0, 0, 96, 7, 0, 255);
/// External temperature 1 — °C
pub const EXTERNAL_TEMPERATURE1: ObisCode = ObisCode::new(0, 0, 96, 7, 1, 255);

// ============================================================
// Events / Status
// ============================================================

/// Event log 1
pub const EVENT_LOG1: ObisCode = ObisCode::new(0, 0, 96, 8, 0, 255);
/// Event log 2
pub const EVENT_LOG2: ObisCode = ObisCode::new(0, 0, 96, 8, 1, 255);

// ============================================================
// Firmware
// ============================================================

/// Firmware version
pub const FIRMWARE_VERSION: ObisCode = ObisCode::new(1, 0, 0, 2, 0, 255);
/// Firmware status
pub const FIRMWARE_STATUS: ObisCode = ObisCode::new(1, 0, 0, 2, 1, 255);

// ============================================================
// Supply frequency
// ============================================================

/// Supply frequency
pub const SUPPLY_FREQUENCY: ObisCode = ObisCode::new(1, 0, 14, 7, 0, 255);

// ============================================================
// Meter status / control
// ============================================================

/// Relay status (open/closed)
pub const RELAY_STATUS: ObisCode = ObisCode::new(0, 0, 96, 2, 0, 255);
/// Meter running status
pub const METER_RUNNING_STATUS: ObisCode = ObisCode::new(0, 0, 96, 5, 0, 255);

// ============================================================
// Billing period data
// ============================================================

/// Active energy import, billing period 1
pub const BILLING_ACTIVE_IMPORT_P1: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 1);
/// Active energy import, billing period 2
pub const BILLING_ACTIVE_IMPORT_P2: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 2);
/// Active energy import, billing period 3
pub const BILLING_ACTIVE_IMPORT_P3: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 3);
/// Active energy import, billing period 4
pub const BILLING_ACTIVE_IMPORT_P4: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 4);
/// Active energy import, billing period 5
pub const BILLING_ACTIVE_IMPORT_P5: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 5);
/// Active energy import, billing period 6..12
pub const BILLING_ACTIVE_IMPORT_P6: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 6);
pub const BILLING_ACTIVE_IMPORT_P7: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 7);
pub const BILLING_ACTIVE_IMPORT_P8: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 8);
pub const BILLING_ACTIVE_IMPORT_P9: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 9);
pub const BILLING_ACTIVE_IMPORT_P10: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 10);
pub const BILLING_ACTIVE_IMPORT_P11: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 11);
pub const BILLING_ACTIVE_IMPORT_P12: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 12);

// ============================================================
// Maximum demand
// ============================================================

/// Maximum current demand — A
pub const MAX_CURRENT_DEMAND: ObisCode = ObisCode::new(1, 0, 1, 6, 0, 255);
/// Maximum active power demand — W
pub const MAX_ACTIVE_POWER_DEMAND: ObisCode = ObisCode::new(1, 0, 15, 6, 0, 255);
/// Maximum apparent power demand — VA
pub const MAX_APPARENT_POWER_DEMAND: ObisCode = ObisCode::new(1, 0, 17, 6, 0, 255);
