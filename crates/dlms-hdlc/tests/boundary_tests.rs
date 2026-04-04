//! HDLC Boundary & Fuzz-like Tests
//!
//! Covers malformed frames, oversized payloads, nested escaping,
//! consecutive DISC, edge-case addresses, and decoder robustness.

use dlms_hdlc::connection::{ConnectionState, HdlcConnection};
use dlms_hdlc::control::{ControlField, FrameType};
use dlms_hdlc::frame::{HdlcFrame, HDLC_ESCAPE, HDLC_FLAG};
use dlms_hdlc::segment::{reassemble_payload, segment_payload};
use dlms_hdlc::{HdlcAddress, HdlcConfig};

// ────────────────────────────────────────────────────────────────
// Malformed / Degenerate Frames
// ────────────────────────────────────────────────────────────────

#[test]
fn test_decode_empty_input() {
    assert!(HdlcFrame::decode(&[]).is_err());
}

#[test]
fn test_decode_single_flag() {
    assert!(HdlcFrame::decode(&[0x7E]).is_err());
}

#[test]
fn test_decode_two_flags_only() {
    // 0x7E 0x7E — no content between flags
    assert!(HdlcFrame::decode(&[0x7E, 0x7E]).is_err());
}

#[test]
fn test_decode_flags_with_one_byte() {
    // Not enough data for address + control + HCS + FCS
    assert!(HdlcFrame::decode(&[0x7E, 0x01, 0x7E]).is_err());
}

#[test]
fn test_decode_garbage_between_flags() {
    let data = [0x7E, 0xFF, 0xFF, 0xFF, 0xFF, 0x7E];
    // Will fail CRC, but should NOT panic
    let result = HdlcFrame::decode(&data);
    assert!(result.is_err());
}

#[test]
fn test_decode_wrong_opening_flag() {
    let data = [0x00, 0x01, 0x02, 0x03, 0x7E];
    assert!(HdlcFrame::decode(&data).is_err());
}

#[test]
fn test_decode_no_closing_flag() {
    let mut frame = make_ua_frame();
    let bytes = frame.encode();
    // Remove closing flag
    let truncated = &bytes[..bytes.len() - 1];
    assert!(HdlcFrame::decode(truncated).is_err());
}

#[test]
fn test_decode_inner_flag_terminates_early() {
    // A spurious 0x7E in the middle should cause early termination / CRC fail
    let data = [0x7E, 0x41, 0x7E, 0x00, 0x00, 0x7E];
    assert!(HdlcFrame::decode(&data).is_err());
}

// ────────────────────────────────────────────────────────────────
// Escape Sequence Edge Cases
// ────────────────────────────────────────────────────────────────

#[test]
fn test_escape_at_end_of_frame() {
    // 0x7D as last byte before closing flag (incomplete escape)
    let data = [HDLC_FLAG, 0x03, 0x63, HDLC_ESCAPE, HDLC_FLAG];
    assert!(HdlcFrame::decode(&data).is_err());
}

#[test]
fn test_escape_with_flag_after() {
    // 0x7D followed directly by 0x7E (not valid escape pair)
    let data = [
        HDLC_FLAG,
        0x03,
        0x63,
        HDLC_ESCAPE,
        HDLC_FLAG,
        0x00,
        0x00,
        HDLC_FLAG,
    ];
    // The escape + flag combo: 0x7D 0x7E means the 0x7E terminates the frame
    // leaving the escape incomplete → error
    assert!(HdlcFrame::decode(&data).is_err());
}

#[test]
fn test_nested_escaped_flag_in_payload() {
    // Payload that is ALL 0x7E bytes — every single byte must be escaped
    let payload = vec![0x7E; 20];
    let addr = HdlcAddress::new(1, 1, 0);
    let mut frame = HdlcFrame::new(
        addr,
        ControlField::information(0, 0, false),
        payload.clone(),
    );
    let bytes = frame.encode();

    // Verify no unescaped flags in body
    let inner = &bytes[1..bytes.len() - 1];
    let mut i = 0;
    while i < inner.len() {
        assert_ne!(inner[i], 0x7E, "unescaped flag at position {}", i);
        if inner[i] == HDLC_ESCAPE {
            i += 2;
        } else {
            i += 1;
        }
    }

    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.information, payload);
}

#[test]
fn test_nested_escaped_escape_in_payload() {
    // Payload that is ALL 0x7D bytes
    let payload = vec![0x7D; 20];
    let addr = HdlcAddress::new(1, 1, 0);
    let mut frame = HdlcFrame::new(
        addr,
        ControlField::information(0, 0, false),
        payload.clone(),
    );
    let bytes = frame.encode();

    // Every 0x7D must be followed by 0x5D
    let inner = &bytes[1..bytes.len() - 1];
    let mut i = 0;
    while i < inner.len() {
        if inner[i] == HDLC_ESCAPE {
            assert_eq!(inner[i + 1], 0x5D, "escape pair wrong at position {}", i);
            i += 2;
        } else {
            i += 1;
        }
    }

    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.information, payload);
}

#[test]
fn test_alternating_flag_escape_payload() {
    // Alternating 0x7E 0x7D pattern
    let payload: Vec<u8> = (0..50)
        .map(|i| if i % 2 == 0 { 0x7E } else { 0x7D })
        .collect();
    let addr = HdlcAddress::new(1, 1, 0);
    let mut frame = HdlcFrame::new(
        addr,
        ControlField::information(0, 0, false),
        payload.clone(),
    );
    let bytes = frame.encode();
    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.information, payload);
}

#[test]
fn test_escape_mask_xor_variants() {
    // The decoder uses `data[i+1] ^ 0x20` for unknown escape bytes.
    // Test that 0x7D 0x7C → 0x5C (0x7C ^ 0x20 = 0x5C), etc.
    // This is a decoder behavior test — not typical but should be consistent.
    let data = [
        HDLC_FLAG,
        0x03,
        0x63,
        HDLC_ESCAPE,
        0x7C,
        0x00,
        0x00,
        HDLC_FLAG,
    ];
    // This will likely fail CRC but should not panic
    let _ = HdlcFrame::decode(&data);
}

// ────────────────────────────────────────────────────────────────
// Oversized / Large Frames
// ────────────────────────────────────────────────────────────────

#[test]
fn test_very_large_payload_1kb() {
    let payload = vec![0xAB; 1024];
    let addr = HdlcAddress::new(1, 1, 0);
    let mut frame = HdlcFrame::new(addr, ControlField::information(3, 5, true), payload.clone());
    let bytes = frame.encode();
    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.information, payload);
    assert_eq!(decoded.control.send_seq, 3);
    assert_eq!(decoded.control.recv_seq, 5);
    assert!(decoded.control.poll_final);
}

#[test]
fn test_large_payload_4kb() {
    let payload: Vec<u8> = (0..4096).map(|i| (i & 0xFF) as u8).collect();
    let addr = HdlcAddress::new(1, 1, 0);
    let mut frame = HdlcFrame::new(
        addr,
        ControlField::information(0, 0, false),
        payload.clone(),
    );
    let bytes = frame.encode();
    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.information, payload);
}

#[test]
fn test_payload_all_0xff() {
    // 0xFF doesn't need escaping but is a good edge value
    let payload = vec![0xFF; 256];
    let addr = HdlcAddress::new(1, 1, 0);
    let mut frame = HdlcFrame::new(
        addr,
        ControlField::information(0, 0, false),
        payload.clone(),
    );
    let bytes = frame.encode();
    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.information, payload);
}

#[test]
fn test_payload_all_zeros() {
    let payload = vec![0x00; 256];
    let addr = HdlcAddress::new(1, 1, 0);
    let mut frame = HdlcFrame::new(
        addr,
        ControlField::information(0, 0, false),
        payload.clone(),
    );
    let bytes = frame.encode();
    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.information, payload);
}

// ────────────────────────────────────────────────────────────────
// CRC Corruption Tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_corrupt_first_byte_after_flag() {
    let mut frame = make_ua_frame();
    let bytes = frame.encode();
    let mut corrupted = bytes.clone();
    corrupted[1] ^= 0x01;
    assert!(HdlcFrame::decode(&corrupted).is_err());
    // Original still works
    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.control.frame_type, FrameType::UA);
}

#[test]
fn test_corrupt_last_byte_before_closing_flag() {
    let mut frame = make_ua_frame();
    let bytes = frame.encode();
    let mut corrupted = bytes.clone();
    corrupted[bytes.len() - 2] ^= 0x80;
    assert!(HdlcFrame::decode(&corrupted).is_err());
}

#[test]
fn test_corrupt_every_position() {
    let mut frame = make_iframe(&[1, 2, 3, 4, 5]);
    let bytes = frame.encode();
    for pos in 1..bytes.len() - 1 {
        let mut corrupted = bytes.clone();
        corrupted[pos] ^= 0xFF;
        // Every single-byte corruption must be caught
        assert!(
            HdlcFrame::decode(&corrupted).is_err(),
            "corruption at position {} was not detected",
            pos
        );
    }
}

// ────────────────────────────────────────────────────────────────
// Consecutive DISC Frames
// ────────────────────────────────────────────────────────────────

#[test]
fn test_consecutive_disc_frames() {
    let addr = HdlcAddress::new(1, 1, 0);
    let config = HdlcConfig::default();
    let mut conn = HdlcConnection::new(addr, config);

    // Connect
    let _ = conn.connect().unwrap();
    let ua = HdlcFrame::new(addr, ControlField::ua(true), vec![]);
    conn.handle_ua(&ua).unwrap();
    assert_eq!(conn.state, ConnectionState::Connected);

    // First DISC
    let _disc1 = conn.disconnect().unwrap();
    assert_eq!(conn.state, ConnectionState::Disconnecting);

    // Second DISC should fail (already disconnecting)
    assert!(conn.disconnect().is_err());

    // Complete disconnect
    let dm = HdlcFrame::new(addr, ControlField::dm(true), vec![]);
    conn.handle_dm(&dm).unwrap();
    assert_eq!(conn.state, ConnectionState::Disconnected);

    // DISC when disconnected should also fail
    assert!(conn.disconnect().is_err());
}

#[test]
fn test_disc_without_connect() {
    let addr = HdlcAddress::new(1, 1, 0);
    let config = HdlcConfig::default();
    let mut conn = HdlcConnection::new(addr, config);
    assert!(conn.disconnect().is_err());
}

#[test]
fn test_multiple_connect_disc_cycles() {
    let addr = HdlcAddress::new(1, 1, 0);
    let config = HdlcConfig::default();

    for _ in 0..5 {
        let mut conn = HdlcConnection::new(addr, config);
        let _ = conn.connect().unwrap();
        let ua = HdlcFrame::new(addr, ControlField::ua(true), vec![]);
        conn.handle_ua(&ua).unwrap();
        assert_eq!(conn.state, ConnectionState::Connected);

        let _ = conn.send(vec![1, 2]).unwrap();
        let _ = conn.disconnect().unwrap();
        let dm = HdlcFrame::new(addr, ControlField::dm(true), vec![]);
        conn.handle_dm(&dm).unwrap();
        assert_eq!(conn.state, ConnectionState::Disconnected);
    }
}

// ────────────────────────────────────────────────────────────────
// Sequence Number Edge Cases
// ────────────────────────────────────────────────────────────────

#[test]
fn test_sequence_wraps_multiple_times() {
    let addr = HdlcAddress::new(1, 1, 0);
    let config = HdlcConfig::default();
    let mut conn = HdlcConnection::new(addr, config);

    let _ = conn.connect().unwrap();
    let ua = HdlcFrame::new(addr, ControlField::ua(true), vec![]);
    conn.handle_ua(&ua).unwrap();

    // Send 24 frames — sequence should wrap 3 times
    for _cycle in 0..3 {
        for seq in 0..8u8 {
            let frame = conn.send(vec![seq]).unwrap();
            assert_eq!(frame.control.send_seq, seq);
        }
    }
    assert_eq!(conn.send_seq, 0); // should be back to 0
}

#[test]
fn test_receive_wrong_sequence_number() {
    let addr = HdlcAddress::new(1, 1, 0);
    let config = HdlcConfig::default();
    let mut conn = HdlcConnection::new(addr, config);

    let _ = conn.connect().unwrap();
    let ua = HdlcFrame::new(addr, ControlField::ua(true), vec![]);
    conn.handle_ua(&ua).unwrap();

    // Expecting seq 0, but receive seq 2
    let bad_frame = HdlcFrame::new(addr, ControlField::information(2, 0, false), vec![1]);
    assert!(conn.receive(&bad_frame).is_err());

    // conn.recv_seq should NOT have advanced
    assert_eq!(conn.recv_seq, 0);
}

#[test]
fn test_receive_non_iframe() {
    let addr = HdlcAddress::new(1, 1, 0);
    let config = HdlcConfig::default();
    let mut conn = HdlcConnection::new(addr, config);

    let _ = conn.connect().unwrap();
    let ua = HdlcFrame::new(addr, ControlField::ua(true), vec![]);
    conn.handle_ua(&ua).unwrap();

    // Receiving RR instead of I-frame should fail
    let rr = HdlcFrame::new(addr, ControlField::rr(0, true), vec![]);
    assert!(conn.receive(&rr).is_err());
}

// ────────────────────────────────────────────────────────────────
// Segmentation Edge Cases
// ────────────────────────────────────────────────────────────────

#[test]
fn test_segment_exact_boundary() {
    let addr = HdlcAddress::new(1, 1, 0);
    let payload = vec![0xAA; 128]; // exactly max_info_len
    let frames = segment_payload(&addr, &payload, 128, 0, 0);
    assert_eq!(frames.len(), 1);
}

#[test]
fn test_segment_one_byte_over_boundary() {
    let addr = HdlcAddress::new(1, 1, 0);
    let payload = vec![0xAA; 129];
    let frames = segment_payload(&addr, &payload, 128, 0, 0);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].information.len(), 128);
    assert_eq!(frames[1].information.len(), 1);
    assert!(frames[1].control.poll_final);
}

#[test]
fn test_segment_with_max_info_len_1() {
    let addr = HdlcAddress::new(1, 1, 0);
    let payload = vec![1, 2, 3, 4, 5];
    let frames = segment_payload(&addr, &payload, 1, 0, 0);
    assert_eq!(frames.len(), 5);
    for (i, f) in frames.iter().enumerate() {
        assert_eq!(f.control.send_seq, (i as u8) & 0x07);
        assert_eq!(f.information.len(), 1);
        assert_eq!(f.control.poll_final, i == 4);
    }
}

#[test]
fn test_reassemble_wrong_frame_type() {
    let addr = HdlcAddress::new(1, 1, 0);
    let good = HdlcFrame::new(addr, ControlField::information(0, 0, false), vec![1, 2]);
    let bad = HdlcFrame::new(addr, ControlField::rr(1, true), vec![]);
    assert!(reassemble_payload(&[good, bad]).is_err());
}

#[test]
fn test_reassemble_gap_in_sequence() {
    let addr = HdlcAddress::new(1, 1, 0);
    let f0 = HdlcFrame::new(addr, ControlField::information(0, 0, false), vec![1]);
    let f2 = HdlcFrame::new(addr, ControlField::information(2, 0, true), vec![2]); // gap!
    assert!(reassemble_payload(&[f0, f2]).is_err());
}

#[test]
fn test_reassemble_empty_list() {
    let result = reassemble_payload(&[]);
    // Empty list → empty payload (no frames to check)
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_segment_roundtrip_large() {
    let addr = HdlcAddress::new(1, 1, 0);
    let payload: Vec<u8> = (0..2000).map(|i| (i * 7 + 3) as u8).collect();
    let frames = segment_payload(&addr, &payload, 50, 0, 3);
    let reassembled = reassemble_payload(&frames).unwrap();
    assert_eq!(reassembled, payload);
}

// ────────────────────────────────────────────────────────────────
// Address Edge Cases
// ────────────────────────────────────────────────────────────────

#[test]
fn test_address_max_values() {
    let addr = HdlcAddress::new(0x7F, 0xFFFF, 0x3F);
    let ctrl = ControlField::ua(true);
    let mut frame = HdlcFrame::new(addr, ctrl, vec![]);
    let bytes = frame.encode();
    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.address.client, 0x7F);
}

#[test]
fn test_broadcast_snrm_roundtrip() {
    // Client=0, broadcast
    let addr = HdlcAddress::new(0, 0xFFFF, 0);
    let ctrl = ControlField::snrm(true);
    let mut frame = HdlcFrame::new(addr, ctrl, vec![]);
    let bytes = frame.encode();
    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(decoded.address.client, 0);
    assert_eq!(decoded.control.frame_type, FrameType::SNRM);
}

// ────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────

fn make_ua_frame() -> HdlcFrame {
    HdlcFrame::new(HdlcAddress::new(1, 1, 0), ControlField::ua(true), vec![])
}

fn make_iframe(data: &[u8]) -> HdlcFrame {
    HdlcFrame::new(
        HdlcAddress::new(1, 1, 0),
        ControlField::information(0, 0, false),
        data.to_vec(),
    )
}
