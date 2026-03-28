//! ZigBee SAS Join Interface (IC 102)
//!
//! ZigBee SAS join configuration.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.102

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// ZigBee SAS Join Interface Class (IC 102)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: join_mode (enum)
/// - 3: join_capability (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct ZigbeeSasJoin {
    logical_name: ObisCode,
    join_mode: u8,
    join_capability: u8,
}

impl ZigbeeSasJoin {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            join_mode: 0,
            join_capability: 0,
        }
    }
}

impl CosemClass for ZigbeeSasJoin {
    const CLASS_ID: u16 = 102;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Enum(self.join_mode)),
            3 => Ok(DlmsType::UInt8(self.join_capability)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.join_mode = value.as_enum().ok_or(CosemError::TypeMismatch {
                    expected: 6,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.join_capability = value.as_u8().ok_or(CosemError::TypeMismatch {
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
        assert_eq!(ZigbeeSasJoin::CLASS_ID, 102);
    }
}
