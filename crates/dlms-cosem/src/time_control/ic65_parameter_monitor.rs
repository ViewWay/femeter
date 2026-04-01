//!
//! Interface Class 65: Parameter Monitor
//!
//! Reference: Blue Book Part 2 §5.65
//!
//! Parameter Monitor monitors parameter values and triggers actions.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 65: Parameter Monitor
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | monitored_value | 2 | CHOICE | dynamic |
/// | threshold_reached | 3 | boolean | dynamic |
/// | threshold_params | 4 | structure | static |
/// | captured_value | 5 | octet-string | dynamic |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | set_threshold | 1 | Set threshold |
#[derive(Debug, Clone)]
pub struct ParameterMonitor {
    logical_name: ObisCode,
    monitored_value: DlmsType,
    threshold_reached: bool,
    threshold_params: DlmsType,
    captured_value: DlmsType,
}

impl ParameterMonitor {
    /// Create a new Parameter Monitor object
    pub fn new(logical_name: ObisCode, threshold_params: DlmsType) -> Self {
        Self {
            logical_name,
            monitored_value: DlmsType::Null,
            threshold_reached: false,
            threshold_params,
            captured_value: DlmsType::OctetString(alloc::vec![]),
        }
    }

    pub const fn is_threshold_reached(&self) -> bool {
        self.threshold_reached
    }
}

impl CosemClass for ParameterMonitor {
    const CLASS_ID: u16 = 65;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        5
    }

    fn method_count() -> u8 {
        1
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.monitored_value.clone()),
            3 => Ok(DlmsType::Boolean(self.threshold_reached)),
            4 => Ok(self.threshold_params.clone()),
            5 => Ok(self.captured_value.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 3 => Err(CosemError::ReadOnly),
            2 => Err(CosemError::ReadOnly),
            4 => {
                self.threshold_params = value;
                Ok(())
            }
            5 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // set_threshold
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_monitor_class_id() {
        let pm = ParameterMonitor::new(
            ObisCode::new(0, 0, 65, 0, 0, 255),
            DlmsType::Structure(alloc::vec![]),
        );
        assert_eq!(ParameterMonitor::CLASS_ID, 65);
        assert_eq!(ParameterMonitor::VERSION, 1);
    }
}
