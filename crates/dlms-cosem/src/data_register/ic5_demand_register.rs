//!
//! Interface Class 5: Demand Register
//!
//! Reference: Blue Book Part 2 §5.5
//!
//! The Demand Register stores periodic demand values (average power over a time period).

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::{CosemDateTime, DlmsType},
    units::Unit,
};

/// COSEM IC 5: Demand Register
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | current_average_value | 2 | CHOICE (numeric) | dynamic |
/// | last_average_value | 3 | CHOICE (numeric) | dynamic |
/// | scaler_unit | 4 | structure{scaler, unit} | static |
/// | status | 5 | unsigned | dynamic |
/// | capture_time | 6 | date-time | dynamic |
/// | start_time | 7 | date-time | dynamic |
/// | period | 8 | double-long-unsigned | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | reset | 1 | Reset the demand register |
/// | next_period | 2 | Move to next period |
#[derive(Debug, Clone)]
pub struct DemandRegister {
    logical_name: ObisCode,
    current_average_value: DlmsType,
    last_average_value: DlmsType,
    scaler: i8,
    unit: Unit,
    status: u8,
    capture_time: CosemDateTime,
    start_time: CosemDateTime,
    period: u32,
}

impl DemandRegister {
    /// Create a new Demand Register object
    pub fn new(logical_name: ObisCode, scaler: i8, unit: Unit, period: u32) -> Self {
        let default_time = CosemDateTime {
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
            deviation: -32768,  // 0x8000 = not specified for i16
            clock_status: 0,
        };
        Self {
            logical_name,
            current_average_value: DlmsType::Null,
            last_average_value: DlmsType::Null,
            scaler,
            unit,
            status: 0,
            capture_time: default_time,
            start_time: default_time,
            period,
        }
    }

    pub const fn get_period(&self) -> u32 {
        self.period
    }

    pub const fn get_scaler(&self) -> i8 {
        self.scaler
    }

    pub const fn get_unit(&self) -> Unit {
        self.unit
    }
}

impl CosemClass for DemandRegister {
    const CLASS_ID: u16 = 5;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        8
    }

    fn method_count() -> u8 {
        2
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.current_average_value.clone()),
            3 => Ok(self.last_average_value.clone()),
            4 => Ok(DlmsType::Structure(alloc::vec![
                DlmsType::Int8(self.scaler),
                DlmsType::UInt16(self.unit as u16),
            ])),
            5 => Ok(DlmsType::UInt8(self.status)),
            6 => Ok(DlmsType::DateTime(self.capture_time)),
            7 => Ok(DlmsType::DateTime(self.start_time)),
            8 => Ok(DlmsType::UInt32(self.period)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 3 | 4 | 8 => Err(CosemError::ReadOnly),
            2 => {
                self.current_average_value = value;
                Ok(())
            }
            5 => {
                if let DlmsType::UInt8(s) = value {
                    self.status = s;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 17,
                        got: value.tag(),
                    })
                }
            }
            6 => {
                if let DlmsType::DateTime(dt) = value {
                    self.capture_time = dt;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 25,
                        got: value.tag(),
                    })
                }
            }
            7 => {
                if let DlmsType::DateTime(dt) = value {
                    self.start_time = dt;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 25,
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
                // reset: Reset the demand register
                self.current_average_value = DlmsType::Null;
                self.last_average_value = DlmsType::Null;
                self.status = 0;
                Ok(DlmsType::Null)
            }
            2 => {
                // next_period: Move to next period
                self.last_average_value = self.current_average_value.clone();
                self.current_average_value = DlmsType::Null;
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
    fn test_demand_register_class_id() {
        let reg = DemandRegister::new(ObisCode::new(1, 0, 1, 6, 0, 255), 0, Unit::Watt, 900);
        assert_eq!(DemandRegister::CLASS_ID, 5);
        assert_eq!(DemandRegister::method_count(), 2);
        assert_eq!(reg.get_period(), 900); // 15 minutes
    }

    #[test]
    fn test_demand_register_reset() {
        let mut reg = DemandRegister::new(
            ObisCode::new(1, 0, 1, 6, 0, 255),
            0,
            Unit::Watt,
            900,
        );
        reg.current_average_value = DlmsType::UInt32(1000);
        let result = reg.execute_method(1, DlmsType::Null).unwrap();
        assert_eq!(result, DlmsType::Null);
        assert_eq!(reg.current_average_value, DlmsType::Null);
    }

    #[test]
    fn test_demand_register_next_period() {
        let mut reg = DemandRegister::new(
            ObisCode::new(1, 0, 1, 6, 0, 255),
            0,
            Unit::Watt,
            900,
        );
        reg.current_average_value = DlmsType::UInt32(1000);
        reg.last_average_value = DlmsType::UInt32(900);

        reg.execute_method(2, DlmsType::Null).unwrap();
        assert_eq!(reg.last_average_value, DlmsType::UInt32(1000));
        assert_eq!(reg.current_average_value, DlmsType::Null);
    }
}
