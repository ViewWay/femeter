//!
//! Interface Class 15: Association LN (Logical Name)
//!
//! Reference: Blue Book Part 2 §6.5
//!
//! Association LN is the MOST IMPORTANT management IC. It manages logical-name
//! based connections between client and server, including the complete object
//! list exposed by the meter.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 15: Association LN
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | object_list | 2 | array of structure | static |
/// | associated_partners_id | 3 | structure | static |
/// | application_context_name | 4 | octet-string | static |
/// | xdlms_context_info | 5 | structure | static |
/// | authentication_mechanism_name | 6 | octet-string | static |
/// | lls_secret | 7 | octet-string | static |
/// | association_status | 8 | enum | dynamic |
/// | security_setup_reference | 9 | octet-string | static |
/// | user_list | 10 | array | dynamic |
/// | current_user | 11 | structure | dynamic |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | reply_to_HLS_authentication | 1 | HLS authentication response |
/// | change_HLS_secret | 2 | Change HLS secret |
/// | add_user | 3 | Add a new user |
/// | remove_user | 4 | Remove a user |
#[derive(Debug, Clone)]
pub struct AssociationLn {
    logical_name: ObisCode,
    object_list: DlmsType,
    associated_partners_id: DlmsType,
    application_context_name: DlmsType,
    xdlms_context_info: DlmsType,
    authentication_mechanism_name: DlmsType,
    lls_secret: DlmsType,
    association_status: u8,
    security_setup_reference: DlmsType,
    user_list: DlmsType,
    current_user: DlmsType,
}

impl AssociationLn {
    /// Create a new Association LN object
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            object_list: DlmsType::Array(alloc::vec![]),
            associated_partners_id: DlmsType::Structure(alloc::vec![]),
            application_context_name: DlmsType::OctetString(alloc::vec![]),
            xdlms_context_info: DlmsType::Structure(alloc::vec![
                DlmsType::UInt8(0), // conformance (lower)
                DlmsType::UInt8(0), // conformance (mid)
                DlmsType::UInt8(0), // conformance (upper)
                DlmsType::UInt8(32), // max_receive_pdu_size
                DlmsType::UInt8(32), // max_send_pdu_size
            ]),
            authentication_mechanism_name: DlmsType::OctetString(alloc::vec![]),
            lls_secret: DlmsType::OctetString(alloc::vec![]),
            association_status: 0,
            security_setup_reference: DlmsType::OctetString(alloc::vec![]),
            user_list: DlmsType::Array(alloc::vec![]),
            current_user: DlmsType::Null,
        }
    }

    /// Set the object list (list of all COSEM objects in the meter)
    pub fn set_object_list(&mut self, object_list: DlmsType) {
        self.object_list = object_list;
    }

    /// Set the association status
    pub fn set_status(&mut self, status: u8) {
        self.association_status = status;
    }

    /// Get current association status
    pub const fn get_status(&self) -> u8 {
        self.association_status
    }
}

impl CosemClass for AssociationLn {
    const CLASS_ID: u16 = 15;
    const VERSION: u8 = 3;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        11
    }

    fn method_count() -> u8 {
        4
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.object_list.clone()),
            3 => Ok(self.associated_partners_id.clone()),
            4 => Ok(self.application_context_name.clone()),
            5 => Ok(self.xdlms_context_info.clone()),
            6 => Ok(self.authentication_mechanism_name.clone()),
            7 => Ok(self.lls_secret.clone()),
            8 => Ok(DlmsType::UInt8(self.association_status)),
            9 => Ok(self.security_setup_reference.clone()),
            10 => Ok(self.user_list.clone()),
            11 => Ok(self.current_user.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 2 | 3 | 4 | 5 | 6 => Err(CosemError::ReadOnly),
            7 => {
                self.lls_secret = value;
                Ok(())
            }
            8 => Err(CosemError::ReadOnly),
            9 => {
                self.security_setup_reference = value;
                Ok(())
            }
            10 => {
                self.user_list = value;
                Ok(())
            }
            11 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                // reply_to_HLS_authentication
                Ok(DlmsType::Null)
            }
            2 => {
                // change_HLS_secret
                Ok(DlmsType::Null)
            }
            3 => {
                // add_user
                Ok(DlmsType::Null)
            }
            4 => {
                // remove_user
                Ok(DlmsType::Null)
            }
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_association_ln_class_id() {
        let aln = AssociationLn::new(ObisCode::new(0, 0, 40, 0, 1, 255));
        assert_eq!(AssociationLn::CLASS_ID, 15);
        assert_eq!(AssociationLn::VERSION, 3);
        assert_eq!(AssociationLn::attribute_count(), 11);
        assert_eq!(AssociationLn::method_count(), 4);
    }
}
