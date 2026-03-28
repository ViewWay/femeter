//! APDU type definitions and error types
//!
//! Reference: IEC 62056-53 (DLMS/COSEM Application Layer)

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::vec::Vec;
use dlms_core::{DlmsType, DataAccessError, ObisCode};

// ============================================================
// APDU Tag Definitions (IEC 62056-53 §8.2)
// ============================================================

/// APDU tag first byte (PDU type)
pub const TAG_GET_REQUEST: u8 = 0xC0;
pub const TAG_GET_RESPONSE: u8 = 0xC4;
pub const TAG_SET_REQUEST: u8 = 0xC1;
pub const TAG_SET_RESPONSE: u8 = 0xC5;
pub const TAG_ACTION_REQUEST: u8 = 0xC2;
pub const TAG_ACTION_RESPONSE: u8 = 0xC6;
pub const TAG_EVENT_NOTIFICATION: u8 = 0xC7;
pub const TAG_GENERAL_BLOCK_TRANSFER: u8 = 0xC8;
pub const TAG_EXCEPTION_RESPONSE: u8 = 0xC3;

/// APDU tag second byte (PDU subtype)
pub const TAG_SUBTYPE_NORMAL: u8 = 0x01;
pub const TAG_SUBTYPE_NEXT: u8 = 0x02;
pub const TAG_SUBTYPE_WITH_LIST: u8 = 0x03;
pub const TAG_SUBTYPE_DATA: u8 = 0x01;
pub const TAG_SUBTYPE_BLOCK: u8 = 0x02;
pub const TAG_SUBTYPE_DATA_ACCESS_ERROR: u8 = 0x03;

/// General block transfer command numbers
pub const BTF_LAST_BLOCK: u8 = 0x01;
pub const BTF_GET_REQUEST: u8 = 0x02;
pub const BTF_GET_RESPONSE: u8 = 0x03;
pub const BTF_SET_REQUEST: u8 = 0x04;
pub const BTF_SET_RESPONSE: u8 = 0x05;
pub const BTF_ACTION_REQUEST: u8 = 0x06;
pub const BTF_ACTION_RESPONSE: u8 = 0x07;

/// Attribute and method descriptor access selection
pub const ACCESS_GET: u8 = 0x01;
pub const ACCESS_SET: u8 = 0x02;
pub const ACCESS_ACTION: u8 = 0x03;

/// Selective access type
pub const SELECTIVE_ACCESS_BY_RANGE: u8 = 0x01;
pub const SELECTIVE_ACCESS_BY_ENTRY: u8 = 0x02;

/// Priority levels for EventNotification
pub const PRIORITY_NORMAL: u8 = 0x00;
pub const PRIORITY_HIGH: u8 = 0x01;

/// Service error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum ServiceError {
    OperationNotPossible = 0x01,
    ServiceNotSupported = 0x02,
    OtherReason = 0xFF,
}

impl ServiceError {
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0x01 => Some(Self::OperationNotPossible),
            0x02 => Some(Self::ServiceNotSupported),
            0xFF => Some(Self::OtherReason),
            _ => None,
        }
    }

    pub fn to_code(self) -> u8 {
        self as u8
    }
}

/// APDU-specific error
#[derive(Debug, Clone, PartialEq)]
pub enum ApduError {
    InvalidTag(u8),
    TooShort,
    TooLong,
    InvalidInvokeId(u8),
    InvalidBlockSize(u32),
    BlockSequenceError,
    DataAccessError(DataAccessError),
    ServiceError(ServiceError),
    ServiceNotSupported,
    UnknownPdu,
    InvalidLength,
    UnexpectedEnd,
    InvalidData,
    InvalidRange,
    TypeMismatch,
    BufferOverflow,
    EncodeError,
    DecodeError,
}

impl From<DataAccessError> for ApduError {
    fn from(e: DataAccessError) -> Self {
        Self::DataAccessError(e)
    }
}

impl core::fmt::Display for ApduError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidTag(t) => write!(f, "Invalid APDU tag: 0x{:02X}", t),
            Self::TooShort => write!(f, "APDU too short"),
            Self::TooLong => write!(f, "APDU too long"),
            Self::InvalidInvokeId(id) => write!(f, "Invalid InvokeId: {}", id),
            Self::InvalidBlockSize(size) => write!(f, "Invalid block size: {}", size),
            Self::BlockSequenceError => write!(f, "Block sequence error"),
            Self::DataAccessError(e) => write!(f, "Data access error: {:?}", e),
            Self::ServiceError(e) => write!(f, "Service error: {:?}", e),
            Self::ServiceNotSupported => write!(f, "Service not supported"),
            Self::UnknownPdu => write!(f, "Unknown PDU type"),
            Self::InvalidLength => write!(f, "Invalid length"),
            Self::UnexpectedEnd => write!(f, "Unexpected end of data"),
            Self::InvalidData => write!(f, "Invalid data"),
            Self::InvalidRange => write!(f, "Invalid range"),
            Self::TypeMismatch => write!(f, "Type mismatch"),
            Self::BufferOverflow => write!(f, "Buffer overflow"),
            Self::EncodeError => write!(f, "Encode error"),
            Self::DecodeError => write!(f, "Decode error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ApduError {}

/// Invoke ID (0-255, typically 1-255)
///
/// Invoke ID is used to match requests with responses.
/// Value 0 is not recommended for use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct InvokeId(pub u8);

impl InvokeId {
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    pub const fn get(&self) -> u8 {
        self.0
    }

    /// Next invoke ID (wrapping at 256, skips 0)
    pub fn next(&self) -> Self {
        Self(if self.0 == 255 { 1 } else { self.0 + 1 })
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for InvokeId {
    fn default() -> Self {
        Self(1)
    }
}

/// COSEM object identifier (class ID + logical name)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CosemObjectId {
    pub class_id: u16,
    pub logical_name: ObisCode,
}

impl CosemObjectId {
    pub const fn new(class_id: u16, logical_name: ObisCode) -> Self {
        Self { class_id, logical_name }
    }

    /// Size in bytes: 2 (class) + 6 (OBIS) = 8 bytes
    pub const fn encoded_size() -> usize {
        8
    }
}

/// Attribute descriptor (class + logical name + attribute id)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeDescriptor {
    pub class_id: u16,
    pub instance: ObisCode,
    pub attribute_id: u8,
}

impl AttributeDescriptor {
    pub const fn new(class_id: u16, instance: ObisCode, attribute_id: u8) -> Self {
        Self { class_id, instance, attribute_id }
    }

    /// Encoded size: 2 + 6 + 1 = 9 bytes
    pub const fn encoded_size() -> usize {
        9
    }
}

/// Method descriptor (class + logical name + method id)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodDescriptor {
    pub class_id: u16,
    pub instance: ObisCode,
    pub method_id: u8,
}

impl MethodDescriptor {
    pub const fn new(class_id: u16, instance: ObisCode, method_id: u8) -> Self {
        Self { class_id, instance, method_id }
    }

    /// Encoded size: 2 + 6 + 1 = 9 bytes
    pub const fn encoded_size() -> usize {
        9
    }
}

/// Access selector for Get/Set operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessSelector {
    /// Normal access
    None,
    /// Access with selective access descriptor (raw encoded bytes)
    WithRawData(Vec<u8>),
}

/// Access request for Get operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRequest {
    pub descriptor: AttributeDescriptor,
    pub access_selector: AccessSelector,
}

impl AccessRequest {
    pub const fn new(descriptor: AttributeDescriptor) -> Self {
        Self { descriptor, access_selector: AccessSelector::None }
    }

    pub fn with_selective_raw(descriptor: AttributeDescriptor, data: Vec<u8>) -> Self {
        Self { descriptor, access_selector: AccessSelector::WithRawData(data) }
    }
}

/// Access result for Get responses
#[derive(Debug, Clone, PartialEq)]
pub enum AccessResult {
    Success(DlmsType),
    Error(DataAccessError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoke_id_next() {
        let id = InvokeId::new(1);
        assert_eq!(id.next().get(), 2);

        let id = InvokeId::new(255);
        assert_eq!(id.next().get(), 1); // wraps
    }

    #[test]
    fn test_invoke_id_valid() {
        assert!(InvokeId::new(1).is_valid());
        assert!(InvokeId::new(255).is_valid());
        assert!(!InvokeId::new(0).is_valid());
    }

    #[test]
    fn test_attribute_descriptor_size() {
        let _desc = AttributeDescriptor::new(
            3,
            ObisCode::new(1, 0, 1, 8, 0, 255),
            2,
        );
        assert_eq!(AttributeDescriptor::encoded_size(), 9);
    }

    #[test]
    fn test_method_descriptor_size() {
        let _desc = MethodDescriptor::new(
            70,
            ObisCode::new(0, 0, 96, 3, 10, 255),
            1,
        );
        assert_eq!(MethodDescriptor::encoded_size(), 9);
    }
}
