//!
//! Interface Class 11: Special Days Table
//!
//! Reference: Blue Book Part 2 §5.11
//!
//! Special Days Table defines special calendar days (holidays, etc.).

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 11: Special Days Table
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | entries | 2 | array of structure | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | add_entry | 1 | Add a special day |
/// | remove_entry | 2 | Remove a special day |
#[derive(Debug, Clone)]
pub struct SpecialDaysTable {
    logical_name: ObisCode,
    entries: DlmsType,
}

impl SpecialDaysTable {
    /// Create a new Special Days Table object
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            entries: DlmsType::Array(alloc::vec![]),
        }
    }
}

impl CosemClass for SpecialDaysTable {
    const CLASS_ID: u16 = 11;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        2
    }

    fn method_count() -> u8 {
        2
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
            1 | 2 => Ok(DlmsType::Null),
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_special_days_table_class_id() {
        let sdt = SpecialDaysTable::new(ObisCode::new(0, 0, 11, 0, 1, 255));
        assert_eq!(SpecialDaysTable::CLASS_ID, 11);
    }
}
