//! PRIME Physical Layer Counters Interface (IC 81)
//!
//! PRIME NB OFDM PLC physical layer counters.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.81

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// PRIME Physical Layer Counters Interface Class (IC 81)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: phy_crc_errors (double-long-unsigned)
/// - 3: phy_terminated_packets (double-long-unsigned)
/// - 4: phy_rssi (long)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct PrimePhyCounters {
    logical_name: ObisCode,
    phy_crc_errors: u32,
    phy_terminated_packets: u32,
    phy_rssi: i32,
}

impl PrimePhyCounters {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            phy_crc_errors: 0,
            phy_terminated_packets: 0,
            phy_rssi: -100,
        }
    }
}

impl CosemClass for PrimePhyCounters {
    const CLASS_ID: u16 = 81;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.phy_crc_errors)),
            3 => Ok(DlmsType::UInt32(self.phy_terminated_packets)),
            4 => Ok(DlmsType::Int32(self.phy_rssi)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, _id: u8, _value: DlmsType) -> Result<(), CosemError> {
        Err(CosemError::ReadOnly)
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
        assert_eq!(PrimePhyCounters::CLASS_ID, 81);
    }
}
