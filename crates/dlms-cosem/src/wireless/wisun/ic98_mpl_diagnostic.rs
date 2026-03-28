//! MPL Diagnostic Interface (IC 98)
//!
//! MPL (Multicast Protocol for Low-Power and Lossy Networks) diagnostic.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.98

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// MPL Diagnostic Interface Class (IC 98)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: multicast_rx_packets (double-long-unsigned)
/// - 3: multicast_tx_packets (double-long-unsigned)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct MplDiagnostic {
    logical_name: ObisCode,
    multicast_rx_packets: u32,
    multicast_tx_packets: u32,
}

impl MplDiagnostic {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            multicast_rx_packets: 0,
            multicast_tx_packets: 0,
        }
    }
}

impl CosemClass for MplDiagnostic {
    const CLASS_ID: u16 = 98;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.multicast_rx_packets)),
            3 => Ok(DlmsType::UInt32(self.multicast_tx_packets)),
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
        3
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
        assert_eq!(MplDiagnostic::CLASS_ID, 98);
    }
}
