//!
//! Interface Class 10: Schedule
//!
//! Reference: Blue Book Part 2 §5.10
//!
//! Schedule defines time-based triggers for actions.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 10: Schedule
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | entries | 2 | array of structure | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | enable | 1 | Enable the schedule |
/// | disable | 2 | Disable the schedule |
/// | execute | 3 | Execute scheduled actions |
#[derive(Debug, Clone)]
pub struct Schedule {
    logical_name: ObisCode,
    entries: DlmsType,
}

impl Schedule {
    /// Create a new Schedule object
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            entries: DlmsType::Array(alloc::vec![]),
        }
    }
}

impl CosemClass for Schedule {
    const CLASS_ID: u16 = 10;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        2
    }

    fn method_count() -> u8 {
        3
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.entries.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.entries = value;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1..=3 => Ok(DlmsType::Null),
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_class_id() {
        let s = Schedule::new(ObisCode::new(0, 0, 11, 0, 0, 255));
        assert_eq!(Schedule::CLASS_ID, 10);
        assert_eq!(Schedule::method_count(), 3);
    }
}
