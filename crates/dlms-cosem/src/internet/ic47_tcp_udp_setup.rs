//! TCP-UDP Setup Interface (IC 47)
//!
//! Setup for TCP/UDP network communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.9.47

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// TCP-UDP Setup Interface Class (IC 47)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: TCP/UDP network configuration.
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct TcpUdpSetup {
    logical_name: ObisCode,
}

impl TcpUdpSetup {
    /// Create a new TcpUdpSetup instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for TcpUdpSetup {
    const CLASS_ID: u16 = 47;
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
        assert_eq!(TcpUdpSetup::CLASS_ID, 47);
    }

    #[test]
    fn test_creation() {
        let setup = TcpUdpSetup::new(ObisCode::new(0, 0, 43, 0, 0, 255));
        assert_eq!(setup.logical_name(), &ObisCode::new(0, 0, 43, 0, 0, 255));
    }
}
