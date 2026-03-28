//! Long frame segmentation/reassembly
//!
//! Reference: Green Book Ed.9 §8.4.3
//! HDLC frames have a maximum information field size.
//! Long data must be segmented into multiple I-frames.

use alloc::vec::Vec;
use alloc::vec;
use crate::frame::HdlcFrame;
use crate::address::HdlcAddress;
use crate::control::ControlField;
use dlms_core::errors::HdlcError;

/// Segment a large payload into HDLC I-frames
pub fn segment_payload(
    address: &HdlcAddress,
    payload: &[u8],
    max_info_len: usize,
    start_send_seq: u8,
    recv_seq: u8,
) -> Vec<HdlcFrame> {
    if payload.is_empty() {
        return vec![HdlcFrame::new(
            *address,
            ControlField::information(start_send_seq, recv_seq, true),
            Vec::new(),
        )];
    }

    let mut frames = Vec::new();
    let mut offset = 0;
    let mut send_seq = start_send_seq;

    while offset < payload.len() {
        let end = (offset + max_info_len).min(payload.len());
        let chunk = payload[offset..end].to_vec();
        let is_last = end >= payload.len();
        let pf = is_last; // Poll/Final set on last frame

        let frame = HdlcFrame::new(
            *address,
            ControlField::information(send_seq & 0x07, recv_seq & 0x07, pf),
            chunk,
        );
        frames.push(frame);
        send_seq += 1;
        offset = end;
    }

    frames
}

/// Reassemble segmented HDLC I-frames into a single payload
pub fn reassemble_payload(frames: &[HdlcFrame]) -> Result<Vec<u8>, HdlcError> {
    let mut payload = Vec::new();
    for (i, frame) in frames.iter().enumerate() {
        if frame.control.frame_type != crate::control::FrameType::I {
            return Err(HdlcError::UnexpectedFrame);
        }
        // Verify sequence numbers
        let expected_seq = (i as u8) & 0x07;
        if frame.control.send_seq != expected_seq {
            return Err(HdlcError::SegmentationError);
        }
        payload.extend_from_slice(&frame.information);
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::FrameType;

    #[test]
    fn test_segment_small_payload() {
        let addr = HdlcAddress::new(1, 1, 0);
        let payload = [0x01, 0x02, 0x03];
        let frames = segment_payload(&addr, &payload, 128, 0, 0);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].information, payload);
    }

    #[test]
    fn test_segment_large_payload() {
        let addr = HdlcAddress::new(1, 1, 0);
        let payload: Vec<u8> = ((0u16..300).map(|x| x as u8)).collect();
        let frames = segment_payload(&addr, &payload, 128, 0, 0);
        assert_eq!(frames.len(), 3); // 128 + 128 + 44
        assert!(frames[0].control.send_seq == 0);
        assert!(frames[1].control.send_seq == 1);
        assert!(frames[2].control.send_seq == 2);
        assert!(frames[2].control.poll_final); // last frame has P/F
    }

    #[test]
    fn test_reassemble() {
        let addr = HdlcAddress::new(1, 1, 0);
        let payload: Vec<u8> = ((0u16..300).map(|x| x as u8)).collect();
        let frames = segment_payload(&addr, &payload, 128, 0, 0);
        let reassembled = reassemble_payload(&frames).unwrap();
        assert_eq!(reassembled, payload);
    }

    #[test]
    fn test_segment_empty() {
        let addr = HdlcAddress::new(1, 1, 0);
        let frames = segment_payload(&addr, &[], 128, 0, 0);
        assert_eq!(frames.len(), 1);
    }
}
