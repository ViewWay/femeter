//!
//! Interface Class 4: Extended Register
//!
//! Reference: Blue Book Part 2 §5.4
//!
//! Extended Register adds status and capture_time to Register, providing
//! additional metadata about when the value was captured and its validity.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::{CosemDateTime, DlmsType},
    units::Unit,
};

/// COSEM IC 4: Extended Register
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | value | 2 | CHOICE (numeric) | dynamic |
/// | scaler_unit | 3 | structure{scaler, unit} | static |
/// | status | 4 | unsigned | dynamic |
/// | capture_time | 5 | date-time | dynamic |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct ExtendedRegister {
    logical_name: ObisCode,
    value: DlmsType,
    scaler: i8,
    unit: Unit,
    status: u8,
    capture_time: CosemDateTime,
}

impl ExtendedRegister {
    /// Create a new Extended Register object
    pub const fn new(logical_name: ObisCode, scaler: i8, unit: Unit) -> Self {
        Self {
            logical_name,
            value: DlmsType::Null,
            scaler,
            unit,
            status: 0,
            capture_time: CosemDateTime {
                date: dlms_core::types::CosemDate {
                    year: 0xFFFF,
                    month: 0xFF,
                    day: 0xFF,
                    day_of_week: 0xFF,
                },
                time: dlms_core::types::CosemTime {
                    hour: 0xFF,
                    minute: 0xFF,
                    second: 0xFF,
                    hundredths: 0xFF,
                },
                deviation: -32768, // 0x8000 = not specified for i16
                clock_status: 0,
            },
        }
    }

    pub fn get_value(&self) -> &DlmsType {
        &self.value
    }

    pub fn set_value(&mut self, value: DlmsType) {
        self.value = value;
    }

    pub fn get_status(&self) -> u8 {
        self.status
    }

    pub fn set_status(&mut self, status: u8) {
        self.status = status;
    }

    pub fn get_capture_time(&self) -> &CosemDateTime {
        &self.capture_time
    }

    pub fn set_capture_time(&mut self, time: CosemDateTime) {
        self.capture_time = time;
    }

    pub const fn get_scaler(&self) -> i8 {
        self.scaler
    }

    pub const fn get_unit(&self) -> Unit {
        self.unit
    }
}

impl CosemClass for ExtendedRegister {
    const CLASS_ID: u16 = 4;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        5
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.value.clone()),
            3 => Ok(DlmsType::Structure(alloc::vec![
                DlmsType::Int8(self.scaler),
                DlmsType::UInt16(self.unit as u16),
            ])),
            4 => Ok(DlmsType::UInt8(self.status)),
            5 => Ok(DlmsType::DateTime(self.capture_time)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.value = value;
                Ok(())
            }
            3 => Err(CosemError::ReadOnly),
            4 => {
                if let DlmsType::UInt8(s) = value {
                    self.status = s;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 17, // unsigned
                        got: value.tag(),
                    })
                }
            }
            5 => {
                if let DlmsType::DateTime(dt) = value {
                    self.capture_time = dt;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 25, // date-time
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
    fn test_extended_register_class_id() {
        let reg = ExtendedRegister::new(ObisCode::new(1, 0, 1, 8, 0, 255), -3, Unit::WattHour);
        assert_eq!(ExtendedRegister::CLASS_ID, 4);
        assert_eq!(ExtendedRegister::attribute_count(), 5);
    }

    #[test]
    fn test_extended_register_status() {
        let mut _reg = ExtendedRegister::new(ObisCode::new(1, 0, 1, 8, 0, 255), 0, Unit::WattHour);
        _reg.set_status(0x42);
        assert_eq!(_reg.get_status(), 0x42);
    }
}
