//!
//! Interface Class 7: Profile Generic
//!
//! Reference: Blue Book Part 2 §5.7
//!
//! The Profile Generic (Load Profile) is the most complex IC in Group 1.
//! It stores time-series data captured at regular intervals, typically used
//! for load profiling (e.g., 15-minute interval power consumption).

use alloc::vec::Vec;

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// Sort method for profile entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SortMethod {
    /// First in, first out
    Fifo = 0,
    /// Last entry is oldest
    Lifo = 1,
}

/// Capture object definition
#[derive(Debug, Clone)]
pub struct CaptureObject {
    /// OBIS code of the object to capture
    pub obis: ObisCode,
    /// Attribute ID to capture
    pub attribute_id: u8,
    /// Data index (for array attributes)
    pub data_index: u8,
}

/// COSEM IC 7: Profile Generic
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | buffer | 2 | array of structure | dynamic |
/// | capture_objects | 3 | array of structure | static |
/// | capture_period | 4 | double-long-unsigned | static |
/// | sort_method | 5 | enum | static |
/// | sort_object | 6 | structure | static |
/// | sort_offset | 7 | double-long-unsigned | static |
/// | entries_in_use | 8 | double-long-unsigned | dynamic |
/// | profile_entries | 9 | double-long-unsigned | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | reset | 1 | Clear the buffer |
/// | capture | 2 | Capture current values |
#[derive(Debug, Clone)]
pub struct ProfileGeneric {
    logical_name: ObisCode,
    /// Buffer containing profile entries
    buffer: DlmsType,
    /// Objects to capture in each entry
    capture_objects: Vec<CaptureObject>,
    /// Capture period in seconds (0 = on demand)
    capture_period: u32,
    /// Sorting method
    sort_method: SortMethod,
    /// Object used for sorting
    sort_object: DlmsType,
    /// Offset for sorting
    sort_offset: u32,
    /// Maximum number of entries
    profile_entries: u32,
}

impl ProfileGeneric {
    /// Create a new Profile Generic object
    pub fn new(
        logical_name: ObisCode,
        capture_objects: Vec<CaptureObject>,
        capture_period: u32,
        sort_method: SortMethod,
        profile_entries: u32,
    ) -> Self {
        Self {
            logical_name,
            buffer: DlmsType::Array(alloc::vec![]),
            capture_objects,
            capture_period,
            sort_method,
            sort_object: DlmsType::Null,
            sort_offset: 0,
            profile_entries,
        }
    }

    /// Get the number of entries currently in use
    pub fn entries_in_use(&self) -> u32 {
        if let DlmsType::Array(entries) = &self.buffer {
            entries.len() as u32
        } else {
            0
        }
    }

    /// Set the buffer content
    pub fn set_buffer(&mut self, buffer: DlmsType) {
        self.buffer = buffer;
    }

    /// Get capture objects
    pub fn get_capture_objects(&self) -> &[CaptureObject] {
        &self.capture_objects
    }
}

impl CosemClass for ProfileGeneric {
    const CLASS_ID: u16 = 7;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        9
    }

    fn method_count() -> u8 {
        2
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.buffer.clone()),
            3 => {
                // Convert capture_objects to DLMS format
                let objects: alloc::vec::Vec<DlmsType> = self
                    .capture_objects
                    .iter()
                    .map(|co| {
                        DlmsType::Structure(alloc::vec![
                            DlmsType::OctetString(co.obis.to_bytes().to_vec()),
                            DlmsType::UInt8(co.attribute_id),
                            DlmsType::UInt8(co.data_index),
                        ])
                    })
                    .collect();
                Ok(DlmsType::Array(objects))
            }
            4 => Ok(DlmsType::UInt32(self.capture_period)),
            5 => Ok(DlmsType::UInt8(self.sort_method as u8)),
            6 => Ok(self.sort_object.clone()),
            7 => Ok(DlmsType::UInt32(self.sort_offset)),
            8 => Ok(DlmsType::UInt32(self.entries_in_use())),
            9 => Ok(DlmsType::UInt32(self.profile_entries)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 3 | 5 | 6 | 7 | 8 | 9 => Err(CosemError::ReadOnly),
            2 => {
                self.buffer = value;
                Ok(())
            }
            4 => {
                if let DlmsType::UInt32(period) = value {
                    self.capture_period = period;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 6, // double-long-unsigned
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
                // reset: Clear the buffer
                self.buffer = DlmsType::Array(alloc::vec![]);
                Ok(DlmsType::Null)
            }
            2 => {
                // capture: Capture current values (placeholder)
                // In real implementation, this would read from capture_objects
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
    fn test_profile_generic_class_id() {
        let _pg = ProfileGeneric::new(
            ObisCode::new(1, 0, 99, 1, 0, 255),
            alloc::vec![],
            900,
            SortMethod::Fifo,
            1000,
        );
        assert_eq!(ProfileGeneric::CLASS_ID, 7);
        assert_eq!(ProfileGeneric::VERSION, 1);
        assert_eq!(ProfileGeneric::attribute_count(), 9);
        assert_eq!(ProfileGeneric::method_count(), 2);
    }

    #[test]
    fn test_profile_generic_entries_in_use() {
        let mut pg = ProfileGeneric::new(
            ObisCode::new(1, 0, 99, 1, 0, 255),
            alloc::vec![],
            900,
            SortMethod::Fifo,
            1000,
        );
        assert_eq!(pg.entries_in_use(), 0);

        // Add some entries
        let entries = alloc::vec![DlmsType::Structure(alloc::vec![]), DlmsType::Structure(alloc::vec![])];
        pg.set_buffer(DlmsType::Array(entries));
        assert_eq!(pg.entries_in_use(), 2);
    }

    #[test]
    fn test_profile_generic_reset() {
        let mut pg = ProfileGeneric::new(
            ObisCode::new(1, 0, 99, 1, 0, 255),
            alloc::vec![],
            900,
            SortMethod::Fifo,
            1000,
        );
        let entries = alloc::vec![DlmsType::Structure(alloc::vec![])];
        pg.set_buffer(DlmsType::Array(entries));
        assert_eq!(pg.entries_in_use(), 1);

        pg.execute_method(1, DlmsType::Null).unwrap();
        assert_eq!(pg.entries_in_use(), 0);
    }
}
