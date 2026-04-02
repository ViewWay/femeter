//!
//! IC 103 — Modem Port Setup
//!
//! Configuration for modem communication port (PSTN/4G modem).
//!
//! Attributes:
//!   1. logical_name          (octet-string)
//!   2. default_baudrate      (unsigned)
//!   3. modem_connection_type (enum)
//!   4. initialization_string (octet-string)
//!   5. phone_number          (octet-string)
//!   6. dial_command          (octet-string)
//!   7. answer_mode           (enum)
//!   8. connection_timeout    (unsigned)

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectionType {
    Direct = 0,
    DialUp = 1,
    Dedicated = 2,
    Gprs = 3,
    Lte = 4,
    Nbiot = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AnswerMode {
    Off = 0,
    Auto = 1,
    Manual = 2,
}

pub struct ModemPortSetup {
    logical_name: ObisCode,
    baudrate: u32,
    conn_type: ConnectionType,
    init_string: alloc::vec::Vec<u8>,
    phone_number: alloc::vec::Vec<u8>,
    dial_command: alloc::vec::Vec<u8>,
    answer_mode: AnswerMode,
    connection_timeout: u16,
}

impl ModemPortSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            baudrate: 115200,
            conn_type: ConnectionType::Gprs,
            init_string: b"ATZ\r".to_vec(),
            phone_number: alloc::vec::Vec::new(),
            dial_command: b"ATDT".to_vec(),
            answer_mode: AnswerMode::Auto,
            connection_timeout: 60,
        }
    }
    pub fn conn_type(&self) -> ConnectionType {
        self.conn_type
    }
    pub fn set_conn_type(&mut self, t: ConnectionType) {
        self.conn_type = t;
    }
}

impl CosemClass for ModemPortSetup {
    const CLASS_ID: u16 = 103;
    const VERSION: u8 = 1;
    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }
    fn attribute_count() -> u8 {
        8
    }
    fn method_count() -> u8 {
        0
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.baudrate)),
            3 => Ok(DlmsType::Enum(self.conn_type as u8)),
            4 => Ok(DlmsType::OctetString(self.init_string.clone())),
            5 => Ok(DlmsType::OctetString(self.phone_number.clone())),
            6 => Ok(DlmsType::OctetString(self.dial_command.clone())),
            7 => Ok(DlmsType::Enum(self.answer_mode as u8)),
            8 => Ok(DlmsType::UInt16(self.connection_timeout)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.baudrate = value.as_u32().ok_or(CosemError::TypeMismatch {
                    expected: 0x12,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 => {
                let t = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 0x55,
                    got: value.tag(),
                })?;
                self.conn_type = match t {
                    0 => ConnectionType::Direct,
                    1 => ConnectionType::DialUp,
                    2 => ConnectionType::Dedicated,
                    3 => ConnectionType::Gprs,
                    4 => ConnectionType::Lte,
                    5 => ConnectionType::Nbiot,
                    _ => return Err(CosemError::InvalidParameter),
                };
                Ok(())
            }
            4 => {
                if let DlmsType::OctetString(s) = value {
                    self.init_string = s;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            5 => {
                if let DlmsType::OctetString(n) = value {
                    self.phone_number = n;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            6 => {
                if let DlmsType::OctetString(d) = value {
                    self.dial_command = d;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: value.tag(),
                    })
                }
            }
            7 => {
                let m = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 0x55,
                    got: value.tag(),
                })?;
                self.answer_mode = match m {
                    0 => AnswerMode::Off,
                    1 => AnswerMode::Auto,
                    2 => AnswerMode::Manual,
                    _ => return Err(CosemError::InvalidParameter),
                };
                Ok(())
            }
            8 => {
                self.connection_timeout = value.as_u16().ok_or(CosemError::TypeMismatch {
                    expected: 0x12,
                    got: value.tag(),
                })?;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlms_core::traits::CosemClass;
    #[test]
    fn test_creation() {
        let port = ModemPortSetup::new(ObisCode::new(0, 0, 103, 0, 0, 255));
        assert_eq!(<ModemPortSetup as CosemClass>::CLASS_ID, 103);
        assert_eq!(port.conn_type(), ConnectionType::Gprs);
    }
    #[test]
    fn test_set_conn_type() {
        let mut port = ModemPortSetup::new(ObisCode::new(0, 0, 103, 0, 0, 255));
        port.set_conn_type(ConnectionType::Nbiot);
        assert_eq!(port.get_attribute(3).unwrap(), DlmsType::Enum(5));
    }
    #[test]
    fn test_set_phone_number() {
        let mut port = ModemPortSetup::new(ObisCode::new(0, 0, 103, 0, 0, 255));
        port.set_attribute(5, DlmsType::OctetString(b"10086".to_vec()))
            .unwrap();
        assert_eq!(
            port.get_attribute(5).unwrap(),
            DlmsType::OctetString(b"10086".to_vec())
        );
    }
}
