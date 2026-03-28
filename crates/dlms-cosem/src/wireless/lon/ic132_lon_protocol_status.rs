//! ISO/IEC 14908 Protocol Status Interface (IC 132)
//!
//! LON protocol status information.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.132

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// ISO/IEC 14908 Protocol Status Interface Class (IC 132)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: mode (enum)
/// - 3: max_packet_length (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct LonProtocolStatus {
    logical_name: ObisCode,
    mode: u8,
    max_packet_length: u16,
}

impl LonProtocolStatus {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            mode: 0,
            max_packet_length: 256,
        }
    }
}

impl CosemClass for LonProtocolStatus {
    const CLASS_ID: u16 = 132;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Enum(self.mode)),
            3 => Ok(DlmsType::UInt16(self.max_packet_length)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, _id: u8, _value: DlmsType) -> Result<(), CosemError> {
        Err(CosemError::ReadOnly)
    }

    fn execute_method(&mut self, _id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        Err(CosemError::NoSuchMethod(1))
    }

    fn attribute_count() -> u8 {
        3
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
        assert_eq!(LonProtocolStatus::CLASS_ID, 132);
        assert_eq!(LonProtocolStatus::VERSION, 1);
    }
}
