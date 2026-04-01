//! Wi-SUN Setup Interface (IC 95)
//!
//! Wi-SUN network setup configuration.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.95

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// Wi-SUN Setup Interface Class (IC 95)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: network_id (octet-string)
/// - 3: network_key (octet-string)
/// - 4: channel_number (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct WisunSetup {
    logical_name: ObisCode,
    network_id: alloc::vec::Vec<u8>,
    network_key: alloc::vec::Vec<u8>,
    channel_number: u8,
}

impl WisunSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            network_id: alloc::vec::Vec::new(),
            network_key: alloc::vec::Vec::new(),
            channel_number: 0,
        }
    }
}

impl CosemClass for WisunSetup {
    const CLASS_ID: u16 = 95;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::OctetString(self.network_id.clone())),
            3 => Ok(DlmsType::OctetString(self.network_key.clone())),
            4 => Ok(DlmsType::UInt8(self.channel_number)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                if let DlmsType::OctetString(data) = value {
                    self.network_id = data;
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
                    self.network_key = data;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            4 => {
                self.channel_number = value.as_u8().ok_or(CosemError::TypeMismatch {
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
        assert_eq!(WisunSetup::CLASS_ID, 95);
    }
}
