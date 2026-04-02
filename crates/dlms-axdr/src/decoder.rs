//! A-XDR decoder

use crate::AxdrError;
use alloc::vec::Vec;
use dlms_core::types::*;
use dlms_core::DlmsType;

/// A-XDR decoder - reads COSEM data types from a byte buffer
pub struct AxdrDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> AxdrDecoder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Current position in buffer
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Remaining bytes
    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    /// Decode one DlmsType from current position
    pub fn decode(&mut self) -> Result<DlmsType, AxdrError> {
        let tag = self.read_byte()?;
        match tag {
            TAG_NULL => Ok(DlmsType::Null),
            TAG_BOOLEAN => {
                let v = self.read_byte()?;
                Ok(DlmsType::Boolean(v != 0))
            }
            TAG_INTEGER => {
                let v = self.read_byte()?;
                Ok(DlmsType::Int8(v as i8))
            }
            TAG_LONG => {
                let v = self.read_i16()?;
                Ok(DlmsType::Int16(v))
            }
            TAG_UNSIGNED => {
                let v = self.read_byte()?;
                Ok(DlmsType::UInt8(v))
            }
            TAG_LONG_UNSIGNED => {
                let v = self.read_u16()?;
                Ok(DlmsType::UInt16(v))
            }
            TAG_DOUBLE_LONG => {
                let v = self.read_i32()?;
                Ok(DlmsType::Int32(v))
            }
            TAG_DOUBLE_LONG_UNSIGNED => {
                let v = self.read_u32()?;
                Ok(DlmsType::UInt32(v))
            }
            TAG_LONG64 => {
                let v = self.read_i64()?;
                Ok(DlmsType::Int64(v))
            }
            TAG_LONG64_UNSIGNED => {
                let v = self.read_u64()?;
                Ok(DlmsType::UInt64(v))
            }
            TAG_ENUM => {
                let v = self.read_byte()?;
                Ok(DlmsType::Enum(v))
            }
            TAG_FLOAT32 => {
                let b = self.read_bytes(4)?;
                Ok(DlmsType::Float32(f32::from_be_bytes([
                    b[0], b[1], b[2], b[3],
                ])))
            }
            TAG_FLOAT64 => {
                let b = self.read_bytes(8)?;
                Ok(DlmsType::Float64(f64::from_be_bytes([
                    b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                ])))
            }
            TAG_BIT_STRING => {
                let num_bits = self.decode_length()?;
                let num_bytes = num_bits.div_ceil(8);
                let data = self.read_bytes(num_bytes)?.to_vec();
                Ok(DlmsType::BitString(data))
            }
            TAG_OCTET_STRING => {
                let len = self.decode_length()?;
                let data = self.read_bytes(len)?.to_vec();
                Ok(DlmsType::OctetString(data))
            }
            TAG_VISIBLE_STRING => {
                let len = self.decode_length()?;
                let data = self.read_bytes(len)?.to_vec();
                Ok(DlmsType::VisibleString(data))
            }
            TAG_UTF8_STRING => {
                let len = self.decode_length()?;
                let data = self.read_bytes(len)?.to_vec();
                Ok(DlmsType::Utf8String(data))
            }
            TAG_BCD => {
                let num_bytes = self.read_byte()? as usize;
                let data = self.read_bytes(num_bytes)?.to_vec();
                Ok(DlmsType::Bcd(num_bytes as u8, data))
            }
            TAG_COMPACT_ARRAY => {
                let element_type = self.read_byte()?;
                // element count is encoded as uint32
                let _tag = self.read_byte()?;
                let element_count = self.read_u32()?;
                let data_len = self.remaining();
                let data = self.read_bytes(data_len)?.to_vec();
                Ok(DlmsType::CompactArray {
                    element_type,
                    element_count,
                    data,
                })
            }
            TAG_DATETIME => {
                let b = self.read_bytes(12)?;
                Ok(DlmsType::DateTime(CosemDateTime {
                    date: CosemDate {
                        year: u16::from_be_bytes([b[0], b[1]]),
                        month: b[2],
                        day: b[3],
                        day_of_week: b[4],
                    },
                    time: CosemTime {
                        hour: b[5],
                        minute: b[6],
                        second: b[7],
                        hundredths: b[8],
                    },
                    deviation: i16::from_be_bytes([b[9], b[10]]),
                    clock_status: b[11],
                }))
            }
            TAG_DATE => {
                let b = self.read_bytes(5)?;
                Ok(DlmsType::Date(CosemDate {
                    year: u16::from_be_bytes([b[0], b[1]]),
                    month: b[2],
                    day: b[3],
                    day_of_week: b[4],
                }))
            }
            TAG_TIME => {
                let b = self.read_bytes(4)?;
                Ok(DlmsType::Time(CosemTime {
                    hour: b[0],
                    minute: b[1],
                    second: b[2],
                    hundredths: b[3],
                }))
            }
            TAG_ARRAY => {
                let count = self.decode_length()?;
                let mut items = Vec::with_capacity(count.min(256));
                for _ in 0..count {
                    items.push(self.decode()?);
                }
                Ok(DlmsType::Array(items))
            }
            TAG_STRUCTURE => {
                let count = self.decode_length()?;
                let mut items = Vec::with_capacity(count.min(256));
                for _ in 0..count {
                    items.push(self.decode()?);
                }
                Ok(DlmsType::Structure(items))
            }
            TAG_DELTA_INTEGER => {
                let v = self.read_byte()?;
                Ok(DlmsType::DeltaInt8(v as i8))
            }
            TAG_DELTA_LONG => {
                let v = self.read_i16()?;
                Ok(DlmsType::DeltaInt16(v))
            }
            TAG_DELTA_DOUBLE_LONG => {
                let v = self.read_i32()?;
                Ok(DlmsType::DeltaInt32(v))
            }
            TAG_DELTA_UNSIGNED => {
                let v = self.read_byte()?;
                Ok(DlmsType::DeltaUInt8(v))
            }
            TAG_DELTA_LONG_UNSIGNED => {
                let v = self.read_u16()?;
                Ok(DlmsType::DeltaUInt16(v))
            }
            TAG_DELTA_DOUBLE_LONG_UNSIGNED => {
                let v = self.read_u32()?;
                Ok(DlmsType::DeltaUInt32(v))
            }
            _ => Err(AxdrError::InvalidTag(tag)),
        }
    }

    /// Decode with expected tag check
    pub fn decode_with_type(&mut self, expected: u8) -> Result<DlmsType, AxdrError> {
        let saved_pos = self.pos;
        let tag = self.read_byte()?;
        if tag != expected {
            self.pos = saved_pos;
            return Err(AxdrError::TypeMismatch);
        }
        self.pos = saved_pos;
        self.decode()
    }

    // --- Helpers ---

    fn read_byte(&mut self) -> Result<u8, AxdrError> {
        if self.pos >= self.buf.len() {
            return Err(AxdrError::UnexpectedEnd);
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], AxdrError> {
        if self.pos + n > self.buf.len() {
            return Err(AxdrError::UnexpectedEnd);
        }
        let slice = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_u16(&mut self) -> Result<u16, AxdrError> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn read_i16(&mut self) -> Result<i16, AxdrError> {
        let b = self.read_bytes(2)?;
        Ok(i16::from_be_bytes([b[0], b[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, AxdrError> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_i32(&mut self) -> Result<i32, AxdrError> {
        let b = self.read_bytes(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, AxdrError> {
        let b = self.read_bytes(8)?;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_i64(&mut self) -> Result<i64, AxdrError> {
        let b = self.read_bytes(8)?;
        Ok(i64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    /// Decode A-XDR length field
    fn decode_length(&mut self) -> Result<usize, AxdrError> {
        let first = self.read_byte()?;
        if first < 128 {
            Ok(first as usize)
        } else if first == 0x81 {
            let len = self.read_byte()?;
            Ok(len as usize)
        } else if first == 0x82 {
            let len = self.read_u16()?;
            Ok(len as usize)
        } else {
            Err(AxdrError::InvalidLength)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AxdrEncoder;
    use alloc::vec;

    #[test]
    fn test_decode_null() {
        let mut dec = AxdrDecoder::new(&[0x00]);
        assert_eq!(dec.decode().unwrap(), DlmsType::Null);
    }

    #[test]
    fn test_decode_bool() {
        let mut dec = AxdrDecoder::new(&[0x03, 0x01]);
        assert_eq!(dec.decode().unwrap(), DlmsType::from_bool(true));
    }

    #[test]
    fn test_decode_int8() {
        let mut dec = AxdrDecoder::new(&[0x0F, 0xFF]);
        assert_eq!(dec.decode().unwrap(), DlmsType::from_i8(-1));
    }

    #[test]
    fn test_decode_uint16() {
        let mut dec = AxdrDecoder::new(&[0x12, 0x12, 0x34]);
        assert_eq!(dec.decode().unwrap(), DlmsType::from_u16(0x1234));
    }

    #[test]
    fn test_decode_uint32() {
        let mut dec = AxdrDecoder::new(&[0x06, 0x12, 0x34, 0x56, 0x78]);
        assert_eq!(dec.decode().unwrap(), DlmsType::from_u32(0x12345678));
    }

    #[test]
    fn test_decode_enum() {
        let mut dec = AxdrDecoder::new(&[0x16, 0x05]);
        assert_eq!(dec.decode().unwrap(), DlmsType::from_enum(5));
    }

    #[test]
    fn test_decode_octet_string() {
        let mut dec = AxdrDecoder::new(&[0x09, 0x03, 0x01, 0x02, 0x03]);
        assert_eq!(
            dec.decode().unwrap(),
            DlmsType::from_octet_string(vec![1, 2, 3])
        );
    }

    #[test]
    fn test_decode_float32() {
        let mut dec = AxdrDecoder::new(&[0x17, 0x3F, 0x80, 0x00, 0x00]);
        let result = dec.decode().unwrap();
        match result {
            DlmsType::Float32(v) => assert!((v - 1.0f32).abs() < f32::EPSILON),
            _ => panic!("Expected Float32"),
        }
    }

    #[test]
    fn test_decode_array() {
        let mut dec = AxdrDecoder::new(&[0x01, 0x02, 0x11, 0x01, 0x11, 0x02]);
        let result = dec.decode().unwrap();
        assert_eq!(
            result,
            DlmsType::Array(vec![DlmsType::from_u8(1), DlmsType::from_u8(2)])
        );
    }

    #[test]
    fn test_decode_structure() {
        let mut dec = AxdrDecoder::new(&[0x02, 0x02, 0x03, 0x01, 0x12, 0x00, 0x64]);
        let result = dec.decode().unwrap();
        assert_eq!(
            result,
            DlmsType::Structure(vec![DlmsType::from_bool(true), DlmsType::from_u16(100)])
        );
    }

    #[test]
    fn test_roundtrip_all_numeric() {
        let values = vec![
            DlmsType::from_i8(-42),
            DlmsType::from_i16(-1000),
            DlmsType::from_i32(-100000),
            DlmsType::from_i64(-999999999),
            DlmsType::from_u8(200),
            DlmsType::from_u16(60000),
            DlmsType::from_u32(3000000000),
            DlmsType::from_u64(999999999999),
            DlmsType::from_f32(3.14),
            DlmsType::from_f64(2.718281828),
            DlmsType::from_enum(42),
        ];
        for v in &values {
            let mut enc = AxdrEncoder::new();
            enc.encode(v).unwrap();
            let mut dec = AxdrDecoder::new(enc.to_bytes());
            let decoded = dec.decode().unwrap();
            assert_eq!(&decoded, v, "Roundtrip failed for {:?}", v);
        }
    }

    #[test]
    fn test_roundtrip_datetime() {
        let dt = CosemDateTime {
            date: CosemDate {
                year: 2024,
                month: 6,
                day: 15,
                day_of_week: 6,
            },
            time: CosemTime {
                hour: 14,
                minute: 30,
                second: 0,
                hundredths: 0,
            },
            deviation: 480,
            clock_status: 0,
        };
        let v = DlmsType::DateTime(dt);
        let mut enc = AxdrEncoder::new();
        enc.encode(&v).unwrap();
        let mut dec = AxdrDecoder::new(enc.to_bytes());
        let decoded = dec.decode().unwrap();
        assert_eq!(decoded, v);
    }

    #[test]
    fn test_unexpected_end() {
        let mut dec = AxdrDecoder::new(&[0x06]); // uint32 tag but no data
        assert!(dec.decode().is_err());
    }

    #[test]
    fn test_invalid_tag() {
        let mut dec = AxdrDecoder::new(&[0xFE]);
        assert!(matches!(dec.decode(), Err(AxdrError::InvalidTag(0xFE))));
    }

    // ============================================================
    // Phase C — Boundary Tests
    // ============================================================

    #[test]
    fn test_decode_empty_array() {
        let mut enc = AxdrEncoder::new();
        enc.encode(&DlmsType::Array(alloc::vec![])).unwrap();
        let mut dec = AxdrDecoder::new(enc.to_bytes());
        let decoded = dec.decode().unwrap();
        assert!(matches!(decoded, DlmsType::Array(ref items) if items.is_empty()));
    }

    #[test]
    fn test_decode_empty_structure() {
        let mut enc = AxdrEncoder::new();
        enc.encode(&DlmsType::Structure(alloc::vec![])).unwrap();
        let mut dec = AxdrDecoder::new(enc.to_bytes());
        let decoded = dec.decode().unwrap();
        assert!(matches!(decoded, DlmsType::Structure(ref items) if items.is_empty()));
    }

    #[test]
    fn test_decode_nested_structure_roundtrip() {
        let inner = alloc::vec![DlmsType::from_u32(10), DlmsType::from_u32(20)];
        let outer = alloc::vec![DlmsType::Structure(inner), DlmsType::from_u32(30)];
        let original = DlmsType::Structure(outer);
        let mut enc = AxdrEncoder::new();
        enc.encode(&original).unwrap();
        let mut dec = AxdrDecoder::new(enc.to_bytes());
        let decoded = dec.decode().unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_all_integer_types() {
        let values = [
            DlmsType::UInt8(0),
            DlmsType::UInt8(255),
            DlmsType::Int8(-128),
            DlmsType::Int8(127),
            DlmsType::UInt16(0),
            DlmsType::UInt16(65535),
            DlmsType::Int16(-32768),
            DlmsType::Int16(32767),
            DlmsType::UInt32(0),
            DlmsType::UInt32(u32::MAX),
            DlmsType::Int32(i32::MIN),
            DlmsType::Int32(i32::MAX),
            DlmsType::UInt64(0),
            DlmsType::UInt64(u64::MAX),
            DlmsType::Int64(i64::MIN),
            DlmsType::Int64(i64::MAX),
        ];
        for v in &values {
            let mut enc = AxdrEncoder::new();
            enc.encode(v).unwrap();
            let mut dec = AxdrDecoder::new(enc.to_bytes());
            let decoded = dec.decode().unwrap();
            assert_eq!(decoded, *v, "Roundtrip failed for {:?}", v);
        }
    }

    #[test]
    fn test_decode_empty_input() {
        let mut dec = AxdrDecoder::new(&[]);
        assert!(dec.decode().is_err());
    }

    #[test]
    fn test_decode_truncated_data() {
        // Tag byte present but not enough data
        let mut dec = AxdrDecoder::new(&[0x0F]); // structure tag
        assert!(dec.decode().is_err());
    }

    #[test]
    fn test_decode_multiple_values() {
        let mut enc = AxdrEncoder::new();
        enc.encode(&DlmsType::from_u32(1)).unwrap();
        enc.encode(&DlmsType::from_u32(2)).unwrap();
        enc.encode(&DlmsType::from_u32(3)).unwrap();
        let bytes = enc.to_bytes();
        let mut dec = AxdrDecoder::new(&bytes);
        assert_eq!(dec.decode().unwrap(), DlmsType::from_u32(1));
        assert_eq!(dec.decode().unwrap(), DlmsType::from_u32(2));
        assert_eq!(dec.decode().unwrap(), DlmsType::from_u32(3));
        assert!(dec.decode().is_err()); // no more data
    }

    #[test]
    fn test_decode_empty_octet_string() {
        let mut enc = AxdrEncoder::new();
        enc.encode(&DlmsType::from_octet_string(alloc::vec![]))
            .unwrap();
        let mut dec = AxdrDecoder::new(enc.to_bytes());
        let decoded = dec.decode().unwrap();
        assert!(matches!(decoded, DlmsType::OctetString(ref v) if v.is_empty()));
    }

    #[test]
    fn test_decode_null_phase_c() {
        let mut enc = AxdrEncoder::new();
        enc.encode(&DlmsType::Null).unwrap();
        let mut dec = AxdrDecoder::new(enc.to_bytes());
        assert_eq!(dec.decode().unwrap(), DlmsType::Null);
    }

    #[test]
    fn test_decode_boolean() {
        for v in [true, false] {
            let mut enc = AxdrEncoder::new();
            enc.encode(&DlmsType::Boolean(v)).unwrap();
            let mut dec = AxdrDecoder::new(enc.to_bytes());
            assert_eq!(dec.decode().unwrap(), DlmsType::Boolean(v));
        }
    }
}
