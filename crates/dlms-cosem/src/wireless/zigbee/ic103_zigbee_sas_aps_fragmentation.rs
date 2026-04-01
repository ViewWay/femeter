//! ZigBee SAS APS Fragmentation Interface (IC 103)
//!
//! ZigBee SAS APS fragmentation configuration.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.103

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// ZigBee SAS APS Fragmentation Interface Class (IC 103)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: fragmentation_mode (enum)
/// - 3: max_incoming_transfer_size (unsigned)
/// - 4: max_outgoing_transfer_size (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct ZigbeeSasApsFragmentation {
    logical_name: ObisCode,
    fragmentation_mode: u8,
    max_incoming_transfer_size: u16,
    max_outgoing_transfer_size: u16,
}

impl ZigbeeSasApsFragmentation {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            fragmentation_mode: 0,
            max_incoming_transfer_size: 128,
            max_outgoing_transfer_size: 128,
        }
    }
}

impl CosemClass for ZigbeeSasApsFragmentation {
    const CLASS_ID: u16 = 103;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Enum(self.fragmentation_mode)),
            3 => Ok(DlmsType::UInt16(self.max_incoming_transfer_size)),
            4 => Ok(DlmsType::UInt16(self.max_outgoing_transfer_size)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.fragmentation_mode = value.as_enum().ok_or(CosemError::TypeMismatch {
                    expected: 6,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.max_incoming_transfer_size =
                    value.as_u16().ok_or(CosemError::TypeMismatch {
                        expected: 18,
                        got: value.tag(),
                    })?;
                Ok(())
            }
            4 => {
                self.max_outgoing_transfer_size =
                    value.as_u16().ok_or(CosemError::TypeMismatch {
                        expected: 18,
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
        4
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
        assert_eq!(ZigbeeSasApsFragmentation::CLASS_ID, 103);
    }
}
