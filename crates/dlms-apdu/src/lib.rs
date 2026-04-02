//! DLMS/COSEM APDU (Application Protocol Data Unit) codec
//!
//! This crate provides encoding and decoding for DLMS/COSEM Application Layer
//! PDUs as specified in IEC 62056-53.
//!
//! # Supported APDUs
//!
//! - **Get-Request/Get-Response**: Read attribute values
//! - **Set-Request/Set-Response**: Write attribute values
//! - **Action-Request/Action-Response**: Execute methods
//! - **Event-Notification**: Push events from meter to client
//! - **General-Block-Transfer**: Transfer large data blocks
//! - **Exception-Response**: Report service errors
//! - **Initiate-Request/Initiate-Response**: Protocol parameter negotiation
//!
//! # Example
//!
//! ```rust
//! use dlms_apdu::{GetRequest, InvokeId};
//! use dlms_core::ObisCode;
//! use crate::types::AttributeDescriptor;
//!
//! // Create a Get-Request
//! let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
//! let request = GetRequest::Normal(dlms_apdu::get::GetRequestNormal::new(
//!     InvokeId::new(1),
//!     desc,
//! ));
//!
//! // Encode to bytes
//! let _encoded = request.encode().unwrap();
//! ```

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::vec::Vec;

// Public modules
pub mod action;
pub mod block_transfer;
pub mod codec;
pub mod event;
pub mod exception;
pub mod get;
pub mod initiate;
pub mod selective;
pub mod set;
pub mod types;

// Core re-exports
pub use action::{
    ActionRequest, ActionRequestListItem, ActionRequestNext, ActionRequestNormal,
    ActionRequestWithList, ActionResponse, ActionResponseBlock, ActionResponseListItem,
    ActionResponseNormal, ActionResponseWithList,
};
pub use block_transfer::{BlockTransferCommand, GeneralBlockTransfer};
pub use codec::{ApduDecoder, ApduEncoder};
pub use event::{EventCode, EventNotification, Priority};
pub use exception::ExceptionResponse;
pub use get::{
    GetRequest, GetRequestNext, GetRequestNormal, GetRequestWithList, GetResponse,
    GetResponseBlock, GetResponseError, GetResponseNormal,
};
pub use initiate::{conformance, InitiateRequest, InitiateResponse};
pub use selective::{apply_selective_access, SelectiveAccess};
pub use set::{
    SetRequest, SetRequestItem, SetRequestNormal, SetRequestWithList, SetResponse,
    SetResponseBlock, SetResponseError, SetResponseNormal,
};
pub use types::*;

// ============================================================
// Top-level APDU encoding/decoding
// ============================================================

/// Encoded APDU enum that can represent any APDU type
#[derive(Debug, Clone, PartialEq)]
pub enum Apdu {
    GetRequest(GetRequest),
    GetResponse(GetResponse),
    SetRequest(SetRequest),
    SetResponse(SetResponse),
    ActionRequest(ActionRequest),
    ActionResponse(ActionResponse),
    EventNotification(EventNotification),
    GeneralBlockTransfer(GeneralBlockTransfer),
    ExceptionResponse(ExceptionResponse),
    InitiateRequest(InitiateRequest),
    InitiateResponse(InitiateResponse),
}

impl Apdu {
    /// Get the invoke ID if this APDU has one
    pub fn invoke_id(&self) -> Option<InvokeId> {
        match self {
            Self::GetRequest(r) => Some(r.invoke_id()),
            Self::GetResponse(r) => Some(r.invoke_id()),
            Self::SetRequest(r) => Some(r.invoke_id()),
            Self::SetResponse(r) => Some(r.invoke_id()),
            Self::ActionRequest(r) => Some(r.invoke_id()),
            Self::ActionResponse(r) => Some(r.invoke_id()),
            Self::GeneralBlockTransfer(r) => Some(r.invoke_id),
            Self::InitiateRequest(r) => Some(r.invoke_id),
            Self::InitiateResponse(r) => Some(r.invoke_id),
            Self::EventNotification(_) => None,
            Self::ExceptionResponse(r) => Some(r.invoke_id),
        }
    }

    /// Encode the APDU to bytes
    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        match self {
            Self::GetRequest(r) => r.encode(),
            Self::GetResponse(r) => r.encode(),
            Self::SetRequest(r) => r.encode(),
            Self::SetResponse(r) => r.encode(),
            Self::ActionRequest(r) => r.encode(),
            Self::ActionResponse(r) => r.encode(),
            Self::EventNotification(r) => r.encode(),
            Self::GeneralBlockTransfer(r) => Ok(r.encode()),
            Self::ExceptionResponse(r) => Ok(r.encode()),
            Self::InitiateRequest(r) => Ok(r.encode()),
            Self::InitiateResponse(r) => Ok(r.encode()),
        }
    }

    /// Decode an APDU from bytes
    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        if data.len() < 2 {
            return Err(ApduError::TooShort);
        }

        let tag_type = data[0];

        match tag_type {
            TAG_GET_REQUEST => Ok(Self::GetRequest(GetRequest::decode(data)?)),
            TAG_GET_RESPONSE => Ok(Self::GetResponse(GetResponse::decode(data)?)),
            TAG_SET_REQUEST => Ok(Self::SetRequest(SetRequest::decode(data)?)),
            TAG_SET_RESPONSE => Ok(Self::SetResponse(SetResponse::decode(data)?)),
            TAG_ACTION_REQUEST => Ok(Self::ActionRequest(ActionRequest::decode(data)?)),
            TAG_ACTION_RESPONSE => Ok(Self::ActionResponse(ActionResponse::decode(data)?)),
            TAG_EVENT_NOTIFICATION => Ok(Self::EventNotification(EventNotification::decode(data)?)),
            TAG_GENERAL_BLOCK_TRANSFER => Ok(Self::GeneralBlockTransfer(
                GeneralBlockTransfer::decode(data)?,
            )),
            TAG_EXCEPTION_RESPONSE => Ok(Self::ExceptionResponse(ExceptionResponse::decode(data)?)),
            0xFF => {
                // Initiate Request/Response have tag 0xFF
                if data.len() >= 2 && data[1] == 0x01 {
                    Ok(Self::InitiateRequest(InitiateRequest::decode(data)?))
                } else {
                    Ok(Self::InitiateResponse(InitiateResponse::decode(data)?))
                }
            }
            _ => Err(ApduError::InvalidTag(tag_type)),
        }
    }
}

/// Encode any APDU to bytes
pub fn encode_apdu(apdu: &Apdu) -> Result<Vec<u8>, ApduError> {
    apdu.encode()
}

/// Decode an APDU from bytes
pub fn decode_apdu(data: &[u8]) -> Result<Apdu, ApduError> {
    Apdu::decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use dlms_core::{DataAccessError, DlmsType, ObisCode};

    #[test]
    fn test_apdu_get_request_roundtrip() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let req = GetRequest::Normal(GetRequestNormal::new(InvokeId::new(1), desc));
        let apdu = Apdu::GetRequest(req.clone());
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id(), apdu.invoke_id());
        match decoded {
            Apdu::GetRequest(GetRequest::Normal(r)) => {
                assert_eq!(r.invoke_id.get(), 1);
            }
            _ => panic!("Wrong APDU type"),
        }
    }

    #[test]
    fn test_apdu_get_response_roundtrip() {
        let resp = GetResponse::Data(GetResponseNormal::success(
            InvokeId::new(1),
            DlmsType::from_u32(100),
        ));
        let apdu = Apdu::GetResponse(resp);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();

        match decoded {
            Apdu::GetResponse(GetResponse::Data(r)) => {
                assert_eq!(r.invoke_id.get(), 1);
            }
            _ => panic!("Wrong APDU type"),
        }
    }

    #[test]
    fn test_apdu_set_request_roundtrip() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let req = SetRequest::Normal(SetRequestNormal::new(
            InvokeId::new(1),
            desc,
            DlmsType::from_u32(100),
        ));
        let apdu = Apdu::SetRequest(req);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();

        match decoded {
            Apdu::SetRequest(SetRequest::Normal(r)) => {
                assert_eq!(r.invoke_id.get(), 1);
            }
            _ => panic!("Wrong APDU type"),
        }
    }

    #[test]
    fn test_apdu_action_request_roundtrip() {
        let method = MethodDescriptor::new(70, ObisCode::new(0, 0, 96, 3, 10, 255), 1);
        let req = ActionRequest::Normal(ActionRequestNormal::new(InvokeId::new(1), method));
        let apdu = Apdu::ActionRequest(req);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();

        match decoded {
            Apdu::ActionRequest(ActionRequest::Normal(r)) => {
                assert_eq!(r.invoke_id.get(), 1);
            }
            _ => panic!("Wrong APDU type"),
        }
    }

    #[test]
    fn test_apdu_exception_response_roundtrip() {
        let resp = ExceptionResponse::operation_not_possible(InvokeId::new(1));
        let apdu = Apdu::ExceptionResponse(resp);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();

        match decoded {
            Apdu::ExceptionResponse(r) => {
                assert_eq!(r.invoke_id.get(), 1);
            }
            _ => panic!("Wrong APDU type"),
        }
    }

    #[test]
    fn test_apdu_initiate_request_roundtrip() {
        let req = InitiateRequest::new(InvokeId::new(1), conformance::standard_meter(), 2048, 2048);
        let apdu = Apdu::InitiateRequest(req);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();

        match decoded {
            Apdu::InitiateRequest(r) => {
                assert_eq!(r.invoke_id.get(), 1);
            }
            _ => panic!("Wrong APDU type"),
        }
    }

    // ============================================================
    // Phase C — Boundary Tests
    // ============================================================

    #[test]
    fn test_apdu_decode_empty() {
        assert!(Apdu::decode(&[]).is_err());
        assert!(Apdu::decode(&[0x00]).is_err());
    }

    #[test]
    fn test_apdu_decode_invalid_tag() {
        assert!(Apdu::decode(&[0x00, 0x00]).is_err());
        assert!(Apdu::decode(&[0xFE, 0x00]).is_err());
    }

    #[test]
    fn test_get_request_with_list() {
        use crate::get::{GetRequestListItem, GetRequestWithList};
        use crate::types::{AccessSelector, AttributeDescriptor};
        let descs = alloc::vec![
            GetRequestListItem {
                descriptor: AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2),
                access_selector: AccessSelector::None
            },
            GetRequestListItem {
                descriptor: AttributeDescriptor::new(8, ObisCode::new(0, 0, 1, 0, 0, 255), 2),
                access_selector: AccessSelector::None
            },
        ];
        let req = GetRequest::WithList(GetRequestWithList::new(InvokeId::new(5), descs));
        let apdu = Apdu::GetRequest(req);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();
        assert_eq!(decoded.invoke_id(), Some(InvokeId::new(5)));
    }

    #[test]
    fn test_get_request_next_block() {
        use crate::get::GetRequestNext;
        let req = GetRequest::Next(GetRequestNext::new(InvokeId::new(3), 42));
        let apdu = Apdu::GetRequest(req);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();
        assert_eq!(decoded.invoke_id(), Some(InvokeId::new(3)));
    }

    #[test]
    fn test_set_request_roundtrip_with_value() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let value = DlmsType::Array(alloc::vec![
            DlmsType::from_u32(1),
            DlmsType::from_u32(2),
            DlmsType::from_u32(3)
        ]);
        let req = SetRequest::Normal(SetRequestNormal::new(InvokeId::new(2), desc, value));
        let apdu = Apdu::SetRequest(req);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();
        assert_eq!(decoded.invoke_id(), Some(InvokeId::new(2)));
    }

    #[test]
    fn test_action_request_roundtrip() {
        use crate::action::ActionRequestNormal;
        let method = MethodDescriptor::new(70, ObisCode::new(0, 0, 96, 3, 10, 255), 1);
        let req = ActionRequest::Normal(ActionRequestNormal::new(InvokeId::new(1), method));
        let apdu = Apdu::ActionRequest(req);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();
        assert_eq!(decoded.invoke_id(), Some(InvokeId::new(1)));
    }

    #[test]
    fn test_exception_response_variants() {
        let exceptions = [
            ExceptionResponse::operation_not_possible(InvokeId::new(1)),
            ExceptionResponse::service_not_supported(InvokeId::new(2)),
        ];
        for resp in exceptions {
            let apdu = Apdu::ExceptionResponse(resp);
            let encoded = apdu.encode().unwrap();
            let decoded = Apdu::decode(&encoded).unwrap();
            assert_eq!(decoded.invoke_id(), apdu.invoke_id());
        }
    }

    #[test]
    fn test_multiple_invoke_ids() {
        for id in 0..=255u8 {
            let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
            let req = GetRequest::Normal(GetRequestNormal::new(InvokeId::new(id), desc));
            let apdu = Apdu::GetRequest(req);
            let encoded = apdu.encode().unwrap();
            let decoded = Apdu::decode(&encoded).unwrap();
            assert_eq!(decoded.invoke_id(), Some(InvokeId::new(id)));
        }
    }

    #[test]
    fn test_get_response_with_complex_value() {
        use alloc::vec;
        let value = DlmsType::Structure(vec![
            DlmsType::from_u32(100),
            DlmsType::from_f32(220.5),
            DlmsType::from_octet_string(vec![1, 0, 1, 8, 0, 255]),
        ]);
        let resp = GetResponse::Data(GetResponseNormal::success(InvokeId::new(1), value));
        let apdu = Apdu::GetResponse(resp);
        let encoded = apdu.encode().unwrap();
        let decoded = Apdu::decode(&encoded).unwrap();
        assert_eq!(decoded.invoke_id(), Some(InvokeId::new(1)));
    }
}
