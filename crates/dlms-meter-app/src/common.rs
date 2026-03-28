//! Common shared types for the meter application
//!
//! This module provides shared data structures used across
//! multiple meter application modules.

extern crate alloc;

use dlms_core::obis::ObisCode;

/// Tariff schedule for time-of-use billing
#[derive(Debug, Clone, PartialEq)]
pub struct TariffSchedule {
    /// Tariff ID (1-8 for multi-tariff meters)
    pub tariff_id: u8,
    /// Hour of day when this tariff starts (0-23)
    pub start_hour: u8,
    /// Minute when this tariff starts (0-59)
    pub start_minute: u8,
    /// Day of week mask (bit 0=Monday, bit 6=Sunday, 0xFF=all days)
    pub day_mask: u8,
    /// Season mask (bit 0=Spring, bit 3=Winter, 0xFF=all seasons)
    pub season_mask: u8,
}

impl TariffSchedule {
    /// Create a new tariff schedule
    pub const fn new(tariff_id: u8, start_hour: u8, start_minute: u8) -> Self {
        Self {
            tariff_id,
            start_hour,
            start_minute,
            day_mask: 0xFF,
            season_mask: 0xFF,
        }
    }

    /// Check if this schedule applies to a given time
    pub fn applies_at(&self, hour: u8, minute: u8) -> bool {
        let schedule_mins = self.start_hour as u16 * 60 + self.start_minute as u16;
        let current_mins = hour as u16 * 60 + minute as u16;
        current_mins >= schedule_mins
    }
}

/// Demand calculation configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DemandConfig {
    /// Demand integration period in seconds (typically 15, 30, or 60 minutes)
    pub integration_period_s: u32,
    /// Sliding window block size in seconds
    pub block_size_s: u32,
    /// Maximum number of demand values to store
    pub max_values: usize,
}

impl DemandConfig {
    /// Create default demand configuration (15-minute integration)
    pub const fn default_15min() -> Self {
        Self {
            integration_period_s: 900,
            block_size_s: 60,
            max_values: 96, // 24 hours of 15-min data
        }
    }

    /// Create 30-minute demand configuration
    pub const fn default_30min() -> Self {
        Self {
            integration_period_s: 1800,
            block_size_s: 60,
            max_values: 48, // 24 hours of 30-min data
        }
    }
}

/// Alarm threshold configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmThreshold {
    /// OBIS code of the monitored attribute
    pub obis_code: ObisCode,
    /// Threshold value
    pub threshold: i64,
    /// Hysteresis value (deadband)
    pub hysteresis: i64,
    /// Alarm type
    pub alarm_type: AlarmType,
}

/// Alarm classification type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AlarmType {
    /// High threshold exceeded
    High = 0,
    /// Low threshold exceeded
    Low = 1,
    /// Rate of change exceeded
    RateOfChange = 2,
    /// Value changed
    ValueChange = 3,
}

impl AlarmType {
    /// Create from u8
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::High),
            1 => Some(Self::Low),
            2 => Some(Self::RateOfChange),
            3 => Some(Self::ValueChange),
            _ => None,
        }
    }

    /// Get the numeric code
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Display entry for meter LCD/screen
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayEntry {
    /// Display sequence number
    pub sequence: u8,
    /// OBIS code to display
    pub obis_code: ObisCode,
    /// Display format template
    pub format: DisplayFormat,
    /// Scroll rate in seconds (0 = manual scroll)
    pub scroll_rate_s: u8,
}

/// Display format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DisplayFormat {
    /// Numeric with 7 decimals
    Numeric7 = 0,
    /// Numeric with 6 decimals
    Numeric6 = 1,
    /// Numeric with 5 decimals
    Numeric5 = 2,
    /// Numeric with 4 decimals
    Numeric4 = 3,
    /// Numeric with 3 decimals
    Numeric3 = 4,
    /// Numeric with 2 decimals
    Numeric2 = 5,
    /// Numeric with 1 decimal
    Numeric1 = 6,
    /// Integer only
    Integer = 7,
    /// Text string
    Text = 8,
}

impl DisplayFormat {
    /// Get decimal places for numeric formats
    pub fn decimal_places(self) -> u8 {
        match self {
            Self::Numeric7 => 7,
            Self::Numeric6 => 6,
            Self::Numeric5 => 5,
            Self::Numeric4 => 4,
            Self::Numeric3 => 3,
            Self::Numeric2 => 2,
            Self::Numeric1 => 1,
            Self::Integer | Self::Text => 0,
        }
    }
}

/// Billing period configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BillingPeriod {
    /// Period identifier (1-12 for monthly, 1-4 for quarterly)
    pub period_id: u8,
    /// Tariff to apply for this period
    pub tariff_id: u8,
    /// Billing status
    pub status: BillingStatus,
    // Note: CosemDateTime fields removed to make this Copy
    // In real implementation, dates would be handled separately
}

/// Billing period status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BillingStatus {
    /// Period not yet started
    NotActive = 0,
    /// Period currently active
    Active = 1,
    /// Period completed
    Completed = 2,
    /// Period suspended
    Suspended = 3,
}

impl BillingStatus {
    /// Create from u8
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::NotActive),
            1 => Some(Self::Active),
            2 => Some(Self::Completed),
            3 => Some(Self::Suspended),
            _ => None,
        }
    }

    /// Get the numeric code
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Energy accumulator for a single phase
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseEnergy {
    /// Active energy import (Wh)
    pub active_import: i64,
    /// Active energy export (Wh)
    pub active_export: i64,
    /// Reactive energy import (varh)
    pub reactive_import: i64,
    /// Reactive energy export (varh)
    pub reactive_export: i64,
}

impl PhaseEnergy {
    /// Create zero energy accumulator
    pub const fn zero() -> Self {
        Self {
            active_import: 0,
            active_export: 0,
            reactive_import: 0,
            reactive_export: 0,
        }
    }

    /// Add another energy value
    pub fn add(&mut self, other: &PhaseEnergy) {
        self.active_import = self.active_import.saturating_add(other.active_import);
        self.active_export = self.active_export.saturating_add(other.active_export);
        self.reactive_import = self.reactive_import.saturating_add(other.reactive_import);
        self.reactive_export = self.reactive_export.saturating_add(other.reactive_export);
    }
}

impl Default for PhaseEnergy {
    fn default() -> Self {
        Self::zero()
    }
}

/// Power quality metrics
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerQuality {
    /// Voltage THD percentage (x100)
    pub voltage_thd: u16,
    /// Current THD percentage (x100)
    pub current_thd: u16,
    /// Power factor (x1000, range -1000 to 1000)
    pub power_factor: i16,
    /// Frequency in mHz (milliHertz, e.g., 50000 = 50.000 Hz)
    pub frequency_mhz: u32,
}

impl PowerQuality {
    /// Create default power quality values
    pub const fn default() -> Self {
        Self {
            voltage_thd: 0,
            current_thd: 0,
            power_factor: 1000, // Unity power factor
            frequency_mhz: 50000,
        }
    }
}

impl Default for PowerQuality {
    fn default() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tariff_schedule() {
        let schedule = TariffSchedule::new(1, 8, 0);
        assert_eq!(schedule.tariff_id, 1);
        assert!(schedule.applies_at(9, 0));
        assert!(!schedule.applies_at(7, 0));
    }

    #[test]
    fn test_demand_config() {
        let config = DemandConfig::default_15min();
        assert_eq!(config.integration_period_s, 900);
        assert_eq!(config.max_values, 96);
    }

    #[test]
    fn test_alarm_type() {
        assert_eq!(AlarmType::from_code(0), Some(AlarmType::High));
        assert_eq!(AlarmType::from_code(1), Some(AlarmType::Low));
        assert_eq!(AlarmType::from_code(99), None);
        assert_eq!(AlarmType::High.code(), 0);
    }

    #[test]
    fn test_billing_status() {
        assert_eq!(BillingStatus::from_code(1), Some(BillingStatus::Active));
        assert_eq!(BillingStatus::Active.code(), 1);
    }

    #[test]
    fn test_display_format() {
        assert_eq!(DisplayFormat::Numeric3.decimal_places(), 3);
        assert_eq!(DisplayFormat::Integer.decimal_places(), 0);
    }

    #[test]
    fn test_phase_energy() {
        let mut total = PhaseEnergy::zero();
        let add = PhaseEnergy {
            active_import: 100,
            active_export: 50,
            reactive_import: 25,
            reactive_export: 10,
        };
        total.add(&add);
        assert_eq!(total.active_import, 100);
        assert_eq!(total.active_export, 50);
    }

    #[test]
    fn test_power_quality() {
        let pq = PowerQuality::default();
        assert_eq!(pq.power_factor, 1000);
        assert_eq!(pq.frequency_mhz, 50000);
    }
}
