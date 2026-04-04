//! COSEM Server — maps MeterApp measurement data to COSEM IC objects
//!
//! This module creates and manages COSEM interface class objects (IC3 Register,
//! IC5 Demand Register, IC8 Clock, IC7 Profile Generic) backed by real
//! measurement data from the MeterApp. It handles GetRequest/GetResponse
//! by looking up objects by OBIS code and returning attribute values.
//!
//! Reference: Blue Book Part 2 Ed.16

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use dlms_core::{
    errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType, units::Unit,
};

use crate::meter_app::MeterApp;

// Re-export from dlms-cosem
pub use dlms_cosem::data_register::ic3_register::Register;
pub use dlms_cosem::data_register::ic5_demand_register::DemandRegister;
pub use dlms_cosem::data_register::ic7_profile_generic::{
    CaptureObject, ProfileGeneric, SortMethod,
};
pub use dlms_cosem::time_control::ic8_clock::Clock;

/// A COSEM object identifier: (class_id, obis_code)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CosemObjectId {
    pub class_id: u16,
    pub obis: ObisCode,
}

/// Enum holding any supported COSEM object
pub enum CosemObject {
    Register(Register),
    DemandRegister(DemandRegister),
    Clock(Clock),
    ProfileGeneric(ProfileGeneric),
}

impl CosemObject {
    pub fn class_id(&self) -> u16 {
        match self {
            Self::Register(_) => 3,
            Self::DemandRegister(_) => 5,
            Self::Clock(_) => 8,
            Self::ProfileGeneric(_) => 7,
        }
    }

    pub fn logical_name(&self) -> &ObisCode {
        match self {
            Self::Register(r) => r.logical_name(),
            Self::DemandRegister(r) => r.logical_name(),
            Self::Clock(c) => c.logical_name(),
            Self::ProfileGeneric(p) => p.logical_name(),
        }
    }

    pub fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match self {
            Self::Register(r) => r.get_attribute(id),
            Self::DemandRegister(r) => r.get_attribute(id),
            Self::Clock(c) => c.get_attribute(id),
            Self::ProfileGeneric(p) => p.get_attribute(id),
        }
    }

    pub fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match self {
            Self::Register(r) => r.set_attribute(id, value),
            Self::DemandRegister(r) => r.set_attribute(id, value),
            Self::Clock(c) => c.set_attribute(id, value),
            Self::ProfileGeneric(p) => p.set_attribute(id, value),
        }
    }

    pub fn execute_method(&mut self, id: u8, params: DlmsType) -> Result<DlmsType, CosemError> {
        match self {
            Self::Register(r) => r.execute_method(id, params),
            Self::DemandRegister(r) => r.execute_method(id, params),
            Self::Clock(c) => c.execute_method(id, params),
            Self::ProfileGeneric(p) => p.execute_method(id, params),
        }
    }
}

/// COSEM Server that manages all interface class instances
pub struct CosemServer {
    /// All registered COSEM objects, keyed by (class_id, obis)
    objects: BTreeMap<CosemObjectId, CosemObject>,
}

impl CosemServer {
    /// Create a new COSEM server
    pub fn new() -> Self {
        Self {
            objects: BTreeMap::new(),
        }
    }

    /// Register a COSEM object
    pub fn register_object(&mut self, obj: CosemObject) {
        let id = CosemObjectId {
            class_id: obj.class_id(),
            obis: *obj.logical_name(),
        };
        self.objects.insert(id, obj);
    }

    /// Look up an object by class_id and OBIS code
    pub fn get_object(&self, class_id: u16, obis: &ObisCode) -> Option<&CosemObject> {
        let id = CosemObjectId {
            class_id,
            obis: *obis,
        };
        self.objects.get(&id)
    }

    /// Look up a mutable object by class_id and OBIS code
    pub fn get_object_mut(&mut self, class_id: u16, obis: &ObisCode) -> Option<&mut CosemObject> {
        let id = CosemObjectId {
            class_id,
            obis: *obis,
        };
        self.objects.get_mut(&id)
    }

    /// Get all registered objects
    pub fn objects(&self) -> &BTreeMap<CosemObjectId, CosemObject> {
        &self.objects
    }

    /// Handle a GetRequest: read attribute from a COSEM object
    pub fn get_attribute(
        &self,
        class_id: u16,
        obis: &ObisCode,
        attribute_id: u8,
    ) -> Result<DlmsType, CosemError> {
        let obj = self
            .get_object(class_id, obis)
            .ok_or(CosemError::ObjectNotFound)?;
        obj.get_attribute(attribute_id)
    }

    /// Handle a SetRequest: write attribute to a COSEM object
    pub fn set_attribute(
        &mut self,
        class_id: u16,
        obis: &ObisCode,
        attribute_id: u8,
        value: DlmsType,
    ) -> Result<(), CosemError> {
        let obj = self
            .get_object_mut(class_id, obis)
            .ok_or(CosemError::ObjectNotFound)?;
        obj.set_attribute(attribute_id, value)
    }

    /// Handle an ActionRequest: execute method on a COSEM object
    pub fn execute_method(
        &mut self,
        class_id: u16,
        obis: &ObisCode,
        method_id: u8,
        params: DlmsType,
    ) -> Result<DlmsType, CosemError> {
        let obj = self
            .get_object_mut(class_id, obis)
            .ok_or(CosemError::ObjectNotFound)?;
        obj.execute_method(method_id, params)
    }

    /// Initialize standard meter COSEM objects
    ///
    /// Creates the typical set of objects for a GB/T 17215.301 compliant
    /// smart meter:
    /// - IC8 Clock (0.0.1.0.0.255)
    /// - IC3 Registers for voltage, current, power, energy
    /// - IC5 Demand Register (1.0.1.6.0.255)
    /// - IC7 Profile Generic (1.0.99.1.0.255)
    pub fn init_standard_objects(&mut self) {
        // IC8 Clock
        self.register_object(CosemObject::Clock(Clock::new(
            ObisCode::new(0, 0, 1, 0, 0, 255),
            480, // UTC+8 (Beijing time)
        )));

        // IC3 Register — Voltage L1
        self.register_object(CosemObject::Register(Register::new(
            ObisCode::new(1, 0, 32, 7, 0, 255),
            -1,
            Unit::Volt,
        )));

        // IC3 Register — Voltage L2
        self.register_object(CosemObject::Register(Register::new(
            ObisCode::new(1, 0, 52, 7, 0, 255),
            -1,
            Unit::Volt,
        )));

        // IC3 Register — Voltage L3
        self.register_object(CosemObject::Register(Register::new(
            ObisCode::new(1, 0, 72, 7, 0, 255),
            -1,
            Unit::Volt,
        )));

        // IC3 Register — Current L1
        self.register_object(CosemObject::Register(Register::new(
            ObisCode::new(1, 0, 31, 7, 0, 255),
            -3,
            Unit::Ampere,
        )));

        // IC3 Register — Active power (W)
        self.register_object(CosemObject::Register(Register::new(
            ObisCode::new(1, 0, 1, 7, 0, 255),
            0,
            Unit::Watt,
        )));

        // IC3 Register — Total active energy import (Wh)
        self.register_object(CosemObject::Register(Register::new(
            ObisCode::new(1, 0, 1, 8, 0, 255),
            -3,
            Unit::WattHour,
        )));

        // IC3 Register — Total active energy export
        self.register_object(CosemObject::Register(Register::new(
            ObisCode::new(1, 0, 2, 8, 0, 255),
            -3,
            Unit::WattHour,
        )));

        // IC5 Demand Register — Active power demand (W)
        self.register_object(CosemObject::DemandRegister(DemandRegister::new(
            ObisCode::new(1, 0, 1, 6, 0, 255),
            0,
            Unit::Watt,
            900, // 15-minute period
        )));

        // IC7 Profile Generic — Load profile
        self.register_object(CosemObject::ProfileGeneric(ProfileGeneric::new(
            ObisCode::new(1, 0, 99, 1, 0, 255),
            alloc::vec![
                CaptureObject {
                    obis: ObisCode::new(0, 0, 1, 0, 0, 255),
                    attribute_id: 2,
                    data_index: 0,
                },
                CaptureObject {
                    obis: ObisCode::new(1, 0, 1, 8, 0, 255),
                    attribute_id: 2,
                    data_index: 0,
                },
                CaptureObject {
                    obis: ObisCode::new(1, 0, 1, 7, 0, 255),
                    attribute_id: 2,
                    data_index: 0,
                },
            ],
            900,
            SortMethod::Fifo,
            96,
        )));
    }

    /// Update all COSEM objects with current measurement data from MeterApp
    pub fn sync_from_meter_app(&mut self, app: &MeterApp) {
        let data = app.read_meter_data();

        // Update Clock (IC8, attr 2 = time)
        if let Some(obj) = self.get_object_mut(8, &ObisCode::new(0, 0, 1, 0, 0, 255)) {
            let _ = obj.set_attribute(2, DlmsType::DateTime(data.current_time));
        }

        // Update Voltage registers
        for (phase, obis_bytes) in [
            (0, [1, 0, 32, 7, 0, 255]),
            (1, [1, 0, 52, 7, 0, 255]),
            (2, [1, 0, 72, 7, 0, 255]),
        ] {
            if let Some(v) = app.measurement.voltage(phase) {
                if let Some(obj) = self.get_object_mut(3, &ObisCode::from_bytes(&obis_bytes)) {
                    let _ = obj.set_attribute(2, DlmsType::UInt32(v as u32));
                }
            }
        }

        // Update Current L1
        if let Some(i) = app.measurement.current(0) {
            if let Some(obj) = self.get_object_mut(3, &ObisCode::new(1, 0, 31, 7, 0, 255)) {
                let _ = obj.set_attribute(2, DlmsType::UInt32(i));
            }
        }

        // Update Active Power
        if let Some(obj) = self.get_object_mut(3, &ObisCode::new(1, 0, 1, 7, 0, 255)) {
            let _ = obj.set_attribute(
                2,
                DlmsType::Int32(app.measurement.instant_power(0).unwrap_or(0)),
            );
        }

        // Update Energy registers
        if let Some(obj) = self.get_object_mut(3, &ObisCode::new(1, 0, 1, 8, 0, 255)) {
            let _ = obj.set_attribute(2, app.measurement.energy_to_dlms(data.energy_import));
        }

        if let Some(obj) = self.get_object_mut(3, &ObisCode::new(1, 0, 2, 8, 0, 255)) {
            let _ = obj.set_attribute(2, app.measurement.energy_to_dlms(data.energy_export));
        }

        // Update Demand Register
        if let Some(obj) = self.get_object_mut(5, &ObisCode::new(1, 0, 1, 6, 0, 255)) {
            let _ = obj.set_attribute(2, DlmsType::Int32(data.current_demand));
            let _ = obj.set_attribute(6, DlmsType::DateTime(data.current_time));
        }
    }

    /// List all registered objects as (class_id, obis) tuples
    pub fn object_list(&self) -> Vec<(u16, ObisCode)> {
        self.objects
            .iter()
            .map(|(id, _)| (id.class_id, id.obis))
            .collect()
    }
}

impl Default for CosemServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosem_server_new() {
        let server = CosemServer::new();
        assert_eq!(server.objects().len(), 0);
    }

    #[test]
    fn test_init_standard_objects() {
        let mut server = CosemServer::new();
        server.init_standard_objects();
        assert!(server.objects().len() > 8);

        // Verify Clock object
        let clock = server.get_object(8, &ObisCode::new(0, 0, 1, 0, 0, 255));
        assert!(clock.is_some());
        assert_eq!(clock.unwrap().class_id(), 8);

        // Verify energy register
        let energy = server.get_object(3, &ObisCode::new(1, 0, 1, 8, 0, 255));
        assert!(energy.is_some());
    }

    #[test]
    fn test_get_attribute() {
        let mut server = CosemServer::new();
        server.init_standard_objects();

        // Read Clock logical_name (attr 1)
        let result = server.get_attribute(8, &ObisCode::new(0, 0, 1, 0, 0, 255), 1);
        assert!(result.is_ok());

        // Read non-existent object
        let result = server.get_attribute(99, &ObisCode::new(9, 9, 9, 9, 9, 9), 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_attribute() {
        let mut server = CosemServer::new();
        server.init_standard_objects();

        // Set energy value (attr 2)
        let result = server.set_attribute(
            3,
            &ObisCode::new(1, 0, 1, 8, 0, 255),
            2,
            DlmsType::Int64(12345),
        );
        assert!(result.is_ok());

        // Verify it was set
        let val = server
            .get_attribute(3, &ObisCode::new(1, 0, 1, 8, 0, 255), 2)
            .unwrap();
        assert_eq!(val, DlmsType::Int64(12345));
    }

    #[test]
    fn test_execute_method() {
        let mut server = CosemServer::new();
        server.init_standard_objects();

        // Set demand value first
        server
            .set_attribute(
                5,
                &ObisCode::new(1, 0, 1, 6, 0, 255),
                2,
                DlmsType::Int32(1000),
            )
            .unwrap();

        // Reset demand register (method 1)
        let result =
            server.execute_method(5, &ObisCode::new(1, 0, 1, 6, 0, 255), 1, DlmsType::Null);
        assert!(result.is_ok());

        // Verify reset cleared the value
        let val = server
            .get_attribute(5, &ObisCode::new(1, 0, 1, 6, 0, 255), 2)
            .unwrap();
        assert_eq!(val, DlmsType::Null);
    }

    #[test]
    fn test_object_list() {
        let mut server = CosemServer::new();
        server.init_standard_objects();
        let list = server.object_list();
        assert!(list.len() > 8);
    }

    #[test]
    fn test_sync_from_meter_app() {
        let mut server = CosemServer::new();
        server.init_standard_objects();
        let app = MeterApp::new();
        server.sync_from_meter_app(&app);

        // Verify energy register was updated
        let energy = server.get_attribute(3, &ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        assert!(energy.is_ok());
    }

    #[test]
    fn test_nonexistent_object() {
        let server = CosemServer::new();
        let result = server.get_attribute(3, &ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        assert!(matches!(result, Err(CosemError::ObjectNotFound)));
    }

    #[test]
    fn test_cosem_object_enum_dispatch() {
        let obj = CosemObject::Register(Register::new(
            ObisCode::new(1, 0, 1, 8, 0, 255),
            -3,
            Unit::WattHour,
        ));
        assert_eq!(obj.class_id(), 3);
        assert!(obj.get_attribute(1).is_ok()); // logical_name
        assert!(obj.get_attribute(3).is_ok()); // scaler_unit
        assert!(obj.get_attribute(99).is_err()); // no such attribute
    }
}
