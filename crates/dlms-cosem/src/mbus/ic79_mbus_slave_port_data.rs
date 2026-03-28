//! M-Bus Slave Port Data Interface (IC 79)
//!
//! Data from M-Bus slave port readings.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.8.79

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// M-Bus Slave Port Data Interface Class (IC 79)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None
///
/// Note: Data captured from M-Bus slave port.
/// Implementation varies by device capabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct MBusSlavePortData {
    logical_name: ObisCode,
}

impl MBusSlavePortData {
    /// Create a new MBusSlavePortData instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for MBusSlavePortData {
    const CLASS_ID: u16 = 79;
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
        assert_eq!(MBusSlavePortData::CLASS_ID, 79);
    }

    #[test]
    fn test_creation() {
        let data = MBusSlavePortData::new(ObisCode::new(0, 0, 99, 1, 0, 255));
        assert_eq!(data.logical_name(), &ObisCode::new(0, 0, 99, 1, 0, 255));
    }
}
