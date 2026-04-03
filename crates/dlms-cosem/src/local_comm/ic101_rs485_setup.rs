//!
//! IC 101 — RS485 Port Setup
//!
//! Configuration for RS485 communication port.
//!
//! Attributes:
//!   1. logical_name          (octet-string)
//!   2. default_baudrate      (unsigned)
//!   3. default_parity        (enum)
//!   4. data_bits             (unsigned)
//!   5. stop_bits             (enum)
//!   6. flow_control          (bit-string)
//!   7. half_duplex           (boolean)
//!   8. terminator_resistance (boolean)
//!   9. bias_resistance       (boolean)
//!  10. auto_baudrate         (boolean)

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Parity {
    None = 0,
    Odd = 1,
    Even = 2,
    Mark = 3,
    Space = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum StopBits {
    One = 0,
    OneHalf = 1,
    Two = 2,
}

pub struct Rs485PortSetup {
    logical_name: ObisCode,
    baudrate: u32,
    parity: Parity,
    data_bits: u8,
    stop_bits: StopBits,
    half_duplex: bool,
    term_resistance: bool,
    bias_resistance: bool,
    auto_baudrate: bool,
}

impl Rs485PortSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            baudrate: 9600,
            parity: Parity::Even,
            data_bits: 8,
            stop_bits: StopBits::One,
            half_duplex: true,
            term_resistance: false,
            bias_resistance: false,
            auto_baudrate: false,
        }
    }
    pub fn baudrate(&self) -> u32 {
        self.baudrate
    }
    pub fn set_baudrate(&mut self, r: u32) {
        self.baudrate = r;
    }
    pub fn is_half_duplex(&self) -> bool {
        self.half_duplex
    }
}

impl CosemClass for Rs485PortSetup {
    const CLASS_ID: u16 = 101;
    const VERSION: u8 = 1;
    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }
    fn attribute_count() -> u8 {
        10
    }
    fn method_count() -> u8 {
        0
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.baudrate)),
            3 => Ok(DlmsType::Enum(self.parity as u8)),
            4 => Ok(DlmsType::UInt8(self.data_bits)),
            5 => Ok(DlmsType::Enum(self.stop_bits as u8)),
            6 => Ok(DlmsType::BitString(alloc::vec![0])),
            7 => Ok(DlmsType::Boolean(self.half_duplex)),
            8 => Ok(DlmsType::Boolean(self.term_resistance)),
            9 => Ok(DlmsType::Boolean(self.bias_resistance)),
            10 => Ok(DlmsType::Boolean(self.auto_baudrate)),
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
            3..=5 => Ok(()), // simplified
            7 => {
                if let DlmsType::Boolean(h) = value {
                    self.half_duplex = h;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 3,
                        got: value.tag(),
                    })
                }
            }
            8 => {
                if let DlmsType::Boolean(t) = value {
                    self.term_resistance = t;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 3,
                        got: value.tag(),
                    })
                }
            }
            9 => {
                if let DlmsType::Boolean(b) = value {
                    self.bias_resistance = b;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 3,
                        got: value.tag(),
                    })
                }
            }
            10 => {
                if let DlmsType::Boolean(a) = value {
                    self.auto_baudrate = a;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 3,
                        got: value.tag(),
                    })
                }
            }
            6 => Ok(()), // flow control simplified
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
        let port = Rs485PortSetup::new(ObisCode::new(0, 0, 101, 0, 0, 255));
        assert_eq!(<Rs485PortSetup as CosemClass>::CLASS_ID, 101);
        assert_eq!(port.baudrate(), 9600);
        assert!(port.is_half_duplex());
    }
    #[test]
    fn test_set_baudrate() {
        let mut port = Rs485PortSetup::new(ObisCode::new(0, 0, 101, 0, 0, 255));
        port.set_baudrate(19200);
        assert_eq!(port.get_attribute(2).unwrap(), DlmsType::UInt32(19200));
    }
}
