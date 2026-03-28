//! S-FSK Phy&MAC Setup Interface (IC 50)
//!
//! S-FSK Physical and MAC layer setup for PLC communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.50

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// S-FSK Phy&MAC Setup Interface Class (IC 50)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: mac_clock_presc (unsigned)
/// - 3: mac_repeat_ind (unsigned)
/// - 4: sub_class_rep (unsigned)
/// - 5: tone_map (bit-string)
/// - 6: control_field (unsigned)
/// - 7: response_wait_time (unsigned)
/// - 8: crc_type (enum)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SfskPhyMacSetup {
    logical_name: ObisCode,
    mac_clock_presc: u8,
    mac_repeat_ind: u8,
    sub_class_rep: u8,
    tone_map: u8,
    control_field: u8,
    response_wait_time: u8,
    crc_type: u8,
}

impl SfskPhyMacSetup {
    /// Create a new SfskPhyMacSetup instance with default values
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            mac_clock_presc: 0,
            mac_repeat_ind: 0,
            sub_class_rep: 0,
            tone_map: 0,
            control_field: 0,
            response_wait_time: 100,
            crc_type: 0,
        }
    }

    /// Create a new SfskPhyMacSetup instance with specific values
    pub fn with_params(
        logical_name: ObisCode,
        mac_clock_presc: u8,
        mac_repeat_ind: u8,
        sub_class_rep: u8,
        tone_map: u8,
        control_field: u8,
        response_wait_time: u8,
        crc_type: u8,
    ) -> Self {
        Self {
            logical_name,
            mac_clock_presc,
            mac_repeat_ind,
            sub_class_rep,
            tone_map,
            control_field,
            response_wait_time,
            crc_type,
        }
    }

    // Getters
    pub fn mac_clock_presc(&self) -> u8 {
        self.mac_clock_presc
    }

    pub fn mac_repeat_ind(&self) -> u8 {
        self.mac_repeat_ind
    }

    pub fn sub_class_rep(&self) -> u8 {
        self.sub_class_rep
    }

    pub fn tone_map(&self) -> u8 {
        self.tone_map
    }

    pub fn control_field(&self) -> u8 {
        self.control_field
    }

    pub fn response_wait_time(&self) -> u8 {
        self.response_wait_time
    }

    pub fn crc_type(&self) -> u8 {
        self.crc_type
    }
}

impl CosemClass for SfskPhyMacSetup {
    const CLASS_ID: u16 = 50;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.mac_clock_presc)),
            3 => Ok(DlmsType::UInt8(self.mac_repeat_ind)),
            4 => Ok(DlmsType::UInt8(self.sub_class_rep)),
            5 => Ok(DlmsType::BitString(alloc::vec![self.tone_map])),
            6 => Ok(DlmsType::UInt8(self.control_field)),
            7 => Ok(DlmsType::UInt8(self.response_wait_time)),
            8 => Ok(DlmsType::Enum(self.crc_type)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.mac_clock_presc = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17, // unsigned
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.mac_repeat_ind = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            4 => {
                self.sub_class_rep = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            5 => {
                if let DlmsType::BitString(data) = value {
                    self.tone_map = data.first().copied().unwrap_or(0);
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 4, // bit-string
                        got: value.tag(),
                    })
                }
            }
            6 => {
                self.control_field = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            7 => {
                self.response_wait_time = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 17,
                    got: value.tag(),
                })?;
                Ok(())
            }
            8 => {
                self.crc_type = value.as_enum().ok_or(CosemError::TypeMismatch {
                    expected: 6, // enum
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
        8
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
        assert_eq!(SfskPhyMacSetup::CLASS_ID, 50);
        assert_eq!(SfskPhyMacSetup::VERSION, 1);
    }

    #[test]
    fn test_creation() {
        let setup = SfskPhyMacSetup::new(ObisCode::new(0, 0, 50, 0, 0, 255));
        assert_eq!(setup.logical_name(), &ObisCode::new(0, 0, 50, 0, 0, 255));
    }

    #[test]
    fn test_get_attributes() {
        let setup = SfskPhyMacSetup::new(ObisCode::new(0, 0, 50, 0, 0, 255));
        assert!(setup.get_attribute(1).is_ok());
        assert!(setup.get_attribute(2).is_ok());
        assert!(setup.get_attribute(9).is_err());
    }

    #[test]
    fn test_set_attributes() {
        let mut setup = SfskPhyMacSetup::new(ObisCode::new(0, 0, 50, 0, 0, 255));
        assert!(setup.set_attribute(2, DlmsType::UInt8(42)).is_ok());
        assert_eq!(setup.mac_clock_presc(), 42);
    }
}
