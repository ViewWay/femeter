//! SCHC-LPWAN Setup Interface (IC 126)
//!
//! SCHC (Static Context Header Compression) for LPWAN setup.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.126

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// SCHC-LPWAN Setup Interface Class (IC 126)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: device_id (octet-string)
/// - 3: fragmentation_mode (enum)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SchcLpwanSetup {
    logical_name: ObisCode,
    device_id: alloc::vec::Vec<u8>,
    fragmentation_mode: u8,
}

impl SchcLpwanSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            device_id: alloc::vec::Vec::new(),
            fragmentation_mode: 0,
        }
    }
}

impl CosemClass for SchcLpwanSetup {
    const CLASS_ID: u16 = 126;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::OctetString(self.device_id.clone())),
            3 => Ok(DlmsType::Enum(self.fragmentation_mode)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                if let DlmsType::OctetString(data) = value {
                    self.device_id = data;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            3 => {
                self.fragmentation_mode = value.as_enum().ok_or(CosemError::TypeMismatch {
                    expected: 6,
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
        assert_eq!(SchcLpwanSetup::CLASS_ID, 126);
    }
}
