//! ZigBee Network Control Interface (IC 104)
//!
//! ZigBee network control and management.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.104

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// ZigBee Network Control Interface Class (IC 104)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: network_layer_state (enum)
/// - 3: network_short_address (unsigned)
/// - 4: network_extended_address (octet-string)
///
/// Methods:
/// - 1: reset
/// - 2: form_network
/// - 3: join_network
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct ZigbeeNetworkControl {
    logical_name: ObisCode,
    network_layer_state: u8,
    network_short_address: u16,
    network_extended_address: alloc::vec::Vec<u8>,
}

impl ZigbeeNetworkControl {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            network_layer_state: 0,
            network_short_address: 0xFFFF,
            network_extended_address: alloc::vec![0; 8],
        }
    }
}

impl CosemClass for ZigbeeNetworkControl {
    const CLASS_ID: u16 = 104;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Enum(self.network_layer_state)),
            3 => Ok(DlmsType::UInt16(self.network_short_address)),
            4 => Ok(DlmsType::OctetString(self.network_extended_address.clone())),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, _id: u8, _value: DlmsType) -> Result<(), CosemError> {
        Err(CosemError::ReadOnly)
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // reset
            2 => Ok(DlmsType::Null), // form_network
            3 => Ok(DlmsType::Null), // join_network
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }

    fn attribute_count() -> u8 {
        4
    }

    fn method_count() -> u8 {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_id() {
        assert_eq!(ZigbeeNetworkControl::CLASS_ID, 104);
    }
}
