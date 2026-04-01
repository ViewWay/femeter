//!
//! Interface Class 116: IEC 62055-41 Attributes
//!
//! Reference: Blue Book Part 2 §5.116
//!
//! IEC 62055-41 defines attributes for standard transfer specification (STS)
//! token-based prepaid systems.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 116: IEC 62055-41 Attributes
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | integrated_total | 2 | double-long | dynamic |
/// | carrier_count | 3 | unsigned | dynamic |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct Iec62055Attributes {
    logical_name: ObisCode,
    integrated_total: i64,
    carrier_count: u8,
}

impl Iec62055Attributes {
    /// Create a new IEC 62055-41 Attributes object
    pub const fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            integrated_total: 0,
            carrier_count: 0,
        }
    }

    pub const fn get_integrated_total(&self) -> i64 {
        self.integrated_total
    }

    pub const fn get_carrier_count(&self) -> u8 {
        self.carrier_count
    }
}

impl CosemClass for Iec62055Attributes {
    const CLASS_ID: u16 = 116;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        3
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Int64(self.integrated_total)),
            3 => Ok(DlmsType::UInt8(self.carrier_count)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, _value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => Err(CosemError::ReadOnly),
            3 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iec62055_class_id() {
        let iec = Iec62055Attributes::new(ObisCode::new(0, 0, 116, 0, 0, 255));
        assert_eq!(Iec62055Attributes::CLASS_ID, 116);
    }
}
