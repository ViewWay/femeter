//! Exception Response PDU
//!
//! Reference: IEC 62056-53 §8.4.5

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::vec::Vec;
use crate::types::{ApduError, InvokeId, TAG_EXCEPTION_RESPONSE, ServiceError};
use crate::codec::{ApduEncoder, ApduDecoder};

/// Exception Response PDU
///
/// Sent when a service cannot be performed or is not supported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptionResponse {
    pub invoke_id: InvokeId,
    pub error: ServiceError,
}

impl ExceptionResponse {
    pub fn new(invoke_id: InvokeId, error: ServiceError) -> Self {
        Self { invoke_id, error }
    }

    /// Operation not possible error
    pub fn operation_not_possible(invoke_id: InvokeId) -> Self {
        Self {
            invoke_id,
            error: ServiceError::OperationNotPossible,
        }
    }

    /// Service not supported error
    pub fn service_not_supported(invoke_id: InvokeId) -> Self {
        Self {
            invoke_id,
            error: ServiceError::ServiceNotSupported,
        }
    }

    /// Other reason error
    pub fn other_reason(invoke_id: InvokeId) -> Self {
        Self {
            invoke_id,
            error: ServiceError::OtherReason,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_EXCEPTION_RESPONSE, 0x00);
        enc.write_invoke_id(self.invoke_id);
        enc.write_byte(self.error.to_code());
        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, _subtype) = dec.read_tag()?;
        if tag_type != TAG_EXCEPTION_RESPONSE {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let error_code = dec.read_byte()?;
        let error = ServiceError::from_code(error_code)
            .ok_or(ApduError::InvalidData)?;

        Ok(Self { invoke_id, error })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_response_encode() {
        let resp = ExceptionResponse::operation_not_possible(InvokeId::new(1));
        let encoded = resp.encode();

        assert_eq!(encoded[0], TAG_EXCEPTION_RESPONSE);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 0x01); // OperationNotPossible
    }

    #[test]
    fn test_exception_response_service_not_supported() {
        let resp = ExceptionResponse::service_not_supported(InvokeId::new(42));
        let encoded = resp.encode();

        assert_eq!(encoded[0], TAG_EXCEPTION_RESPONSE);
        assert_eq!(encoded[2], 42); // invoke_id
        assert_eq!(encoded[3], 0x02); // ServiceNotSupported
    }

    #[test]
    fn test_exception_response_other_reason() {
        let resp = ExceptionResponse::other_reason(InvokeId::new(1));
        let encoded = resp.encode();

        assert_eq!(encoded[0], TAG_EXCEPTION_RESPONSE);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 0xFF); // OtherReason
    }

    #[test]
    fn test_exception_response_roundtrip() {
        let resp = ExceptionResponse::new(InvokeId::new(99), ServiceError::OperationNotPossible);
        let encoded = resp.encode();
        let decoded = ExceptionResponse::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, resp.invoke_id);
        assert_eq!(decoded.error, resp.error);
    }

    #[test]
    fn test_exception_response_decode_all_errors() {
        let errors = [
            ServiceError::OperationNotPossible,
            ServiceError::ServiceNotSupported,
            ServiceError::OtherReason,
        ];

        for error in &errors {
            let resp = ExceptionResponse::new(InvokeId::new(1), *error);
            let encoded = resp.encode();
            let decoded = ExceptionResponse::decode(&encoded).unwrap();
            assert_eq!(decoded.error, *error);
        }
    }
}
