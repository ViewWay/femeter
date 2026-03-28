//! HS-PLC HDLC SSAS Setup Interface (IC 143)
//!
//! HS-PLC ISO/IEC 12139-1 HDLC SSAS setup.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.143

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// HS-PLC HDLC SSAS Setup Interface Class (IC 143)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: hdlc_address (unsigned)
/// - 3: hdlc_max_info_tx (unsigned)
/// - 4: hdlc_max_info_rx (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct HsHdlcSsasSetup {
    logical_name: ObisCode,
    hdlc_address: u16,
    hdlc_max_info_tx: u16,
    hdlc_max_info_rx: u16,
}

impl HsHdlcSsasSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            hdlc_address: 0,
            hdlc_max_info_tx: 128,
            hdlc_max_info_rx: 128,
        }
    }
}

impl CosemClass for HsHdlcSsasSetup {
    const CLASS_ID: u16 = 143;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt16(self.hdlc_address)),
            3 => Ok(DlmsType::UInt16(self.hdlc_max_info_tx)),
            4 => Ok(DlmsType::UInt16(self.hdlc_max_info_rx)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.hdlc_address = value.as_u16().ok_or(CosemError::TypeMismatch {
                    expected: 18,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.hdlc_max_info_tx = value.as_u16().ok_or(CosemError::TypeMismatch {
                    expected: 18,
                    got: value.tag(),
                })?;
                Ok(())
            }
            4 => {
                self.hdlc_max_info_rx = value.as_u16().ok_or(CosemError::TypeMismatch {
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
        assert_eq!(HsHdlcSsasSetup::CLASS_ID, 143);
    }
}
