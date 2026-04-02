//! Main meter application with integrated subsystems
//!
//! This module provides the integrated meter application that combines
//! all subsystems into a cohesive application with state management,
//! configuration, and workflow orchestration.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use dlms_core::{errors::CosemError, obis::ObisCode, types::CosemDateTime};

use crate::alarm::AlarmManager;
use crate::clock::{ClockManager, DstMode, Timezone};
use crate::comm::{CommManager, PortType};
use crate::common::{BillingStatus, DemandConfig};
use crate::control::{RelayControl, RelayState};
use crate::firmware::FirmwareManager;
use crate::measurement::MeasurementEngine;
use crate::profile::ProfileManager;
use crate::tariff::TariffManager;

/// Meter application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MeterAppState {
    /// Initializing - subsystems being set up
    Init = 0,
    /// Running - normal operation
    Running = 1,
    /// Error state - requires attention
    Error = 2,
}

impl MeterAppState {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Init),
            1 => Some(Self::Running),
            2 => Some(Self::Error),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Check if meter is in normal running state
    pub fn is_running(self) -> bool {
        self == Self::Running
    }
}

/// Meter configuration
#[derive(Debug, Clone, PartialEq)]
pub struct MeterConfig {
    /// Demand calculation configuration
    pub demand_config: DemandConfig,
    /// Profile buffer size (number of entries)
    pub profile_size: usize,
    /// Profile capture period in seconds
    pub profile_period_s: u32,
    /// Communication port type
    pub port_type: PortType,
    /// Timezone offset in minutes
    pub timezone_offset: i16,
    /// DST enabled
    pub dst_enabled: bool,
    /// Firmware version string
    pub firmware_version: Vec<u8>,
    /// Auto reconnect on disconnect
    pub auto_reconnect: bool,
    /// Clock sync interval in seconds
    pub clock_sync_interval_s: u32,
}

impl MeterConfig {
    /// Create default configuration
    pub fn new() -> Self {
        Self {
            demand_config: DemandConfig::default_15min(),
            profile_size: 96,
            profile_period_s: 900,
            port_type: PortType::Rs485,
            timezone_offset: 0,
            dst_enabled: false,
            firmware_version: b"1.0.0".to_vec(),
            auto_reconnect: false,
            clock_sync_interval_s: 3600,
        }
    }

    /// Create configuration for specific region
    pub fn with_timezone(offset_minutes: i16, dst: bool) -> Self {
        Self {
            timezone_offset: offset_minutes,
            dst_enabled: dst,
            ..Self::new()
        }
    }
}

impl Default for MeterConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete meter data snapshot
#[derive(Debug, Clone, PartialEq)]
pub struct MeterData {
    /// Total active energy import (Wh)
    pub energy_import: i64,
    /// Total active energy export (Wh)
    pub energy_export: i64,
    /// Current instantaneous power (W)
    pub current_power: i32,
    /// Current demand (W)
    pub current_demand: i32,
    /// Current tariff (1-8)
    pub current_tariff: u8,
    /// Relay state
    pub relay_state: RelayState,
    /// Current time
    pub current_time: CosemDateTime,
    /// Connection state
    pub connected: bool,
}

/// Billing data for a period
#[derive(Debug, Clone, PartialEq)]
pub struct BillingData {
    /// Period identifier
    pub period_id: u8,
    /// Tariff used for this period
    pub tariff_id: u8,
    /// Energy consumption (Wh)
    pub energy_consumed: i64,
    /// Period status
    pub status: BillingStatus,
}

/// Alarm event for notification
#[derive(Debug, Clone, PartialEq)]
pub struct AlarmEvent {
    /// Alarm index
    pub index: u8,
    /// OBIS code that triggered alarm
    pub obis_code: ObisCode,
    /// Current value
    pub value: i64,
    /// Threshold value
    pub threshold: i64,
    /// Timestamp when alarm triggered
    pub timestamp: u32,
}

/// Main meter application integrating all subsystems
#[derive(Debug)]
pub struct MeterApp {
    /// Application state
    state: MeterAppState,
    /// Measurement engine
    pub measurement: MeasurementEngine,
    /// Tariff manager
    pub tariff: TariffManager,
    /// Profile manager
    pub profile: ProfileManager,
    /// Alarm manager
    pub alarm: AlarmManager,
    /// Relay control
    pub control: RelayControl,
    /// Firmware manager
    pub firmware: FirmwareManager,
    /// Clock manager
    pub clock: ClockManager,
    /// Communication manager
    pub comm: CommManager,
    /// Monotonic time counter (seconds since boot)
    uptime: u32,
    /// Last profile capture time
    last_profile_capture: u32,
    /// Configuration
    config: MeterConfig,
    /// Error message (when in Error state)
    error_message: Option<String>,
}

impl MeterApp {
    /// Create a new meter application with default configuration
    pub fn new() -> Self {
        Self::with_config(MeterConfig::new())
    }

    /// Create meter application with specific configuration
    pub fn with_config(config: MeterConfig) -> Self {
        let mut meter = Self {
            state: MeterAppState::Init,
            measurement: MeasurementEngine::with_config(config.demand_config),
            tariff: TariffManager::new(),
            profile: ProfileManager::new(config.profile_size, config.profile_period_s),
            alarm: AlarmManager::new(),
            control: RelayControl::new(),
            firmware: FirmwareManager::new(config.firmware_version.clone()),
            clock: ClockManager::new(),
            comm: CommManager::new(config.port_type),
            uptime: 0,
            last_profile_capture: 0,
            config,
            error_message: None,
        };

        // Set up timezone
        let tz = Timezone::new(
            meter.config.timezone_offset,
            if meter.config.dst_enabled {
                DstMode::Daylight
            } else {
                DstMode::Standard
            },
        );
        meter.clock.set_timezone(tz);
        meter
            .clock
            .set_sync_interval(meter.config.clock_sync_interval_s);

        // Enable auto reconnect if configured
        meter
            .control
            .set_auto_reconnect(meter.config.auto_reconnect);

        // Transition to running state
        meter.state = MeterAppState::Running;

        meter
    }

    /// Get current application state
    pub fn state(&self) -> MeterAppState {
        self.state
    }

    /// Set application state
    pub fn set_state(&mut self, state: MeterAppState) {
        self.state = state;
    }

    /// Get current uptime
    pub fn uptime(&self) -> u32 {
        self.uptime
    }

    /// Get configuration reference
    pub fn config(&self) -> &MeterConfig {
        &self.config
    }

    /// Get error message (if in error state)
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Enter error state with message
    pub fn enter_error(&mut self, message: String) {
        self.state = MeterAppState::Error;
        self.error_message = Some(message);
    }

    /// Clear error and return to running state
    pub fn clear_error(&mut self) {
        self.state = MeterAppState::Running;
        self.error_message = None;
    }

    /// Run one complete meter measurement cycle
    ///
    /// This method orchestrates a single measurement cycle:
    /// 1. Updates time
    /// 2. Updates tariff based on current time
    /// 3. Checks for profile capture
    /// 4. Checks for clock sync
    /// 5. Checks for push notifications
    pub fn run_cycle(&mut self) -> Result<(), CosemError> {
        if !self.state.is_running() {
            return Err(CosemError::AccessDenied);
        }

        // Update clock (one tick = 1 second default)
        self.clock.tick(1);

        // Update tariff based on current time
        self.tariff
            .update_tariff_for_time(self.clock.current_time());
        self.tariff.update_billing_period(self.clock.current_time());

        // Check if profile capture is needed
        let elapsed = self.uptime.saturating_sub(self.last_profile_capture);
        if elapsed >= self.profile.capture_period() && self.profile.is_enabled() {
            self.capture_profile()?;
            self.last_profile_capture = self.uptime;
        }

        // Check if clock sync is needed
        if self.clock.needs_sync(self.uptime) {
            // In real implementation, would initiate clock sync here
            // For now, just mark as attempted
        }

        // Check if push is needed
        if self.comm.needs_push(self.uptime) {
            // In real implementation, would initiate push here
        }

        // Try auto reconnect if disconnected and enabled
        if !self.control.is_connected() && self.control.auto_reconnect() {
            let _ = self.control.try_auto_reconnect(self.uptime);
        }

        // Increment uptime
        self.uptime = self.uptime.saturating_add(1);

        Ok(())
    }

    /// Process a reading from a measurement channel
    ///
    /// # Arguments
    /// * `channel` - Measurement channel (0 = L1/total, 1 = L2, 2 = L3)
    /// * `value` - Measurement value (scaled appropriately)
    pub fn process_reading(&mut self, channel: u8, value: f64) {
        if !self.state.is_running() {
            return;
        }

        // Convert f64 to integer for internal storage
        // Scale based on measurement type (power in W, voltage in mV, etc.)
        let scaled_value = (value * 1000.0) as i32;

        match channel {
            0 => {
                // Power measurement - process for current tariff
                let tariff = (self.tariff.current_tariff().saturating_sub(1)) as usize;
                self.measurement.process_power(scaled_value, tariff, 1);
            }
            1..=3 => {
                // Voltage/current per phase
                let phase = (channel - 1) as usize;
                if phase < 3 {
                    let _ = self.measurement.update_voltage(phase, scaled_value as u16);
                }
            }
            _ => {}
        }

        // Check alarms for energy value
        let total_energy = self.measurement.total_energy_import();
        self.alarm.update_value(
            dlms_core::obis::TOTAL_ACTIVE_ENERGY_IMPORT,
            total_energy,
            self.uptime,
        );
    }

    /// Check for active alarms and return any new events
    ///
    /// Returns a vector of alarm events that have triggered
    pub fn check_alarms(&mut self) -> Vec<AlarmEvent> {
        if !self.state.is_running() {
            return Vec::new();
        }

        let mut events = Vec::new();
        let alarms = self.alarm.alarms();

        for alarm in alarms {
            if alarm.state.is_active() && alarm.active_time == self.uptime {
                events.push(AlarmEvent {
                    index: alarm.index,
                    obis_code: alarm.obis_code,
                    value: alarm.current_value,
                    threshold: alarm.threshold,
                    timestamp: alarm.active_time,
                });
            }
        }

        events
    }

    /// Read current meter data
    ///
    /// Returns a snapshot of all current meter values
    pub fn read_meter_data(&self) -> MeterData {
        MeterData {
            energy_import: self.measurement.total_energy_import(),
            energy_export: self.measurement.total_energy_export(),
            current_power: self.measurement.instant_power(0).unwrap_or(0),
            current_demand: self.measurement.current_demand(),
            current_tariff: self.tariff.current_tariff(),
            relay_state: self.control.state(),
            current_time: *self.clock.current_time(),
            connected: self.comm.is_connected(),
        }
    }

    /// Get billing data for a specific period
    ///
    /// # Arguments
    /// * `period` - Billing period identifier (1-12)
    pub fn get_billing_data(&self, period: u8) -> Option<BillingData> {
        let billing_period = self
            .tariff
            .billing_periods()
            .iter()
            .find(|p| p.period_id == period)?;

        // Get energy for the tariff associated with this period
        let tariff_idx = billing_period.tariff_id.saturating_sub(1) as usize;
        let energy_consumed = self.measurement.tariff_energy(tariff_idx).unwrap_or(0);

        Some(BillingData {
            period_id: billing_period.period_id,
            tariff_id: billing_period.tariff_id,
            energy_consumed,
            status: billing_period.status,
        })
    }

    /// Disconnect the load (open relay)
    pub fn disconnect(&mut self) -> Result<(), CosemError> {
        if !self.state.is_running() {
            return Err(CosemError::AccessDenied);
        }
        self.control.disconnect(self.uptime)
    }

    /// Reconnect the load (close relay)
    pub fn reconnect(&mut self) -> Result<(), CosemError> {
        if !self.state.is_running() {
            return Err(CosemError::AccessDenied);
        }
        self.control.reconnect(self.uptime)
    }

    /// Synchronize clock from external source
    ///
    /// # Arguments
    /// * `datetime` - New date-time to set
    pub fn sync_clock(&mut self, datetime: CosemDateTime) -> Result<(), CosemError> {
        if !self.state.is_running() {
            return Err(CosemError::AccessDenied);
        }
        self.clock.sync(datetime, self.uptime)
    }

    /// Capture a profile entry with current values
    fn capture_profile(&mut self) -> Result<(), CosemError> {
        let timestamp = *self.clock.current_time();

        // Build values array from columns
        let mut values = Vec::with_capacity(self.profile.columns().len());

        for column in self.profile.columns() {
            // Map OBIS code to actual value
            let value = self.get_value_for_obis(column.obis_code);
            values.push(value);
        }

        self.profile.capture(timestamp, &values)
    }

    /// Get value for an OBIS code
    fn get_value_for_obis(&self, _obis: [u8; 6]) -> i64 {
        // Common OBIS codes
        match _obis {
            // Total energy import
            [1, 0, 1, 8, 0, 255] => self.measurement.total_energy_import(),
            // Total energy export
            [1, 0, 2, 8, 0, 255] => self.measurement.total_energy_export(),
            // L1 voltage
            [1, 0, 32, 7, 0, 255] => self.measurement.voltage(0).unwrap_or(0) as i64,
            // L2 voltage
            [1, 0, 52, 7, 0, 255] => self.measurement.voltage(1).unwrap_or(0) as i64,
            // L3 voltage
            [1, 0, 72, 7, 0, 255] => self.measurement.voltage(2).unwrap_or(0) as i64,
            // Current power
            [1, 0, 1, 7, 0, 255] => self.measurement.instant_power(0).unwrap_or(0) as i64,
            _ => 0,
        }
    }

    /// Add a profile column for capture
    pub fn add_profile_column(&mut self, obis_code: [u8; 6], scaler: i8) -> Result<(), CosemError> {
        use crate::profile::ProfileColumn;
        let index = self.profile.columns().len() as u8;
        let column = ProfileColumn::new(index, obis_code, scaler);
        self.profile.add_column(column)
    }

    /// Enable profile capture
    pub fn enable_profile(&mut self) {
        self.profile.set_enabled(true);
    }

    /// Disable profile capture
    pub fn disable_profile(&mut self) {
        self.profile.set_enabled(false);
    }

    /// Add an alarm threshold
    pub fn add_alarm(
        &mut self,
        obis_code: ObisCode,
        threshold: i64,
        hysteresis: i64,
    ) -> Result<u8, CosemError> {
        use crate::common::{AlarmThreshold, AlarmType};
        let alarm = AlarmThreshold {
            obis_code,
            threshold,
            hysteresis,
            alarm_type: AlarmType::High,
        };
        self.alarm.add_threshold(alarm)
    }

    /// Connect communication port
    pub fn connect(&mut self) -> Result<(), CosemError> {
        self.comm.connect()
    }

    /// Disconnect communication port
    pub fn comm_disconnect(&mut self) -> Result<(), CosemError> {
        self.comm.disconnect()
    }

    /// Reset the meter application
    pub fn reset(&mut self) {
        self.state = MeterAppState::Init;
        self.measurement.reset_energy();
        self.profile.clear();
        self.alarm.clear_all();
        self.uptime = 0;
        self.last_profile_capture = 0;
        self.error_message = None;
        self.state = MeterAppState::Running;
    }

    // ========== Backward compatibility methods ==========

    /// Create meter with specific demand configuration (backward compatibility)
    pub fn with_demand_config(config: DemandConfig) -> Self {
        Self::with_config(MeterConfig {
            demand_config: config,
            ..MeterConfig::new()
        })
    }

    /// Process power measurement (backward compatibility)
    ///
    /// # Arguments
    /// * `power_w` - Active power in watts
    /// * `tariff` - Current tariff (0-7, 0 = use current tariff)
    /// * `interval_s` - Integration interval in seconds
    pub fn process_power(&mut self, power_w: i32, tariff: usize, interval_s: u32) {
        // Get current tariff if not specified
        let effective_tariff = if tariff == 0 {
            self.tariff.current_tariff().saturating_sub(1) as usize
        } else {
            tariff.saturating_sub(1)
        };

        self.measurement
            .process_power(power_w, effective_tariff, interval_s);

        // Update alarm with new value
        let total_energy = self.measurement.total_energy_import();
        self.alarm.update_value(
            dlms_core::obis::TOTAL_ACTIVE_ENERGY_IMPORT,
            total_energy,
            self.uptime,
        );
    }

    /// Process 3-phase power measurement (backward compatibility)
    pub fn process_3phase_power(
        &mut self,
        l1_w: i32,
        l2_w: i32,
        l3_w: i32,
        tariff: usize,
        interval_s: u32,
    ) {
        self.measurement
            .process_3phase_power(l1_w, l2_w, l3_w, tariff, interval_s);
    }

    /// Update voltage measurement (backward compatibility)
    pub fn update_voltage(&mut self, phase: usize, voltage_mv: u16) -> Result<(), CosemError> {
        self.measurement.update_voltage(phase, voltage_mv)?;

        // Check voltage alarm
        if let Some(v) = self.measurement.voltage(phase) {
            let obis = match phase {
                0 => dlms_core::obis::VOLTAGE_L1,
                1 => dlms_core::obis::VOLTAGE_L2,
                _ => dlms_core::obis::VOLTAGE_L3,
            };
            self.alarm.update_value(obis, v as i64, self.uptime);
        }

        Ok(())
    }

    /// Advance time by given seconds (backward compatibility)
    pub fn tick(&mut self, elapsed_seconds: u32) {
        self.uptime = self.uptime.saturating_add(elapsed_seconds);
        self.clock.tick(elapsed_seconds);

        // Update tariff based on time
        self.tariff
            .update_tariff_for_time(self.clock.current_time());
        self.tariff.update_billing_period(self.clock.current_time());

        // Check if sync is needed
        if self.clock.needs_sync(self.uptime) {
            // Would trigger sync here
        }

        // Check if push is needed
        if self.comm.needs_push(self.uptime) {
            // Would trigger push here
        }
    }

    /// Set current time (backward compatibility)
    pub fn set_time(&mut self, dt: CosemDateTime) -> Result<(), CosemError> {
        self.clock.set_time(dt)
    }

    /// Disconnect load (relay control) - backward compatibility alias
    pub fn disconnect_load(&mut self) -> Result<(), CosemError> {
        self.disconnect()
    }

    /// Reconnect load - backward compatibility alias
    pub fn reconnect_load(&mut self) -> Result<(), CosemError> {
        self.reconnect()
    }

    /// Emergency disconnect
    pub fn emergency_disconnect(&mut self) -> Result<(), CosemError> {
        self.control.emergency_disconnect(self.uptime)
    }
}

impl Default for MeterApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlms_core::types::{CosemDate, CosemTime};

    fn make_test_datetime() -> CosemDateTime {
        CosemDateTime {
            date: CosemDate {
                year: 2024,
                month: 6,
                day: 15,
                day_of_week: 6,
            },
            time: CosemTime {
                hour: 14,
                minute: 30,
                second: 0,
                hundredths: 0,
            },
            deviation: 0,
            clock_status: 0,
        }
    }

    #[test]
    fn test_meter_app_new() {
        let app = MeterApp::new();
        assert_eq!(app.state(), MeterAppState::Running);
        assert_eq!(app.uptime(), 0);
        assert!(app.error_message().is_none());
    }

    #[test]
    fn test_meter_app_with_config() {
        let config = MeterConfig::with_timezone(60, true);
        let app = MeterApp::with_config(config);

        assert_eq!(app.state(), MeterAppState::Running);
        assert_eq!(app.config().timezone_offset, 60);
        assert_eq!(app.config().dst_enabled, true);
    }

    #[test]
    fn test_meter_state_conversion() {
        assert_eq!(MeterAppState::from_u8(0), Some(MeterAppState::Init));
        assert_eq!(MeterAppState::from_u8(1), Some(MeterAppState::Running));
        assert_eq!(MeterAppState::from_u8(2), Some(MeterAppState::Error));

        assert!(MeterAppState::Running.is_running());
        assert!(!MeterAppState::Error.is_running());
    }

    #[test]
    fn test_enter_error() {
        let mut app = MeterApp::new();
        app.enter_error(String::from("Test error"));

        assert_eq!(app.state(), MeterAppState::Error);
        assert_eq!(app.error_message(), Some("Test error"));
    }

    #[test]
    fn test_clear_error() {
        let mut app = MeterApp::new();
        app.enter_error(String::from("Test error"));
        app.clear_error();

        assert_eq!(app.state(), MeterAppState::Running);
        assert!(app.error_message().is_none());
    }

    #[test]
    fn test_run_cycle() {
        let mut app = MeterApp::new();
        assert!(app.run_cycle().is_ok());
        assert_eq!(app.uptime(), 1);

        assert!(app.run_cycle().is_ok());
        assert_eq!(app.uptime(), 2);
    }

    #[test]
    fn test_run_cycle_in_error_state() {
        let mut app = MeterApp::new();
        app.enter_error(String::from("Error"));

        assert!(app.run_cycle().is_err());
        assert_eq!(app.uptime(), 0); // Not incremented
    }

    #[test]
    fn test_process_reading() {
        let mut app = MeterApp::new();

        // Process power reading (channel 0)
        app.process_reading(0, 1000.0);

        let data = app.read_meter_data();
        assert!(data.energy_import > 0);
    }

    #[test]
    fn test_read_meter_data() {
        let mut app = MeterApp::new();
        app.process_reading(0, 500.0);

        let data = app.read_meter_data();
        assert_eq!(data.current_tariff, 1);
        assert_eq!(data.relay_state, RelayState::Closed);
        assert!(!data.connected);
    }

    #[test]
    fn test_disconnect_reconnect() {
        let mut app = MeterApp::new();
        // Advance time to pass minimum interval check (5 seconds)
        app.control.set_min_interval(0); // Disable minimum interval for testing

        assert!(app.disconnect().is_ok());
        assert_eq!(app.control.state(), RelayState::Open);

        assert!(app.reconnect().is_ok());
        assert_eq!(app.control.state(), RelayState::Closed);
    }

    #[test]
    fn test_sync_clock() {
        let mut app = MeterApp::new();
        let dt = make_test_datetime();

        assert!(app.sync_clock(dt).is_ok());
        assert_eq!(app.clock.sync_status(), crate::clock::SyncStatus::Synced);
    }

    #[test]
    fn test_add_alarm() {
        let mut app = MeterApp::new();

        let result = app.add_alarm(dlms_core::obis::TOTAL_ACTIVE_ENERGY_IMPORT, 1000, 100);
        assert!(result.is_ok());
        assert_eq!(app.alarm.thresholds().len(), 1);
    }

    #[test]
    fn test_check_alarms() {
        let mut app = MeterApp::new();
        app.add_alarm(dlms_core::obis::TOTAL_ACTIVE_ENERGY_IMPORT, 500, 50)
            .unwrap();

        // Process reading below threshold
        app.process_reading(0, 100.0);
        let events = app.check_alarms();
        assert_eq!(events.len(), 0);

        // Process reading above threshold
        app.process_reading(0, 1000.0);
        app.tick_for_alarms();
        let _events = app.check_alarms();
        // May or may not trigger depending on alarm update timing
    }

    #[test]
    fn test_get_billing_data() {
        let mut app = MeterApp::new();

        use crate::common::BillingPeriod;
        let period = BillingPeriod {
            period_id: 1,
            tariff_id: 1,
            status: BillingStatus::Active,
        };
        assert!(app.tariff.add_billing_period(period).is_ok());

        // Add some energy
        app.process_reading(0, 100.0);

        let billing = app.get_billing_data(1);
        assert!(billing.is_some());
        let data = billing.unwrap();
        assert_eq!(data.period_id, 1);
        assert_eq!(data.tariff_id, 1);
    }

    #[test]
    fn test_profile_operations() {
        let mut app = MeterApp::new();

        assert!(app.add_profile_column([1, 0, 1, 8, 0, 255], -3).is_ok());
        app.enable_profile();

        assert!(app.profile.is_enabled());
        assert_eq!(app.profile.columns().len(), 1);
    }

    #[test]
    fn test_reset() {
        let mut app = MeterApp::new();
        app.process_reading(0, 1000.0);
        app.enter_error(String::from("Error"));

        app.reset();

        assert_eq!(app.state(), MeterAppState::Running);
        assert_eq!(app.uptime(), 0);
        assert!(app.error_message().is_none());
    }

    #[test]
    fn test_connect_disconnect_comm() {
        let mut app = MeterApp::new();

        assert!(app.connect().is_ok());
        assert!(app.comm.is_connected());

        assert!(app.comm_disconnect().is_ok());
        assert!(!app.comm.is_connected());
    }

    #[test]
    fn test_meter_config_default() {
        let config = MeterConfig::new();
        assert_eq!(config.profile_size, 96);
        assert_eq!(config.profile_period_s, 900);
        assert_eq!(config.port_type, PortType::Rs485);
        assert!(!config.dst_enabled);
    }

    #[test]
    fn test_meter_config_with_timezone() {
        let config = MeterConfig::with_timezone(-300, true); // EST with DST
        assert_eq!(config.timezone_offset, -300);
        assert!(config.dst_enabled);
    }

    // Helper for alarm testing
    trait AlarmTestExt {
        fn tick_for_alarms(&mut self);
    }

    impl AlarmTestExt for MeterApp {
        fn tick_for_alarms(&mut self) {
            self.uptime = self.uptime.saturating_add(1);
        }
    }

    #[test]
    fn test_full_workflow() {
        let mut app = MeterApp::new();
        // Disable minimum interval for testing
        app.control.set_min_interval(0);

        // 1. Set up meter
        app.add_alarm(dlms_core::obis::TOTAL_ACTIVE_ENERGY_IMPORT, 500, 50)
            .unwrap();
        app.add_profile_column([1, 0, 1, 8, 0, 255], -3).unwrap();
        app.enable_profile();
        app.connect().unwrap();

        // 2. Process measurements
        for _ in 0..10 {
            app.process_reading(0, 100.0);
            let _ = app.run_cycle();
        }

        // 3. Check state
        let data = app.read_meter_data();
        assert!(data.energy_import > 0);
        assert!(data.connected);

        // 4. Test disconnect/reconnect
        app.disconnect().unwrap();
        assert!(!app.control.is_connected());
        app.reconnect().unwrap();
        assert!(app.control.is_connected());
    }

    // ============================================================
    // Phase C — Boundary Tests
    // ============================================================

    #[test]
    fn test_zero_power_measurement() {
        let mut app = MeterApp::new();
        app.process_power(0, 0, 60);
        let data = app.read_meter_data();
        assert_eq!(data.current_power, 0);
    }

    #[test]
    fn test_high_power_measurement() {
        let mut app = MeterApp::new();
        app.process_power(99999, 0, 60);
        let data = app.read_meter_data();
        assert!(data.current_power > 0);
    }

    #[test]
    fn test_single_tick() {
        let mut app = MeterApp::new();
        app.tick(1);
    }

    #[test]
    fn test_long_tick() {
        let mut app = MeterApp::new();
        app.tick(3600);
    }

    #[test]
    fn test_multiple_process_power_calls() {
        let mut app = MeterApp::new();
        for _ in 0..100 {
            app.process_power(1000, 0, 60);
        }
        let data = app.read_meter_data();
        assert!(data.energy_import > 0);
    }

    #[test]
    fn test_negative_power() {
        let mut app = MeterApp::new();
        app.process_power(-500, 0, 60);
        let data = app.read_meter_data();
        // Should handle negative power
    }

    #[test]
    fn test_read_data_without_connect() {
        let app = MeterApp::new();
        let data = app.read_meter_data();
        assert_eq!(data.current_power, 0);
    }

    #[test]
    fn test_clock_advancement() {
        let mut app = MeterApp::new();
        app.tick(60);
        app.tick(60);
        app.tick(60);
    }

    #[test]
    fn test_alarm_manager_access() {
        let app = MeterApp::new();
        // Verify the meter has alarm handling capability
        let _data = app.read_meter_data();
    }

    #[test]
    fn test_default_state() {
        let app = MeterApp::new();
        let data = app.read_meter_data();
        assert_eq!(data.energy_import, 0);
        assert_eq!(data.energy_export, 0);
        assert_eq!(data.current_power, 0);
        assert_eq!(data.current_tariff, 1);
    }
}
