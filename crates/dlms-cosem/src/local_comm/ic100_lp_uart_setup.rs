//!
//! IC 100 — LPUART Port Setup
//!
//! Configuration for LPUART (Low Power UART) communication port.
//!
//! Reference: Blue Book Part 2
//!
//! Attributes:
//!   1. logical_name          (octet-string)
//!   2. default_baudrate      (unsigned)
//!   3. default_parity        (enum)
//!   4. data_bits             (unsigned)
//!   5. stop_bits             (enum)
//!   6. flow_control          (bit-string)
//!   7. auto_baudrate         (boolean)

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

pub struct LpUartPortSetup {
    logical_name: ObisCode,
    baudrate: u32,
    parity: Parity,
    data_bits: u8,
    stop_bits: StopBits,
    flow_control: bool,
    auto_baudrate: bool,
}

impl LpUartPortSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            baudrate: 2400,
            parity: Parity::Even,
            data_bits: 8,
            stop_bits: StopBits::One,
            flow_control: false,
            auto_baudrate: false,
        }
    }

    pub fn baudrate(&self) -> u32 {
        self.baudrate
    }
    pub fn parity(&self) -> Parity {
        self.parity
    }
    pub fn data_bits(&self) -> u8 {
        self.data_bits
    }
    pub fn stop_bits(&self) -> StopBits {
        self.stop_bits
    }

    pub fn set_baudrate(&mut self, rate: u32) {
        self.baudrate = rate;
    }
    pub fn set_parity(&mut self, p: Parity) {
        self.parity = p;
    }
    pub fn set_data_bits(&mut self, bits: u8) {
        self.data_bits = bits.clamp(5, 9);
    }
    pub fn set_stop_bits(&mut self, sb: StopBits) {
        self.stop_bits = sb;
    }
}

impl CosemClass for LpUartPortSetup {
    const CLASS_ID: u16 = 100;
    const VERSION: u8 = 1;
    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }
    fn attribute_count() -> u8 {
        7
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
            6 => Ok(DlmsType::BitString(alloc::vec![if self.flow_control {
                0x80
            } else {
                0
            }])),
            7 => Ok(DlmsType::Boolean(self.auto_baudrate)),
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
                let p = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 0x55,
                    got: value.tag(),
                })?;
                self.parity = match p {
                    0 => Parity::None,
                    1 => Parity::Odd,
                    2 => Parity::Even,
                    3 => Parity::Mark,
                    4 => Parity::Space,
                    _ => return Err(CosemError::InvalidParameter),
                };
                Ok(())
            }
            4 => {
                self.data_bits = value
                    .as_u8()
                    .ok_or(CosemError::TypeMismatch {
                        expected: 0x12,
                        got: value.tag(),
                    })?
                    .clamp(5, 9);
                Ok(())
            }
            5 => {
                let sb = value.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 0x55,
                    got: value.tag(),
                })?;
                self.stop_bits = match sb {
                    0 => StopBits::One,
                    1 => StopBits::OneHalf,
                    2 => StopBits::Two,
                    _ => return Err(CosemError::InvalidParameter),
                };
                Ok(())
            }
            6 => {
                if let DlmsType::BitString(ref b) = value {
                    self.flow_control = b.first().map(|&x| x & 0x80 != 0).unwrap_or(false);
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 4,
                        got: value.tag(),
                    })
                }
            }
            7 => {
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
        let port = LpUartPortSetup::new(ObisCode::new(0, 0, 100, 0, 0, 255));
        assert_eq!(<LpUartPortSetup as CosemClass>::CLASS_ID, 100);
        assert_eq!(port.baudrate(), 2400);
    }
    #[test]
    fn test_set_get() {
        let mut port = LpUartPortSetup::new(ObisCode::new(0, 0, 100, 0, 0, 255));
        port.set_baudrate(9600);
        port.set_parity(Parity::None);
        port.set_data_bits(7);
        port.set_stop_bits(StopBits::Two);
        assert_eq!(port.baudrate(), 9600);
        assert_eq!(port.parity(), Parity::None);
        assert_eq!(port.data_bits(), 7);
        assert_eq!(port.stop_bits(), StopBits::Two);
        // Verify via get_attribute
        assert_eq!(port.get_attribute(2).unwrap(), DlmsType::UInt32(9600));
        assert_eq!(port.get_attribute(3).unwrap(), DlmsType::Enum(0));
        assert_eq!(port.get_attribute(4).unwrap(), DlmsType::UInt8(7));
        assert_eq!(port.get_attribute(5).unwrap(), DlmsType::Enum(2));
    }
}
