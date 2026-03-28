//! Meter simulator for testing and development
//!
//! This module provides a complete DLMS/COSEM smart meter simulator
//! with all interface classes and COSEM objects.

use dlms_core::{
    errors::CosemError,
    obis::{ObisCode, CLOCK, TOTAL_ACTIVE_ENERGY_IMPORT, VOLTAGE_L1, CURRENT_L1, ACTIVE_POWER},
    traits::CosemClass,
    types::DlmsType,
};
use dlms_cosem::{
    data_register::ic3_register::Register,
    time_control::ic8_clock::Clock,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Meter simulator with all COSEM objects
pub struct MeterSimulator {
    /// All COSEM objects indexed by OBIS code
    objects: HashMap<Vec<u8>, Arc<Mutex<dyn CosemObject>>>,
    /// Meter identifier
    meter_id: String,
    /// Firmware version
    firmware_version: String,
    /// Is running flag
    running: bool,
}

/// Trait for COSEM objects in the simulator
pub trait CosemObject: Send + Sync {
    /// Get object logical name
    fn logical_name(&self) -> &ObisCode;

    /// Get an attribute value
    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError>;

    /// Set an attribute value
    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError>;

    /// Execute a method
    fn execute_method(&mut self, id: u8, params: DlmsType) -> Result<DlmsType, CosemError>;
}

/// Wrapper for Register objects
#[derive(Debug, Clone)]
struct RegisterWrapper {
    inner: Register,
}

impl CosemObject for RegisterWrapper {
    fn logical_name(&self) -> &ObisCode {
        self.inner.logical_name()
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        self.inner.get_attribute(id)
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        self.inner.set_attribute(id, value)
    }

    fn execute_method(&mut self, _id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        Err(CosemError::NotImplemented)
    }
}

/// Wrapper for Clock objects
#[derive(Debug, Clone)]
struct ClockWrapper {
    inner: Clock,
}

impl CosemObject for ClockWrapper {
    fn logical_name(&self) -> &ObisCode {
        self.inner.logical_name()
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        self.inner.get_attribute(id)
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        self.inner.set_attribute(id, value)
    }

    fn execute_method(&mut self, _id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        Err(CosemError::NotImplemented)
    }
}

impl MeterSimulator {
    /// Create a new meter simulator
    pub fn new(meter_id: String) -> Self {
        let mut simulator = Self {
            objects: HashMap::new(),
            meter_id,
            firmware_version: "1.0.0".to_string(),
            running: false,
        };

        simulator.init_objects();
        simulator
    }

    /// Initialize all standard COSEM objects
    fn init_objects(&mut self) {
        // Clock object (0.0.1.0.0.255)
        let clock = Clock::new(CLOCK, 0); // deviation = 0
        let clock_wrapper = Arc::new(Mutex::new(ClockWrapper { inner: clock }));
        self.objects.insert(CLOCK.to_bytes().to_vec(), clock_wrapper);

        // Total active energy import (1.0.1.8.0.255)
        let mut energy = Register::new(
            TOTAL_ACTIVE_ENERGY_IMPORT,
            -3, // scaler: 10^-3 (kWh)
            dlms_core::units::Unit::WattHour,
        );
        energy.set_attribute(2, DlmsType::from_i64(0)).ok();
        let energy_wrapper = Arc::new(Mutex::new(RegisterWrapper { inner: energy }));
        self.objects.insert(TOTAL_ACTIVE_ENERGY_IMPORT.to_bytes().to_vec(), energy_wrapper);

        // Voltage L1 (1.0.32.7.0.255)
        let mut voltage = Register::new(
            VOLTAGE_L1,
            -1, // scaler: 0.1 V
            dlms_core::units::Unit::Volt,
        );
        voltage.set_attribute(2, DlmsType::from_i32(2300)).ok(); // 230.0 V
        let voltage_wrapper = Arc::new(Mutex::new(RegisterWrapper { inner: voltage }));
        self.objects.insert(VOLTAGE_L1.to_bytes().to_vec(), voltage_wrapper);

        // Current L1 (1.0.31.7.0.255)
        let mut current = Register::new(
            CURRENT_L1,
            -3, // scaler: mA
            dlms_core::units::Unit::Ampere,
        );
        current.set_attribute(2, DlmsType::from_i64(0)).ok();
        let current_wrapper = Arc::new(Mutex::new(RegisterWrapper { inner: current }));
        self.objects.insert(CURRENT_L1.to_bytes().to_vec(), current_wrapper);

        // Active power (1.0.1.7.0.255)
        let mut power = Register::new(
            ACTIVE_POWER,
            0, // scaler: W
            dlms_core::units::Unit::Watt,
        );
        power.set_attribute(2, DlmsType::from_i32(0)).ok();
        let power_wrapper = Arc::new(Mutex::new(RegisterWrapper { inner: power }));
        self.objects.insert(ACTIVE_POWER.to_bytes().to_vec(), power_wrapper);
    }

    /// Get the meter ID
    pub fn meter_id(&self) -> &str {
        &self.meter_id
    }

    /// Get the firmware version
    pub fn firmware_version(&self) -> &str {
        &self.firmware_version
    }

    /// Get all object OBIS codes
    pub fn object_list(&self) -> Vec<ObisCode> {
        let mut result = Vec::new();
        for bytes in self.objects.keys() {
            if bytes.len() == 6 {
                let arr: [u8; 6] = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]];
                let obis = ObisCode::from_bytes(&arr);
                result.push(obis);
            }
        }
        result
    }

    /// Get an object by OBIS code
    pub fn get_object(&self, obis: &ObisCode) -> Option<Arc<Mutex<dyn CosemObject>>> {
        let key = obis.to_bytes().to_vec();
        self.objects.get(&key).cloned()
    }

    /// Read an attribute from an object
    pub fn read_attribute(&self, obis: &ObisCode, attr_id: u8) -> Result<DlmsType, CosemError> {
        if let Some(obj) = self.get_object(obis) {
            obj.lock().unwrap().get_attribute(attr_id)
        } else {
            Err(CosemError::ObjectNotFound)
        }
    }

    /// Write an attribute to an object
    pub fn write_attribute(&self, obis: &ObisCode, attr_id: u8, value: DlmsType) -> Result<(), CosemError> {
        if let Some(obj) = self.get_object(obis) {
            obj.lock().unwrap().set_attribute(attr_id, value)
        } else {
            Err(CosemError::ObjectNotFound)
        }
    }

    /// Simulate power consumption
    pub fn simulate_load(&self, power_w: i32) {
        if let Some(obj) = self.get_object(&ACTIVE_POWER) {
            let _ = obj.lock().unwrap().set_attribute(2, DlmsType::from_i32(power_w));
        }
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
}

impl Default for MeterSimulator {
    fn default() -> Self {
        Self::new("SIM-METER-001".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_new() {
        let sim = MeterSimulator::new("TEST-001".to_string());
        assert_eq!(sim.meter_id(), "TEST-001");
        assert!(!sim.is_running());
    }

    #[test]
    fn test_object_list() {
        let sim = MeterSimulator::new("TEST-001".to_string());
        let list = sim.object_list();
        assert!(list.len() >= 5); // At least 5 standard objects
    }

    #[test]
    fn test_read_clock() {
        let sim = MeterSimulator::new("TEST-001".to_string());
        let result = sim.read_attribute(&CLOCK, 2); // value
        assert!(result.is_ok());
    }

    #[test]
    fn test_simulate_load() {
        let sim = MeterSimulator::new("TEST-001".to_string());
        sim.simulate_load(1500);

        let result = sim.read_attribute(&ACTIVE_POWER, 2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), DlmsType::from_i32(1500));
    }
}
