//! LoRaWAN Setup Interface (IC 128)
//!
//! LoRaWAN network setup configuration.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.128

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// LoRaWAN Setup Interface Class (IC 128)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: device_eui (octet-string)
/// - 3: app_eui (octet-string)
/// - 4: app_key (octet-string)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct LorawanSetup {
    logical_name: ObisCode,
    device_eui: alloc::vec::Vec<u8>,
    app_eui: alloc::vec::Vec<u8>,
    app_key: alloc::vec::Vec<u8>,
}

impl LorawanSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            device_eui: alloc::vec::Vec::new(),
            app_eui: alloc::vec::Vec::new(),
            app_key: alloc::vec::Vec::new(),
        }
    }
}

impl CosemClass for LorawanSetup {
    const CLASS_ID: u16 = 128;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::OctetString(self.device_eui.clone())),
            3 => Ok(DlmsType::OctetString(self.app_eui.clone())),
            4 => Ok(DlmsType::OctetString(self.app_key.clone())),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                if let DlmsType::OctetString(data) = value {
                    self.device_eui = data;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            3 => {
                if let DlmsType::OctetString(data) = value {
                    self.app_eui = data;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            4 => {
                if let DlmsType::OctetString(data) = value {
                    self.app_key = data;
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
        assert_eq!(LorawanSetup::CLASS_ID, 128);
    }
}
