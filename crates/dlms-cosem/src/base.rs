//!
//! Base utilities and common implementations for COSEM interface classes
//!
//! This module provides helper types and functions used across multiple
//! interface classes.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    types::DlmsType,
};

/// Common attribute IDs for COSEM objects
pub mod attr {
    pub const LOGICAL_NAME: u8 = 1;
    pub const VALUE: u8 = 2;
    pub const SCALER_UNIT: u8 = 3;
    pub const STATUS: u8 = 4;
    pub const CAPTURE_TIME: u8 = 5;
}

/// Create standard attribute 1 (logical_name) response
pub fn get_logical_name(obj: &ObisCode) -> Result<DlmsType, CosemError> {
    Ok(DlmsType::OctetString(obj.to_bytes().to_vec()))
}

/// Validate attribute ID exists
pub fn validate_attr(id: u8, count: u8) -> Result<(), CosemError> {
    if id == 0 || id > count {
        Err(CosemError::NoSuchAttribute(id))
    } else {
        Ok(())
    }
}

/// Validate method ID exists
pub fn validate_method(id: u8, count: u8) -> Result<(), CosemError> {
    if id == 0 || id > count {
        Err(CosemError::NoSuchMethod(id))
    } else {
        Ok(())
    }
}

/// Get read-only attribute error
pub fn read_only() -> Result<DlmsType, CosemError> {
    Err(CosemError::ReadOnly)
}

/// Set read-only attribute error
pub fn set_read_only(_value: DlmsType) -> Result<(), CosemError> {
    Err(CosemError::ReadOnly)
}

/// Default get_attribute implementation for unimplemented attributes
pub fn default_get_attribute(id: u8, count: u8) -> Result<DlmsType, CosemError> {
    validate_attr(id, count)?;
    Err(CosemError::NotImplemented)
}

/// Default set_attribute implementation for unimplemented attributes
pub fn default_set_attribute(id: u8, _value: DlmsType, count: u8) -> Result<(), CosemError> {
    validate_attr(id, count)?;
    Err(CosemError::NotImplemented)
}

/// Default execute_method implementation for unimplemented methods
pub fn default_execute_method(id: u8, _params: DlmsType, count: u8) -> Result<DlmsType, CosemError> {
    validate_method(id, count)?;
    Err(CosemError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_attr() {
        assert!(validate_attr(1, 5).is_ok());
        assert!(validate_attr(5, 5).is_ok());
        assert!(validate_attr(0, 5).is_err());
        assert!(validate_attr(6, 5).is_err());
    }

    #[test]
    fn test_get_logical_name() {
        let obis = ObisCode::new(1, 0, 1, 8, 0, 255);
        let result = get_logical_name(&obis).unwrap();
        assert_eq!(result, DlmsType::OctetString(alloc::vec![1, 0, 1, 8, 0, 255]));
    }
}
