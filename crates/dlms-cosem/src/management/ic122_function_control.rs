//!
//! Interface Class 122: Function Control
//!
//! Reference: Blue Book Part 2 §6.122
//!
//! Function Control provides remote control of device functions.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 122: Function Control
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | activation_state | 2 | boolean | dynamic |
/// | service_id | 3 | unsigned | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | activate | 1 | Activate the function |
/// | deactivate | 2 | Deactivate the function |
#[derive(Debug, Clone)]
pub struct FunctionControl {
    logical_name: ObisCode,
    activation_state: bool,
    service_id: u8,
}

impl FunctionControl {
    /// Create a new Function Control object
    pub const fn new(logical_name: ObisCode, service_id: u8) -> Self {
        Self {
            logical_name,
            activation_state: false,
            service_id,
        }
    }

    pub const fn is_active(&self) -> bool {
        self.activation_state
    }
}

impl CosemClass for FunctionControl {
    const CLASS_ID: u16 = 122;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        3
    }

    fn method_count() -> u8 {
        2
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Boolean(self.activation_state)),
            3 => Ok(DlmsType::UInt8(self.service_id)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 3 => Err(CosemError::ReadOnly),
            2 => {
                if let DlmsType::Boolean(state) = value {
                    self.activation_state = state;
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
            1 => {
                self.activation_state = true;
                Ok(DlmsType::Null)
            }
            2 => {
                self.activation_state = false;
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
    fn test_function_control_class_id() {
        let fc = FunctionControl::new(ObisCode::new(0, 0, 122, 0, 0, 255), 1);
        assert_eq!(FunctionControl::CLASS_ID, 122);
        assert!(!fc.is_active());
    }
}
