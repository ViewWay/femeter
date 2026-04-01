//!
//! Interface Class 66: Measurement Data Monitoring Objects
//!
//! Reference: Blue Book Part 2 §5.66
//!
//! Measurement Data provides monitoring of measurement values and their status.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 66: Measurement Data Monitoring Objects
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | object_reference | 2 | octet-string | static |
/// | measurement_type | 3 | enum | static |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct MeasurementData {
    logical_name: ObisCode,
    object_reference: DlmsType,
    measurement_type: u8,
}

impl MeasurementData {
    /// Create a new Measurement Data object
    pub fn new(logical_name: ObisCode, object_reference: DlmsType, measurement_type: u8) -> Self {
        Self {
            logical_name,
            object_reference,
            measurement_type,
        }
    }

    pub const fn get_measurement_type(&self) -> u8 {
        self.measurement_type
    }
}

impl CosemClass for MeasurementData {
    const CLASS_ID: u16 = 66;
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
            2 => Ok(self.object_reference.clone()),
            3 => Ok(DlmsType::UInt8(self.measurement_type)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.object_reference = value;
                Ok(())
            }
            3 => {
                if let DlmsType::UInt8(mt) = value {
                    self.measurement_type = mt;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 22,
                        got: value.tag(),
                    })
                }
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measurement_data_class_id() {
        let md = MeasurementData::new(
            ObisCode::new(0, 0, 66, 0, 0, 255),
            DlmsType::OctetString(alloc::vec![]),
            0,
        );
        assert_eq!(MeasurementData::CLASS_ID, 66);
        assert_eq!(md.get_measurement_type(), 0);
    }
}
