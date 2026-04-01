//! RPL Diagnostic Interface (IC 97)
//!
//! RPL (Routing Protocol for Low-Power and Lossy Networks) diagnostic.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.97

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// RPL Diagnostic Interface Class (IC 97)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: rank (unsigned)
/// - 3: parent_address (octet-string)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct RplDiagnostic {
    logical_name: ObisCode,
    rank: u16,
    parent_address: alloc::vec::Vec<u8>,
}

impl RplDiagnostic {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            rank: 0,
            parent_address: alloc::vec::Vec::new(),
        }
    }
}

impl CosemClass for RplDiagnostic {
    const CLASS_ID: u16 = 97;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt16(self.rank)),
            3 => Ok(DlmsType::OctetString(self.parent_address.clone())),
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
        assert_eq!(RplDiagnostic::CLASS_ID, 97);
    }
}
