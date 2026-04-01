//! Display Interface (IC 24)
//!
//! The Display interface controls the display on a smart meter,
//! managing what information is shown and when.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.6.24

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// COSEM Display Interface Class (IC 24)
///
/// Attributes:
/// - 1: logical_name (octet-string)
///
/// Methods: None (implementation varies by device)
///
/// Note: This interface is device-specific. The actual attributes
/// and methods depend on the display capabilities of the meter.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct Display {
    logical_name: ObisCode,
}

impl Display {
    /// Create a new Display instance
    ///
    /// # Arguments
    ///
    /// * `logical_name` - OBIS code identifying this display object
    pub fn new(logical_name: ObisCode) -> Self {
        Self { logical_name }
    }
}

impl CosemClass for Display {
    const CLASS_ID: u16 = 24;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            _ => Err(CosemError::NotImplemented),
        }
    }

    fn set_attribute(&mut self, _id: u8, _value: DlmsType) -> Result<(), CosemError> {
        Err(CosemError::NotImplemented)
    }

    fn execute_method(&mut self, _id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        Err(CosemError::NotImplemented)
    }

    fn attribute_count() -> u8 {
        1
    }

    fn method_count() -> u8 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_display() -> Display {
        Display::new(ObisCode::new(0, 0, 24, 0, 0, 255))
    }

    #[test]
    fn test_display_creation() {
        let display = create_test_display();
        assert_eq!(display.logical_name(), &ObisCode::new(0, 0, 24, 0, 0, 255));
    }

    #[test]
    fn test_get_logical_name() {
        let display = create_test_display();
        let ln = display.get_attribute(1).unwrap();
        assert!(matches!(ln, DlmsType::OctetString(_)));
    }

    #[test]
    fn test_class_id() {
        assert_eq!(Display::CLASS_ID, 24);
        assert_eq!(Display::VERSION, 0);
    }

    #[test]
    fn test_attribute_count() {
        assert_eq!(Display::attribute_count(), 1);
        assert_eq!(Display::method_count(), 0);
    }
}
