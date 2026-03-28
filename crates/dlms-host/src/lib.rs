//! DLMS/COSEM Host-side Tools
//!
//! Provides CLI, simulator, sniffer, and test runner for DLMS/COSEM development.
//! This crate requires std.
//!
//! # Re-exports
//!
//! This crate re-exports all types from `dlms_meter_app` for convenience.

pub mod cli;
pub mod simulator;
pub mod sniffer;
pub mod test_runner;

// Re-export CLI types
pub use cli::{Cli, Commands};

// Re-export simulator types
pub use simulator::{MeterSimulator, SimulatorApp};

// Re-export sniffer types
pub use sniffer::{ProtocolSniffer, CapturedFrame, Direction, DecodedFrame, FrameType};

// Re-export test runner types
pub use test_runner::{TestRunner, IntegrationTest, TestResult, TestSummary,
                     list_serial_ports, find_available_port};

// Re-export everything from dlms_meter_app
pub use dlms_meter_app::{
    // Main types
    MeterApp,

    // Common types
    AlarmThreshold, AlarmType, BillingPeriod, BillingStatus, DemandConfig,
    DisplayEntry, DisplayFormat, PhaseEnergy, PowerQuality, TariffSchedule,

    // Measurement
    MeasurementEngine, Phase, MAX_TARIFFS,

    // Tariff
    DayOfWeek, Season, TariffManager,

    // Profile
    ProfileColumn, ProfileEntry, ProfileManager, HistoricalManager, HistoricalEntry,

    // Alarm
    AlarmManager, AlarmRecord, AlarmState, AlarmCallback,

    // Control
    RelayControl, RelayState, ControlMode, OutputState,

    // Firmware
    FirmwareManager, ImageInfo, TransferState, TransferStats,

    // Clock
    ClockManager, Timezone, DstMode, SyncStatus, ClockStats,

    // Communication
    CommManager, PortType, ConnectionState, PushConfig, PushTrigger, CommStats,
};

/// Builder for creating configured MeterApp instances
///
/// This builder provides a fluent API for configuring a meter application
/// before creating it.
///
/// # Example
///
/// ```rust
/// use dlms_host::{MeterAppBuilder, DemandConfig};
///
/// let meter = MeterAppBuilder::new()
///     .with_demand(DemandConfig::default_15min())
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct MeterAppBuilder {
    demand_config: Option<DemandConfig>,
    meter_id: Option<String>,
}

impl MeterAppBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set demand configuration
    pub fn with_demand(mut self, config: DemandConfig) -> Self {
        self.demand_config = Some(config);
        self
    }

    /// Set meter ID (for simulator identification)
    pub fn with_meter_id(mut self, id: String) -> Self {
        self.meter_id = Some(id);
        self
    }

    /// Build the MeterApp
    pub fn build(self) -> MeterApp {
        if let Some(config) = self.demand_config {
            MeterApp::with_demand_config(config)
        } else {
            MeterApp::new()
        }
    }

    /// Build a SimulatorApp with this configuration
    pub fn build_simulator(self) -> SimulatorApp {
        if let Some(config) = self.demand_config {
            SimulatorApp::with_demand_config(config)
        } else {
            SimulatorApp::new()
        }
    }
}
