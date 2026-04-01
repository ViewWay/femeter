//!
//! Interface Class 64: Security Setup
//!
//! Reference: Blue Book Part 2 §6.64
//!
//! Security Setup manages security parameters including keys and encryption.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 64: Security Setup
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | version | 2 | enum | static |
/// | conformance | 3 | bit-string | static |
/// | system_title | 4 | octet-string(SIZE(8)) | static |
/// | server_system_title | 5 | octet-string(SIZE(8)) | static |
/// | certification_reference | 6 | visible-string | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | security_activate | 1 | Activate security |
/// | global_key_transfer | 2 | Transfer global key |
/// | generate_key_pair | 3 | Generate key pair |
/// | generate_key | 4 | Generate a key |
/// | transfer_key | 5 | Transfer a key |
/// | agree_on_key | 6 | Agree on a key |
#[derive(Debug, Clone)]
pub struct SecuritySetup {
    logical_name: ObisCode,
    version: u8,
    conformance: DlmsType,
    system_title: DlmsType,
    server_system_title: DlmsType,
    certification_reference: DlmsType,
}

impl SecuritySetup {
    /// Create a new Security Setup object
    pub fn new(logical_name: ObisCode, system_title: DlmsType) -> Self {
        Self {
            logical_name,
            version: 1,
            conformance: DlmsType::OctetString(alloc::vec![0, 0, 0]),
            system_title: system_title.clone(),
            server_system_title: system_title,
            certification_reference: DlmsType::VisibleString(alloc::vec![]),
        }
    }

    pub const fn get_version(&self) -> u8 {
        self.version
    }
}

impl CosemClass for SecuritySetup {
    const CLASS_ID: u16 = 64;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        6
    }

    fn method_count() -> u8 {
        6
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.version)),
            3 => Ok(self.conformance.clone()),
            4 => Ok(self.system_title.clone()),
            5 => Ok(self.server_system_title.clone()),
            6 => Ok(self.certification_reference.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 2 => Err(CosemError::ReadOnly),
            3 => {
                self.conformance = value;
                Ok(())
            }
            4 => {
                self.system_title = value;
                Ok(())
            }
            5 => {
                self.server_system_title = value;
                Ok(())
            }
            6 => {
                self.certification_reference = value;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // security_activate
            2 => Ok(DlmsType::Null), // global_key_transfer
            3 => Ok(DlmsType::Null), // generate_key_pair
            4 => Ok(DlmsType::Null), // generate_key
            5 => Ok(DlmsType::Null), // transfer_key
            6 => Ok(DlmsType::Null), // agree_on_key
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_setup_class_id() {
        let ss = SecuritySetup::new(
            ObisCode::new(0, 0, 43, 0, 1, 255),
            DlmsType::OctetString(alloc::vec![0, 0, 0, 0, 0, 0, 0, 0]),
        );
        assert_eq!(SecuritySetup::CLASS_ID, 64);
        assert_eq!(SecuritySetup::VERSION, 1);
        assert_eq!(SecuritySetup::method_count(), 6);
    }
}
