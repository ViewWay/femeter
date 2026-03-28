//! G3-PLC Hybrid RF MAC Layer Counters Interface (IC 160)
//!
//! G3-PLC Hybrid RF MAC layer counters.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.160

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// G3-PLC Hybrid RF MAC Layer Counters Interface Class (IC 160)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: mac_tx_packets (double-long-unsigned)
/// - 3: mac_rx_packets (double-long-unsigned)
/// - 4: mac_crc_errors (double-long-unsigned)
///
/// Methods:
/// - 1: reset
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct G3HybridRfMacCounters {
    logical_name: ObisCode,
    mac_tx_packets: u32,
    mac_rx_packets: u32,
    mac_crc_errors: u32,
}

impl G3HybridRfMacCounters {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            mac_tx_packets: 0,
            mac_rx_packets: 0,
            mac_crc_errors: 0,
        }
    }

    pub fn reset(&mut self) {
        self.mac_tx_packets = 0;
        self.mac_rx_packets = 0;
        self.mac_crc_errors = 0;
    }
}

impl CosemClass for G3HybridRfMacCounters {
    const CLASS_ID: u16 = 160;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.mac_tx_packets)),
            3 => Ok(DlmsType::UInt32(self.mac_rx_packets)),
            4 => Ok(DlmsType::UInt32(self.mac_crc_errors)),
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
        assert_eq!(G3HybridRfMacCounters::CLASS_ID, 160);
    }
}
