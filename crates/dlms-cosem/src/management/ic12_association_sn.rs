//!
//! Interface Class 12: Association SN (Short Name)
//!
//! Reference: Blue Book Part 2 §6.2
//!
//! Association SN manages short-name based connections between client and server.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 12: Association SN
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | object_list | 2 | array of structure | static |
/// | authentication_mechanism_name | 3 | octet-string | static |
/// | secret | 4 | octet-string | static |
/// | association_status | 5 | enum | dynamic |
/// | security_setup_reference | 6 | octet-string | static |
/// | user_list | 7 | array | dynamic |
/// | current_user | 8 | structure | dynamic |
/// | client_id | 9 | octet-string | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | reply_to_HLS_authentication | 1 | HLS authentication response |
/// | change_secret | 2 | Change LLS secret |
/// | add_user | 3 | Add a new user |
/// | remove_user | 4 | Remove a user |
#[derive(Debug, Clone)]
pub struct AssociationSn {
    logical_name: ObisCode,
    object_list: DlmsType,
    secret: DlmsType,
    association_status: u8,
    security_setup_reference: DlmsType,
    user_list: DlmsType,
    current_user: DlmsType,
    client_id: DlmsType,
}

impl AssociationSn {
    /// Create a new Association SN object
    pub fn new(logical_name: ObisCode, client_id: DlmsType) -> Self {
        Self {
            logical_name,
            object_list: DlmsType::Array(alloc::vec![]),
            secret: DlmsType::OctetString(alloc::vec![]),
            association_status: 0,
            security_setup_reference: DlmsType::OctetString(alloc::vec![]),
            user_list: DlmsType::Array(alloc::vec![]),
            current_user: DlmsType::Null,
            client_id,
        }
    }

    pub const fn get_status(&self) -> u8 {
        self.association_status
    }
}

impl CosemClass for AssociationSn {
    const CLASS_ID: u16 = 12;
    const VERSION: u8 = 4;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        9
    }

    fn method_count() -> u8 {
        4
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.object_list.clone()),
            3 => Ok(DlmsType::OctetString(alloc::vec![])), // authentication_mechanism_name
            4 => Ok(self.secret.clone()),
            5 => Ok(DlmsType::UInt8(self.association_status)),
            6 => Ok(self.security_setup_reference.clone()),
            7 => Ok(self.user_list.clone()),
            8 => Ok(self.current_user.clone()),
            9 => Ok(self.client_id.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 2 | 3 | 9 => Err(CosemError::ReadOnly),
            4 => {
                self.secret = value;
                Ok(())
            }
            5 => Err(CosemError::ReadOnly),
            6 => {
                self.security_setup_reference = value;
                Ok(())
            }
            7 => {
                self.user_list = value;
                Ok(())
            }
            8 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // reply_to_HLS_authentication
            2 => Ok(DlmsType::Null), // change_secret
            3 => Ok(DlmsType::Null), // add_user
            4 => Ok(DlmsType::Null), // remove_user
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_association_sn_class_id() {
        let asn = AssociationSn::new(
            ObisCode::new(0, 0, 40, 0, 2, 255),
            DlmsType::OctetString(alloc::vec![]),
        );
        assert_eq!(AssociationSn::CLASS_ID, 12);
        assert_eq!(AssociationSn::VERSION, 4);
    }
}
