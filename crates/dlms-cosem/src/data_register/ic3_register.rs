//!
//! Interface Class 3: Register
//!
//! Reference: Blue Book Part 2 §5.3
//!
//! The Register interface class stores a single value with an associated
//! scaler and unit, typically used for metering values like energy, power, etc.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
    units::Unit,
};

/// COSEM IC 3: Register
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | value | 2 | CHOICE (numeric) | dynamic |
/// | scaler_unit | 3 | structure{scaler:long, unit:enum} | static |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct Register {
    /// OBIS code identifying this object
    logical_name: ObisCode,
    /// The stored value (numeric)
    value: DlmsType,
    /// Scaler (power of 10) and physical unit
    scaler: i8,
    unit: Unit,
}

impl Register {
    /// Create a new Register object
    pub const fn new(logical_name: ObisCode, scaler: i8, unit: Unit) -> Self {
        Self {
            logical_name,
            value: DlmsType::Null,
            scaler,
            unit,
        }
    }

    /// Get the current value
    pub fn get_value(&self) -> &DlmsType {
        &self.value
    }

    /// Set the value
    pub fn set_value(&mut self, value: DlmsType) {
        self.value = value;
    }

    /// Get the scaler
    pub const fn get_scaler(&self) -> i8 {
        self.scaler
    }

    /// Get the unit
    pub const fn get_unit(&self) -> Unit {
        self.unit
    }

    /// Set scaler and unit
    pub fn set_scaler_unit(&mut self, scaler: i8, unit: Unit) {
        self.scaler = scaler;
        self.unit = unit;
    }
}

impl CosemClass for Register {
    const CLASS_ID: u16 = 3;
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
            2 => Ok(self.value.clone()),
            3 => Ok(DlmsType::Structure(alloc::vec![
                DlmsType::Int8(self.scaler),
                DlmsType::UInt16(self.unit as u16),
            ])),
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
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_class_id() {
        let reg = Register::new(ObisCode::new(1, 0, 1, 8, 0, 255), -3, Unit::WattHour);
        assert_eq!(Register::CLASS_ID, 3);
        assert_eq!(Register::VERSION, 0);
    }

    #[test]
    fn test_register_scaler_unit() {
        let reg = Register::new(ObisCode::new(1, 0, 1, 8, 0, 255), -3, Unit::WattHour);
        assert_eq!(reg.get_scaler(), -3);
        assert_eq!(reg.get_unit(), Unit::WattHour);

        let scaler_unit = reg.get_attribute(3).unwrap();
        if let DlmsType::Structure(items) = scaler_unit {
            assert_eq!(items[0], DlmsType::Int8(-3));
            assert_eq!(items[1], DlmsType::UInt16(30)); // WattHour code
        } else {
            panic!("Expected Structure");
        }
    }

    #[test]
    fn test_register_value() {
        let mut _reg = Register::new(ObisCode::new(1, 0, 1, 8, 0, 255), 0, Unit::WattHour);
        _reg.set_value(DlmsType::UInt32(12345));
        assert_eq!(_reg.get_value(), &DlmsType::UInt32(12345));
    }
}
