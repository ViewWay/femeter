//! IEC Local Port Setup Interface (IC 13)
//!
//! Local port for optical head/RS-485 communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.13

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// IEC Local Port Setup Interface Class (IC 13)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: Used for local port configuration (optical head, RS-485).
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct IecLocalPortSetup {
    logical_name: ObisCode,
}

impl IecLocalPortSetup {
    /// Create a new IecLocalPortSetup instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for IecLocalPortSetup {
    const CLASS_ID: u16 = 13;
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
        assert_eq!(IecLocalPortSetup::CLASS_ID, 13);
    }

    #[test]
    fn test_creation() {
        let port = IecLocalPortSetup::new(ObisCode::new(0, 0, 13, 0, 0, 255));
        assert_eq!(port.logical_name(), &ObisCode::new(0, 0, 13, 0, 0, 255));
    }
}
