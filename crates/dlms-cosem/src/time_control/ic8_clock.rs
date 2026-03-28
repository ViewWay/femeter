//!
//! Interface Class 8: Clock
//!
//! Reference: Blue Book Part 2 §5.8
//!
//! The Clock interface class provides time and calendar functions for the meter.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::{CosemDate, CosemTime, DlmsType},
};

/// COSEM IC 8: Clock
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | time | 2 | time | dynamic |
/// | timezone | 3 | long | static |
/// | status | 4 | unsigned | dynamic |
/// | daylight_savings_begin | 5 | date | static |
/// | daylight_savings_end | 6 | date | static |
/// | daylight_savings_deviation | 7 | long | static |
/// | daylight_savings_enabled | 8 | boolean | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | adjust_to_quarter | 1 | Adjust time to nearest quarter hour |
/// | adjust_to_measuring_period | 2 | Adjust to measuring period |
/// | adjust_to_minute | 3 | Adjust to nearest minute |
/// | adjust_to_hour | 4 | Adjust to nearest hour |
/// | adjust_to_day | 5 | Adjust to nearest day |
/// | adjust_to_first_day_of_month | 6 | Adjust to first day of month |
/// | adjust_to_first_day_of_year | 7 | Adjust to first day of year |
/// | shift_timezone | 8 | Shift timezone |
#[derive(Debug, Clone)]
pub struct Clock {
    logical_name: ObisCode,
    time: CosemTime,
    timezone: i16,
    status: u8,
    daylight_savings_begin: CosemDate,
    daylight_savings_end: CosemDate,
    daylight_savings_deviation: i16,
    daylight_savings_enabled: bool,
}

impl Clock {
    /// Create a new Clock object
    pub fn new(logical_name: ObisCode, timezone: i16) -> Self {
        Self {
            logical_name,
            time: CosemTime {
                hour: 0xFF,
                minute: 0xFF,
                second: 0xFF,
                hundredths: 0xFF,
            },
            timezone,
            status: 0,
            daylight_savings_begin: CosemDate {
                year: 0xFFFF,
                month: 0xFF,
                day: 0xFF,
                day_of_week: 0xFF,
            },
            daylight_savings_end: CosemDate {
                year: 0xFFFF,
                month: 0xFF,
                day: 0xFF,
                day_of_week: 0xFF,
            },
            daylight_savings_deviation: 0,
            daylight_savings_enabled: false,
        }
    }

    pub const fn get_timezone(&self) -> i16 {
        self.timezone
    }

    pub const fn get_status(&self) -> u8 {
        self.status
    }
}

impl CosemClass for Clock {
    const CLASS_ID: u16 = 8;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        8
    }

    fn method_count() -> u8 {
        8
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Time(self.time)),
            3 => Ok(DlmsType::Int16(self.timezone)),
            4 => Ok(DlmsType::UInt8(self.status)),
            5 => Ok(DlmsType::Date(self.daylight_savings_begin)),
            6 => Ok(DlmsType::Date(self.daylight_savings_end)),
            7 => Ok(DlmsType::Int16(self.daylight_savings_deviation)),
            8 => Ok(DlmsType::Boolean(self.daylight_savings_enabled)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                if let DlmsType::Time(t) = value {
                    self.time = t;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 27,
                        got: value.tag(),
                    })
                }
            }
            3 => {
                if let DlmsType::Int16(tz) = value {
                    self.timezone = tz;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 16,
                        got: value.tag(),
                    })
                }
            }
            4 => Err(CosemError::ReadOnly),
            5 => {
                if let DlmsType::Date(d) = value {
                    self.daylight_savings_begin = d;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 26,
                        got: value.tag(),
                    })
                }
            }
            6 => {
                if let DlmsType::Date(d) = value {
                    self.daylight_savings_end = d;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 26,
                        got: value.tag(),
                    })
                }
            }
            7 => {
                if let DlmsType::Int16(d) = value {
                    self.daylight_savings_deviation = d;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 16,
                        got: value.tag(),
                    })
                }
            }
            8 => {
                if let DlmsType::Boolean(e) = value {
                    self.daylight_savings_enabled = e;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 3,
                        got: value.tag(),
                    })
                }
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 => Ok(DlmsType::Null),
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_class_id() {
        let clock = Clock::new(ObisCode::new(0, 0, 1, 0, 0, 255), 60);
        assert_eq!(Clock::CLASS_ID, 8);
        assert_eq!(Clock::method_count(), 8);
        assert_eq!(clock.get_timezone(), 60);
    }
}
