//! G3-PLC Hybrid RF MAC Setup Interface (IC 161)
//!
//! G3-PLC Hybrid RF MAC setup.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.161

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// G3-PLC Hybrid RF MAC Setup Interface Class (IC 161)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: mac_node_type (enum)
/// - 3: mac_address (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct G3HybridRfMacSetup {
    logical_name: ObisCode,
    mac_node_type: u8,
    mac_address: u16,
}

impl G3HybridRfMacSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            mac_node_type: 0,
            mac_address: 0,
        }
    }
}

impl CosemClass for G3HybridRfMacSetup {
    const CLASS_ID: u16 = 161;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Enum(self.mac_node_type)),
            3 => Ok(DlmsType::UInt16(self.mac_address)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.mac_node_type = value.as_enum().ok_or(CosemError::TypeMismatch {
                    expected: 6,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.mac_address = value.as_u16().ok_or(CosemError::TypeMismatch {
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
        assert_eq!(G3HybridRfMacSetup::CLASS_ID, 161);
        assert_eq!(G3HybridRfMacSetup::VERSION, 1);
    }
}
