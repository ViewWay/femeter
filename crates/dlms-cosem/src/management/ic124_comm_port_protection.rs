//!
//! Interface Class 124: Communication Port Protection
//!
//! Reference: Blue Book Part 2 §6.124
//!
//! Communication Port Protection protects communication ports from
//! unauthorized access.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 124: Communication Port Protection
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | protection_scheme | 2 | octet-string | static |
/// | protection_key | 3 | octet-string | static |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct CommPortProtection {
    logical_name: ObisCode,
    protection_scheme: DlmsType,
    protection_key: DlmsType,
}

impl CommPortProtection {
    /// Create a new Communication Port Protection object
    pub fn new(
        logical_name: ObisCode,
        protection_scheme: DlmsType,
        protection_key: DlmsType,
    ) -> Self {
        Self {
            logical_name,
            protection_scheme,
            protection_key,
        }
    }
}

impl CosemClass for CommPortProtection {
    const CLASS_ID: u16 = 124;
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
            2 => Ok(self.protection_scheme.clone()),
            3 => Ok(self.protection_key.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.protection_scheme = value;
                Ok(())
            }
            3 => {
                self.protection_key = value;
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
    fn test_comm_port_protection_class_id() {
        let _cpp = CommPortProtection::new(
            ObisCode::new(0, 0, 124, 0, 0, 255),
            DlmsType::OctetString(alloc::vec![]),
            DlmsType::OctetString(alloc::vec![]),
        );
        assert_eq!(CommPortProtection::CLASS_ID, 124);
    }
}
