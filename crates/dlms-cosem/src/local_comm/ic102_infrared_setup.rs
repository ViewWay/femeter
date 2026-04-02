//!
//! IC 102 — Infrared Optical Port Setup
//!
//! Configuration for infrared optical communication port.
//!
//! Attributes:
//!   1. logical_name          (octet-string)
//!   2. default_baudrate      (unsigned)
//!   3. default_parity        (enum)
//!   4. data_bits             (unsigned)
//!   5. stop_bits             (enum)
//!   6. auto_baudrate         (boolean)
//!   7. led_control           (boolean)
//!   8. reading_timeout       (unsigned)
//!   9. response_delay        (unsigned)

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

pub struct InfraredPortSetup {
    logical_name: ObisCode,
    baudrate: u32,
    parity: u8,
    data_bits: u8,
    stop_bits: u8,
    auto_baudrate: bool,
    led_control: bool,
    reading_timeout: u16,
    response_delay: u16,
}

impl InfraredPortSetup {
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            baudrate: 9600,
            parity: 2,
            data_bits: 8,
            stop_bits: 0,
            auto_baudrate: true,
            led_control: true,
            reading_timeout: 300,
            response_delay: 500,
        }
    }
    pub fn baudrate(&self) -> u32 {
        self.baudrate
    }
    pub fn set_baudrate(&mut self, r: u32) {
        self.baudrate = r;
    }
}

impl CosemClass for InfraredPortSetup {
    const CLASS_ID: u16 = 102;
    const VERSION: u8 = 1;
    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }
    fn attribute_count() -> u8 {
        9
    }
    fn method_count() -> u8 {
        0
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.baudrate)),
            3 => Ok(DlmsType::Enum(self.parity)),
            4 => Ok(DlmsType::UInt8(self.data_bits)),
            5 => Ok(DlmsType::Enum(self.stop_bits)),
            6 => Ok(DlmsType::Boolean(self.auto_baudrate)),
            7 => Ok(DlmsType::Boolean(self.led_control)),
            8 => Ok(DlmsType::UInt16(self.reading_timeout)),
            9 => Ok(DlmsType::UInt16(self.response_delay)),
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
            6 => {
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
            7 => {
                if let DlmsType::Boolean(l) = value {
                    self.led_control = l;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 3,
                        got: value.tag(),
                    })
                }
            }
            8 => {
                self.reading_timeout = value.as_u16().ok_or(CosemError::TypeMismatch {
                    expected: 0x12,
                    got: value.tag(),
                })?;
                Ok(())
            }
            9 => {
                self.response_delay = value.as_u16().ok_or(CosemError::TypeMismatch {
                    expected: 0x12,
                    got: value.tag(),
                })?;
                Ok(())
            }
            3 | 4 | 5 => Ok(()), // simplified
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
        let port = InfraredPortSetup::new(ObisCode::new(0, 0, 102, 0, 0, 255));
        assert_eq!(<InfraredPortSetup as CosemClass>::CLASS_ID, 102);
        assert_eq!(port.baudrate(), 9600);
    }
    #[test]
    fn test_get_set() {
        let mut port = InfraredPortSetup::new(ObisCode::new(0, 0, 102, 0, 0, 255));
        port.set_baudrate(300);
        assert_eq!(port.baudrate(), 300);
        port.set_attribute(8, DlmsType::UInt16(600)).unwrap();
        assert_eq!(port.get_attribute(8).unwrap(), DlmsType::UInt16(600));
    }
}
