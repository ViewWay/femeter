//! HS-PLC IP SSAS Setup Interface (IC 142)
//!
//! HS-PLC ISO/IEC 12139-1 IP SSAS setup.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.7.142

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// HS-PLC IP SSAS Setup Interface Class (IC 142)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: ip_address (octet-string)
/// - 3: subnet_mask (octet-string)
/// - 4: gateway (octet-string)
///
/// Methods: None
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct HsIpSsasSetup {
    logical_name: ObisCode,
    ip_address: alloc::vec::Vec<u8>,
    subnet_mask: alloc::vec::Vec<u8>,
    gateway: alloc::vec::Vec<u8>,
}

impl HsIpSsasSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            ip_address: alloc::vec![0, 0, 0, 0],
            subnet_mask: alloc::vec![255, 255, 255, 0],
            gateway: alloc::vec![0, 0, 0, 0],
        }
    }
}

impl CosemClass for HsIpSsasSetup {
    const CLASS_ID: u16 = 142;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::OctetString(self.ip_address.clone())),
            3 => Ok(DlmsType::OctetString(self.subnet_mask.clone())),
            4 => Ok(DlmsType::OctetString(self.gateway.clone())),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                if let DlmsType::OctetString(data) = value {
                    self.ip_address = data;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            3 => {
                if let DlmsType::OctetString(data) = value {
                    self.subnet_mask = data;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            4 => {
                if let DlmsType::OctetString(data) = value {
                    self.gateway = data;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
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
        assert_eq!(HsIpSsasSetup::CLASS_ID, 142);
    }
}
