//! PRIME MAC Functional Parameters Interface (IC 83)
//!
//! PRIME NB OFDM PLC MAC functional parameters.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.83

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// PRIME MAC Functional Parameters Interface Class (IC 83)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: max_connections (unsigned)
/// - 3: repetitions_number (unsigned)
/// - 4: csma_ca_min_backoff (unsigned)
/// - 5: csma_ca_max_backoff (unsigned)
/// - 6: max_csma_ca_nb (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct PrimeMacFunctionalParams {
    logical_name: ObisCode,
    max_connections: u8,
    repetitions_number: u8,
    csma_ca_min_backoff: u8,
    csma_ca_max_backoff: u8,
    max_csma_ca_nb: u8,
}

impl PrimeMacFunctionalParams {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            max_connections: 8,
            repetitions_number: 3,
            csma_ca_min_backoff: 0,
            csma_ca_max_backoff: 10,
            max_csma_ca_nb: 5,
        }
    }
}

impl CosemClass for PrimeMacFunctionalParams {
    const CLASS_ID: u16 = 83;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.max_connections)),
            3 => Ok(DlmsType::UInt8(self.repetitions_number)),
            4 => Ok(DlmsType::UInt8(self.csma_ca_min_backoff)),
            5 => Ok(DlmsType::UInt8(self.csma_ca_max_backoff)),
            6 => Ok(DlmsType::UInt8(self.max_csma_ca_nb)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.max_connections = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.repetitions_number = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            4 => {
                self.csma_ca_min_backoff = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            5 => {
                self.csma_ca_max_backoff = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            6 => {
                self.max_csma_ca_nb = value.as_u8().ok_or(CosemError::TypeMismatch {
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
        6
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
        assert_eq!(PrimeMacFunctionalParams::CLASS_ID, 83);
    }
}
