//! M-Bus Slave Port Identification Interface (IC 81)
//!
//! Identification of M-Bus slave port connected meters.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.8.81

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// M-Bus Slave Port Identification Interface Class (IC 81)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: Identification of M-Bus slave port meters.
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct MBusSlavePortIdentification {
    logical_name: ObisCode,
}

impl MBusSlavePortIdentification {
    /// Create a new MBusSlavePortIdentification instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for MBusSlavePortIdentification {
    const CLASS_ID: u16 = 81;
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
        assert_eq!(MBusSlavePortIdentification::CLASS_ID, 81);
    }

    #[test]
    fn test_creation() {
        let id = MBusSlavePortIdentification::new(ObisCode::new(0, 0, 101, 1, 0, 255));
        assert_eq!(id.logical_name(), &ObisCode::new(0, 0, 101, 1, 0, 255));
    }
}
