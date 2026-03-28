//! IEC Twisted Pair Setup Interface (IC 15)
//!
//! Setup for twisted pair (RS-485) communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.15

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// IEC Twisted Pair Setup Interface Class (IC 15)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: RS-485 communication port configuration.
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct IecTwistedPairSetup {
    logical_name: ObisCode,
}

impl IecTwistedPairSetup {
    /// Create a new IecTwistedPairSetup instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for IecTwistedPairSetup {
    const CLASS_ID: u16 = 15;
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
        assert_eq!(IecTwistedPairSetup::CLASS_ID, 15);
    }

    #[test]
    fn test_creation() {
        let setup = IecTwistedPairSetup::new(ObisCode::new(0, 0, 15, 0, 0, 255));
        assert_eq!(setup.logical_name(), &ObisCode::new(0, 0, 15, 0, 0, 255));
    }
}
