//! LoRaWAN Diagnostic Interface (IC 129)
//!
//! LoRaWAN diagnostic counters.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.129

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// LoRaWAN Diagnostic Interface Class (IC 129)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: uplink_frames (double-long-unsigned)
/// - 3: downlink_frames (double-long-unsigned)
/// - 4: join_attempts (double-long-unsigned)
///
/// Methods:
/// - 1: reset
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct LorawanDiagnostic {
    logical_name: ObisCode,
    uplink_frames: u32,
    downlink_frames: u32,
    join_attempts: u32,
}

impl LorawanDiagnostic {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            uplink_frames: 0,
            downlink_frames: 0,
            join_attempts: 0,
        }
    }

    pub fn reset(&mut self) {
        self.uplink_frames = 0;
        self.downlink_frames = 0;
        self.join_attempts = 0;
    }
}

impl CosemClass for LorawanDiagnostic {
    const CLASS_ID: u16 = 129;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.uplink_frames)),
            3 => Ok(DlmsType::UInt32(self.downlink_frames)),
            4 => Ok(DlmsType::UInt32(self.join_attempts)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, _id: u8, _value: DlmsType) -> Result<(), CosemError> {
        Err(CosemError::ReadOnly)
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                self.reset();
                Ok(DlmsType::Null)
            }
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }

    fn attribute_count() -> u8 {
        4
    }

    fn method_count() -> u8 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_id() {
        assert_eq!(LorawanDiagnostic::CLASS_ID, 129);
    }
}
