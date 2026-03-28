//!
//! Interface Class 61: Register Table
//!
//! Reference: Blue Book Part 2 §5.61
//!
//! The Register Table provides tabular storage for multiple related register values.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 61: Register Table
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | table_id | 2 | unsigned | static |
/// | table_description | 3 | visible-string | static |
/// | table_content | 4 | array of structure | dynamic |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct RegisterTable {
    logical_name: ObisCode,
    table_id: u8,
    table_description: DlmsType,
    table_content: DlmsType,
}

impl RegisterTable {
    /// Create a new Register Table object
    pub fn new(
        logical_name: ObisCode,
        table_id: u8,
        table_description: DlmsType,
        table_content: DlmsType,
    ) -> Self {
        Self {
            logical_name,
            table_id,
            table_description,
            table_content,
        }
    }

    pub const fn get_table_id(&self) -> u8 {
        self.table_id
    }

    pub fn set_table_content(&mut self, content: DlmsType) {
        self.table_content = content;
    }
}

impl CosemClass for RegisterTable {
    const CLASS_ID: u16 = 61;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        4
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.table_id)),
            3 => Ok(self.table_description.clone()),
            4 => Ok(self.table_content.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 2 | 3 => Err(CosemError::ReadOnly),
            4 => {
                self.table_content = value;
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
    fn test_register_table_class_id() {
        let rt = RegisterTable::new(
            ObisCode::new(0, 0, 61, 0, 0, 255),
            1,
            DlmsType::VisibleString(alloc::vec![]),
            DlmsType::Array(alloc::vec![]),
        );
        assert_eq!(RegisterTable::CLASS_ID, 61);
        assert_eq!(rt.get_table_id(), 1);
    }
}
