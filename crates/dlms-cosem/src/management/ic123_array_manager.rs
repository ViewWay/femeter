//!
//! Interface Class 123: Array Manager
//!
//! Reference: Blue Book Part 2 §6.123
//!
//! Array Manager manages array attributes of other COSEM objects.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 123: Array Manager
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | array_element_type | 2 | octet-string | static |
/// | array_content | 3 | array | dynamic |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | insert_element | 1 | Insert an element |
/// | delete_element | 2 | Delete an element |
/// | replace_element | 3 | Replace an element |
#[derive(Debug, Clone)]
pub struct ArrayManager {
    logical_name: ObisCode,
    array_element_type: DlmsType,
    array_content: DlmsType,
}

impl ArrayManager {
    /// Create a new Array Manager object
    pub fn new(logical_name: ObisCode, array_element_type: DlmsType) -> Self {
        Self {
            logical_name,
            array_element_type,
            array_content: DlmsType::Array(alloc::vec![]),
        }
    }
}

impl CosemClass for ArrayManager {
    const CLASS_ID: u16 = 123;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        3
    }

    fn method_count() -> u8 {
        3
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.array_element_type.clone()),
            3 => Ok(self.array_content.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 2 => Err(CosemError::ReadOnly),
            3 => {
                self.array_content = value;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // insert_element
            2 => Ok(DlmsType::Null), // delete_element
            3 => Ok(DlmsType::Null), // replace_element
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_manager_class_id() {
        let _am = ArrayManager::new(
            ObisCode::new(0, 0, 123, 0, 0, 255),
            DlmsType::OctetString(alloc::vec![]),
        );
        assert_eq!(ArrayManager::CLASS_ID, 123);
        assert_eq!(ArrayManager::method_count(), 3);
    }
}
