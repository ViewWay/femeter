//!
//! Interface Class 62: Compact Data
//!
//! Reference: Blue Book Part 2 §5.62
//!
//! Compact Data provides space-efficient storage for regularly captured data.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 62: Compact Data
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | buffer | 2 | octet-string | dynamic |
/// | capture_objects | 3 | array of structure | static |
/// | collection_time | 4 | date-time | dynamic |
/// | data_overflow | 5 | boolean | dynamic |
/// | template_id | 6 | unsigned | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | reset | 1 | Clear the buffer |
/// | capture | 2 | Capture current values |
/// | clean_up | 3 | Clean up old data |
#[derive(Debug, Clone)]
pub struct CompactData {
    logical_name: ObisCode,
    buffer: DlmsType,
    capture_objects: DlmsType,
    collection_time: DlmsType,
    data_overflow: bool,
    template_id: u8,
}

impl CompactData {
    /// Create a new Compact Data object
    pub fn new(
        logical_name: ObisCode,
        capture_objects: DlmsType,
        template_id: u8,
    ) -> Self {
        Self {
            logical_name,
            buffer: DlmsType::OctetString(alloc::vec![]),
            capture_objects,
            collection_time: DlmsType::Null,
            data_overflow: false,
            template_id,
        }
    }

    pub const fn get_template_id(&self) -> u8 {
        self.template_id
    }
}

impl CosemClass for CompactData {
    const CLASS_ID: u16 = 62;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        6
    }

    fn method_count() -> u8 {
        3
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.buffer.clone()),
            3 => Ok(self.capture_objects.clone()),
            4 => Ok(self.collection_time.clone()),
            5 => Ok(DlmsType::Boolean(self.data_overflow)),
            6 => Ok(DlmsType::UInt8(self.template_id)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 3 | 6 => Err(CosemError::ReadOnly),
            2 => {
                self.buffer = value;
                Ok(())
            }
            4 => {
                self.collection_time = value;
                Ok(())
            }
            5 => {
                if let DlmsType::Boolean(overflow) = value {
                    self.data_overflow = overflow;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 3, // boolean
                        got: value.tag(),
                    })
                }
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                // reset
                self.buffer = DlmsType::OctetString(alloc::vec![]);
                self.data_overflow = false;
                Ok(DlmsType::Null)
            }
            2 => {
                // capture
                Ok(DlmsType::Null)
            }
            3 => {
                // clean_up
                Ok(DlmsType::Null)
            }
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_data_class_id() {
        let _cd = CompactData::new(
            ObisCode::new(0, 0, 62, 0, 0, 255),
            DlmsType::Array(alloc::vec![]),
            1,
        );
        assert_eq!(CompactData::CLASS_ID, 62);
        assert_eq!(CompactData::VERSION, 1);
        assert_eq!(CompactData::method_count(), 3);
    }
}
