//! PRIME Application Identification Interface (IC 86)
//!
//! PRIME NB OFDM PLC application identification.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.86

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// PRIME Application Identification Interface Class (IC 86)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: vendor_id (unsigned)
/// - 3: product_id (unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct PrimeAppIdentification {
    logical_name: ObisCode,
    vendor_id: u16,
    product_id: u16,
}

impl PrimeAppIdentification {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            vendor_id: 0,
            product_id: 0,
        }
    }

    pub fn with_ids(logical_name: ObisCode, vendor_id: u16, product_id: u16) -> Self {
        Self {
            logical_name,
            vendor_id,
            product_id,
        }
    }
}

impl CosemClass for PrimeAppIdentification {
    const CLASS_ID: u16 = 86;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt16(self.vendor_id)),
            3 => Ok(DlmsType::UInt16(self.product_id)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.vendor_id = value.as_u16().ok_or(CosemError::TypeMismatch {
                    expected: 18,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.product_id = value.as_u16().ok_or(CosemError::TypeMismatch {
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
        assert_eq!(PrimeAppIdentification::CLASS_ID, 86);
    }
}
