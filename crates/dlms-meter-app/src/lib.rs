//!
//! DLMS/COSEM Smart Meter Application Layer
//!
//! This crate provides the application-layer functionality for a DLMS/COSEM smart meter,
//! including measurement, tariff management, load profiling, alarms, relay control,
//! firmware updates, clock management, and communication.
//!
//! # Features
//!
//! - **no_std** compatible with `alloc` support
//! - Comprehensive measurement engine with multi-tariff support
//! - Time-of-use tariff management
//! - Load profile capture
//! - Threshold-based alarm monitoring
//! - Relay control for load disconnection
//! - Firmware image transfer
//! - RTC synchronization with DST support
//! - HDLC communication management
//!
//! # Usage
//!
//! ```rust,no_run
//! use dlms_meter_app::{MeterApp, common::DemandConfig};
//!
//! // Create meter application with default configuration
//! let mut meter = MeterApp::new();
//!
//! // Process measurements
//! meter.process_power(1000, 0, 60);
//!
//! // Update clock
//! meter.tick(60);
//! ```
//!

#![no_std]

extern crate alloc;

// Public modules
pub mod common;
pub mod measurement;
pub mod tariff;
pub mod profile;
pub mod alarm;
pub mod control;
pub mod firmware;
pub mod clock;
pub mod comm;

// Re-exports
pub use common::{
    AlarmThreshold, AlarmType, BillingPeriod, BillingStatus, DemandConfig, DisplayEntry,
    DisplayFormat, PhaseEnergy, PowerQuality, TariffSchedule,
};
pub use measurement::{MeasurementEngine, Phase, MAX_TARIFFS};
pub use tariff::{DayOfWeek, Season, TariffManager};
pub use profile::{ProfileColumn, ProfileEntry, ProfileManager, HistoricalManager, HistoricalEntry};
pub use alarm::{AlarmManager, AlarmRecord, AlarmState, AlarmCallback};
pub use control::{RelayControl, RelayState, ControlMode, OutputState};
pub use firmware::{FirmwareManager, ImageInfo, TransferState, TransferStats};
pub use clock::{ClockManager, Timezone, DstMode, SyncStatus, ClockStats};
pub use comm::{CommManager, PortType, ConnectionState, PushConfig, PushTrigger, CommStats};

use dlms_core::{errors::CosemError, types::CosemDateTime};

/// Main meter application structure
///
/// Combines all subsystems into a single cohesive application.
#[derive(Debug)]
pub struct MeterApp {
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
}

impl MeterApp {
    /// Create a new meter application with default configuration
    pub fn new() -> Self {
        Self {
            measurement: MeasurementEngine::new(),
            tariff: TariffManager::new(),
            profile: ProfileManager::new(96, 900), // 96 entries, 15-minute interval
            alarm: AlarmManager::new(),
            control: RelayControl::new(),
            firmware: FirmwareManager::new(b"1.0.0".to_vec()),
            clock: ClockManager::new(),
            comm: CommManager::new(comm::PortType::Rs485),
            uptime: 0,
        }
    }

    /// Create meter with specific demand configuration
    pub fn with_demand_config(config: DemandConfig) -> Self {
        Self {
            measurement: MeasurementEngine::with_config(config),
            ..Self::new()
        }
    }

    /// Get current uptime
    pub fn uptime(&self) -> u32 {
        self.uptime
    }

    /// Process power measurement
    pub fn process_power(&mut self, power_w: i32, tariff: usize, interval_s: u32) {
        // Get current tariff if not specified
        let effective_tariff = if tariff == 0 {
            self.clock.current_time();
            self.tariff.current_tariff() as usize - 1
        } else {
            tariff - 1
        };

        self.measurement.process_power(power_w, effective_tariff, interval_s);

        // Update alarm with new value
        let total_energy = self.measurement.total_energy_import();
        self.alarm
            .update_value(dlms_core::obis::TOTAL_ACTIVE_ENERGY_IMPORT, total_energy, self.uptime);
    }

    /// Process 3-phase power measurement
    pub fn process_3phase_power(&mut self, l1_w: i32, l2_w: i32, l3_w: i32, tariff: usize, interval_s: u32) {
        self.measurement.process_3phase_power(l1_w, l2_w, l3_w, tariff, interval_s);
    }

    /// Update voltage measurement
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

    /// Advance time (tick)
    pub fn tick(&mut self, elapsed_seconds: u32) {
        self.uptime = self.uptime.saturating_add(elapsed_seconds);
        self.clock.tick(elapsed_seconds);

        // Update tariff based on time
        if let Some(dt) = CosemDateTime::from_clock(&self.clock) {
            self.tariff.update_tariff_for_time(&dt);
        }

        // Check if sync is needed
        if self.clock.needs_sync(self.uptime) {
            // Would trigger sync here
        }

        // Check if push is needed
        if self.comm.needs_push(self.uptime) {
            // Would trigger push here
        }
    }

    /// Set current time
    pub fn set_time(&mut self, dt: CosemDateTime) -> Result<(), CosemError> {
        self.clock.set_time(dt)
    }

    /// Connect communication
    pub fn connect(&mut self) -> Result<(), CosemError> {
        self.comm.connect()
    }

    /// Disconnect communication
    pub fn disconnect(&mut self) -> Result<(), CosemError> {
        self.comm.disconnect()
    }

    /// Disconnect load (relay control)
    pub fn disconnect_load(&mut self) -> Result<(), CosemError> {
        self.control.disconnect(self.uptime)
    }

    /// Reconnect load
    pub fn reconnect_load(&mut self) -> Result<(), CosemError> {
        self.control.reconnect(self.uptime)
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

// Helper for CosemDateTime conversion
trait ClockExt {
    fn from_clock(clock: &ClockManager) -> Option<CosemDateTime>;
}

impl ClockExt for CosemDateTime {
    fn from_clock(_clock: &ClockManager) -> Option<Self> {
        // In real implementation, would convert from clock
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_app_new() {
        let app = MeterApp::new();
        assert_eq!(app.uptime(), 0);
        assert_eq!(app.measurement.total_energy_import(), 0);
    }

    #[test]
    fn test_process_power() {
        let mut app = MeterApp::new();
        app.process_power(1000, 1, 3600);
        assert_eq!(app.measurement.total_energy_import(), 1000);
    }

    #[test]
    fn test_tick() {
        let mut app = MeterApp::new();
        app.tick(60);
        assert_eq!(app.uptime(), 60);

        app.tick(60);
        assert_eq!(app.uptime(), 120);
    }

    #[test]
    fn test_connect_disconnect() {
        let mut app = MeterApp::new();
        assert!(app.connect().is_ok());
        assert!(app.comm.is_connected());

        assert!(app.disconnect().is_ok());
        assert!(!app.comm.is_connected());
    }

    #[test]
    fn test_load_disconnect() {
        let mut app = MeterApp::new();
        app.tick(10); // Advance time to pass minimum interval
        assert!(app.disconnect_load().is_ok());
        assert!(!app.control.is_connected());
    }

    #[test]
    fn test_emergency_disconnect() {
        let mut app = MeterApp::new();
        app.tick(10); // Advance time to pass minimum interval
        assert!(app.emergency_disconnect().is_ok());
        assert!(app.control.is_emergency_active());
    }
}
