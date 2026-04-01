//! S-FSK Reporting System List Interface (IC 56)
//!
//! S-FSK reporting system list management.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.56

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// S-FSK Reporting System List Interface Class (IC 56)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: reporting_system_list (array of structure)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SfskReportingSystemList {
    logical_name: ObisCode,
    reporting_system_list: alloc::vec::Vec<SfskReportingSystem>,
}

/// S-FSK Reporting System entry
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SfskReportingSystem {
    pub system_id: u16,
    pub system_name: alloc::string::String,
}

impl SfskReportingSystemList {
    /// Create a new SfskReportingSystemList instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            reporting_system_list: alloc::vec::Vec::new(),
        }
    }

    pub fn reporting_system_list(&self) -> &[SfskReportingSystem] {
        &self.reporting_system_list
    }

    pub fn add_system(&mut self, system: SfskReportingSystem) {
        self.reporting_system_list.push(system);
    }

    pub fn clear(&mut self) {
        self.reporting_system_list.clear();
    }
}

impl CosemClass for SfskReportingSystemList {
    const CLASS_ID: u16 = 56;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => {
                let items: alloc::vec::Vec<DlmsType> = self
                    .reporting_system_list
                    .iter()
                    .map(|sys| {
                        DlmsType::Structure(alloc::vec![
                            DlmsType::UInt16(sys.system_id),
                            DlmsType::OctetString(sys.system_name.as_bytes().to_vec()),
                        ])
                    })
                    .collect();
                Ok(DlmsType::Array(items))
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                if let DlmsType::Array(items) = value {
                    self.reporting_system_list.clear();
                    for item in items {
                        if let DlmsType::Structure(fields) = item {
                            if fields.len() >= 2 {
                                let system_id = fields[0].as_u16().unwrap_or(0);
                                let system_name =
                                    if let DlmsType::OctetString(ref bytes) = fields[1] {
                                        alloc::string::String::from_utf8_lossy(bytes).into_owned()
                                    } else {
                                        alloc::string::String::new()
                                    };
                                self.reporting_system_list.push(SfskReportingSystem {
                                    system_id,
                                    system_name,
                                });
                            }
                        }
                    }
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 1, // array
                        got: value.tag(),
                    })
                }
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, _id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        Err(CosemError::NoSuchMethod(1))
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

    #[test]
    fn test_class_id() {
        assert_eq!(SfskReportingSystemList::CLASS_ID, 56);
    }

    #[test]
    fn test_add_system() {
        let mut list = SfskReportingSystemList::new(ObisCode::new(0, 0, 56, 0, 0, 255));
        list.add_system(SfskReportingSystem {
            system_id: 1,
            system_name: alloc::string::String::from("System1"),
        });
        assert_eq!(list.reporting_system_list().len(), 1);
    }
}
