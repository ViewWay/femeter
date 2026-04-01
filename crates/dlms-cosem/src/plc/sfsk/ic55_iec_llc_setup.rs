//! IEC 61334-4-32 LLC Setup Interface (IC 55)
//!
//! IEC 61334-4-32 LLC setup for PLC communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.55

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// IEC 61334-4-32 LLC Setup Interface Class (IC 55)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: mac_max_fsdu (unsigned)
/// - 3: mac_tx_wait_time (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct IecLlcSetup {
    logical_name: ObisCode,
    mac_max_fsdu: u8,
    mac_tx_wait_time: u8,
}

impl IecLlcSetup {
    /// Create a new IecLlcSetup instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            mac_max_fsdu: 128,
            mac_tx_wait_time: 100,
        }
    }

    pub fn mac_max_fsdu(&self) -> u8 {
        self.mac_max_fsdu
    }

    pub fn mac_tx_wait_time(&self) -> u8 {
        self.mac_tx_wait_time
    }
}

impl CosemClass for IecLlcSetup {
    const CLASS_ID: u16 = 55;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.mac_max_fsdu)),
            3 => Ok(DlmsType::UInt8(self.mac_tx_wait_time)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.mac_max_fsdu = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.mac_tx_wait_time = value.as_u8().ok_or(CosemError::TypeMismatch {
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
        assert_eq!(IecLlcSetup::CLASS_ID, 55);
        assert_eq!(IecLlcSetup::VERSION, 1);
    }
}
