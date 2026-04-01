//! SMS Setup Interface (IC 59)
//!
//! Setup for SMS (Short Message Service) communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.9.59

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// SMS Setup Interface Class (IC 59)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: SMS communication configuration.
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SmsSetup {
    logical_name: ObisCode,
}

impl SmsSetup {
    /// Create a new SmsSetup instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for SmsSetup {
    const CLASS_ID: u16 = 59;
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

    #[test]
    fn test_class_id() {
        assert_eq!(SmsSetup::CLASS_ID, 59);
    }

    #[test]
    fn test_creation() {
        let setup = SmsSetup::new(ObisCode::new(0, 0, 39, 0, 0, 255));
        assert_eq!(setup.logical_name(), &ObisCode::new(0, 0, 39, 0, 0, 255));
    }
}
