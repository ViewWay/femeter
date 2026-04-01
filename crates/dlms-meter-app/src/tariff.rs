//! Tariff manager for time-of-use billing and schedule management
//!
//! This module provides:
//! - Time-of-use tariff scheduling
//! - Calendar-based tariff selection
//! - Seasonal tariff handling
//! - Billing period management

extern crate alloc;

use alloc::vec::Vec;
use dlms_core::{errors::CosemError, types::CosemDateTime};

use crate::common::{BillingPeriod, BillingStatus, TariffSchedule};

/// Maximum number of tariff schedules
const MAX_SCHEDULES: usize = 32;

/// Tariff manager for handling time-of-use billing
#[derive(Debug)]
pub struct TariffManager {
    /// Active tariff schedules
    schedules: Vec<TariffSchedule>,
    /// Current active tariff
    current_tariff: u8,
    /// Billing periods
    billing_periods: Vec<BillingPeriod>,
    /// Active billing period index
    active_period: Option<usize>,
}

impl TariffManager {
    /// Create a new tariff manager
    pub fn new() -> Self {
        Self {
            schedules: Vec::with_capacity(8),
            current_tariff: 1, // Default to tariff 1
            billing_periods: Vec::with_capacity(12),
            active_period: None,
        }
    }

    /// Add a tariff schedule
    pub fn add_schedule(&mut self, schedule: TariffSchedule) -> Result<(), CosemError> {
        if self.schedules.len() >= MAX_SCHEDULES {
            return Err(CosemError::NotImplemented);
        }
        if schedule.tariff_id == 0 || schedule.tariff_id > 8 {
            return Err(CosemError::InvalidParameter);
        }
        self.schedules.push(schedule);
        Ok(())
    }

    /// Remove all schedules
    pub fn clear_schedules(&mut self) {
        self.schedules.clear();
    }

    /// Get current tariff based on time
    pub fn current_tariff(&self) -> u8 {
        self.current_tariff
    }

    /// Update current tariff based on date-time
    pub fn update_tariff_for_time(&mut self, datetime: &CosemDateTime) -> u8 {
        let hour = datetime.time.hour;
        let minute = datetime.time.minute;
        let day_of_week = (datetime.date.day_of_week.wrapping_sub(1)) % 7;

        // Find matching schedule - collect IDs first to avoid borrow issues
        let mut matched_tariff = None;

        for schedule in &self.schedules {
            // Check day mask
            if schedule.day_mask & (1 << day_of_week) == 0 {
                continue;
            }

            // Check if time matches
            if schedule.applies_at(hour, minute) {
                matched_tariff = Some(schedule.tariff_id);
                break;
            }
        }

        // Update tariff if match found
        if let Some(tariff_id) = matched_tariff {
            self.current_tariff = tariff_id;
            tariff_id
        } else {
            self.current_tariff
        }
    }

    /// Set current tariff manually
    pub fn set_tariff(&mut self, tariff: u8) -> Result<(), CosemError> {
        if tariff == 0 || tariff > 8 {
            return Err(CosemError::InvalidParameter);
        }
        self.current_tariff = tariff;
        Ok(())
    }

    /// Get all schedules
    pub fn schedules(&self) -> &[TariffSchedule] {
        &self.schedules
    }

    /// Add a billing period
    pub fn add_billing_period(&mut self, period: BillingPeriod) -> Result<(), CosemError> {
        if self.billing_periods.len() >= 12 {
            return Err(CosemError::NotImplemented);
        }
        self.billing_periods.push(period);
        Ok(())
    }

    /// Clear all billing periods
    pub fn clear_billing_periods(&mut self) {
        self.billing_periods.clear();
        self.active_period = None;
    }

    /// Get all billing periods
    pub fn billing_periods(&self) -> &[BillingPeriod] {
        &self.billing_periods
    }

    /// Get active billing period
    pub fn active_period(&self) -> Option<&BillingPeriod> {
        self.active_period
            .and_then(|idx| self.billing_periods.get(idx))
    }

    /// Update active billing period based on current time
    pub fn update_billing_period(&mut self, _datetime: &CosemDateTime) {
        // Simplified implementation - in real version would check datetime ranges
        // For now, just update the active period status
        if let Some(idx) = self.active_period {
            if let Some(period) = self.billing_periods.get_mut(idx) {
                period.status = BillingStatus::Active;
            }
        }
    }

    /// Close current billing period
    pub fn close_billing_period(&mut self) -> Result<(), CosemError> {
        if let Some(idx) = self.active_period {
            if let Some(period) = self.billing_periods.get_mut(idx) {
                period.status = BillingStatus::Completed;
                self.active_period = None;
                return Ok(());
            }
        }
        Err(CosemError::NotImplemented)
    }

    /// Check if datetime falls within billing period
    #[allow(dead_code)]
    fn is_datetime_in_period(&self, _dt: &CosemDateTime, _period: &BillingPeriod) -> bool {
        // Simplified check - in real implementation would compare date ranges
        // For now, just check if period is marked active
        true
    }

    /// Get tariff ID for a specific billing period
    pub fn period_tariff(&self, period_id: u8) -> Option<u8> {
        self.billing_periods
            .iter()
            .find(|p| p.period_id == period_id)
            .map(|p| p.tariff_id)
    }
}

impl Default for TariffManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Day of week enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DayOfWeek {
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
    Sunday = 7,
}

impl DayOfWeek {
    /// Create from u8 (1-7, where 1=Monday)
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Monday),
            2 => Some(Self::Tuesday),
            3 => Some(Self::Wednesday),
            4 => Some(Self::Thursday),
            5 => Some(Self::Friday),
            6 => Some(Self::Saturday),
            7 => Some(Self::Sunday),
            _ => None,
        }
    }

    /// Get bit mask for this day (for day_mask field)
    pub fn bit_mask(self) -> u8 {
        1 << (self as u8 - 1)
    }

    /// Create day mask from multiple days
    pub fn mask_from_days(days: &[DayOfWeek]) -> u8 {
        days.iter().fold(0u8, |mask, day| mask | day.bit_mask())
    }

    /// Get all weekdays mask (Mon-Fri)
    pub fn weekday_mask() -> u8 {
        0b00011111
    }

    /// Get all weekend mask (Sat-Sun)
    pub fn weekend_mask() -> u8 {
        0b11000000
    }

    /// Get all days mask
    pub fn all_days_mask() -> u8 {
        0xFF
    }
}

/// Season enumeration for seasonal tariffs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Season {
    Spring = 0,
    Summer = 1,
    Autumn = 2,
    Winter = 3,
}

impl Season {
    /// Get bit mask for this season
    pub fn bit_mask(self) -> u8 {
        1 << (self as u8)
    }

    /// Create season mask from multiple seasons
    pub fn mask_from_seasons(seasons: &[Season]) -> u8 {
        seasons
            .iter()
            .fold(0u8, |mask, season| mask | season.bit_mask())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlms_core::types::{CosemDate, CosemTime};

    fn make_test_datetime(hour: u8, minute: u8) -> CosemDateTime {
        CosemDateTime {
            date: CosemDate {
                year: 2024,
                month: 1,
                day: 15,
                day_of_week: 1, // Monday
            },
            time: CosemTime {
                hour,
                minute,
                second: 0,
                hundredths: 0,
            },
            deviation: 0,
            clock_status: 0,
        }
    }

    #[test]
    fn test_tariff_manager_new() {
        let manager = TariffManager::new();
        assert_eq!(manager.current_tariff(), 1);
        assert!(manager.schedules().is_empty());
    }

    #[test]
    fn test_add_schedule() {
        let mut manager = TariffManager::new();
        let schedule = TariffSchedule::new(2, 8, 0);

        assert!(manager.add_schedule(schedule).is_ok());
        assert_eq!(manager.schedules().len(), 1);
    }

    #[test]
    fn test_add_invalid_tariff() {
        let mut manager = TariffManager::new();
        let schedule = TariffSchedule::new(9, 8, 0); // Invalid tariff ID

        assert!(manager.add_schedule(schedule).is_err());
    }

    #[test]
    fn test_update_tariff_for_time() {
        let mut manager = TariffManager::new();

        // Add schedule: tariff 2 starts at 08:00
        let mut schedule = TariffSchedule::new(2, 8, 0);
        schedule.day_mask = DayOfWeek::all_days_mask();
        manager.add_schedule(schedule).unwrap();

        // Before 08:00 should stay on tariff 1 (no schedule match)
        let dt = make_test_datetime(7, 0);
        manager.update_tariff_for_time(&dt);
        assert_eq!(manager.current_tariff(), 1); // No match yet

        // At 08:00 should be tariff 2 (schedule matches)
        let dt = make_test_datetime(8, 0);
        let tariff = manager.update_tariff_for_time(&dt);
        assert_eq!(tariff, 2);
    }

    #[test]
    fn test_set_tariff() {
        let mut manager = TariffManager::new();

        assert!(manager.set_tariff(3).is_ok());
        assert_eq!(manager.current_tariff(), 3);

        assert!(manager.set_tariff(9).is_err()); // Invalid
    }

    #[test]
    fn test_billing_period() {
        let mut manager = TariffManager::new();

        let period = BillingPeriod {
            period_id: 1,
            tariff_id: 1,
            status: BillingStatus::Active,
        };

        assert!(manager.add_billing_period(period).is_ok());
        assert_eq!(manager.billing_periods().len(), 1);
    }

    #[test]
    fn test_close_billing_period() {
        let mut manager = TariffManager::new();

        let period = BillingPeriod {
            period_id: 1,
            tariff_id: 1,
            status: BillingStatus::Active,
        };
        manager.add_billing_period(period).unwrap();
        manager.active_period = Some(0);

        assert!(manager.close_billing_period().is_ok());
        assert_eq!(
            manager.billing_periods()[0].status,
            BillingStatus::Completed
        );
    }

    #[test]
    fn test_day_of_week() {
        assert_eq!(DayOfWeek::from_u8(1), Some(DayOfWeek::Monday));
        assert_eq!(DayOfWeek::from_u8(7), Some(DayOfWeek::Sunday));
        assert_eq!(DayOfWeek::from_u8(0), None);
        assert_eq!(DayOfWeek::Monday.bit_mask(), 0x01);
        assert_eq!(DayOfWeek::Sunday.bit_mask(), 0x40);
    }

    #[test]
    fn test_day_masks() {
        assert_eq!(DayOfWeek::weekday_mask(), 0b00011111);
        assert_eq!(DayOfWeek::weekend_mask(), 0b11000000);
        assert_eq!(DayOfWeek::all_days_mask(), 0xFF);
    }

    #[test]
    fn test_day_mask_from_days() {
        let days = &[DayOfWeek::Monday, DayOfWeek::Wednesday, DayOfWeek::Friday];
        let mask = DayOfWeek::mask_from_days(days);
        assert_eq!(mask, 0b0010101); // bits 0, 2, 4 set
    }

    #[test]
    fn test_season() {
        assert_eq!(Season::Spring.bit_mask(), 0x01);
        assert_eq!(Season::Winter.bit_mask(), 0x08);

        let seasons = &[Season::Spring, Season::Summer];
        let mask = Season::mask_from_seasons(seasons);
        assert_eq!(mask, 0b00000011);
    }

    #[test]
    fn test_period_tariff() {
        let mut manager = TariffManager::new();

        let period = BillingPeriod {
            period_id: 2,
            tariff_id: 3,
            status: BillingStatus::Active,
        };
        manager.add_billing_period(period).unwrap();

        assert_eq!(manager.period_tariff(2), Some(3));
        assert_eq!(manager.period_tariff(1), None);
    }
}
