//! ISO/IEC 8802-2 LLC Type 3 Setup Interface (IC 59)
//!
//! LLC Type 3 (acknowledged connectionless) setup.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.59

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// ISO/IEC 8802-2 LLC Type 3 Setup Interface Class (IC 59)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: max_num_unconfirmed_frames (unsigned)
/// - 3: ack_timeout_time (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct LlcType3Setup {
    logical_name: ObisCode,
    max_num_unconfirmed_frames: u8,
    ack_timeout_time: u16,
}

impl LlcType3Setup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            max_num_unconfirmed_frames: 10,
            ack_timeout_time: 100,
        }
    }
}

impl CosemClass for LlcType3Setup {
    const CLASS_ID: u16 = 59;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.max_num_unconfirmed_frames)),
            3 => Ok(DlmsType::UInt16(self.ack_timeout_time)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.max_num_unconfirmed_frames =
                    value.as_u8().ok_or(CosemError::TypeMismatch {
                        expected: 17,
                        got: value.tag(),
                    })?;
                Ok(())
            }
            3 => {
                self.ack_timeout_time = value.as_u16().ok_or(CosemError::TypeMismatch {
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
        assert_eq!(LlcType3Setup::CLASS_ID, 59);
    }
}
