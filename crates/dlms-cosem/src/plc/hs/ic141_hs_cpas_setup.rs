//! HS-PLC CPAS Setup Interface (IC 141)
//!
//! HS-PLC ISO/IEC 12139-1 CPAS setup.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.141

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// HS-PLC CPAS Setup Interface Class (IC 141)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: cpas_address (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct HsCpasSetup {
    logical_name: ObisCode,
    cpas_address: u16,
}

impl HsCpasSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            cpas_address: 0,
        }
    }
}

impl CosemClass for HsCpasSetup {
    const CLASS_ID: u16 = 141;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt16(self.cpas_address)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.cpas_address = value.as_u16().ok_or(CosemError::TypeMismatch {
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
        2
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
        assert_eq!(HsCpasSetup::CLASS_ID, 141);
    }
}
