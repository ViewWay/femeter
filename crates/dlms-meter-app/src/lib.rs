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
pub mod alarm;
pub mod clock;
pub mod comm;
pub mod common;
pub mod control;
pub mod cosem_server;
pub mod firmware;
pub mod measurement;
pub mod meter_app;
pub mod profile;
pub mod tariff;

// Re-exports
pub use alarm::{AlarmCallback, AlarmManager, AlarmRecord, AlarmState};
pub use clock::{ClockManager, ClockStats, DstMode, SyncStatus, Timezone};
pub use comm::{CommManager, CommStats, ConnectionState, PortType, PushConfig, PushTrigger};
pub use common::{
    AlarmThreshold, AlarmType, BillingPeriod, BillingStatus, DemandConfig, DisplayEntry,
    DisplayFormat, PhaseEnergy, PowerQuality, TariffSchedule,
};
pub use control::{ControlMode, OutputState, RelayControl, RelayState};
pub use cosem_server::CosemServer;
pub use firmware::{FirmwareManager, ImageInfo, TransferState, TransferStats};
pub use measurement::{MeasurementEngine, Phase, MAX_TARIFFS};
pub use meter_app::{AlarmEvent, BillingData, MeterApp, MeterAppState, MeterConfig, MeterData};
pub use profile::{
    HistoricalEntry, HistoricalManager, ProfileColumn, ProfileEntry, ProfileManager,
};
pub use tariff::{DayOfWeek, Season, TariffManager};
