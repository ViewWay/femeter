//!
//! Interface Class 6: Register Activation
//!
//! Reference: Blue Book Part 2 §5.6
//!
//! Register Activation controls which register(s) are active for a specific
//! tariff or billing period.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM IC 6: Register Activation
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | registers_assigned | 2 | array of structure | static |
/// | tariff_rate | 3 | unsigned | static |
/// | activation_time | 4 | date-time | static |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct RegisterActivation {
    logical_name: ObisCode,
    /// List of registers assigned to this activation
    registers_assigned: DlmsType,
    /// Tariff rate identifier
    tariff_rate: u8,
    /// When this activation becomes active
    activation_time: DlmsType,
}

impl RegisterActivation {
    /// Create a new Register Activation object
    pub fn new(
        logical_name: ObisCode,
        registers_assigned: DlmsType,
        tariff_rate: u8,
        activation_time: DlmsType,
    ) -> Self {
        Self {
            logical_name,
            registers_assigned,
            tariff_rate,
            activation_time,
        }
    }

    pub const fn get_tariff_rate(&self) -> u8 {
        self.tariff_rate
    }
}

impl CosemClass for RegisterActivation {
    const CLASS_ID: u16 = 6;
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
            2 => Ok(self.registers_assigned.clone()),
            3 => Ok(DlmsType::UInt8(self.tariff_rate)),
            4 => Ok(self.activation_time.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.registers_assigned = value;
                Ok(())
            }
            3 => {
                if let DlmsType::UInt8(rate) = value {
                    self.tariff_rate = rate;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 17,
                        got: value.tag(),
                    })
                }
            }
            4 => {
                self.activation_time = value;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_activation_class_id() {
        let ra = RegisterActivation::new(
            ObisCode::new(1, 0, 0, 0, 0, 255),
            DlmsType::Array(alloc::vec![]),
            1,
            DlmsType::Null,
        );
        assert_eq!(RegisterActivation::CLASS_ID, 6);
        assert_eq!(ra.get_tariff_rate(), 1);
    }
}
