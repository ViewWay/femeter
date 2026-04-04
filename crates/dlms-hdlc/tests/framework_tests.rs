//! Tests that exercise the test helpers and mock transport themselves,
//! plus additional integration scenarios enabled by the helpers.

#[path = "helpers/mod.rs"]
mod helpers;

use dlms_hdlc::control::{ControlField, FrameType};
use dlms_hdlc::frame::HdlcFrame;
use dlms_hdlc::HdlcAddress;
use helpers::*;

// ────────────────────────────────────────────────────────────────
// FrameBuilder tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_builder_ua_roundtrip() {
    let mut frame = FrameBuilder::new().ua(true).build();
    assert_frame_roundtrip(&mut frame);
    assert_eq!(frame.control.frame_type, FrameType::UA);
}

#[test]
fn test_builder_snrm_with_payload() {
    let mut frame = FrameBuilder::new()
        .client_address(0x10)
        .snrm(true)
        .payload(vec![0x01, 0x02, 0x03])
        .build();
    assert_frame_roundtrip(&mut frame);
    assert_eq!(frame.control.frame_type, FrameType::SNRM);
    assert_eq!(frame.information, vec![0x01, 0x02, 0x03]);
}

#[test]
fn test_builder_encode_shorthand() {
    let bytes = FrameBuilder::new().ua(true).encode();
    assert!(bytes.starts_with(&[0x7E]));
    assert!(bytes.ends_with(&[0x7E]));
    assert_no_unescaped_special(&bytes);
}

#[test]
fn test_builder_iframe_with_special_payload() {
    let mut frame = FrameBuilder::new()
        .information(3, 5, true)
        .payload(helpers::PayloadGenerator::new(42).hdlc_heavy(50))
        .build();
    assert_frame_roundtrip(&mut frame);
}

// ────────────────────────────────────────────────────────────────
// ConnectedSession tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_connected_session_exchange() {
    let mut session = ConnectedSession::new();

    // Send request
    let request = session.send(vec![0xC0, 0x01, 0x01]);
    assert_eq!(request.control.send_seq, 0);

    // Server response
    let response = session.server_response(0, vec![0xC4, 0x01, 0x01, 0x00]);
    let data = session.receive(&response);
    assert_eq!(data, vec![0xC4, 0x01, 0x01, 0x00]);
}

#[test]
fn test_connected_session_multi_exchange() {
    let mut session = ConnectedSession::new();

    for i in 0..10u8 {
        let req = session.send(vec![i]);
        assert_eq!(req.control.send_seq, i & 0x07);

        let resp = session.server_response(i, vec![0xFF, i]);
        let data = session.receive(&resp);
        assert_eq!(data, vec![0xFF, i]);
    }
}

// ────────────────────────────────────────────────────────────────
// MockTransport tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_mock_transport_single_frame() {
    let mut frame = FrameBuilder::new().ua(true).build();
    let mut transport = MockTransport::new();
    transport.write_frame(&mut frame);

    let read = transport.read_frame().unwrap();
    assert_eq!(read.control.frame_type, FrameType::UA);
    assert!(transport.remaining().is_empty());
}

#[test]
fn test_mock_transport_multiple_frames() {
    let mut frames = vec![
        FrameBuilder::new().ua(true).build(),
        FrameBuilder::new().rr(0, true).build(),
        FrameBuilder::new()
            .information(0, 0, false)
            .payload(vec![1, 2, 3])
            .build(),
    ];
    let mut transport = MockTransport::new();
    transport.write_frames(&mut frames);

    let read = transport.read_all_frames();
    assert_eq!(read.len(), 3);
    assert_eq!(read[0].control.frame_type, FrameType::UA);
    assert_eq!(read[1].control.frame_type, FrameType::RR);
    assert_eq!(read[2].control.frame_type, FrameType::I);
    assert_eq!(read[2].information, vec![1, 2, 3]);
}

#[test]
fn test_mock_transport_with_inter_frame_garbage() {
    let mut frame = FrameBuilder::new().ua(true).build();
    let mut transport = MockTransport::new();

    // Write garbage bytes before frame
    transport.write(&[0xFF, 0xAA, 0x00]);
    transport.write_frame(&mut frame);
    // More garbage after
    transport.write(&[0xBB, 0xCC]);

    let read = transport.read_frame().unwrap();
    assert_eq!(read.control.frame_type, FrameType::UA);
}

#[test]
fn test_mock_transport_empty() {
    let mut transport = MockTransport::new();
    assert!(transport.read_frame().is_none());
    assert!(transport.read_frame_bytes().is_none());
}

#[test]
fn test_mock_transport_incomplete_frame() {
    let mut transport = MockTransport::new();
    transport.write(&[0x7E, 0x03, 0x63]); // no closing flag
    assert!(transport.read_frame().is_none());
}

#[test]
fn test_mock_transport_raw_bytes_roundtrip() {
    let mut frame = FrameBuilder::new()
        .information(0, 0, false)
        .payload(vec![0x7E, 0x7D, 0x00, 0xFF])
        .build();

    let mut transport = MockTransport::new();
    transport.write_frame(&mut frame);

    let raw = transport.read_frame_bytes().unwrap();
    assert!(raw.starts_with(&[0x7E]));
    assert!(raw.ends_with(&[0x7E]));

    let decoded = HdlcFrame::decode(&raw).unwrap();
    assert_eq!(decoded.information, vec![0x7E, 0x7D, 0x00, 0xFF]);
}

// ────────────────────────────────────────────────────────────────
// PayloadGenerator tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_payload_generator_deterministic() {
    let mut gen1 = PayloadGenerator::new(123);
    let mut gen2 = PayloadGenerator::new(123);
    assert_eq!(gen1.bytes(100), gen2.bytes(100));
}

#[test]
fn test_payload_generator_different_seeds() {
    let mut gen1 = PayloadGenerator::new(1);
    let mut gen2 = PayloadGenerator::new(2);
    assert_ne!(gen1.bytes(100), gen2.bytes(100));
}

#[test]
fn test_payload_generator_all_bytes() {
    let gen = PayloadGenerator::new(0);
    let data = gen.all_byte_values();
    assert_eq!(data.len(), 256);
    assert_eq!(data[0], 0);
    assert_eq!(data[255], 255);

    // Roundtrip through HDLC
    let mut frame = HdlcFrame::new(
        HdlcAddress::new(1, 1, 0),
        ControlField::information(0, 0, false),
        data.clone(),
    );
    assert_frame_roundtrip(&mut frame);
}

#[test]
fn test_payload_generator_hdlc_heavy_roundtrip() {
    let gen = PayloadGenerator::new(0);
    let data = gen.hdlc_heavy(200);
    let mut frame = HdlcFrame::new(
        HdlcAddress::new(1, 1, 0),
        ControlField::information(0, 0, false),
        data.clone(),
    );
    let bytes = frame.encode();
    assert_no_unescaped_special(&bytes);
    assert_frame_roundtrip(&mut frame);
}

// ────────────────────────────────────────────────────────────────
// Assertion helpers tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_assert_decode_fails_various() {
    assert_decode_fails(&[]);
    assert_decode_fails(&[0x7E]);
    assert_decode_fails(&[0x7E, 0x7E]);
    assert_decode_fails(&[0x00, 0x01, 0x02]);
    assert_decode_fails(&[0x7E, 0xFF, 0xFF, 0xFF, 0x7E]);
}

#[test]
fn test_assert_no_unescaped_special_with_clean_frame() {
    let frame = FrameBuilder::new().ua(true).build();
    let mut f = frame;
    let bytes = f.encode();
    assert_no_unescaped_special(&bytes);
}
