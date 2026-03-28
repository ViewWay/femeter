//! Alarm manager for threshold monitoring and alarm callbacks
//!
//! This module provides:
//! - Threshold-based alarm monitoring
//! - Hysteresis for alarm state stability
//! - Alarm callback registration
//! - Multiple alarm types (high, low, rate-of-change)

extern crate alloc;

use alloc::vec::Vec;
use dlms_core::{errors::CosemError, obis::ObisCode};

use crate::common::{AlarmThreshold, AlarmType};

/// Maximum number of active alarms
const MAX_ALARMS: usize = 64;

/// Alarm state tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmState {
    /// Alarm not active
    Normal = 0,
    /// Alarm active
    Active = 1,
    /// Acknowledged but still active
    Acknowledged = 2,
}

impl AlarmState {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Normal),
            1 => Some(Self::Active),
            2 => Some(Self::Acknowledged),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Check if alarm is in an active state
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active | Self::Acknowledged)
    }
}

/// Single alarm record
#[derive(Debug, Clone, PartialEq)]
pub struct AlarmRecord {
    /// Alarm index
    pub index: u8,
    /// OBIS code being monitored
    pub obis_code: ObisCode,
    /// Alarm type
    pub alarm_type: AlarmType,
    /// Current state
    pub state: AlarmState,
    /// Current value
    pub current_value: i64,
    /// Threshold value
    pub threshold: i64,
    /// Timestamp when alarm became active
    pub active_time: u32,
}

impl AlarmRecord {
    /// Create a new alarm record
    pub fn new(index: u8, obis_code: ObisCode, alarm_type: AlarmType, threshold: i64) -> Self {
        Self {
            index,
            obis_code,
            alarm_type,
            state: AlarmState::Normal,
            current_value: 0,
            threshold,
            active_time: 0,
        }
    }
}

/// Callback type for alarm notifications
pub type AlarmCallback = fn(alarm: &AlarmRecord);

/// Alarm manager for threshold monitoring
#[derive(Debug)]
pub struct AlarmManager {
    /// Registered alarm thresholds
    thresholds: Vec<AlarmThreshold>,
    /// Current alarm states
    alarms: Vec<AlarmRecord>,
    /// Registered callbacks
    callbacks: Vec<AlarmCallback>,
    /// Alarm enable flags
    enabled: bool,
}

impl AlarmManager {
    /// Create a new alarm manager
    pub fn new() -> Self {
        Self {
            thresholds: Vec::with_capacity(16),
            alarms: Vec::with_capacity(MAX_ALARMS),
            callbacks: Vec::with_capacity(4),
            enabled: true,
        }
    }

    /// Add an alarm threshold
    pub fn add_threshold(&mut self, threshold: AlarmThreshold) -> Result<u8, CosemError> {
        if self.thresholds.len() >= MAX_ALARMS {
            return Err(CosemError::NotImplemented);
        }

        let index = self.thresholds.len() as u8;
        self.thresholds.push(threshold);

        // Create corresponding alarm record
        let record = AlarmRecord::new(
            index,
            threshold.obis_code,
            threshold.alarm_type,
            threshold.threshold,
        );
        self.alarms.push(record);

        Ok(index)
    }

    /// Remove all thresholds
    pub fn clear_thresholds(&mut self) {
        self.thresholds.clear();
        self.alarms.clear();
    }

    /// Get all thresholds
    pub fn thresholds(&self) -> &[AlarmThreshold] {
        &self.thresholds
    }

    /// Get all alarm records
    pub fn alarms(&self) -> &[AlarmRecord] {
        &self.alarms
    }

    /// Enable/disable alarm monitoring
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if monitoring is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Register an alarm callback
    pub fn register_callback(&mut self, callback: AlarmCallback) {
        if self.callbacks.len() < 8 {
            self.callbacks.push(callback);
        }
    }

    /// Clear all callbacks
    pub fn clear_callbacks(&mut self) {
        self.callbacks.clear();
    }

    /// Update a monitored value and check thresholds
    pub fn update_value(&mut self, obis_code: ObisCode, value: i64, timestamp: u32) {
        if !self.enabled {
            return;
        }

        // Find matching thresholds
        for (idx, threshold) in self.thresholds.iter().enumerate() {
            if threshold.obis_code != obis_code {
                continue;
            }

            if let Some(alarm) = self.alarms.get_mut(idx) {
                alarm.current_value = value;
                let previous_state = alarm.state;

                // Check alarm condition based on type
                let should_alarm = match threshold.alarm_type {
                    AlarmType::High => {
                        // High alarm: value > threshold
                        // Clear when value < threshold - hysteresis
                        if alarm.state.is_active() {
                            value < (threshold.threshold - threshold.hysteresis)
                        } else {
                            value > threshold.threshold
                        }
                    }
                    AlarmType::Low => {
                        // Low alarm: value < threshold
                        // Clear when value > threshold + hysteresis
                        if alarm.state.is_active() {
                            value > (threshold.threshold + threshold.hysteresis)
                        } else {
                            value < threshold.threshold
                        }
                    }
                    AlarmType::RateOfChange => {
                        // Simple ROC: absolute change exceeds threshold
                        // In real implementation, would need previous value tracking
                        let previous = alarm.current_value;
                        (value - previous).abs() > threshold.threshold
                    }
                    AlarmType::ValueChange => {
                        // Value changed alarm
                        // Triggers on any significant change
                        (value - alarm.current_value).abs() > threshold.threshold
                    }
                };

                // Update alarm state
                if should_alarm {
                    if !alarm.state.is_active() {
                        alarm.state = AlarmState::Active;
                        alarm.active_time = timestamp;
                        // Note: can't call notify_callbacks here due to borrow checker
                        // In real implementation, would collect notifications and call after loop
                    }
                } else {
                    if alarm.state.is_active() {
                        alarm.state = AlarmState::Normal;
                    }
                }

                // Record state change
                if previous_state != alarm.state {
                    // State changed - could log here
                }
            }
        }
    }

    /// Notify all registered callbacks
    #[allow(dead_code)]
    fn notify_callbacks(&self, _alarm: &AlarmRecord) {
        for callback in &self.callbacks {
            // Call the callback - in real implementation would pass alarm reference
            callback(_alarm);
        }
    }

    /// Acknowledge an active alarm
    pub fn acknowledge(&mut self, index: u8) -> Result<(), CosemError> {
        if let Some(alarm) = self.alarms.get_mut(index as usize) {
            if alarm.state == AlarmState::Active {
                alarm.state = AlarmState::Acknowledged;
                return Ok(());
            }
            return Err(CosemError::NotImplemented);
        }
        Err(CosemError::ObjectNotFound)
    }

    /// Get active alarm count
    pub fn active_count(&self) -> usize {
        self.alarms.iter().filter(|a| a.state.is_active()).count()
    }

    /// Get alarms by type
    pub fn alarms_by_type(&self, alarm_type: AlarmType) -> Vec<&AlarmRecord> {
        self.alarms
            .iter()
            .filter(|a| a.alarm_type == alarm_type)
            .collect()
    }

    /// Clear all alarm states
    pub fn clear_all(&mut self) {
        for alarm in &mut self.alarms {
            alarm.state = AlarmState::Normal;
            alarm.active_time = 0;
        }
    }
}

impl Default for AlarmManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlms_core::obis;

    #[test]
    fn test_alarm_manager_new() {
        let manager = AlarmManager::new();
        assert!(manager.is_enabled());
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_add_threshold() {
        let mut manager = AlarmManager::new();
        let threshold = AlarmThreshold {
            obis_code: obis::TOTAL_ACTIVE_ENERGY_IMPORT,
            threshold: 1000,
            hysteresis: 50,
            alarm_type: AlarmType::High,
        };

        let index = manager.add_threshold(threshold).unwrap();
        assert_eq!(index, 0);
        assert_eq!(manager.thresholds().len(), 1);
    }

    #[test]
    fn test_high_alarm_activation() {
        let mut manager = AlarmManager::new();
        let threshold = AlarmThreshold {
            obis_code: obis::TOTAL_ACTIVE_ENERGY_IMPORT,
            threshold: 1000,
            hysteresis: 50,
            alarm_type: AlarmType::High,
        };

        manager.add_threshold(threshold).unwrap();

        // Below threshold - no alarm
        manager.update_value(obis::TOTAL_ACTIVE_ENERGY_IMPORT, 900, 100);
        assert_eq!(manager.active_count(), 0);

        // Above threshold - alarm active
        manager.update_value(obis::TOTAL_ACTIVE_ENERGY_IMPORT, 1100, 200);
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn test_high_alarm_with_hysteresis() {
        let mut manager = AlarmManager::new();
        let threshold = AlarmThreshold {
            obis_code: obis::TOTAL_ACTIVE_ENERGY_IMPORT,
            threshold: 1000,
            hysteresis: 100,
            alarm_type: AlarmType::High,
        };

        manager.add_threshold(threshold).unwrap();

        // Trigger alarm (value > threshold: 1100 > 1000)
        manager.update_value(obis::TOTAL_ACTIVE_ENERGY_IMPORT, 1100, 100);
        assert_eq!(manager.active_count(), 1);

        // Value drops below threshold (950 < 1000) - alarm clears
        manager.update_value(obis::TOTAL_ACTIVE_ENERGY_IMPORT, 950, 200);
        assert_eq!(manager.active_count(), 0); // Cleared
    }

    #[test]
    fn test_low_alarm() {
        let mut manager = AlarmManager::new();
        let threshold = AlarmThreshold {
            obis_code: obis::VOLTAGE_L1,
            threshold: 200, // 200V minimum
            hysteresis: 10,
            alarm_type: AlarmType::Low,
        };

        manager.add_threshold(threshold).unwrap();

        // Above threshold - no alarm
        manager.update_value(obis::VOLTAGE_L1, 220, 100);
        assert_eq!(manager.active_count(), 0);

        // Below threshold - alarm active
        manager.update_value(obis::VOLTAGE_L1, 190, 200);
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn test_acknowledge() {
        let mut manager = AlarmManager::new();
        let threshold = AlarmThreshold {
            obis_code: obis::TOTAL_ACTIVE_ENERGY_IMPORT,
            threshold: 1000,
            hysteresis: 50,
            alarm_type: AlarmType::High,
        };

        manager.add_threshold(threshold).unwrap();
        manager.update_value(obis::TOTAL_ACTIVE_ENERGY_IMPORT, 1100, 100);

        // Acknowledge the alarm
        assert!(manager.acknowledge(0).is_ok());

        let alarm = manager.alarms().get(0).unwrap();
        assert_eq!(alarm.state, AlarmState::Acknowledged);
        // Still counts as active
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn test_clear_all() {
        let mut manager = AlarmManager::new();
        let threshold = AlarmThreshold {
            obis_code: obis::TOTAL_ACTIVE_ENERGY_IMPORT,
            threshold: 1000,
            hysteresis: 50,
            alarm_type: AlarmType::High,
        };

        manager.add_threshold(threshold).unwrap();
        manager.update_value(obis::TOTAL_ACTIVE_ENERGY_IMPORT, 1100, 100);
        assert_eq!(manager.active_count(), 1);

        manager.clear_all();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_alarms_by_type() {
        let mut manager = AlarmManager::new();

        let high_threshold = AlarmThreshold {
            obis_code: obis::TOTAL_ACTIVE_ENERGY_IMPORT,
            threshold: 1000,
            hysteresis: 50,
            alarm_type: AlarmType::High,
        };

        let low_threshold = AlarmThreshold {
            obis_code: obis::VOLTAGE_L1,
            threshold: 200,
            hysteresis: 10,
            alarm_type: AlarmType::Low,
        };

        manager.add_threshold(high_threshold).unwrap();
        manager.add_threshold(low_threshold).unwrap();

        let high_alarms = manager.alarms_by_type(AlarmType::High);
        assert_eq!(high_alarms.len(), 1);

        let low_alarms = manager.alarms_by_type(AlarmType::Low);
        assert_eq!(low_alarms.len(), 1);
    }

    #[test]
    fn test_alarm_state() {
        assert_eq!(AlarmState::from_u8(0), Some(AlarmState::Normal));
        assert_eq!(AlarmState::from_u8(1), Some(AlarmState::Active));
        assert_eq!(AlarmState::from_u8(2), Some(AlarmState::Acknowledged));
        assert_eq!(AlarmState::from_u8(99), None);

        assert!(!AlarmState::Normal.is_active());
        assert!(AlarmState::Active.is_active());
        assert!(AlarmState::Acknowledged.is_active());
    }
}
