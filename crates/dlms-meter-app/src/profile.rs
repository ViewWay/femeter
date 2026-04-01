//! Profile manager for load profile capture and historical data
//!
//! This module provides:
//! - Load profile capture at configurable intervals
//! - Circular buffer storage for profile data
//! - Historical data management
//! - Profile object capture time handling

extern crate alloc;

use alloc::vec::Vec;
use dlms_core::{errors::CosemError, types::CosemDateTime};

/// Maximum number of values in a single profile entry
const MAX_PROFILE_VALUES: usize = 16;

/// Single entry in a load profile
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileEntry {
    /// Capture timestamp
    pub capture_time: CosemDateTime,
    /// Status flags for this entry
    pub status: u8,
    /// Values captured (each value corresponds to a column in the profile)
    pub values: Vec<i64>,
}

impl ProfileEntry {
    /// Create a new profile entry
    pub fn new(capture_time: CosemDateTime) -> Self {
        Self {
            capture_time,
            status: 0,
            values: Vec::with_capacity(4),
        }
    }

    /// Add a value to this entry
    pub fn add_value(&mut self, value: i64) {
        if self.values.len() < MAX_PROFILE_VALUES {
            self.values.push(value);
        }
    }

    /// Get number of values in this entry
    pub fn value_count(&self) -> usize {
        self.values.len()
    }
}

/// Column definition for load profile
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileColumn {
    /// Column index (0-based)
    pub index: u8,
    /// OBIS code of the object to capture
    pub obis_code: [u8; 6],
    /// Scaler for this column (10^scaler)
    pub scaler: i8,
    /// Capture method selector
    pub method: u8,
}

impl ProfileColumn {
    /// Create a new profile column
    pub fn new(index: u8, obis_code: [u8; 6], scaler: i8) -> Self {
        Self {
            index,
            obis_code,
            scaler,
            method: 0,
        }
    }
}

/// Profile manager for handling load profiles
#[derive(Debug)]
pub struct ProfileManager {
    /// Profile columns (what to capture)
    columns: Vec<ProfileColumn>,
    /// Captured entries (circular buffer)
    entries: Vec<ProfileEntry>,
    /// Maximum number of entries to store
    max_entries: usize,
    /// Current write position
    write_index: usize,
    /// Capture period in seconds
    capture_period_s: u32,
    /// Profile is enabled
    enabled: bool,
    /// Total entries captured (for overflow detection)
    total_captured: u32,
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new(max_entries: usize, capture_period_s: u32) -> Self {
        Self {
            columns: Vec::with_capacity(8),
            entries: Vec::with_capacity(max_entries),
            max_entries,
            write_index: 0,
            capture_period_s,
            enabled: false,
            total_captured: 0,
        }
    }

    /// Add a column to the profile
    pub fn add_column(&mut self, column: ProfileColumn) -> Result<(), CosemError> {
        if self.columns.len() >= MAX_PROFILE_VALUES {
            return Err(CosemError::NotImplemented);
        }
        self.columns.push(column);
        Ok(())
    }

    /// Remove all columns
    pub fn clear_columns(&mut self) {
        self.columns.clear();
    }

    /// Get column definitions
    pub fn columns(&self) -> &[ProfileColumn] {
        &self.columns
    }

    /// Enable/disable profile capture
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if profile is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set capture period
    pub fn set_capture_period(&mut self, period_s: u32) {
        self.capture_period_s = period_s;
    }

    /// Get capture period
    pub fn capture_period(&self) -> u32 {
        self.capture_period_s
    }

    /// Capture a new profile entry
    pub fn capture(&mut self, timestamp: CosemDateTime, values: &[i64]) -> Result<(), CosemError> {
        if !self.enabled {
            return Err(CosemError::NotImplemented);
        }

        if values.len() != self.columns.len() {
            return Err(CosemError::InvalidParameter);
        }

        let mut entry = ProfileEntry::new(timestamp);
        for &value in values {
            entry.add_value(value);
        }

        // Store in circular buffer
        if self.entries.len() < self.max_entries {
            self.entries.push(entry);
            self.write_index = self.entries.len() % self.max_entries;
        } else {
            self.entries[self.write_index] = entry;
            self.write_index = (self.write_index + 1) % self.max_entries;
        }

        self.total_captured = self.total_captured.wrapping_add(1);
        Ok(())
    }

    /// Get all captured entries
    pub fn entries(&self) -> &[ProfileEntry] {
        &self.entries
    }

    /// Get entry by index
    pub fn get_entry(&self, index: usize) -> Option<&ProfileEntry> {
        self.entries.get(index)
    }

    /// Get oldest entry
    pub fn oldest_entry(&self) -> Option<&ProfileEntry> {
        if self.entries.is_full() {
            self.entries.get(self.write_index)
        } else {
            self.entries.first()
        }
    }

    /// Get newest entry
    pub fn newest_entry(&self) -> Option<&ProfileEntry> {
        if self.entries.is_empty() {
            return None;
        }
        let idx = if self.write_index == 0 {
            self.entries.len() - 1
        } else {
            self.write_index - 1
        };
        self.entries.get(idx)
    }

    /// Clear all captured entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.write_index = 0;
        self.total_captured = 0;
    }

    /// Get total number of entries captured
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get total captured (including overflowed entries)
    pub fn total_captured(&self) -> u32 {
        self.total_captured
    }

    /// Check if buffer has overflowed
    pub fn has_overflowed(&self) -> bool {
        self.total_captured > self.entries.len() as u32
    }

    /// Resize the profile buffer
    pub fn resize(&mut self, new_size: usize) -> Result<(), CosemError> {
        if new_size == 0 {
            return Err(CosemError::InvalidParameter);
        }

        let old_len = self.entries.len().min(new_size);
        let mut new_entries = Vec::with_capacity(new_size);

        // Keep oldest entries up to new size
        if self.write_index < old_len {
            // Buffer wraps - take from write_index to end, then start to write_index
            new_entries.extend_from_slice(&self.entries[self.write_index..old_len]);
            if self.write_index > 0 {
                new_entries.extend_from_slice(&self.entries[..self.write_index]);
            }
        } else {
            new_entries.extend_from_slice(&self.entries[..old_len]);
        }

        self.entries = new_entries;
        self.max_entries = new_size;
        self.write_index = self.entries.len() % new_size;

        Ok(())
    }
}

/// Historical data entry for billing period
#[derive(Debug, Clone, PartialEq)]
pub struct HistoricalEntry {
    /// Billing period identifier
    pub period: u8,
    /// Entry index within period
    pub index: u8,
    /// Capture timestamp
    pub timestamp: CosemDateTime,
    /// Stored value
    pub value: i64,
    /// Status flags
    pub status: u8,
}

impl HistoricalEntry {
    /// Create a new historical entry
    pub fn new(period: u8, index: u8, timestamp: CosemDateTime, value: i64) -> Self {
        Self {
            period,
            index,
            timestamp,
            value,
            status: 0,
        }
    }
}

/// Historical data manager for billing period data
#[derive(Debug, Clone)]
pub struct HistoricalManager {
    /// Historical entries
    entries: Vec<HistoricalEntry>,
    /// Maximum entries per billing period
    max_per_period: usize,
    /// Current billing period
    current_period: u8,
}

impl HistoricalManager {
    /// Create a new historical data manager
    pub fn new(max_per_period: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_per_period * 12),
            max_per_period,
            current_period: 1,
        }
    }

    /// Add a historical entry
    pub fn add_entry(&mut self, entry: HistoricalEntry) -> Result<(), CosemError> {
        // Check if we'd exceed max for this period
        let period_count = self
            .entries
            .iter()
            .filter(|e| e.period == entry.period)
            .count();
        if period_count >= self.max_per_period {
            return Err(CosemError::NotImplemented);
        }

        self.entries.push(entry);
        Ok(())
    }

    /// Get entries for a specific billing period
    pub fn period_entries(&self, period: u8) -> Vec<&HistoricalEntry> {
        self.entries.iter().filter(|e| e.period == period).collect()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get current billing period
    pub fn current_period(&self) -> u8 {
        self.current_period
    }

    /// Set current billing period
    pub fn set_current_period(&mut self, period: u8) {
        self.current_period = period;
    }
}

// Helper trait for Vec to check if full
trait VecFull {
    fn is_full(&self) -> bool;
}

impl<T> VecFull for Vec<T> {
    fn is_full(&self) -> bool {
        false // Vec never reports as full (dynamically grows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlms_core::types::{CosemDate, CosemTime};

    fn make_test_time() -> CosemDateTime {
        CosemDateTime {
            date: CosemDate {
                year: 2024,
                month: 1,
                day: 15,
                day_of_week: 1,
            },
            time: CosemTime {
                hour: 12,
                minute: 0,
                second: 0,
                hundredths: 0,
            },
            deviation: 0,
            clock_status: 0,
        }
    }

    #[test]
    fn test_profile_manager_new() {
        let manager = ProfileManager::new(100, 900);
        assert!(!manager.is_enabled());
        assert_eq!(manager.capture_period(), 900);
        assert_eq!(manager.entry_count(), 0);
    }

    #[test]
    fn test_add_column() {
        let mut manager = ProfileManager::new(100, 900);
        let column = ProfileColumn::new(0, [1, 0, 1, 8, 0, 255], -3);

        assert!(manager.add_column(column).is_ok());
        assert_eq!(manager.columns().len(), 1);
    }

    #[test]
    fn test_capture_entry() {
        let mut manager = ProfileManager::new(10, 900);
        manager.set_enabled(true);

        let column = ProfileColumn::new(0, [1, 0, 1, 8, 0, 255], -3);
        manager.add_column(column).unwrap();

        let timestamp = make_test_time();
        let values = &[1000]; // Only 1 value for 1 column

        assert!(manager.capture(timestamp, values).is_ok());
        assert_eq!(manager.entry_count(), 1);
    }

    #[test]
    fn test_capture_when_disabled() {
        let mut manager = ProfileManager::new(10, 900);
        // Keep enabled=false

        let timestamp = make_test_time();
        let values = &[1000];

        assert!(manager.capture(timestamp, values).is_err());
    }

    #[test]
    fn test_clear_entries() {
        let mut manager = ProfileManager::new(10, 900);
        manager.set_enabled(true);

        let column = ProfileColumn::new(0, [1, 0, 1, 8, 0, 255], -3);
        manager.add_column(column).unwrap();

        manager.capture(make_test_time(), &[1000]).unwrap();
        manager.clear();

        assert_eq!(manager.entry_count(), 0);
    }

    #[test]
    fn test_historical_manager() {
        let mut manager = HistoricalManager::new(10);
        assert_eq!(manager.current_period(), 1);

        let entry = HistoricalEntry::new(1, 0, make_test_time(), 5000);
        assert!(manager.add_entry(entry).is_ok());

        let period_1_entries = manager.period_entries(1);
        assert_eq!(period_1_entries.len(), 1);
        assert_eq!(period_1_entries[0].value, 5000);
    }

    #[test]
    fn test_profile_column() {
        let column = ProfileColumn::new(0, [1, 0, 1, 8, 0, 255], -3);
        assert_eq!(column.index, 0);
        assert_eq!(column.scaler, -3);
    }

    #[test]
    fn test_profile_entry() {
        let mut entry = ProfileEntry::new(make_test_time());
        assert_eq!(entry.value_count(), 0);

        entry.add_value(100);
        entry.add_value(200);
        assert_eq!(entry.value_count(), 2);
    }
}
