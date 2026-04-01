//! ISO/IEC 14908 Diagnostic Interface (IC 133)
//!
//! LON diagnostic counters.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.133

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// ISO/IEC 14908 Diagnostic Interface Class (IC 133)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: tx_packets (double-long-unsigned)
/// - 3: rx_packets (double-long-unsigned)
/// - 4: crc_errors (double-long-unsigned)
///
/// Methods:
/// - 1: reset
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct LonDiagnostic {
    logical_name: ObisCode,
    tx_packets: u32,
    rx_packets: u32,
    crc_errors: u32,
}

impl LonDiagnostic {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            tx_packets: 0,
            rx_packets: 0,
            crc_errors: 0,
        }
    }

    pub fn reset(&mut self) {
        self.tx_packets = 0;
        self.rx_packets = 0;
        self.crc_errors = 0;
    }
}

impl CosemClass for LonDiagnostic {
    const CLASS_ID: u16 = 133;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.tx_packets)),
            3 => Ok(DlmsType::UInt32(self.rx_packets)),
            4 => Ok(DlmsType::UInt32(self.crc_errors)),
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
        assert_eq!(LonDiagnostic::CLASS_ID, 133);
        assert_eq!(LonDiagnostic::VERSION, 1);
    }
}
