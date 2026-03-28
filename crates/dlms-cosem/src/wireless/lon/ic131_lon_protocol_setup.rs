//! ISO/IEC 14908 Protocol Setup Interface (IC 131)
//!
//! LON protocol setup configuration.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.131

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// ISO/IEC 14908 Protocol Setup Interface Class (IC 131)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: channel_number (unsigned)
/// - 3: subnet_address (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct LonProtocolSetup {
    logical_name: ObisCode,
    channel_number: u8,
    subnet_address: u8,
}

impl LonProtocolSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            channel_number: 0,
            subnet_address: 0,
        }
    }
}

impl CosemClass for LonProtocolSetup {
    const CLASS_ID: u16 = 131;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.channel_number)),
            3 => Ok(DlmsType::UInt8(self.subnet_address)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.channel_number = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.subnet_address = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
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
        assert_eq!(LonProtocolSetup::CLASS_ID, 131);
    }
}
