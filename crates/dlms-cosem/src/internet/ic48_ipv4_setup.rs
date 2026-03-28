//! IP v4 Setup Interface (IC 48)
//!
//! Setup for IPv4 network configuration.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.9.48

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// IPv4 Setup Interface Class (IC 48)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: IPv4 network configuration.
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct IPv4Setup {
    logical_name: ObisCode,
}

impl IPv4Setup {
    /// Create a new IPv4Setup instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for IPv4Setup {
    const CLASS_ID: u16 = 48;
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
        assert_eq!(IPv4Setup::CLASS_ID, 48);
    }

    #[test]
    fn test_creation() {
        let setup = IPv4Setup::new(ObisCode::new(0, 0, 41, 0, 0, 255));
        assert_eq!(setup.logical_name(), &ObisCode::new(0, 0, 41, 0, 0, 255));
    }
}
