#![allow(unexpected_cfgs)]

//! Unified error types for femeter-core.

/// Unified error type for femeter-core operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FemeterError {
    /// Invalid parameter value
    #[error("invalid parameter: {0}")]
    InvalidParameter(&'static str),
    /// Buffer overflow / insufficient capacity
    #[error("buffer overflow")]
    BufferOverflow,
    /// Data out of expected range
    #[error("{field} value {value} out of range [{min}..{max}]")]
    OutOfRange {
        field: &'static str,
        value: i64,
        min: i64,
        max: i64,
    },
    /// Calibration error
    #[error("calibration error: {0}")]
    CalibrationError(&'static str),
    /// Storage error
    #[error("storage error: {0:?}")]
    StorageError(StorageErrorKind),
    /// Communication error
    #[error("communication error: {0:?}")]
    CommunicationError(CommErrorKind),
    /// Security error
    #[error("security error: {0}")]
    SecurityError(&'static str),
    /// Internal consistency error (should never happen)
    #[error("internal error: {0}")]
    InternalError(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageErrorKind {
    WriteFailed,
    ReadFailed,
    CorruptData,
    Full,
    InvalidAddress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommErrorKind {
    Timeout,
    CrcError,
    FrameError,
    ConnectionRefused,
    BufferFull,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let e = FemeterError::InvalidParameter("voltage");
        assert_eq!(format!("{}", e), "invalid parameter: voltage");
    }

    #[test]
    fn test_error_out_of_range() {
        let e = FemeterError::OutOfRange {
            field: "freq",
            value: 6000,
            min: 4500,
            max: 5500,
        };
        assert!(format!("{}", e).contains("freq"));
    }

    #[test]
    fn test_error_clone_eq() {
        let e1 = FemeterError::BufferOverflow;
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_error_size() {
        assert!(core::mem::size_of::<FemeterError>() <= 64);
    }
}
