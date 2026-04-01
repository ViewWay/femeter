//!
//! Interface Class 22: Single Action Schedule
//!
//! Reference: Blue Book Part 2 §5.22
//!
//! Single Action Schedule defines a single scheduled action.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 22: Single Action Schedule
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | executed_time | 2 | date-time | dynamic |
/// | execution_time | 3 | date-time | static |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct SingleActionSchedule {
    logical_name: ObisCode,
    executed_time: DlmsType,
    execution_time: DlmsType,
}

impl SingleActionSchedule {
    /// Create a new Single Action Schedule object
    pub fn new(logical_name: ObisCode, execution_time: DlmsType) -> Self {
        Self {
            logical_name,
            executed_time: DlmsType::Null,
            execution_time,
        }
    }
}

impl CosemClass for SingleActionSchedule {
    const CLASS_ID: u16 = 22;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        3
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.executed_time.clone()),
            3 => Ok(self.execution_time.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => Err(CosemError::ReadOnly),
            3 => {
                self.execution_time = value;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_action_schedule_class_id() {
        let sas = SingleActionSchedule::new(ObisCode::new(0, 0, 22, 0, 0, 255), DlmsType::Null);
        assert_eq!(SingleActionSchedule::CLASS_ID, 22);
    }
}
