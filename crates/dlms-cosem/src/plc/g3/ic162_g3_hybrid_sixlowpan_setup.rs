//! G3-PLC Hybrid 6LoWPAN Adaptation Layer Setup Interface (IC 162)
//!
//! G3-PLC Hybrid 6LoWPAN adaptation layer setup.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.162

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// G3-PLC Hybrid 6LoWPAN Adaptation Layer Setup Interface Class (IC 162)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: context_id (unsigned)
/// - 3: context_prefix (octet-string)
/// - 4: context_compression (boolean)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct G3HybridSixlowpanSetup {
    logical_name: ObisCode,
    context_id: u8,
    context_prefix: alloc::vec::Vec<u8>,
    context_compression: bool,
}

impl G3HybridSixlowpanSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            context_id: 0,
            context_prefix: alloc::vec::Vec::new(),
            context_compression: true,
        }
    }
}

impl CosemClass for G3HybridSixlowpanSetup {
    const CLASS_ID: u16 = 162;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.context_id)),
            3 => Ok(DlmsType::OctetString(self.context_prefix.clone())),
            4 => Ok(DlmsType::Boolean(self.context_compression)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.context_id = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                if let DlmsType::OctetString(data) = value {
                    self.context_prefix = data;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            4 => {
                self.context_compression = value.as_bool().ok_or(CosemError::TypeMismatch {
                    expected: 3,
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
        assert_eq!(G3HybridSixlowpanSetup::CLASS_ID, 162);
        assert_eq!(G3HybridSixlowpanSetup::VERSION, 1);
    }
}
