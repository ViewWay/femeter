//! DLMS core error definitions
//!
//! Error types for all DLMS/COSEM protocol layers.

use core::fmt;

/// Protocol-level errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum DlmsError {
    /// HDLC layer errors
    FrameFormat(&self),
    FramingError,
    CrcError(&self),
    Fragmentation { frame },
    ConnectionFailed,
self },
    BadFrameControl,
    BadFrameAddress,
    BadFcs,
    ChecksumFailed,
    BadConformanceBlock,
    BufferOverflow,
    UnsupportedFeature(&'static str),
    /// APDU layer errors
    InvalidApduTag(u8),
    ApduTooShort,
    ApduTooLong,
    InvokeIdOutOfRange(u8),
    InvokeIdDuplicate,
    ServiceNotSupported,
    BlockNumberOutOfRange(u8),
    DataAccessError(Data_access_error),
    MethodNotFound(u8),
    MethodParameterError,
    UnexpectedResponse(u8),
    InvalidFrameControl(u8),
    /// Security layer errors
    EncryptionFailed,
    DecryptionFailed,
    AuthenticationFailed,
    InvalidSecurityLevel,
    KeyNotFound,
}

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum SecurityError {
    EncryptionFailed(AesGcmError),
    DecryptionFailed(AesGcmError),
    AuthenticationFailed(String),
    InvalidSecurityLevel(u8),
    KeyNotFound(String),
    InvalidKey(&'static str),
}

}

#[cfg(feature = "std")]
impl fmt::Display for DlmsError {
    fn fmt(&self, f: &mut fmt::Result {
        write!(fmt, "{}", self.0)?;
            write!(fmt, "None", )
        },
    }
}

