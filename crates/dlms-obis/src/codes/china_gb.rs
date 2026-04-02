//!
//! China National Standard (GB/T 17215.321) OBIS code extensions
//!
//! This module adds OBIS codes defined by the Chinese national standard
//! for electricity meters, which are not part of the DLMS UA Blue Book.
//!
//! Common extensions:
//! - C=128..255: Manufacturer-specific / country-specific groups
//! - D=96: Status/event registers
//! - D=97: Demand registers (Chinese convention)
//! - D=98: Maximum demand
//! - D=99: Load profile

use dlms_core::ObisCode;

// ============================================================
// Group F = 0, C = 128: Meter status and diagnostics (GB extension)
// ============================================================

/// Meter status word (Chinese GB status register)
pub const METER_STATUS_WORD: ObisCode = ObisCode::new(0, 0, 128, 0, 0, 255);

/// Meter operating time (hours)
pub const METER_OPERATING_TIME: ObisCode = ObisCode::new(0, 0, 128, 1, 0, 255);

/// Battery voltage (V * 100)
pub const BATTERY_VOLTAGE: ObisCode = ObisCode::new(0, 0, 128, 2, 0, 255);

/// Clock battery status
pub const CLOCK_BATTERY_STATUS: ObisCode = ObisCode::new(0, 0, 128, 3, 0, 255);

/// Relay/switch status
pub const RELAY_SWITCH_STATUS: ObisCode = ObisCode::new(0, 0, 128, 4, 0, 255);

/// Power failure count
pub const POWER_FAILURE_COUNT: ObisCode = ObisCode::new(0, 0, 128, 5, 0, 255);

/// Power failure duration (minutes)
pub const POWER_FAILURE_DURATION: ObisCode = ObisCode::new(0, 0, 128, 6, 0, 255);

/// Last power failure time
pub const LAST_POWER_FAILURE_TIME: ObisCode = ObisCode::new(0, 0, 128, 7, 0, 255);

/// Program switch status (open/closed/programming)
pub const PROGRAM_SWITCH_STATUS: ObisCode = ObisCode::new(0, 0, 128, 8, 0, 255);

/// Cover open/close status
pub const COVER_STATUS: ObisCode = ObisCode::new(0, 0, 128, 9, 0, 255);

/// Meter type identifier
pub const METER_TYPE_ID: ObisCode = ObisCode::new(0, 0, 128, 10, 0, 255);

// ============================================================
// Group F = 0, C = 129: Security and authentication (GB extension)
// ============================================================

/// Authentication status
pub const AUTH_STATUS: ObisCode = ObisCode::new(0, 0, 129, 0, 0, 255);

/// Last authentication time
pub const LAST_AUTH_TIME: ObisCode = ObisCode::new(0, 0, 129, 1, 0, 255);

/// Failed authentication count
pub const FAILED_AUTH_COUNT: ObisCode = ObisCode::new(0, 0, 129, 2, 0, 255);

// ============================================================
// Group F = 0, C = 130: Billing (GB extension)
// ============================================================

/// Current billing period number
pub const BILLING_PERIOD_NUMBER: ObisCode = ObisCode::new(0, 0, 130, 0, 0, 255);

/// Current billing period start date
pub const BILLING_PERIOD_START: ObisCode = ObisCode::new(0, 0, 130, 1, 0, 255);

/// Billing period end date
pub const BILLING_PERIOD_END: ObisCode = ObisCode::new(0, 0, 130, 2, 0, 255);

/// Total billing periods
pub const TOTAL_BILLING_PERIODS: ObisCode = ObisCode::new(0, 0, 130, 3, 0, 255);

// ============================================================
// Group F = 0, C = 131: Rate / Tariff (GB extension)
// ============================================================

/// Current tariff rate ID
pub const CURRENT_TARIFF_RATE: ObisCode = ObisCode::new(0, 0, 131, 0, 0, 255);

/// Number of tariff rates
pub const NUM_TARIFF_RATES: ObisCode = ObisCode::new(0, 0, 131, 1, 0, 255);

/// Tariff rate table entry
pub const TARIFF_RATE_TABLE: ObisCode = ObisCode::new(0, 0, 131, 2, 0, 255);

/// Current time of use period
pub const CURRENT_TOU_PERIOD: ObisCode = ObisCode::new(0, 0, 131, 3, 0, 255);

// ============================================================
// Group F = 0, C = 132: Communication (GB extension)
// ============================================================

/// Last successful communication time
pub const LAST_COMM_TIME: ObisCode = ObisCode::new(0, 0, 132, 0, 0, 255);

/// Communication count (total sessions)
pub const COMM_SESSION_COUNT: ObisCode = ObisCode::new(0, 0, 132, 1, 0, 255);

/// Communication error count
pub const COMM_ERROR_COUNT: ObisCode = ObisCode::new(0, 0, 132, 2, 0, 255);

/// RS485 address
pub const RS485_ADDRESS: ObisCode = ObisCode::new(0, 0, 132, 3, 0, 255);

// ============================================================
// Group F = 0, C = 133: Event log (GB extension)
// ============================================================

/// Event log clear control
pub const EVENT_LOG_CLEAR: ObisCode = ObisCode::new(0, 0, 133, 0, 0, 255);

/// Event log entry count
pub const EVENT_LOG_COUNT: ObisCode = ObisCode::new(0, 0, 133, 1, 0, 255);

/// Phase loss event log
pub const PHASE_LOSS_EVENT_LOG: ObisCode = ObisCode::new(0, 0, 133, 2, 0, 255);

/// Over voltage event log
pub const OVER_VOLTAGE_EVENT_LOG: ObisCode = ObisCode::new(0, 0, 133, 3, 0, 255);

/// Under voltage event log
pub const UNDER_VOLTAGE_EVENT_LOG: ObisCode = ObisCode::new(0, 0, 133, 4, 0, 255);

/// Over current event log
pub const OVER_CURRENT_EVENT_LOG: ObisCode = ObisCode::new(0, 0, 133, 5, 0, 255);

/// Power failure event log
pub const POWER_FAILURE_EVENT_LOG: ObisCode = ObisCode::new(0, 0, 133, 6, 0, 255);

/// Meter cover open event log
pub const COVER_OPEN_EVENT_LOG: ObisCode = ObisCode::new(0, 0, 133, 7, 0, 255);

/// Programming event log
pub const PROGRAMMING_EVENT_LOG: ObisCode = ObisCode::new(0, 0, 133, 8, 0, 255);

/// Clock adjustment event log
pub const CLOCK_ADJUST_EVENT_LOG: ObisCode = ObisCode::new(0, 0, 133, 9, 0, 255);

/// Tamper event log
pub const TAMPER_EVENT_LOG: ObisCode = ObisCode::new(0, 0, 133, 10, 0, 255);

// ============================================================
// Group F = 0, C = 134: Harmonic measurement (GB extension)
// ============================================================

/// Voltage THD L1 (%)
pub const VOLTAGE_THD_L1: ObisCode = ObisCode::new(0, 0, 134, 1, 0, 255);

/// Voltage THD L2 (%)
pub const VOLTAGE_THD_L2: ObisCode = ObisCode::new(0, 0, 134, 2, 0, 255);

/// Voltage THD L3 (%)
pub const VOLTAGE_THD_L3: ObisCode = ObisCode::new(0, 0, 134, 3, 0, 255);

/// Current THD L1 (%)
pub const CURRENT_THD_L1: ObisCode = ObisCode::new(0, 0, 134, 11, 0, 255);

/// Current THD L2 (%)
pub const CURRENT_THD_L2: ObisCode = ObisCode::new(0, 0, 134, 12, 0, 255);

/// Current THD L3 (%)
pub const CURRENT_THD_L3: ObisCode = ObisCode::new(0, 0, 134, 13, 0, 255);

// ============================================================
// Group F = 0, C = 135: Load profile capture (GB extension)
// ============================================================

/// Load profile capture interval (minutes)
pub const LOAD_PROFILE_INTERVAL: ObisCode = ObisCode::new(0, 0, 135, 0, 0, 255);

/// Load profile entry count
pub const LOAD_PROFILE_ENTRY_COUNT: ObisCode = ObisCode::new(0, 0, 135, 1, 0, 255);

/// Load profile capture objects
pub const LOAD_PROFILE_CAPTURE_OBJECTS: ObisCode = ObisCode::new(0, 0, 135, 2, 0, 255);

// ============================================================
// Group F = 0, C = 136: Energy quality (GB extension)
// ============================================================

/// Voltage sag count L1
pub const VOLTAGE_SAG_COUNT_L1: ObisCode = ObisCode::new(0, 0, 136, 1, 0, 255);

/// Voltage sag count L2
pub const VOLTAGE_SAG_COUNT_L2: ObisCode = ObisCode::new(0, 0, 136, 2, 0, 255);

/// Voltage sag count L3
pub const VOLTAGE_SAG_COUNT_L3: ObisCode = ObisCode::new(0, 0, 136, 3, 0, 255);

/// Voltage swell count L1
pub const VOLTAGE_SWELL_COUNT_L1: ObisCode = ObisCode::new(0, 0, 136, 11, 0, 255);

/// Voltage swell count L2
pub const VOLTAGE_SWELL_COUNT_L2: ObisCode = ObisCode::new(0, 0, 136, 12, 0, 255);

/// Voltage swell count L3
pub const VOLTAGE_SWELL_COUNT_L3: ObisCode = ObisCode::new(0, 0, 136, 13, 0, 255);
