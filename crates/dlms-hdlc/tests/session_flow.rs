//! DLMS HDLC Session Flow Integration Test
//!
//! Simulates a complete DLMS/COSEM session:
//!   SNRM → UA → AARQ → AARE → GetRequest → GetResponse → DISC → DM
//!
//! This test verifies the full HDLC + APDU pipeline end-to-end.

use dlms_hdlc::*;
use dlms_hdlc::connection::{ConnectionState, HdlcConnection};
use dlms_hdlc::control::FrameType;

#[test]
fn test_full_dlms_session_normal_flow() {
    // --- Setup ---
    let client_addr = HdlcAddress::new(1, 0x10, 0); // client address 0x10
    let server_addr = HdlcAddress::new(1, 1, 0);    // server logical address 1
    let config = HdlcConfig::default();
    let mut client = HdlcConnection::new(client_addr, config);

    // === Step 1: SNRM (Set Normal Response Mode) ===
    let mut snrm = client.connect().expect("SNRM should succeed");
    assert_eq!(client.state, ConnectionState::Connecting);
    assert_eq!(snrm.control.frame_type, FrameType::SNRM);
    assert!(snrm.control.poll_final, "SNRM must have P-bit set");

    // Verify SNRM encodes/decodes correctly
    let snrm_bytes = snrm.encode();
    assert!(snrm_bytes.starts_with(&[0x7E]), "Frame must start with flag");
    assert!(snrm_bytes.ends_with(&[0x7E]), "Frame must end with flag");
    let snrm_decoded = HdlcFrame::decode(&snrm_bytes).expect("SNRM roundtrip");
    assert_eq!(snrm_decoded.control.frame_type, FrameType::SNRM);

    // === Step 2: UA (Unnumbered Acknowledge) ===
    let ua_frame = HdlcFrame::new(
        server_addr,
        dlms_hdlc::control::ControlField::ua(true),
        vec![],
    );
    client.handle_ua(&ua_frame).expect("UA handling should succeed");
    assert_eq!(client.state, ConnectionState::Connected);
    assert_eq!(client.send_seq, 0);
    assert_eq!(client.recv_seq, 0);

    // === Step 3: Send I-frame with APDU data (AARQ equivalent) ===
    // Simulate AARQ APDU payload
    let aarq_payload: Vec<u8> = vec![0x60, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01];
    let mut iframe1 = client.send(aarq_payload.clone()).expect("I-frame send");
    assert_eq!(iframe1.control.frame_type, FrameType::I);
    assert_eq!(iframe1.control.send_seq, 0);
    assert_eq!(iframe1.information, aarq_payload);

    // Verify I-frame roundtrip
    let iframe1_bytes = iframe1.encode();
    let iframe1_decoded = HdlcFrame::decode(&iframe1_bytes).expect("I-frame roundtrip");
    assert_eq!(iframe1_decoded.control.frame_type, FrameType::I);
    assert_eq!(iframe1_decoded.information, aarq_payload);

    // === Step 4: Server sends RR (Receiver Ready) ===
    let rr = client.rr();
    assert_eq!(rr.control.frame_type, FrameType::RR);
    assert_eq!(rr.control.recv_seq, 0); // expecting seq 0

    // === Step 5: Server sends I-frame response (AARE) ===
    let aare_payload: Vec<u8> = vec![0x61, 0x05, 0x00, 0x00, 0x00, 0x01, 0x00];
    let server_response = HdlcFrame::new(
        client_addr,
        dlms_hdlc::control::ControlField::information(0, 0, false),
        aare_payload.clone(),
    );
    let received_data = client.receive(&server_response).expect("receive should succeed");
    assert_eq!(received_data, aare_payload);
    assert_eq!(client.recv_seq, 1);

    // === Step 6: Send GetRequest ===
    let get_request: Vec<u8> = vec![
        0xC0, 0x01, // Get-Request-Normal
        0x01,       // invoke-id = 1
        0x00, 0x01, // class_id = 1 (Data)
        0x01, 0x00, 0x00, 0x02, 0x00, 0xFF, // OBIS 1.0.0.2.0.255 (current summation)
        0x02,       // attribute 2 (value)
    ];
    let iframe2 = client.send(get_request.clone()).expect("GetRequest I-frame");
    assert_eq!(iframe2.control.send_seq, 1);

    // === Step 7: Server sends GetResponse ===
    let get_response: Vec<u8> = vec![
        0xC4, 0x01, // Get-Response-Normal
        0x01,       // invoke-id = 1
        0x00,       // result = success
        0x06, 0x00, 0x00, 0x01, 0x00, // double-long: 65536
    ];
    let server_get_resp = HdlcFrame::new(
        client_addr,
        dlms_hdlc::control::ControlField::information(1, 1, false),
        get_response.clone(),
    );
    let resp_data = client.receive(&server_get_resp).expect("GetResponse receive");
    assert_eq!(resp_data, get_response);
    assert_eq!(client.recv_seq, 2);

    // === Step 8: DISC (Disconnect) ===
    let disc = client.disconnect().expect("DISC should succeed");
    assert_eq!(client.state, ConnectionState::Disconnecting);
    assert_eq!(disc.control.frame_type, FrameType::DISC);
    assert!(disc.control.poll_final);

    // === Step 9: DM (Disconnect Mode) ===
    let dm_frame = HdlcFrame::new(
        server_addr,
        dlms_hdlc::control::ControlField::dm(true),
        vec![],
    );
    client.handle_dm(&dm_frame).expect("DM handling");
    assert_eq!(client.state, ConnectionState::Disconnected);
}

#[test]
fn test_hdlc_frame_with_byte_stuffing() {
    // Payload containing 0x7E and 0x7D (reserved bytes that must be escaped)
    let payload: Vec<u8> = vec![0x7E, 0x7D, 0x01, 0x02, 0x7E, 0x7E, 0x7D];
    let addr = HdlcAddress::new(1, 1, 0);
    let mut frame = HdlcFrame::new(
        addr,
        dlms_hdlc::control::ControlField::information(0, 0, false),
        payload.clone(),
    );

    let encoded = frame.encode();

    // Verify no unescaped 0x7E/0x7D in the data portion (between first and last flag)
    let inner = &encoded[1..encoded.len() - 1];
    let mut i = 0;
    while i < inner.len() {
        assert!(
            inner[i] != 0x7E,
            "Unescaped 0x7E found at index {} in encoded frame",
            i
        );
        if inner[i] == 0x7D {
            i += 2; // skip escape sequence
        } else {
            i += 1;
        }
    }

    // Decode and verify payload matches
    let decoded = HdlcFrame::decode(&encoded).expect("decode stuffed frame");
    assert_eq!(decoded.information, payload);
}

#[test]
fn test_multiple_sequential_frames() {
    let addr = HdlcAddress::new(1, 1, 0);
    let config = HdlcConfig::default();
    let mut conn = HdlcConnection::new(addr, config);

    // Connect
    let _ = conn.connect().unwrap();
    let ua = HdlcFrame::new(addr, dlms_hdlc::control::ControlField::ua(true), vec![]);
    conn.handle_ua(&ua).unwrap();

    // Send 8 frames (fill sequence 0-7, wrapping to 0 next)
    for seq in 0..8u8 {
        let data = vec![seq; 10];
        let frame = conn.send(data).unwrap();
        assert_eq!(
            frame.control.send_seq, seq,
            "send_seq should be {}",
            seq
        );
    }

    // 9th frame wraps to 0
    let frame = conn.send(vec![0x42]).unwrap();
    assert_eq!(frame.control.send_seq, 0, "sequence should wrap to 0");
}

#[test]
fn test_crc_protection_across_session() {
    // Verify CRC integrity through a complete frame exchange
    let addr = HdlcAddress::new(1, 1, 0);
    let payload: Vec<u8> = (0u8..50).collect();
    let mut frame = HdlcFrame::new(
        addr,
        dlms_hdlc::control::ControlField::information(0, 0, false),
        payload,
    );

    let encoded = frame.encode();

    // Corrupt one byte in the middle
    let mut corrupted = encoded.clone();
    let mid = corrupted.len() / 2;
    corrupted[mid] = corrupted[mid].wrapping_add(1);

    // Decode should fail CRC check
    let result = HdlcFrame::decode(&corrupted);
    assert!(result.is_err(), "Corrupted frame should fail CRC check");
}

#[test]
fn test_reject_connect_when_not_disconnected() {
    let addr = HdlcAddress::new(1, 1, 0);
    let config = HdlcConfig::default();
    let mut conn = HdlcConnection::new(addr, config);

    // Already connecting
    let _ = conn.connect().unwrap();
    assert!(conn.connect().is_err(), "double connect should fail");

    // Already connected
    let ua = HdlcFrame::new(addr, dlms_hdlc::control::ControlField::ua(true), vec![]);
    conn.handle_ua(&ua).unwrap();
    assert!(conn.connect().is_err(), "connect when connected should fail");
}

#[test]
fn test_send_when_disconnected_fails() {
    let addr = HdlcAddress::new(1, 1, 0);
    let config = HdlcConfig::default();
    let mut conn = HdlcConnection::new(addr, config);

    let result = conn.send(vec![1, 2, 3]);
    assert!(result.is_err(), "send when disconnected should fail");
}
