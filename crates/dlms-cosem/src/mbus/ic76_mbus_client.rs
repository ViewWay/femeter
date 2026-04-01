//! M-Bus Client Interface (IC 76)
//!
//! Master-Bus client for reading M-Bus meters.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.8.76

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// M-Bus Client Interface Class (IC 76)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: M-Bus client interface for reading other M-Bus meters.
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct MBusClient {
    logical_name: ObisCode,
}

impl MBusClient {
    /// Create a new MBusClient instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for MBusClient {
    const CLASS_ID: u16 = 76;
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
        assert_eq!(MBusClient::CLASS_ID, 76);
    }

    #[test]
    fn test_creation() {
        let client = MBusClient::new(ObisCode::new(0, 0, 96, 1, 0, 255));
        assert_eq!(client.logical_name(), &ObisCode::new(0, 0, 96, 1, 0, 255));
    }
}
