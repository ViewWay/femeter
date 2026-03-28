//!
//! Interface Class 26: Utility Tables
//!
//! Reference: Blue Book Part 2 §5.26
//!
//! Utility Tables provides tabular data storage for various utility-related
//! lookup tables and configuration data.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 26: Utility Tables
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | table_id | 2 | unsigned | static |
/// | table_description | 3 | visible-string | static |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct UtilityTables {
    logical_name: ObisCode,
    table_id: u8,
    table_description: DlmsType,
}

impl UtilityTables {
    /// Create a new Utility Tables object
    pub fn new(logical_name: ObisCode, table_id: u8, table_description: DlmsType) -> Self {
        Self {
            logical_name,
            table_id,
            table_description,
        }
    }

    pub const fn get_table_id(&self) -> u8 {
        self.table_id
    }
}

impl CosemClass for UtilityTables {
    const CLASS_ID: u16 = 26;
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
            2 => Ok(DlmsType::UInt8(self.table_id)),
            3 => Ok(self.table_description.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 2 => Err(CosemError::ReadOnly),
            3 => {
                self.table_description = value;
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
    fn test_utility_tables_class_id() {
        let ut = UtilityTables::new(
            ObisCode::new(0, 0, 26, 0, 0, 255),
            1,
            DlmsType::VisibleString(alloc::vec![]),
        );
        assert_eq!(UtilityTables::CLASS_ID, 26);
        assert_eq!(ut.get_table_id(), 1);
    }
}
