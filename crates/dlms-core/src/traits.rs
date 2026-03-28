//! COSEM interface class trait
//!
//! Unified interface for all 105 COSEM interface classes.

use crate::types::DlmsType;
use crate::obis::ObisCode;
use crate::errors::CosemError;

/// Mandatory interface for all COSEM interface classes.
///
/// Every COSEM object must implement this trait. The default implementations
/// return NotImplemented for unimplemented attributes/methods.
pub trait CosemClass {
    /// COSEM interface class ID (e.g., 1 for Data, 3 for Register)
    const CLASS_ID: u16;
    /// Interface class version
    const VERSION: u8;

    /// Get the logical name (OBIS code) of this object
    fn logical_name(&self) -> &ObisCode;

    /// Get an attribute value by ID
    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        let _ = id;
        Err(CosemError::NotImplemented)
    }

    /// Set an attribute value by ID
    fn set_attribute(&mut self, id: u8, _value: DlmsType) -> Result<(), CosemError> {
        let _ = id;
        Err(CosemError::NotImplemented)
    }

    /// Execute a method by ID
    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        let _ = id;
        Err(CosemError::NotImplemented)
    }

    /// Total number of attributes defined for this class
    fn attribute_count() -> u8;

    /// Total number of methods defined for this class (default: 0)
    fn method_count() -> u8 {
        0
    }
}
