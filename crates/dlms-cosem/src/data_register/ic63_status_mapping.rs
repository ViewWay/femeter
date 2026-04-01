//!
//! Interface Class 63: Status Mapping
//!
//! Reference: Blue Book Part 2 §5.63
//!
//! Status Mapping provides a mapping between internal status codes and
//! their meanings/representations.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 63: Status Mapping
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | mapping_table | 2 | array of structure | static |
/// | status_object_reference | 3 | octet-string | static |
/// | mapped_status | 4 | unsigned | dynamic |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct StatusMapping {
    logical_name: ObisCode,
    mapping_table: DlmsType,
    status_object_reference: DlmsType,
    mapped_status: u8,
}

impl StatusMapping {
    /// Create a new Status Mapping object
    pub fn new(
        logical_name: ObisCode,
        mapping_table: DlmsType,
        status_object_reference: DlmsType,
    ) -> Self {
        Self {
            logical_name,
            mapping_table,
            status_object_reference,
            mapped_status: 0,
        }
    }

    pub const fn get_mapped_status(&self) -> u8 {
        self.mapped_status
    }
}

impl CosemClass for StatusMapping {
    const CLASS_ID: u16 = 63;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        4
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.mapping_table.clone()),
            3 => Ok(self.status_object_reference.clone()),
            4 => Ok(DlmsType::UInt8(self.mapped_status)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1..=3 => Err(CosemError::ReadOnly),
            4 => {
                if let DlmsType::UInt8(status) = value {
                    self.mapped_status = status;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 17,
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
    fn test_status_mapping_class_id() {
        let _sm = StatusMapping::new(
            ObisCode::new(0, 0, 63, 0, 0, 255),
            DlmsType::Array(alloc::vec![]),
            DlmsType::OctetString(alloc::vec![]),
        );
        assert_eq!(StatusMapping::CLASS_ID, 63);
    }
}
