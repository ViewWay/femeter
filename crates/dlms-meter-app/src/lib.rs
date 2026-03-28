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
pub mod meter_app;

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
pub use meter_app::{MeterApp, MeterAppState, MeterConfig, MeterData, BillingData, AlarmEvent};
