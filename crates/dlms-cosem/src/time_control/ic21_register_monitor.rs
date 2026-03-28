//!
//! Interface Class 21: Register Monitor
//!
//! Reference: Blue Book Part 2 §5.21
//!
//! Register Monitor monitors register values and triggers actions on thresholds.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 21: Register Monitor
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | thresholds | 2 | array of structure | dynamic |
/// | monitored_object | 3 | structure | static |
/// | monitored_value | 4 | CHOICE | dynamic |
/// | action_upon_threshold_overflow | 5 | octet-string | static |
/// | threshold_overflow | 6 | boolean | dynamic |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct RegisterMonitor {
    logical_name: ObisCode,
    thresholds: DlmsType,
    monitored_object: DlmsType,
    monitored_value: DlmsType,
    action_upon_threshold_overflow: DlmsType,
    threshold_overflow: bool,
}

impl RegisterMonitor {
    /// Create a new Register Monitor object
    pub fn new(
        logical_name: ObisCode,
        monitored_object: DlmsType,
        action_upon_threshold_overflow: DlmsType,
    ) -> Self {
        Self {
            logical_name,
            thresholds: DlmsType::Array(alloc::vec![]),
            monitored_object,
            monitored_value: DlmsType::Null,
            action_upon_threshold_overflow,
            threshold_overflow: false,
        }
    }

    pub const fn is_threshold_overflow(&self) -> bool {
        self.threshold_overflow
    }
}

impl CosemClass for RegisterMonitor {
    const CLASS_ID: u16 = 21;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        6
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.thresholds.clone()),
            3 => Ok(self.monitored_object.clone()),
            4 => Ok(self.monitored_value.clone()),
            5 => Ok(self.action_upon_threshold_overflow.clone()),
            6 => Ok(DlmsType::Boolean(self.threshold_overflow)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 3 | 5 => Err(CosemError::ReadOnly),
            2 => {
                self.thresholds = value;
                Ok(())
            }
            4 => Err(CosemError::ReadOnly),
            6 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_monitor_class_id() {
        let rm = RegisterMonitor::new(
            ObisCode::new(0, 0, 15, 0, 0, 255),
            DlmsType::Structure(alloc::vec![]),
            DlmsType::OctetString(alloc::vec![]),
        );
        assert_eq!(RegisterMonitor::CLASS_ID, 21);
    }
}
