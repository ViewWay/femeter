//! PRIME MAC Network Admin Data Interface (IC 85)
//!
//! PRIME NB OFDM PLC MAC network administration data.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.85

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// PRIME MAC Network Admin Data Interface Class (IC 85)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: network_id (unsigned)
/// - 3: key_id (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct PrimeMacNetworkAdminData {
    logical_name: ObisCode,
    network_id: u32,
    key_id: u32,
}

impl PrimeMacNetworkAdminData {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            network_id: 0,
            key_id: 0,
        }
    }
}

impl CosemClass for PrimeMacNetworkAdminData {
    const CLASS_ID: u16 = 85;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.network_id)),
            3 => Ok(DlmsType::UInt32(self.key_id)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.network_id = value.as_u32().ok_or(CosemError::TypeMismatch {
                    expected: 19,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.key_id = value.as_u32().ok_or(CosemError::TypeMismatch {
                    expected: 19,
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
        assert_eq!(PrimeMacNetworkAdminData::CLASS_ID, 85);
    }
}
