//!
//! Interface Class 40: Push Setup
//!
//! Reference: Blue Book Part 2 §6.40
//!
//! Push Setup manages the push (event notification) configuration for
//! proactive data transmission from meter to head-end system.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// Push object (destination for push messages)
#[derive(Debug, Clone)]
pub struct PushObject {
    /// OBIS code of the destination object
    pub obis: ObisCode,
    /// Attribute ID
    pub attribute_id: u8,
    /// Data index
    pub data_index: u8,
}

/// COSEM IC 40: Push Setup
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | push_object_list | 2 | array of structure | static |
/// | send_destination_and_method | 3 | structure | static |
/// | communication_window | 4 | structure | static |
/// | random_window_start | 5 | double-long-unsigned | static |
/// | number_of_retries | 6 | unsigned | static |
/// | repetition_delay | 7 | double-long-unsigned | static |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct PushSetup {
    logical_name: ObisCode,
    push_object_list: DlmsType,
    send_destination_and_method: DlmsType,
    communication_window: DlmsType,
    random_window_start: u32,
    number_of_retries: u8,
    repetition_delay: u32,
}

impl PushSetup {
    /// Create a new Push Setup object
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            push_object_list: DlmsType::Array(alloc::vec![]),
            send_destination_and_method: DlmsType::Structure(alloc::vec![]),
            communication_window: DlmsType::Structure(alloc::vec![]),
            random_window_start: 0,
            number_of_retries: 3,
            repetition_delay: 60,
        }
    }

    pub const fn get_number_of_retries(&self) -> u8 {
        self.number_of_retries
    }
}

impl CosemClass for PushSetup {
    const CLASS_ID: u16 = 40;
    const VERSION: u8 = 3;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        7
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.push_object_list.clone()),
            3 => Ok(self.send_destination_and_method.clone()),
            4 => Ok(self.communication_window.clone()),
            5 => Ok(DlmsType::UInt32(self.random_window_start)),
            6 => Ok(DlmsType::UInt8(self.number_of_retries)),
            7 => Ok(DlmsType::UInt32(self.repetition_delay)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.push_object_list = value;
                Ok(())
            }
            3 => {
                self.send_destination_and_method = value;
                Ok(())
            }
            4 => {
                self.communication_window = value;
                Ok(())
            }
            5 => {
                if let DlmsType::UInt32(val) = value {
                    self.random_window_start = val;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 6,
                        got: value.tag(),
                    })
                }
            }
            6 => {
                if let DlmsType::UInt8(val) = value {
                    self.number_of_retries = val;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 17,
                        got: value.tag(),
                    })
                }
            }
            7 => {
                if let DlmsType::UInt32(val) = value {
                    self.repetition_delay = val;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 6,
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
    fn test_push_setup_class_id() {
        let ps = PushSetup::new(ObisCode::new(0, 0, 96, 10, 0, 255));
        assert_eq!(PushSetup::CLASS_ID, 40);
        assert_eq!(PushSetup::VERSION, 3);
    }
}
