//!
//! Interface Class 30: COSEM Data Protection
//!
//! Reference: Blue Book Part 2 §6.30
//!
//! Data Protection manages protected data access control.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 30: COSEM Data Protection
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | protected_objects | 2 | array of structure | static |
/// | protection_mode | 3 | enum | static |
/// | protection_status | 4 | unsigned | dynamic |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | connect | 1 | Connect protection |
/// | disconnect | 2 | Disconnect protection |
/// | change_protection_mode | 3 | Change protection mode |
#[derive(Debug, Clone)]
pub struct DataProtection {
    logical_name: ObisCode,
    protected_objects: DlmsType,
    protection_mode: u8,
    protection_status: u8,
}

impl DataProtection {
    /// Create a new Data Protection object
    pub fn new(logical_name: ObisCode, protection_mode: u8) -> Self {
        Self {
            logical_name,
            protected_objects: DlmsType::Array(alloc::vec![]),
            protection_mode,
            protection_status: 0,
        }
    }

    pub const fn get_protection_mode(&self) -> u8 {
        self.protection_mode
    }
}

impl CosemClass for DataProtection {
    const CLASS_ID: u16 = 30;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        4
    }

    fn method_count() -> u8 {
        3
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.protected_objects.clone()),
            3 => Ok(DlmsType::UInt8(self.protection_mode)),
            4 => Ok(DlmsType::UInt8(self.protection_status)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 3 => Err(CosemError::ReadOnly),
            2 => {
                self.protected_objects = value;
                Ok(())
            }
            4 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // connect
            2 => Ok(DlmsType::Null), // disconnect
            3 => Ok(DlmsType::Null), // change_protection_mode
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_protection_class_id() {
        let dp = DataProtection::new(ObisCode::new(0, 0, 30, 0, 0, 255), 0);
        assert_eq!(DataProtection::CLASS_ID, 30);
    }
}
