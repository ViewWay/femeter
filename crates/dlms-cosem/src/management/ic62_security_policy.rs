//! COSEM Security Policy Interface (IC 62)
//!
//! The Security Policy interface defines security policies for DLMS/COSEM communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.4.62

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM Security Policy Interface Class (IC 62)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None (implementation varies by security policy)
///
/// Note: This interface is used to define and manage security policies
/// for DLMS/COSEM communication. The actual implementation depends
/// on the specific security requirements of the meter.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SecurityPolicy {
    logical_name: ObisCode,
}

impl SecurityPolicy {
    /// Create a new SecurityPolicy instance
    ///
    /// # Arguments
    ///
    /// * `logical_name` - OBIS code identifying this security policy object
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for SecurityPolicy {
    const CLASS_ID: u16 = 62;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            _ => Err(CosemError::NotImplemented),
        }
    }

    fn set_attribute(&mut self, _id: u8, _value: DlmsType) -> Result<(), CosemError> {
        Err(CosemError::NotImplemented)
    }

    fn execute_method(&mut self, _id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        Err(CosemError::NotImplemented)
    }

    fn attribute_count() -> u8 {
        1
    }

    fn method_count() -> u8 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_security_policy() -> SecurityPolicy {
        SecurityPolicy::new(ObisCode::new(0, 0, 62, 0, 0, 255))
    }

    #[test]
    fn test_security_policy_creation() {
        let policy = create_test_security_policy();
        assert_eq!(policy.logical_name(), &ObisCode::new(0, 0, 62, 0, 0, 255));
    }

    #[test]
    fn test_get_logical_name() {
        let policy = create_test_security_policy();
        let ln = policy.get_attribute(1).unwrap();
        assert!(matches!(ln, DlmsType::OctetString(_)));
    }

    #[test]
    fn test_class_id() {
        assert_eq!(SecurityPolicy::CLASS_ID, 62);
        assert_eq!(SecurityPolicy::VERSION, 0);
    }

    #[test]
    fn test_attribute_count() {
        assert_eq!(SecurityPolicy::attribute_count(), 1);
        assert_eq!(SecurityPolicy::method_count(), 0);
    }
}
