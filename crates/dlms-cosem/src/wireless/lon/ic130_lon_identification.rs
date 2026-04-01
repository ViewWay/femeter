//! ISO/IEC 14908 Identification Interface (IC 130)
//!
//! LON identification information.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.130

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// ISO/IEC 14908 Identification Interface Class (IC 130)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: neuron_id (octet-string)
/// - 3: domain_address (octet-string)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct LonIdentification {
    logical_name: ObisCode,
    neuron_id: alloc::vec::Vec<u8>,
    domain_address: alloc::vec::Vec<u8>,
}

impl LonIdentification {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            neuron_id: alloc::vec::Vec::new(),
            domain_address: alloc::vec::Vec::new(),
        }
    }
}

impl CosemClass for LonIdentification {
    const CLASS_ID: u16 = 130;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::OctetString(self.neuron_id.clone())),
            3 => Ok(DlmsType::OctetString(self.domain_address.clone())),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, _id: u8, _value: DlmsType) -> Result<(), CosemError> {
        Err(CosemError::ReadOnly)
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
        assert_eq!(LonIdentification::CLASS_ID, 130);
    }
}
