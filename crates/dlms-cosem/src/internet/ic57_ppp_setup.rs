//! PPP Setup Interface (IC 57)
//!
//! Setup for Point-to-Point Protocol (PPP) communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.9.57

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// PPP Setup Interface Class (IC 57)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: PPP protocol configuration.
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct PppSetup {
    logical_name: ObisCode,
}

impl PppSetup {
    /// Create a new PppSetup instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for PppSetup {
    const CLASS_ID: u16 = 57;
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
        assert_eq!(PppSetup::CLASS_ID, 57);
    }

    #[test]
    fn test_creation() {
        let setup = PppSetup::new(ObisCode::new(0, 0, 46, 0, 0, 255));
        assert_eq!(setup.logical_name(), &ObisCode::new(0, 0, 46, 0, 0, 255));
    }
}
