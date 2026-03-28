//! COSEM Logical Device Interface (IC 43)
//!
//! The Logical Device interface represents a logical entity within a physical meter
//! that contains a collection of COSEM interface class instances.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.4.1

use alloc::vec::Vec;
use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// Information about a COSEM object instance
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct CosemObjectInfo {
    /// Object class ID
    class_id: u16,
    /// Object logical name (OBIS code)
    logical_name: ObisCode,
    /// Object version
    version: u8,
}

impl CosemObjectInfo {
    /// Create a new COSEM object info
    pub fn new(class_id: u16, logical_name: ObisCode, version: u8) -> Self {
        Self {
            class_id,
            logical_name,
            version,
        }
    }

    /// Get the class ID
    pub fn class_id(&self) -> u16 {
        self.class_id
    }

    /// Get the logical name
    pub fn logical_name(&self) -> ObisCode {
        self.logical_name
    }

    /// Get the version
    pub fn version(&self) -> u8 {
        self.version
    }
}

impl From<CosemObjectInfo> for DlmsType {
    fn from(info: CosemObjectInfo) -> Self {
        DlmsType::Structure(alloc::vec![
            DlmsType::UInt16(info.class_id),
            DlmsType::UInt8(info.version),
            DlmsType::OctetString(info.logical_name.to_bytes().to_vec()),
        ])
    }
}

/// COSEM Logical Device Interface Class (IC 43)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: object_list (array)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct LogicalDevice {
    logical_name: ObisCode,
    object_list: Vec<CosemObjectInfo>,
}

impl LogicalDevice {
    /// Create a new LogicalDevice instance
    ///
    /// # Arguments
    ///
    /// * `logical_name` - OBIS code identifying this logical device
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            object_list: Vec::new(),
        }
    }

    /// Get the object list
    pub fn object_list(&self) -> &[CosemObjectInfo] {
        &self.object_list
    }

    /// Add a COSEM object to the logical device
    ///
    /// # Arguments
    ///
    /// * `class_id` - Object class ID
    /// * `logical_name` - Object logical name (OBIS code)
    /// * `version` - Object version
    pub fn add_object(&mut self, class_id: u16, logical_name: ObisCode, version: u8) {
        let info = CosemObjectInfo::new(class_id, logical_name, version);
        self.object_list.push(info);
    }

    /// Remove a COSEM object from the logical device
    ///
    /// # Arguments
    ///
    /// * `class_id` - Object class ID
    /// * `logical_name` - Object logical name (OBIS code)
    ///
    /// # Returns
    ///
    /// `true` if the object was found and removed, `false` otherwise
    pub fn remove_object(&mut self, class_id: u16, logical_name: ObisCode) -> bool {
        let original_len = self.object_list.len();
        self.object_list.retain(|obj| {
            !(obj.class_id == class_id && obj.logical_name == logical_name)
        });
        self.object_list.len() < original_len
    }

    /// Find an object by class ID and logical name
    ///
    /// # Arguments
    ///
    /// * `class_id` - Object class ID
    /// * `logical_name` - Object logical name (OBIS code)
    ///
    /// # Returns
    ///
    /// `Some(&CosemObjectInfo)` if found, `None` otherwise
    pub fn find_object(&self, class_id: u16, logical_name: ObisCode) -> Option<&CosemObjectInfo> {
        self.object_list
            .iter()
            .find(|obj| obj.class_id == class_id && obj.logical_name == logical_name)
    }
}

impl CosemClass for LogicalDevice {
    const CLASS_ID: u16 = 43;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => {
                let objects = self
                    .object_list
                    .iter()
                    .map(|obj| obj.clone().into())
                    .collect();
                Ok(DlmsType::Array(objects))
            }
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
        2
    }

    fn method_count() -> u8 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_logical_device() -> LogicalDevice {
        LogicalDevice::new(ObisCode::new(0, 0, 17, 0, 0, 255))
    }

    #[test]
    fn test_logical_device_creation() {
        let ld = create_test_logical_device();
        assert_eq!(ld.object_list().len(), 0);
    }

    #[test]
    fn test_add_object() {
        let mut ld = create_test_logical_device();
        ld.add_object(1, ObisCode::new(1, 0, 1, 8, 0, 255), 0);
        ld.add_object(3, ObisCode::new(1, 0, 2, 8, 0, 255), 0);

        assert_eq!(ld.object_list().len(), 2);
        assert_eq!(ld.object_list()[0].class_id(), 1);
        assert_eq!(ld.object_list()[1].class_id(), 3);
    }

    #[test]
    fn test_remove_object() {
        let mut ld = create_test_logical_device();
        let obis = ObisCode::new(1, 0, 1, 8, 0, 255);
        ld.add_object(1, obis, 0);

        assert_eq!(ld.remove_object(1, obis), true);
        assert_eq!(ld.object_list().len(), 0);

        // Try to remove non-existent object
        assert_eq!(ld.remove_object(1, obis), false);
    }

    #[test]
    fn test_find_object() {
        let mut ld = create_test_logical_device();
        let obis = ObisCode::new(1, 0, 1, 8, 0, 255);
        ld.add_object(1, obis, 0);

        let found = ld.find_object(1, obis);
        assert!(found.is_some());
        assert_eq!(found.unwrap().class_id(), 1);

        let not_found = ld.find_object(2, ObisCode::new(1, 0, 2, 8, 0, 255));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_attributes() {
        let mut ld = create_test_logical_device();
        ld.add_object(1, ObisCode::new(1, 0, 1, 8, 0, 255), 0);

        let ln = ld.get_attribute(1).unwrap();
        let ol = ld.get_attribute(2).unwrap();

        assert!(matches!(ln, DlmsType::OctetString(_)));
        assert!(matches!(ol, DlmsType::Array(_)));
    }

    #[test]
    fn test_cosem_object_info() {
        let obis = ObisCode::new(1, 0, 1, 8, 0, 255);
        let info = CosemObjectInfo::new(1, obis, 0);

        assert_eq!(info.class_id(), 1);
        assert_eq!(info.logical_name(), obis);
        assert_eq!(info.version(), 0);
    }

    #[test]
    fn test_cosem_object_info_to_dlms() {
        let obis = ObisCode::new(1, 0, 1, 8, 0, 255);
        let info = CosemObjectInfo::new(1, obis, 0);
        let dlms: DlmsType = info.into();

        assert!(matches!(dlms, DlmsType::Structure(_)));
    }
}
