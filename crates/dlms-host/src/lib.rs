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
pub use sniffer::{CapturedFrame, DecodedFrame, Direction, FrameType, ProtocolSniffer};

// Re-export test runner types
pub use test_runner::{
    find_available_port, list_serial_ports, IntegrationTest, TestResult, TestRunner, TestSummary,
};

// Re-export everything from dlms_meter_app
pub use dlms_meter_app::{
    AlarmCallback,

    // Alarm
    AlarmManager,
    AlarmRecord,
    AlarmState,
    // Common types
    AlarmThreshold,
    AlarmType,
    BillingPeriod,
    BillingStatus,
    // Clock
    ClockManager,
    ClockStats,

    // Communication
    CommManager,
    CommStats,
    ConnectionState,
    ControlMode,
    // Tariff
    DayOfWeek,
    DemandConfig,
    DisplayEntry,
    DisplayFormat,
    DstMode,
    // Firmware
    FirmwareManager,
    HistoricalEntry,

    HistoricalManager,
    ImageInfo,
    // Measurement
    MeasurementEngine,
    // Main types
    MeterApp,

    OutputState,

    Phase,
    PhaseEnergy,
    PortType,
    PowerQuality,
    // Profile
    ProfileColumn,
    ProfileEntry,
    ProfileManager,
    PushConfig,
    PushTrigger,
    // Control
    RelayControl,
    RelayState,
    Season,
    SyncStatus,
    TariffManager,

    TariffSchedule,

    Timezone,
    TransferState,
    TransferStats,

    MAX_TARIFFS,
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
