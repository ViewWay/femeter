//! DLMS APDU + HDLC End-to-End Integration Test
//!
//! Tests the complete DLMS/COSEM application layer pipeline:
//!   APDU encode → LLC wrap → HDLC frame → byte stream → decode → verify

use dlms_apdu::*;
use dlms_core::{DlmsType, ObisCode};
use dlms_hdlc::frame::HdlcFrame;
use dlms_hdlc::llc;
use dlms_apdu::types::ServiceError;

/// Wrap APDU bytes in LLC + HDLC I-frame, encode→decode→extract payload
fn apdu_over_hdlc_roundtrip(apdu_bytes: &[u8], send_seq: u8, recv_seq: u8) -> Vec<u8> {
    let with_llc = llc::add_llc_header(true, apdu_bytes);
    let addr = dlms_hdlc::HdlcAddress::new(1, 0x10, 0);
    let control = dlms_hdlc::ControlField::information(send_seq, recv_seq, false);
    let mut frame = HdlcFrame::new(addr, control, with_llc);
    let encoded = frame.encode();
    assert!(encoded.starts_with(&[0x7E]));
    let decoded = HdlcFrame::decode(&encoded).expect("HDLC decode");
    llc::strip_llc_header(&decoded.information).expect("LLC strip").to_vec()
}

// ============================================================
// Get-Request / Get-Response
// ============================================================

#[test]
fn test_get_request_normal_roundtrip() {
    let descriptor = types::AttributeDescriptor::new(
        1, ObisCode::new(1, 0, 1, 8, 0, 255), 2,
    );
    let req = get::GetRequestNormal::new(InvokeId::new(42), descriptor);
    let encoded = req.encode().expect("encode");
    assert_eq!(encoded[0], 0xC0);

    let payload = apdu_over_hdlc_roundtrip(&encoded, 0, 0);
    assert_eq!(payload, encoded);

    let decoded = get::GetRequestNormal::decode(&payload).expect("decode");
    assert_eq!(decoded.invoke_id, InvokeId::new(42));
    assert_eq!(decoded.request.descriptor.class_id, 1);
    assert_eq!(decoded.request.descriptor.attribute_id, 2);
}

#[test]
fn test_get_response_success_roundtrip() {
    let resp = get::GetResponseNormal::success(InvokeId::new(1), DlmsType::UInt32(123456));
    let encoded = resp.encode().expect("encode");
    assert_eq!(encoded[0], 0xC4);

    let payload = apdu_over_hdlc_roundtrip(&encoded, 0, 0);
    let decoded = get::GetResponseNormal::decode(&payload).expect("decode");
    assert_eq!(decoded.invoke_id, InvokeId::new(1));
    match decoded.result {
        types::AccessResult::Success(DlmsType::UInt32(v)) => assert_eq!(v, 123456),
        other => panic!("Expected Success(UInt32), got {:?}", other),
    }
}

#[test]
fn test_get_response_error_roundtrip() {
    use dlms_core::DataAccessError;
    let resp = get::GetResponseError::new(InvokeId::new(5), DataAccessError::ObjectUndefined);
    let encoded = resp.encode();

    let payload = apdu_over_hdlc_roundtrip(&encoded, 0, 0);
    let decoded = get::GetResponseError::decode(&payload).expect("decode");
    assert_eq!(decoded.invoke_id, InvokeId::new(5));
    assert_eq!(decoded.error, DataAccessError::ObjectUndefined);
}

#[test]
fn test_get_response_enum_dispatch() {
    let resp = get::GetResponse::Data(
        get::GetResponseNormal::success(InvokeId::new(3), DlmsType::UInt16(220))
    );
    let encoded = resp.encode().expect("encode");
    let decoded = get::GetResponse::decode(&encoded).expect("decode");
    assert_eq!(decoded.invoke_id(), InvokeId::new(3));
}

// ============================================================
// Set-Request / Set-Response
// ============================================================

#[test]
fn test_set_request_normal_roundtrip() {
    let descriptor = types::AttributeDescriptor::new(
        3, ObisCode::new(0, 0, 96, 1, 0, 255), 2,
    );
    let req = set::SetRequestNormal::new(InvokeId::new(10), descriptor, DlmsType::UInt32(86400));
    let encoded = req.encode().expect("encode");
    assert_eq!(encoded[0], 0xC1);

    let payload = apdu_over_hdlc_roundtrip(&encoded, 0, 0);
    let decoded = set::SetRequestNormal::decode(&payload).expect("decode");
    assert_eq!(decoded.invoke_id, InvokeId::new(10));
    assert_eq!(decoded.item.descriptor.class_id, 3);
}

#[test]
fn test_set_response_success_roundtrip() {
    let resp = set::SetResponseNormal::success(InvokeId::new(10), DlmsType::Null);
    let encoded = resp.encode().expect("encode");

    let payload = apdu_over_hdlc_roundtrip(&encoded, 0, 0);
    let decoded = set::SetResponseNormal::decode(&payload).expect("decode");
    assert_eq!(decoded.invoke_id, InvokeId::new(10));
}

// ============================================================
// Action-Request / Action-Response
// ============================================================

#[test]
fn test_action_request_normal_roundtrip() {
    let descriptor = types::MethodDescriptor::new(
        1, ObisCode::new(0, 0, 96, 1, 0, 255), 1,
    );
    let req = action::ActionRequestNormal::new(InvokeId::new(7), descriptor);
    let encoded = req.encode().expect("encode");
    assert_eq!(encoded[0], 0xC2);

    let payload = apdu_over_hdlc_roundtrip(&encoded, 0, 0);
    let decoded = action::ActionRequestNormal::decode(&payload).expect("decode");
    assert_eq!(decoded.invoke_id, InvokeId::new(7));
    assert_eq!(decoded.method.class_id, 1);
}

#[test]
fn test_action_response_success_roundtrip() {
    let resp = action::ActionResponseNormal::success(InvokeId::new(7), DlmsType::Null);
    let encoded = resp.encode().expect("encode");

    let payload = apdu_over_hdlc_roundtrip(&encoded, 0, 0);
    let decoded = action::ActionResponseNormal::decode(&payload).expect("decode");
    assert_eq!(decoded.invoke_id, InvokeId::new(7));
}

// ============================================================
// Exception Response
// ============================================================

#[test]
fn test_exception_response_roundtrip() {
    let exc = exception::ExceptionResponse::new(InvokeId::new(1), ServiceError::OperationNotPossible);
    let encoded = exc.encode();
    assert_eq!(encoded[0], 0xC3);

    let payload = apdu_over_hdlc_roundtrip(&encoded, 0, 0);
    let decoded = exception::ExceptionResponse::decode(&payload).expect("decode");
    assert_eq!(decoded.error, ServiceError::OperationNotPossible);
}

// ============================================================
// Multi-frame session simulation
// ============================================================

#[test]
fn test_multi_apdu_session() {
    let mut send_seq = 0u8;

    // 1. GetRequest
    let get_desc = types::AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
    let get_req = get::GetRequestNormal::new(InvokeId::new(1), get_desc);
    let get_req_bytes = get_req.encode().expect("encode get");
    let p1 = apdu_over_hdlc_roundtrip(&get_req_bytes, send_seq, 0);
    send_seq += 1;
    assert_eq!(p1, get_req_bytes);

    // 2. GetResponse
    let get_resp = get::GetResponseNormal::success(InvokeId::new(1), DlmsType::UInt32(12345));
    let get_resp_bytes = get_resp.encode().expect("encode get resp");
    let p2 = apdu_over_hdlc_roundtrip(&get_resp_bytes, send_seq, 0);
    assert_eq!(p2, get_resp_bytes);

    // 3. SetRequest
    let set_desc = types::AttributeDescriptor::new(3, ObisCode::new(0, 0, 96, 1, 0, 255), 2);
    let set_req = set::SetRequestNormal::new(InvokeId::new(2), set_desc, DlmsType::UInt32(0));
    let set_req_bytes = set_req.encode().expect("encode set");
    let p3 = apdu_over_hdlc_roundtrip(&set_req_bytes, send_seq, 0);
    send_seq += 1;
    assert_eq!(p3, set_req_bytes);

    // 4. SetResponse
    let set_resp = set::SetResponseNormal::success(InvokeId::new(2), DlmsType::Null);
    let set_resp_bytes = set_resp.encode().expect("encode set resp");
    let p4 = apdu_over_hdlc_roundtrip(&set_resp_bytes, send_seq, 0);
    assert_eq!(p4, set_resp_bytes);

    assert_eq!(send_seq, 2, "should have sent 2 client frames");
}

// ============================================================
// Stress: rapid encode/decode cycle
// ============================================================

#[test]
fn test_rapid_encode_decode_cycle() {
    for i in 0..100u8 {
        let desc = types::AttributeDescriptor::new(1, ObisCode::new(1, 0, i, 8, 0, 255), 2);
        let req = get::GetRequestNormal::new(InvokeId::new(i), desc);
        let encoded = req.encode().expect("encode");

        let payload = apdu_over_hdlc_roundtrip(&encoded, i % 8, 0);
        let decoded = get::GetRequestNormal::decode(&payload).expect("decode");
        assert_eq!(decoded.invoke_id, InvokeId::new(i));
    }
}
