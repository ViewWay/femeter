//! S-FSK MAC Synchronization Timeouts Interface (IC 52)
//!
//! S-FSK MAC synchronization timeout parameters.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.52

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// S-FSK MAC Synchronization Timeouts Interface Class (IC 52)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: sync_cycle (double-long-unsigned)
/// - 3: sync_max_cycle (double-long-unsigned)
/// - 4: noise_cycle (double-long-unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SfskMacSyncTimeouts {
    logical_name: ObisCode,
    sync_cycle: u32,
    sync_max_cycle: u32,
    noise_cycle: u32,
}

impl SfskMacSyncTimeouts {
    /// Create a new SfskMacSyncTimeouts instance with default values
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            sync_cycle: 1000,
            sync_max_cycle: 5000,
            noise_cycle: 500,
        }
    }

    pub fn sync_cycle(&self) -> u32 {
        self.sync_cycle
    }

    pub fn sync_max_cycle(&self) -> u32 {
        self.sync_max_cycle
    }

    pub fn noise_cycle(&self) -> u32 {
        self.noise_cycle
    }
}

impl CosemClass for SfskMacSyncTimeouts {
    const CLASS_ID: u16 = 52;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.sync_cycle)),
            3 => Ok(DlmsType::UInt32(self.sync_max_cycle)),
            4 => Ok(DlmsType::UInt32(self.noise_cycle)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                self.sync_cycle = value.as_u32().ok_or(CosemError::TypeMismatch {
                    expected: 19,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                self.sync_max_cycle = value.as_u32().ok_or(CosemError::TypeMismatch {
                    expected: 19,
                    got: value.tag(),
                })?;
                Ok(())
            }
            4 => {
                self.noise_cycle = value.as_u32().ok_or(CosemError::TypeMismatch {
                    expected: 19,
                    got: value.tag(),
                })?;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, _id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        Err(CosemError::NoSuchMethod(1))
    }

    fn attribute_count() -> u8 {
        4
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
        assert_eq!(SfskMacSyncTimeouts::CLASS_ID, 52);
    }

    #[test]
    fn test_creation() {
        let timeouts = SfskMacSyncTimeouts::new(ObisCode::new(0, 0, 52, 0, 0, 255));
        assert_eq!(timeouts.sync_cycle(), 1000);
        assert_eq!(timeouts.sync_max_cycle(), 5000);
    }
}
