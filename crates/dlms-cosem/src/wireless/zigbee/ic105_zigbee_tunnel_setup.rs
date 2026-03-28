//! ZigBee Tunnel Setup Interface (IC 105)
//!
//! ZigBee tunnel configuration.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.105

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// ZigBee Tunnel Setup Interface Class (IC 105)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: tunnel_mode (enum)
/// - 3: tunnel_protocol (enum)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct ZigbeeTunnelSetup {
    logical_name: ObisCode,
    tunnel_mode: u8,
    tunnel_protocol: u8,
}

impl ZigbeeTunnelSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            tunnel_mode: 0,
            tunnel_protocol: 0,
        }
    }
}

impl CosemClass for ZigbeeTunnelSetup {
    const CLASS_ID: u16 = 105;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Enum(self.tunnel_mode)),
            3 => Ok(DlmsType::Enum(self.tunnel_protocol)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.tunnel_mode = value.as_enum().ok_or(CosemError::TypeMismatch {
                    expected: 6,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.tunnel_protocol = value.as_enum().ok_or(CosemError::TypeMismatch {
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
        assert_eq!(ZigbeeTunnelSetup::CLASS_ID, 105);
    }
}
