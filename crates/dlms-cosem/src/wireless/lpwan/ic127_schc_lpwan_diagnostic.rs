//! SCHC-LPWAN Diagnostic Interface (IC 127)
//!
//! SCHC-LPWAN diagnostic counters.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.127

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// SCHC-LPWAN Diagnostic Interface Class (IC 127)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: tx_packets (double-long-unsigned)
/// - 3: rx_packets (double-long-unsigned)
/// - 4: fragmentation_errors (double-long-unsigned)
///
/// Methods:
/// - 1: reset
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SchcLpwanDiagnostic {
    logical_name: ObisCode,
    tx_packets: u32,
    rx_packets: u32,
    fragmentation_errors: u32,
}

impl SchcLpwanDiagnostic {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            tx_packets: 0,
            rx_packets: 0,
            fragmentation_errors: 0,
        }
    }

    pub fn reset(&mut self) {
        self.tx_packets = 0;
        self.rx_packets = 0;
        self.fragmentation_errors = 0;
    }
}

impl CosemClass for SchcLpwanDiagnostic {
    const CLASS_ID: u16 = 127;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.tx_packets)),
            3 => Ok(DlmsType::UInt32(self.rx_packets)),
            4 => Ok(DlmsType::UInt32(self.fragmentation_errors)),
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
        assert_eq!(SchcLpwanDiagnostic::CLASS_ID, 127);
    }
}
