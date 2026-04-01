//!
//! Interface Class 1: Data
//!
//! Reference: Blue Book Part 2 §5.1
//!
//! The Data interface class is the simplest COSEM object, storing a single
//! value that can be of any DLMS data type (CHOICE).

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 1: Data
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | value | 2 | CHOICE | dynamic |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct Data {
    /// OBIS code identifying this object
    logical_name: ObisCode,
    /// The stored value (can be any DlmsType)
    value: DlmsType,
}

impl Data {
    /// Create a new Data object with the given OBIS code and value
    pub const fn new(logical_name: ObisCode, value: DlmsType) -> Self {
        Self {
            logical_name,
            value,
        }
    }

    /// Get the current value
    pub fn get_value(&self) -> &DlmsType {
        &self.value
    }

    /// Set the value
    pub fn set_value(&mut self, value: DlmsType) {
        self.value = value;
    }
}

impl CosemClass for Data {
    const CLASS_ID: u16 = 1;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        2
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.value.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly), // logical_name is read-only
            2 => {
                self.value = value;
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
    fn test_data_class_id() {
        let data = Data::new(ObisCode::new(0, 0, 1, 0, 0, 255), DlmsType::Null);
        assert_eq!(Data::CLASS_ID, 1);
        assert_eq!(Data::VERSION, 0);
        assert_eq!(data.logical_name(), &ObisCode::new(0, 0, 1, 0, 0, 255));
    }

    #[test]
    fn test_data_get_attribute() {
        let obis = ObisCode::new(1, 0, 1, 8, 0, 255);
        let data = Data::new(obis, DlmsType::UInt32(12345));

        // Test logical_name
        let attr1 = data.get_attribute(1).unwrap();
        assert_eq!(
            attr1,
            DlmsType::OctetString(alloc::vec![1, 0, 1, 8, 0, 255])
        );

        // Test value
        let attr2 = data.get_attribute(2).unwrap();
        assert_eq!(attr2, DlmsType::UInt32(12345));

        // Test invalid attribute
        assert!(data.get_attribute(3).is_err());
        assert!(data.get_attribute(0).is_err());
    }

    #[test]
    fn test_data_set_attribute() {
        let obis = ObisCode::new(1, 0, 1, 8, 0, 255);
        let mut data = Data::new(obis, DlmsType::UInt32(100));

        // Setting attribute 2 (value) should work
        assert!(data.set_attribute(2, DlmsType::UInt32(200)).is_ok());
        assert_eq!(data.value, DlmsType::UInt32(200));

        // Setting attribute 1 (logical_name) should fail (read-only)
        assert!(data.set_attribute(1, DlmsType::Null).is_err());

        // Setting invalid attribute should fail
        assert!(data.set_attribute(3, DlmsType::Null).is_err());
    }
}
