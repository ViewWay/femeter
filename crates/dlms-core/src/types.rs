//! COSEM data type definitions (A-XDR tag mapped)
//!
//! Reference: Green Book Ed.9 §9.5, Blue Book Part 2 §2

use alloc::vec::Vec;
#[cfg(feature = "std")]
use core::fmt;

/// A-XDR tag byte for each COSEM data type
pub const TAG_NULL: u8 = 0;
pub const TAG_ARRAY: u8 = 1;
pub const TAG_STRUCTURE: u8 = 2;
pub const TAG_BOOLEAN: u8 = 3;
pub const TAG_BIT_STRING: u8 = 4;
pub const TAG_DOUBLE_LONG: u8 = 5;
pub const TAG_DOUBLE_LONG_UNSIGNED: u8 = 6;
// 7, 8 reserved
pub const TAG_OCTET_STRING: u8 = 9;
pub const TAG_VISIBLE_STRING: u8 = 10;
pub const TAG_UTF8_STRING: u8 = 12;
pub const TAG_BCD: u8 = 13;
// 14 reserved
pub const TAG_INTEGER: u8 = 15;
pub const TAG_LONG: u8 = 16;
pub const TAG_UNSIGNED: u8 = 17;
pub const TAG_LONG_UNSIGNED: u8 = 18;
pub const TAG_COMPACT_ARRAY: u8 = 19;
pub const TAG_LONG64: u8 = 20;
pub const TAG_LONG64_UNSIGNED: u8 = 21;
pub const TAG_ENUM: u8 = 22;
pub const TAG_FLOAT32: u8 = 23;
pub const TAG_FLOAT64: u8 = 24;
pub const TAG_DATETIME: u8 = 25;
pub const TAG_DATE: u8 = 26;
pub const TAG_TIME: u8 = 27;
pub const TAG_DELTA_INTEGER: u8 = 28;
pub const TAG_DELTA_LONG: u8 = 29;
pub const TAG_DELTA_DOUBLE_LONG: u8 = 30;
pub const TAG_DELTA_UNSIGNED: u8 = 31;
pub const TAG_DELTA_LONG_UNSIGNED: u8 = 32;
pub const TAG_DELTA_DOUBLE_LONG_UNSIGNED: u8 = 33;

/// COSEM date type (OCTET STRING SIZE(5))
/// Reference: Blue Book Part 2 §3.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct CosemDate {
    pub year: u16,       // 0xFFFF = not specified
    pub month: u8,       // 1-12, 0xFD=dst_end, 0xFE=dst_begin, 0xFF=not specified
    pub day: u8,         // 1-31, 0xFD=2nd_last, 0xFE=last, 0xFF=not specified
    pub day_of_week: u8, // 1=Mon..7=Sun, 0xFF=not specified
}

/// COSEM time type (OCTET STRING SIZE(4))
/// Reference: Blue Book Part 2 §3.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct CosemTime {
    pub hour: u8,       // 0-23, 0xFF=not specified
    pub minute: u8,     // 0-59, 0xFF=not specified
    pub second: u8,     // 0-59, 0xFF=not specified
    pub hundredths: u8, // 0-99, 0xFF=not specified
}

/// COSEM date-time type (OCTET STRING SIZE(12))
/// Reference: Blue Book Part 2 §3.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct CosemDateTime {
    pub date: CosemDate,
    pub time: CosemTime,
    pub deviation: i16,   // -720..+720 minutes from UTC, 0x8000=not specified
    pub clock_status: u8, // bit0=invalid, bit1=doubtful, bit7=dst_active
}

/// Clock status flags
pub mod clock_status {
    pub const INVALID: u8 = 0x01;
    pub const DOUBTFUL: u8 = 0x02;
    pub const DIFFERENT_BASE: u8 = 0x04;
    pub const INVALID_STATUS: u8 = 0x08;
    pub const DST_ACTIVE: u8 = 0x80;
}

/// Scaler and unit pair (used in Register IC3)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct ScalerUnit {
    pub scaler: i8,
    pub unit: crate::units::Unit,
}

/// The main COSEM data type enumeration.
/// Maps directly to A-XDR tag values.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
/// COSEM data type enum
///
/// Represents all possible DLMS/COSEM data types mapped to their
/// A-XDR tag encoding per Green Book §9.5.
pub enum DlmsType {
    /// null-data [0]
    Null,
    /// boolean [3]
    Boolean(bool),
    /// bit-string [4]
    BitString(Vec<u8>), // raw bits, length in bits = vec.len()*8 (padded)
    /// double-long (int32) [5]
    Int32(i32),
    /// double-long-unsigned (uint32) [6]
    UInt32(u32),
    /// octet-string [9]
    OctetString(Vec<u8>),
    /// visible-string (ASCII) [10]
    VisibleString(Vec<u8>),
    /// utf8-string [12]
    Utf8String(Vec<u8>),
    /// bcd [13]
    Bcd(u8, Vec<u8>), // (number of bytes, raw digits)
    /// integer (int8) [15]
    Int8(i8),
    /// long (int16) [16]
    Int16(i16),
    /// unsigned (uint8) [17]
    UInt8(u8),
    /// long-unsigned (uint16) [18]
    UInt16(u16),
    /// compact-array [19]
    CompactArray {
        element_type: u8,
        element_count: u32,
        data: Vec<u8>,
    },
    /// long64 (int64) [20]
    Int64(i64),
    /// long64-unsigned (uint64) [21]
    UInt64(u64),
    /// enum [22]
    Enum(u8),
    /// float32 [23]
    Float32(f32),
    /// float64 [24]
    Float64(f64),
    /// date-time [25]
    DateTime(CosemDateTime),
    /// date [26]
    Date(CosemDate),
    /// time [27]
    Time(CosemTime),
    /// delta types [28..=33]
    DeltaInt8(i8),
    DeltaInt16(i16),
    DeltaInt32(i32),
    DeltaUInt8(u8),
    DeltaUInt16(u16),
    DeltaUInt32(u32),
    /// array [1] - homogeneous elements
    Array(Vec<DlmsType>),
    /// structure [2] - heterogeneous elements
    Structure(Vec<DlmsType>),
}

impl DlmsType {
    /// Get the A-XDR tag for this type
    pub fn tag(&self) -> u8 {
        match self {
            Self::Null => TAG_NULL,
            Self::Array(_) => TAG_ARRAY,
            Self::Structure(_) => TAG_STRUCTURE,
            Self::Boolean(_) => TAG_BOOLEAN,
            Self::BitString(_) => TAG_BIT_STRING,
            Self::Int32(_) => TAG_DOUBLE_LONG,
            Self::UInt32(_) => TAG_DOUBLE_LONG_UNSIGNED,
            Self::OctetString(_) => TAG_OCTET_STRING,
            Self::VisibleString(_) => TAG_VISIBLE_STRING,
            Self::Utf8String(_) => TAG_UTF8_STRING,
            Self::Bcd(_, _) => TAG_BCD,
            Self::Int8(_) => TAG_INTEGER,
            Self::Int16(_) => TAG_LONG,
            Self::UInt8(_) => TAG_UNSIGNED,
            Self::UInt16(_) => TAG_LONG_UNSIGNED,
            Self::CompactArray { .. } => TAG_COMPACT_ARRAY,
            Self::Int64(_) => TAG_LONG64,
            Self::UInt64(_) => TAG_LONG64_UNSIGNED,
            Self::Enum(_) => TAG_ENUM,
            Self::Float32(_) => TAG_FLOAT32,
            Self::Float64(_) => TAG_FLOAT64,
            Self::DateTime(_) => TAG_DATETIME,
            Self::Date(_) => TAG_DATE,
            Self::Time(_) => TAG_TIME,
            Self::DeltaInt8(_) => TAG_DELTA_INTEGER,
            Self::DeltaInt16(_) => TAG_DELTA_LONG,
            Self::DeltaInt32(_) => TAG_DELTA_DOUBLE_LONG,
            Self::DeltaUInt8(_) => TAG_DELTA_UNSIGNED,
            Self::DeltaUInt16(_) => TAG_DELTA_LONG_UNSIGNED,
            Self::DeltaUInt32(_) => TAG_DELTA_DOUBLE_LONG_UNSIGNED,
        }
    }

    /// Create DlmsType from A-XDR tag (without value)
    pub fn tag_name(tag: u8) -> Option<&'static str> {
        match tag {
            TAG_NULL => Some("null-data"),
            TAG_ARRAY => Some("array"),
            TAG_STRUCTURE => Some("structure"),
            TAG_BOOLEAN => Some("boolean"),
            TAG_BIT_STRING => Some("bit-string"),
            TAG_DOUBLE_LONG => Some("double-long"),
            TAG_DOUBLE_LONG_UNSIGNED => Some("double-long-unsigned"),
            TAG_OCTET_STRING => Some("octet-string"),
            TAG_VISIBLE_STRING => Some("visible-string"),
            TAG_UTF8_STRING => Some("utf8-string"),
            TAG_BCD => Some("bcd"),
            TAG_INTEGER => Some("integer"),
            TAG_LONG => Some("long"),
            TAG_UNSIGNED => Some("unsigned"),
            TAG_LONG_UNSIGNED => Some("long-unsigned"),
            TAG_COMPACT_ARRAY => Some("compact-array"),
            TAG_LONG64 => Some("long64"),
            TAG_LONG64_UNSIGNED => Some("long64-unsigned"),
            TAG_ENUM => Some("enum"),
            TAG_FLOAT32 => Some("float32"),
            TAG_FLOAT64 => Some("float64"),
            TAG_DATETIME => Some("date-time"),
            TAG_DATE => Some("date"),
            TAG_TIME => Some("time"),
            TAG_DELTA_INTEGER => Some("delta-integer"),
            TAG_DELTA_LONG => Some("delta-long"),
            TAG_DELTA_DOUBLE_LONG => Some("delta-double-long"),
            TAG_DELTA_UNSIGNED => Some("delta-unsigned"),
            TAG_DELTA_LONG_UNSIGNED => Some("delta-long-unsigned"),
            TAG_DELTA_DOUBLE_LONG_UNSIGNED => Some("delta-double-long-unsigned"),
            _ => None,
        }
    }

    // Convenience constructors
    pub fn zero() -> Self {
        Self::Null
    }
    pub fn from_bool(v: bool) -> Self {
        Self::Boolean(v)
    }
    pub fn from_u8(v: u8) -> Self {
        Self::UInt8(v)
    }
    pub fn from_u16(v: u16) -> Self {
        Self::UInt16(v)
    }
    pub fn from_u32(v: u32) -> Self {
        Self::UInt32(v)
    }
    pub fn from_u64(v: u64) -> Self {
        Self::UInt64(v)
    }
    pub fn from_i8(v: i8) -> Self {
        Self::Int8(v)
    }
    pub fn from_i16(v: i16) -> Self {
        Self::Int16(v)
    }
    pub fn from_i32(v: i32) -> Self {
        Self::Int32(v)
    }
    pub fn from_i64(v: i64) -> Self {
        Self::Int64(v)
    }
    pub fn from_f32(v: f32) -> Self {
        Self::Float32(v)
    }
    pub fn from_f64(v: f64) -> Self {
        Self::Float64(v)
    }
    pub fn from_enum(v: u8) -> Self {
        Self::Enum(v)
    }
    pub fn from_octet_string(v: Vec<u8>) -> Self {
        Self::OctetString(v)
    }
    pub fn from_visible_string(v: Vec<u8>) -> Self {
        Self::VisibleString(v)
    }

    // Value extractors with type checking
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_u8(&self) -> Option<u8> {
        match self {
            Self::UInt8(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_u16(&self) -> Option<u16> {
        match self {
            Self::UInt16(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::UInt32(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::UInt64(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_i8(&self) -> Option<i8> {
        match self {
            Self::Int8(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_i16(&self) -> Option<i16> {
        match self {
            Self::Int16(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Self::Int32(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int64(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Float32(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float64(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_enum(&self) -> Option<u8> {
        match self {
            Self::Enum(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_octet_string(&self) -> Option<&[u8]> {
        match self {
            Self::OctetString(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_visible_string(&self) -> Option<&[u8]> {
        match self {
            Self::VisibleString(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[DlmsType]> {
        match self {
            Self::Array(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_structure(&self) -> Option<&[DlmsType]> {
        match self {
            Self::Structure(v) => Some(v),
            _ => None,
        }
    }

    /// Check if this is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Int8(_)
                | Self::Int16(_)
                | Self::Int32(_)
                | Self::Int64(_)
                | Self::UInt8(_)
                | Self::UInt16(_)
                | Self::UInt32(_)
                | Self::UInt64(_)
                | Self::Float32(_)
                | Self::Float64(_)
        )
    }

    /// Convert to i64 if numeric
    pub fn to_i64(&self) -> Option<i64> {
        match self {
            Self::Int8(v) => Some(*v as i64),
            Self::Int16(v) => Some(*v as i64),
            Self::Int32(v) => Some(*v as i64),
            Self::Int64(v) => Some(*v),
            Self::UInt8(v) => Some(*v as i64),
            Self::UInt16(v) => Some(*v as i64),
            Self::UInt32(v) => Some(*v as i64),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
impl fmt::Display for DlmsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Boolean(v) => write!(f, "{}", v),
            Self::Int8(v) => write!(f, "{}", v),
            Self::Int16(v) => write!(f, "{}", v),
            Self::Int32(v) => write!(f, "{}", v),
            Self::Int64(v) => write!(f, "{}", v),
            Self::UInt8(v) => write!(f, "{}", v),
            Self::UInt16(v) => write!(f, "{}", v),
            Self::UInt32(v) => write!(f, "{}", v),
            Self::UInt64(v) => write!(f, "{}", v),
            Self::Enum(v) => write!(f, "enum({})", v),
            Self::Float32(v) => write!(f, "{}", v),
            Self::Float64(v) => write!(f, "{}", v),
            Self::OctetString(v) => {
                write!(f, "0x")?;
                for b in v {
                    write!(f, "{:02X}", b)?;
                }
                Ok(())
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_tag_mapping() {
        assert_eq!(DlmsType::zero().tag(), TAG_NULL);
        assert_eq!(DlmsType::from_bool(true).tag(), TAG_BOOLEAN);
        assert_eq!(DlmsType::from_u32(0).tag(), TAG_DOUBLE_LONG_UNSIGNED);
        assert_eq!(DlmsType::from_enum(0).tag(), TAG_ENUM);
        assert_eq!(DlmsType::Array(vec![]).tag(), TAG_ARRAY);
        assert_eq!(DlmsType::Structure(vec![]).tag(), TAG_STRUCTURE);
    }

    #[test]
    fn test_value_extractors() {
        let v = DlmsType::from_u16(1234);
        assert_eq!(v.as_u16(), Some(1234));
        assert_eq!(v.as_u8(), None);
        assert!(v.is_numeric());

        let v = DlmsType::from_bool(true);
        assert!(!v.is_numeric());
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn test_to_i64() {
        assert_eq!(DlmsType::from_i8(-1).to_i64(), Some(-1));
        assert_eq!(DlmsType::from_u32(100).to_i64(), Some(100));
        assert_eq!(DlmsType::from_f32(1.0).to_i64(), None);
    }

    #[test]
    fn test_tag_name() {
        assert_eq!(DlmsType::tag_name(0), Some("null-data"));
        assert_eq!(DlmsType::tag_name(22), Some("enum"));
        assert_eq!(DlmsType::tag_name(99), None);
    }
}
