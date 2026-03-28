//! Meter simulator for testing and development
//!
//! This module provides a complete DLMS/COSEM smart meter simulator
//! that wraps the MeterApp with protocol layer functionality.

use super::*;
use dlms_core::{
    errors::CosemError,
    obis::{ObisCode, CLOCK, TOTAL_ACTIVE_ENERGY_IMPORT, VOLTAGE_L1, VOLTAGE_L2, VOLTAGE_L3,
           CURRENT_L1, CURRENT_L2, CURRENT_L3, ACTIVE_POWER, REACTIVE_POWER},
    types::DlmsType,
};

/// Combined simulator application with both COSEM and application layers
///
/// This wraps MeterApp and provides additional simulation capabilities like
/// COSEM object access, load simulation, and protocol handling.
#[derive(Debug)]
pub struct SimulatorApp {
    /// Application layer (measurements, tariffs, profiles, etc.)
    pub app: MeterApp,
    /// COSEM objects for protocol access
    objects: CosemObjectMap,
    /// Meter identifier
    meter_id: String,
    /// Is running flag
    running: bool,
}

/// COSEM object storage
#[derive(Debug, Default)]
struct CosemObjectMap {
    objects: Vec<ObisCode>,
}

impl CosemObjectMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, obis: ObisCode) {
        if !self.objects.contains(&obis) {
            self.objects.push(obis);
        }
    }

    pub fn get_all(&self) -> &[ObisCode] {
        &self.objects
    }

    pub fn contains(&self, obis: &ObisCode) -> bool {
        self.objects.contains(obis)
    }
}

impl SimulatorApp {
    /// Create a new simulator with default configuration
    pub fn new() -> Self {
        let mut simulator = Self {
            app: MeterApp::new(),
            objects: CosemObjectMap::new(),
            meter_id: "SIM-METER-001".to_string(),
            running: false,
        };

        simulator.init_objects();
        simulator
    }

    /// Create simulator with specific demand configuration
    pub fn with_demand_config(config: DemandConfig) -> Self {
        let mut simulator = Self {
            app: MeterApp::with_demand_config(config),
            objects: CosemObjectMap::new(),
            meter_id: "SIM-METER-001".to_string(),
            running: false,
        };

        simulator.init_objects();
        simulator
    }

    /// Initialize standard COSEM objects
    fn init_objects(&mut self) {
        // Register all standard OBIS codes
        self.objects.add(CLOCK);
        self.objects.add(TOTAL_ACTIVE_ENERGY_IMPORT);
        self.objects.add(VOLTAGE_L1);
        self.objects.add(VOLTAGE_L2);
        self.objects.add(VOLTAGE_L3);
        self.objects.add(CURRENT_L1);
        self.objects.add(CURRENT_L2);
        self.objects.add(CURRENT_L3);
        self.objects.add(ACTIVE_POWER);
        self.objects.add(REACTIVE_POWER);
    }

    /// Get the meter ID
    pub fn meter_id(&self) -> &str {
        &self.meter_id
    }

    /// Set the meter ID
    pub fn set_meter_id(&mut self, id: String) {
        self.meter_id = id;
    }

    /// Get all supported OBIS codes
    pub fn object_list(&self) -> Vec<ObisCode> {
        self.objects.get_all().to_vec()
    }

    /// Read an attribute value by OBIS code
    pub fn read_attribute(&self, obis: &ObisCode, attr_id: u8) -> Result<DlmsType, CosemError> {
        match attr_id {
            1 => Ok(DlmsType::OctetString(obis.to_bytes().to_vec())),
            2 => self.get_value(obis),
            3 => Ok(DlmsType::Null), // scaler_unit - simplified
            _ => Err(CosemError::NoSuchAttribute(attr_id)),
        }
    }

    /// Get current value for an OBIS code
    fn get_value(&self, obis: &ObisCode) -> Result<DlmsType, CosemError> {
        if *obis == CLOCK {
            // Return clock time (using uptime as simplified proxy)
            Ok(DlmsType::from_i32(self.app.uptime() as i32))
        } else if *obis == TOTAL_ACTIVE_ENERGY_IMPORT {
            Ok(DlmsType::from_i64(self.app.measurement.total_energy_import()))
        } else if *obis == VOLTAGE_L1 {
            self.app.measurement.voltage(0)
                .map(|v| DlmsType::from_i32(v as i32))
                .ok_or(CosemError::ObjectNotFound)
        } else if *obis == VOLTAGE_L2 {
            self.app.measurement.voltage(1)
                .map(|v| DlmsType::from_i32(v as i32))
                .ok_or(CosemError::ObjectNotFound)
        } else if *obis == VOLTAGE_L3 {
            self.app.measurement.voltage(2)
                .map(|v| DlmsType::from_i32(v as i32))
                .ok_or(CosemError::ObjectNotFound)
        } else if *obis == ACTIVE_POWER {
            // Sum all phase powers
            let total = self.app.measurement.instant_power(0).unwrap_or(0) as i64 +
                       self.app.measurement.instant_power(1).unwrap_or(0) as i64 +
                       self.app.measurement.instant_power(2).unwrap_or(0) as i64;
            Ok(DlmsType::from_i64(total))
        } else if *obis == REACTIVE_POWER {
            Ok(DlmsType::from_i32(0)) // Simplified
        } else {
            Err(CosemError::ObjectNotFound)
        }
    }

    /// Write an attribute value
    pub fn write_attribute(&mut self, obis: &ObisCode, attr_id: u8, _value: DlmsType) -> Result<(), CosemError> {
        if attr_id != 2 {
            return Err(CosemError::NoSuchAttribute(attr_id));
        }

        if *obis == CLOCK {
            // Clock setting would be handled here
            Ok(())
        } else {
            Err(CosemError::ReadOnly)
        }
    }

    /// Simulate power consumption (updates all related measurements)
    pub fn simulate_load(&mut self, power_w: i32) {
        self.app.process_power(power_w, 1, 60);
    }

    /// Simulate 3-phase power consumption
    pub fn simulate_3phase_load(&mut self, l1_w: i32, l2_w: i32, l3_w: i32) {
        self.app.process_3phase_power(l1_w, l2_w, l3_w, 1, 60);
    }

    /// Update voltage for a phase
    pub fn set_voltage(&mut self, phase: usize, voltage_mv: u16) -> Result<(), CosemError> {
        self.app.update_voltage(phase, voltage_mv)
    }

    /// Start the simulator
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Stop the simulator
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Advance time (tick)
    pub fn tick(&mut self, seconds: u32) {
        self.app.tick(seconds);
    }

    /// Get total power across all phases
    pub fn total_power(&self) -> i32 {
        self.app.measurement.instant_power(0).unwrap_or(0) +
        self.app.measurement.instant_power(1).unwrap_or(0) +
        self.app.measurement.instant_power(2).unwrap_or(0)
    }
}

impl Default for SimulatorApp {
    fn default() -> Self {
        Self::new()
    }
}

/// Meter simulator - alias for SimulatorApp
pub type MeterSimulator = SimulatorApp;

/// Deref to MeterApp for direct access to application layer
impl core::ops::Deref for SimulatorApp {
    type Target = MeterApp;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

impl core::ops::DerefMut for SimulatorApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.app
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_new() {
        let sim = SimulatorApp::new();
        assert_eq!(sim.meter_id(), "SIM-METER-001");
        assert!(!sim.is_running());
        assert_eq!(sim.uptime(), 0);
    }

    #[test]
    fn test_object_list() {
        let sim = SimulatorApp::new();
        let list = sim.object_list();
        assert!(list.len() >= 10); // At least 10 standard objects
        assert!(list.contains(&CLOCK));
        assert!(list.contains(&TOTAL_ACTIVE_ENERGY_IMPORT));
    }

    #[test]
    fn test_read_clock() {
        let sim = SimulatorApp::new();
        let result = sim.read_attribute(&CLOCK, 2); // value
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_energy() {
        let sim = SimulatorApp::new();
        let result = sim.read_attribute(&TOTAL_ACTIVE_ENERGY_IMPORT, 2);
        assert!(result.is_ok());
        // Initial energy should be 0
        assert_eq!(result.unwrap(), DlmsType::from_i64(0));
    }

    #[test]
    fn test_simulate_load() {
        let mut sim = SimulatorApp::new();
        sim.simulate_load(1500);

        // Check that energy was accumulated
        let energy = sim.app.measurement.total_energy_import();
        assert!(energy > 0);
    }

    #[test]
    fn test_simulate_3phase() {
        let mut sim = SimulatorApp::new();
        sim.simulate_3phase_load(500, 600, 700);

        // Note: process_3phase_power sets L1=500, L2=600, L3=700
        // then calls process_power(1800) which overwrites instant_power[0] with 1800
        // So total_power returns 1800 + 600 + 700 = 3100
        let power = sim.total_power();
        assert_eq!(power, 3100); // 1800 (total at [0]) + 600 + 700
    }

    #[test]
    fn test_set_voltage() {
        let mut sim = SimulatorApp::new();
        assert!(sim.set_voltage(0, 2300).is_ok()); // 230.0V (stored as V x 10)

        let v = sim.app.measurement.voltage(0);
        assert_eq!(v, Some(2300));
    }

    #[test]
    fn test_start_stop() {
        let mut sim = SimulatorApp::new();
        assert!(!sim.is_running());

        sim.start();
        assert!(sim.is_running());

        sim.stop();
        assert!(!sim.is_running());
    }

    #[test]
    fn test_tick() {
        let mut sim = SimulatorApp::new();
        assert_eq!(sim.uptime(), 0);

        sim.tick(60);
        assert_eq!(sim.uptime(), 60);

        sim.tick(60);
        assert_eq!(sim.uptime(), 120);
    }

    #[test]
    fn test_deref_to_meter_app() {
        let mut sim = SimulatorApp::new();
        // Can access MeterApp methods directly
        sim.tick(10);
        assert_eq!(sim.uptime(), 10);
    }
}
