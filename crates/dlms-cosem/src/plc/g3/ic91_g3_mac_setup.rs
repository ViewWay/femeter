//! G3-PLC MAC Setup Interface (IC 91)
//!
//! G3-PLC MAC layer setup.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.91

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// G3-PLC MAC Setup Interface Class (IC 91)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: mac_node_type (enum)
/// - 3: mac_address (unsigned)
/// - 4: max_connections (unsigned)
/// - 5: repetitions_number (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct G3MacSetup {
    logical_name: ObisCode,
    mac_node_type: u8,
    mac_address: u16,
    max_connections: u8,
    repetitions_number: u8,
}

impl G3MacSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            mac_node_type: 0,
            mac_address: 0,
            max_connections: 8,
            repetitions_number: 3,
        }
    }
}

impl CosemClass for G3MacSetup {
    const CLASS_ID: u16 = 91;
    const VERSION: u8 = 4;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Enum(self.mac_node_type)),
            3 => Ok(DlmsType::UInt16(self.mac_address)),
            4 => Ok(DlmsType::UInt8(self.max_connections)),
            5 => Ok(DlmsType::UInt8(self.repetitions_number)),
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
            4 => {
                self.max_connections = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            5 => {
                self.repetitions_number = value.as_u8().ok_or(CosemError::TypeMismatch {
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
        5
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
        assert_eq!(G3MacSetup::CLASS_ID, 91);
        assert_eq!(G3MacSetup::VERSION, 4);
    }
}
