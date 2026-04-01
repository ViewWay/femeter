//! IEC 62056-46 Modem Setup Interface (IC 16)
//!
//! Modem configuration for telephone line communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.16

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// IEC 62056-46 Modem Setup Interface Class (IC 16)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: Modem configuration for telephone line communication.
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct IecModemSetup {
    logical_name: ObisCode,
}

impl IecModemSetup {
    /// Create a new IecModemSetup instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for IecModemSetup {
    const CLASS_ID: u16 = 16;
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
        assert_eq!(IecModemSetup::CLASS_ID, 16);
    }

    #[test]
    fn test_creation() {
        let setup = IecModemSetup::new(ObisCode::new(0, 0, 16, 0, 0, 255));
        assert_eq!(setup.logical_name(), &ObisCode::new(0, 0, 16, 0, 0, 255));
    }
}
