//! S-FSK Active Initiator Interface (IC 51)
//!
//! S-FSK active initiator for PLC communication.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.51

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// S-FSK Active Initiator Interface Class (IC 51)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: primary_address (unsigned)
/// - 3: secondary_address (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SfskActiveInitiator {
    logical_name: ObisCode,
    primary_address: u16,
    secondary_address: u16,
}

impl SfskActiveInitiator {
    /// Create a new SfskActiveInitiator instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            primary_address: 0,
            secondary_address: 0,
        }
    }

    /// Create with specific addresses
    pub fn with_addresses(logical_name: ObisCode, primary: u16, secondary: u16) -> Self {
        Self {
            logical_name,
            primary_address: primary,
            secondary_address: secondary,
        }
    }

    pub fn primary_address(&self) -> u16 {
        self.primary_address
    }

    pub fn secondary_address(&self) -> u16 {
        self.secondary_address
    }
}

impl CosemClass for SfskActiveInitiator {
    const CLASS_ID: u16 = 51;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt16(self.primary_address)),
            3 => Ok(DlmsType::UInt16(self.secondary_address)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.primary_address = value.as_u16().ok_or(CosemError::TypeMismatch {
                    expected: 18,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.secondary_address = value.as_u16().ok_or(CosemError::TypeMismatch {
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
        assert_eq!(SfskActiveInitiator::CLASS_ID, 51);
    }

    #[test]
    fn test_creation() {
        let initiator =
            SfskActiveInitiator::with_addresses(ObisCode::new(0, 0, 51, 0, 0, 255), 100, 200);
        assert_eq!(initiator.primary_address(), 100);
        assert_eq!(initiator.secondary_address(), 200);
    }
}
