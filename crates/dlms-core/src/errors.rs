//! DLMS core error definitions
//!
//! Error types for all DLMS/COSEM protocol layers.

#[cfg(feature = "std")]
use core::fmt;

/// Data access error codes (IEC 62056-6-2 §7.3)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum DataAccessError {
    /// 0 - Success (no error)
    Success = 0,
    /// 1 - Read-write denied
    ReadWriteDenied = 1,
    /// 2 - Object undefined
    ObjectUndefined = 2,
    /// 3 - Hardware fault
    HardwareFault = 3,
    /// 4 - Temporary failure
    TemporaryFailure = 4,
    /// 5 - Read-write denied (permanent)
    ReadWriteDeniedPermanent = 5,
    /// 6 - Unsupported object class
    UnsupportedClass = 6,
    /// 7 - Unavailable object
    UnavailableObject = 7,
    /// 8 - Type unmatched
    TypeUnmatched = 8,
    /// 9 - Scope of access violated
    ScopeViolation = 9,
    /// 10 - Data block unavailable
    DataBlockUnavailable = 10,
    /// 11 - Long get aborted
    LongGetAborted = 11,
    /// 12 - No long get in progress
    NoLongGetInProgress = 12,
    /// 13 - Long set aborted
    LongSetAborted = 13,
    /// 14 - No long set in progress
    NoLongSetInProgress = 14,
    /// 15 - Data block number inconsistent
    DataBlockNumberInconsistent = 15,
    /// 16 - Other reason
    OtherReason = 250,
}

impl DataAccessError {
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            1 => Some(Self::ReadWriteDenied),
            2 => Some(Self::ObjectUndefined),
            3 => Some(Self::HardwareFault),
            4 => Some(Self::TemporaryFailure),
            5 => Some(Self::ReadWriteDeniedPermanent),
            6 => Some(Self::UnsupportedClass),
            7 => Some(Self::UnavailableObject),
            8 => Some(Self::TypeUnmatched),
            9 => Some(Self::ScopeViolation),
            10 => Some(Self::DataBlockUnavailable),
            11 => Some(Self::LongGetAborted),
            12 => Some(Self::NoLongGetInProgress),
            13 => Some(Self::LongSetAborted),
            14 => Some(Self::NoLongSetInProgress),
            15 => Some(Self::DataBlockNumberInconsistent),
            250 => Some(Self::OtherReason),
            _ => None,
        }
    }
}

/// Service error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum ServiceError {
    OperationNotPossible,
    ServiceNotSupported,
    OtherReason,
}

/// General DLMS protocol error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum DlmsError {
    // HDLC layer
    FramingError,
    CrcError,
    Fragmentation,
    ConnectionFailed,
    BadFrameControl,
    BadFrameAddress,
    BadFcs,
    ChecksumFailed,
    BadConformanceBlock,
    BufferOverflow,
    UnsupportedFeature,
    // APDU layer
    InvalidApduTag(u8),
    ApduTooShort,
    ApduTooLong,
    InvokeIdOutOfRange(u8),
    InvokeIdDuplicate,
    ServiceNotSupported,
    BlockNumberOutOfRange(u32),
    DataAccessError(DataAccessError),
    MethodNotFound(u8),
    MethodParameterError,
    UnexpectedResponse(u8),
    InvalidFrameControl(u8),
    // Encoding
    EncodeError,
    DecodeError(&'static str),
    UnexpectedType { expected: u8, got: u8 },
    // Security
    EncryptionFailed,
    DecryptionFailed,
    AuthenticationFailed,
    InvalidSecurityLevel(u8),
    KeyNotFound,
    // General
    InvalidState,
    NotImplemented,
    BufferTooSmall,
    OutOfRange,
}

/// Security-specific errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum SecurityError {
    EncryptionFailed,
    DecryptionFailed,
    AuthenticationFailed,
    InvalidSecurityLevel(u8),
    KeyNotFound,
    InvalidKey,
    InvalidNonce,
    InvalidTag,
    CounterOverflow,
}

/// HDLC-specific errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum HdlcError {
    InvalidFlag,
    InvalidFrameFormat,
    CrcError,
    AddressError,
    ControlError,
    LengthError,
    SegmentationError,
    BufferOverflow,
    UnexpectedFrame,
    ConnectionError,
}

/// APDU-specific errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum ApduError {
    InvalidTag(u8),
    TooShort,
    TooLong,
    InvalidInvokeId(u8),
    InvalidBlockSize,
    BlockSequenceError,
    DataAccessError(DataAccessError),
    ServiceNotSupported,
    UnknownPdu,
}

/// COSEM object model errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum CosemError {
    /// Attribute does not exist for this class
    NoSuchAttribute(u8),
    /// Method does not exist for this class
    NoSuchMethod(u8),
    /// Access denied for this operation
    AccessDenied,
    /// Type mismatch on set_attribute
    TypeMismatch { expected: u8, got: u8 },
    /// Invalid parameter for method
    InvalidParameter,
    /// Object not found
    ObjectNotFound,
    /// Read-only attribute
    ReadOnly,
    /// Write-only attribute
    WriteOnly,
    /// Hardware error
    HardwareError,
    /// Temporary failure
    TemporaryFailure,
    /// NotImplemented
    NotImplemented,
}

#[cfg(feature = "std")]
impl fmt::Display for DlmsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DlmsError {}
