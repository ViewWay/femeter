//! S-FSK MAC Counters Interface (IC 53)
//!
//! S-FSK MAC layer counters for statistics.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.53

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// S-FSK MAC Counters Interface Class (IC 53)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: sync_packets_received (double-long-unsigned)
/// - 3: sync_packets_transmitted (double-long-unsigned)
/// - 4: data_packets_received (double-long-unsigned)
/// - 5: data_packets_transmitted (double-long-unsigned)
/// - 6: crc_errors (double-long-unsigned)
/// - 7: parity_errors (double-long-unsigned)
///
/// Methods:
/// - 1: reset
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct SfskMacCounters {
    logical_name: ObisCode,
    sync_packets_received: u32,
    sync_packets_transmitted: u32,
    data_packets_received: u32,
    data_packets_transmitted: u32,
    crc_errors: u32,
    parity_errors: u32,
}

impl SfskMacCounters {
    /// Create a new SfskMacCounters instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            sync_packets_received: 0,
            sync_packets_transmitted: 0,
            data_packets_received: 0,
            data_packets_transmitted: 0,
            crc_errors: 0,
            parity_errors: 0,
        }
    }

    pub fn sync_packets_received(&self) -> u32 {
        self.sync_packets_received
    }

    pub fn sync_packets_transmitted(&self) -> u32 {
        self.sync_packets_transmitted
    }

    pub fn data_packets_received(&self) -> u32 {
        self.data_packets_received
    }

    pub fn data_packets_transmitted(&self) -> u32 {
        self.data_packets_transmitted
    }

    pub fn crc_errors(&self) -> u32 {
        self.crc_errors
    }

    pub fn parity_errors(&self) -> u32 {
        self.parity_errors
    }

    pub fn increment(&mut self) {
        self.sync_packets_received = self.sync_packets_received.wrapping_add(1);
    }

    pub fn reset(&mut self) {
        self.sync_packets_received = 0;
        self.sync_packets_transmitted = 0;
        self.data_packets_received = 0;
        self.data_packets_transmitted = 0;
        self.crc_errors = 0;
        self.parity_errors = 0;
    }
}

impl CosemClass for SfskMacCounters {
    const CLASS_ID: u16 = 53;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.sync_packets_received)),
            3 => Ok(DlmsType::UInt32(self.sync_packets_transmitted)),
            4 => Ok(DlmsType::UInt32(self.data_packets_received)),
            5 => Ok(DlmsType::UInt32(self.data_packets_transmitted)),
            6 => Ok(DlmsType::UInt32(self.crc_errors)),
            7 => Ok(DlmsType::UInt32(self.parity_errors)),
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
        7
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
        assert_eq!(SfskMacCounters::CLASS_ID, 53);
    }

    #[test]
    fn test_reset() {
        let mut counters = SfskMacCounters::new(ObisCode::new(0, 0, 53, 0, 0, 255));
        counters.increment();
        assert_eq!(counters.sync_packets_received(), 1);
        counters.reset();
        assert_eq!(counters.sync_packets_received(), 0);
    }

    #[test]
    fn test_execute_reset_method() {
        let mut counters = SfskMacCounters::new(ObisCode::new(0, 0, 53, 0, 0, 255));
        counters.increment();
        assert!(counters.execute_method(1, DlmsType::Null).is_ok());
        assert_eq!(counters.sync_packets_received(), 0);
    }
}
