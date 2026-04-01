//!
//! Interface Class 113: Charge
//!
//! Reference: Blue Book Part 2 §5.113
//!
//! Charge defines charging rates and periods for prepaid meters.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 113: Charge
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | charge_id | 2 | unsigned | static |
/// | charge_type | 3 | enum | static |
/// | priority | 4 | unsigned | static |
/// | unit_charge_active | 5 | double-long | dynamic |
/// | total_amount_remaining | 6 | double-long | dynamic |
/// | period | 7 | double-long-unsigned | static |
/// | charge_per_unit | 8 | double-long | static |
/// | currency | 9 | unsigned | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | apply | 1 | Apply charge |
/// | reset | 2 | Reset charge |
#[derive(Debug, Clone)]
pub struct Charge {
    logical_name: ObisCode,
    charge_id: u8,
    charge_type: u8,
    priority: u8,
    unit_charge_active: i64,
    total_amount_remaining: i64,
    period: u32,
    charge_per_unit: i64,
    currency: u16,
}

impl Charge {
    /// Create a new Charge object
    pub fn new(
        logical_name: ObisCode,
        charge_id: u8,
        charge_type: u8,
        charge_per_unit: i64,
    ) -> Self {
        Self {
            logical_name,
            charge_id,
            charge_type,
            priority: 0,
            unit_charge_active: 0,
            total_amount_remaining: 0,
            period: 0,
            charge_per_unit,
            currency: 0,
        }
    }

    pub const fn get_charge_per_unit(&self) -> i64 {
        self.charge_per_unit
    }
}

impl CosemClass for Charge {
    const CLASS_ID: u16 = 113;
    const VERSION: u8 = 0;

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
            2 => Ok(DlmsType::UInt8(self.charge_id)),
            3 => Ok(DlmsType::UInt8(self.charge_type)),
            4 => Ok(DlmsType::UInt8(self.priority)),
            5 => Ok(DlmsType::Int64(self.unit_charge_active)),
            6 => Ok(DlmsType::Int64(self.total_amount_remaining)),
            7 => Ok(DlmsType::UInt32(self.period)),
            8 => Ok(DlmsType::Int64(self.charge_per_unit)),
            9 => Ok(DlmsType::UInt16(self.currency)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1..=3 => Err(CosemError::ReadOnly),
            4 => {
                if let DlmsType::UInt8(p) = value {
                    self.priority = p;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 17,
                        got: value.tag(),
                    })
                }
            }
            5 => Err(CosemError::ReadOnly),
            6 => Err(CosemError::ReadOnly),
            7 => {
                if let DlmsType::UInt32(p) = value {
                    self.period = p;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 6,
                        got: value.tag(),
                    })
                }
            }
            8 => {
                if let DlmsType::Int64(c) = value {
                    self.charge_per_unit = c;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 21,
                        got: value.tag(),
                    })
                }
            }
            9 => {
                if let DlmsType::UInt16(c) = value {
                    self.currency = c;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 18,
                        got: value.tag(),
                    })
                }
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // apply
            2 => {
                // reset
                self.unit_charge_active = 0;
                self.total_amount_remaining = 0;
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
    fn test_charge_class_id() {
        let charge = Charge::new(ObisCode::new(0, 0, 113, 0, 0, 255), 1, 0, 100);
        assert_eq!(Charge::CLASS_ID, 113);
        assert_eq!(charge.get_charge_per_unit(), 100);
    }
}
