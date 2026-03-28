//!
//! Interface Class 67: Sensor Manager
//!
//! Reference: Blue Book Part 2 §5.67
//!
//! Sensor Manager manages sensor inputs and readings.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 67: Sensor Manager
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | sensor_readout | 2 | octet-string | dynamic |
/// | sensor_failure_flag | 3 | boolean | dynamic |
/// | sensor_status | 4 | unsigned | dynamic |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | connect | 1 | Connect sensor |
/// | disconnect | 2 | Disconnect sensor |
#[derive(Debug, Clone)]
pub struct SensorManager {
    logical_name: ObisCode,
    sensor_readout: DlmsType,
    sensor_failure_flag: bool,
    sensor_status: u8,
}

impl SensorManager {
    /// Create a new Sensor Manager object
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            sensor_readout: DlmsType::OctetString(alloc::vec![]),
            sensor_failure_flag: false,
            sensor_status: 0,
        }
    }

    pub const fn get_sensor_status(&self) -> u8 {
        self.sensor_status
    }
}

impl CosemClass for SensorManager {
    const CLASS_ID: u16 = 67;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        4
    }

    fn method_count() -> u8 {
        2
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.sensor_readout.clone()),
            3 => Ok(DlmsType::Boolean(self.sensor_failure_flag)),
            4 => Ok(DlmsType::UInt8(self.sensor_status)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => Err(CosemError::ReadOnly),
            3 => Err(CosemError::ReadOnly),
            4 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // connect
            2 => Ok(DlmsType::Null), // disconnect
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_manager_class_id() {
        let sm = SensorManager::new(ObisCode::new(0, 0, 67, 0, 0, 255));
        assert_eq!(SensorManager::CLASS_ID, 67);
        assert_eq!(SensorManager::method_count(), 2);
    }
}
