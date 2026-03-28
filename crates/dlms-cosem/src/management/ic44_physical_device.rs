//! COSEM Physical Device Interface (IC 44)
//!
//! The Physical Device interface represents the physical meter device itself,
//! providing identification and descriptive information.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.4.2

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM Physical Device Interface Class (IC 44)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: manufacturer (octet-string)
/// - 3: model (octet-string)
/// - 4: version (octet-string)
/// - 5: serial_number (octet-string)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct PhysicalDevice {
    logical_name: ObisCode,
    manufacturer: alloc::string::String,
    model: alloc::string::String,
    version: alloc::string::String,
    serial_number: alloc::string::String,
}

impl PhysicalDevice {
    /// Create a new PhysicalDevice instance
    ///
    /// # Arguments
    ///
    /// * `logical_name` - OBIS code identifying this physical device
    /// * `manufacturer` - Manufacturer name
    /// * `model` - Device model
    /// * `version` - Firmware/hardware version
    /// * `serial_number` - Device serial number
    pub fn new(
        logical_name: ObisCode,
        manufacturer: &str,
        model: &str,
        version: &str,
        serial_number: &str,
    ) -> Self {
        Self {
            logical_name,
            manufacturer: manufacturer.into(),
            model: model.into(),
            version: version.into(),
            serial_number: serial_number.into(),
        }
    }

    /// Get the manufacturer name
    pub fn manufacturer(&self) -> &str {
        &self.manufacturer
    }

    /// Get the device model
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the serial number
    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }

    /// Set the manufacturer name
    pub fn set_manufacturer(&mut self, manufacturer: &str) {
        self.manufacturer = manufacturer.into();
    }

    /// Set the device model
    pub fn set_model(&mut self, model: &str) {
        self.model = model.into();
    }

    /// Set the version
    pub fn set_version(&mut self, version: &str) {
        self.version = version.into();
    }

    /// Set the serial number
    pub fn set_serial_number(&mut self, serial_number: &str) {
        self.serial_number = serial_number.into();
    }
}

impl CosemClass for PhysicalDevice {
    const CLASS_ID: u16 = 44;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::OctetString(
                self.manufacturer.as_bytes().to_vec(),
            )),
            3 => Ok(DlmsType::OctetString(self.model.as_bytes().to_vec())),
            4 => Ok(DlmsType::OctetString(self.version.as_bytes().to_vec())),
            5 => Ok(DlmsType::OctetString(self.serial_number.as_bytes().to_vec())),
            _ => Err(CosemError::NotImplemented),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                if let DlmsType::OctetString(data) = value {
                    let cow = alloc::string::String::from_utf8_lossy(&data);
                    self.manufacturer = cow.into_owned();
                    Ok(())
                } else {
                    Err(CosemError::NotImplemented)
                }
            }
            3 => {
                if let DlmsType::OctetString(data) = value {
                    let cow = alloc::string::String::from_utf8_lossy(&data);
                    self.model = cow.into_owned();
                    Ok(())
                } else {
                    Err(CosemError::NotImplemented)
                }
            }
            4 => {
                if let DlmsType::OctetString(data) = value {
                    let cow = alloc::string::String::from_utf8_lossy(&data);
                    self.version = cow.into_owned();
                    Ok(())
                } else {
                    Err(CosemError::NotImplemented)
                }
            }
            5 => {
                if let DlmsType::OctetString(data) = value {
                    let cow = alloc::string::String::from_utf8_lossy(&data);
                    self.serial_number = cow.into_owned();
                    Ok(())
                } else {
                    Err(CosemError::NotImplemented)
                }
            }
            _ => Err(CosemError::NotImplemented),
        }
    }

    fn execute_method(&mut self, _id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        Err(CosemError::NotImplemented)
    }

    fn attribute_count() -> u8 {
        5
    }

    fn method_count() -> u8 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_physical_device() -> PhysicalDevice {
        PhysicalDevice::new(
            ObisCode::new(0, 0, 42, 0, 0, 255),
            "FeMeter Corp",
            "FM-2000",
            "1.2.3",
            "SN12345678",
        )
    }

    #[test]
    fn test_physical_device_creation() {
        let pd = create_test_physical_device();
        assert_eq!(pd.manufacturer(), "FeMeter Corp");
        assert_eq!(pd.model(), "FM-2000");
        assert_eq!(pd.version(), "1.2.3");
        assert_eq!(pd.serial_number(), "SN12345678");
    }

    #[test]
    fn test_get_attributes() {
        let pd = create_test_physical_device();
        let ln = pd.get_attribute(1).unwrap();
        let mfr = pd.get_attribute(2).unwrap();
        let model = pd.get_attribute(3).unwrap();
        let version = pd.get_attribute(4).unwrap();
        let sn = pd.get_attribute(5).unwrap();

        assert!(matches!(ln, DlmsType::OctetString(_)));
        assert!(matches!(mfr, DlmsType::OctetString(_)));
        assert!(matches!(model, DlmsType::OctetString(_)));
        assert!(matches!(version, DlmsType::OctetString(_)));
        assert!(matches!(sn, DlmsType::OctetString(_)));
    }

    #[test]
    fn test_set_attributes() {
        let mut pd = create_test_physical_device();

        pd.set_attribute(2, DlmsType::OctetString(b"New Manufacturer".to_vec()))
            .unwrap();
        assert_eq!(pd.manufacturer(), "New Manufacturer");

        pd.set_attribute(3, DlmsType::OctetString(b"New Model".to_vec()))
            .unwrap();
        assert_eq!(pd.model(), "New Model");

        pd.set_attribute(4, DlmsType::OctetString(b"2.0.0".to_vec()))
            .unwrap();
        assert_eq!(pd.version(), "2.0.0");

        pd.set_attribute(5, DlmsType::OctetString(b"SN98765432".to_vec()))
            .unwrap();
        assert_eq!(pd.serial_number(), "SN98765432");
    }

    #[test]
    fn test_set_manufacturer() {
        let mut pd = create_test_physical_device();
        pd.set_manufacturer("Updated Manufacturer");
        assert_eq!(pd.manufacturer(), "Updated Manufacturer");
    }

    #[test]
    fn test_set_model() {
        let mut pd = create_test_physical_device();
        pd.set_model("Updated Model");
        assert_eq!(pd.model(), "Updated Model");
    }

    #[test]
    fn test_set_version() {
        let mut pd = create_test_physical_device();
        pd.set_version("2.5.0");
        assert_eq!(pd.version(), "2.5.0");
    }

    #[test]
    fn test_set_serial_number() {
        let mut pd = create_test_physical_device();
        pd.set_serial_number("SN99999999");
        assert_eq!(pd.serial_number(), "SN99999999");
    }
}
