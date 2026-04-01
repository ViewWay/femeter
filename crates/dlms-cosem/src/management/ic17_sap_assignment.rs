//!
//! Interface Class 17: SAP Assignment
//!
//! Reference: Blue Book Part 2 §6.7
//!
//! SAP Assignment manages the mapping between SAP (Service Access Point) and
//! the COSEM objects.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 17: SAP Assignment
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | sap_id | 2 | unsigned | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | connect | 1 | Connect SAP |
#[derive(Debug, Clone)]
pub struct SapAssignment {
    logical_name: ObisCode,
    sap_id: u8,
}

impl SapAssignment {
    /// Create a new SAP Assignment object
    pub const fn new(logical_name: ObisCode, sap_id: u8) -> Self {
        Self {
            logical_name,
            sap_id,
        }
    }

    pub const fn get_sap_id(&self) -> u8 {
        self.sap_id
    }
}

impl CosemClass for SapAssignment {
    const CLASS_ID: u16 = 17;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        2
    }

    fn method_count() -> u8 {
        1
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.sap_id)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, _value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 2 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // connect
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sap_assignment_class_id() {
        let sa = SapAssignment::new(ObisCode::new(0, 0, 41, 0, 0, 255), 1);
        assert_eq!(SapAssignment::CLASS_ID, 17);
        assert_eq!(sa.get_sap_id(), 1);
    }
}
