//! ZigBee SAS Startup Interface (IC 101)
//!
//! ZigBee SAS startup configuration.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.101

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// ZigBee SAS Startup Interface Class (IC 101)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: startup_control (enum)
/// - 3: startup_stack_profile (unsigned)
/// - 4: startup_zigbee_network_key (octet-string)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct ZigbeeSasStartup {
    logical_name: ObisCode,
    startup_control: u8,
    startup_stack_profile: u8,
    startup_zigbee_network_key: alloc::vec::Vec<u8>,
}

impl ZigbeeSasStartup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            startup_control: 0,
            startup_stack_profile: 0,
            startup_zigbee_network_key: alloc::vec::Vec::new(),
        }
    }
}

impl CosemClass for ZigbeeSasStartup {
    const CLASS_ID: u16 = 101;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Enum(self.startup_control)),
            3 => Ok(DlmsType::UInt8(self.startup_stack_profile)),
            4 => Ok(DlmsType::OctetString(
                self.startup_zigbee_network_key.clone(),
            )),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.startup_control = value.as_enum().ok_or(CosemError::TypeMismatch {
                    expected: 6,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.startup_stack_profile = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            4 => {
                if let DlmsType::OctetString(data) = value {
                    self.startup_zigbee_network_key = data;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
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
        assert_eq!(ZigbeeSasStartup::CLASS_ID, 101);
    }
}
