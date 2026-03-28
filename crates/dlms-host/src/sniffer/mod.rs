//! Protocol sniffer for capturing and decoding HDLC frames
//!
//! This module provides tools for capturing DLMS/COSEM traffic and
//! decoding HDLC frames and APDUs.

use std::time::{SystemTime, UNIX_EPOCH};

/// Captured HDLC frame with metadata
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    /// Capture timestamp
    pub timestamp: u64,
    /// Direction (true = received, false = sent)
    pub direction: Direction,
    /// Raw frame bytes
    pub data: Vec<u8>,
    /// Decoded frame info (if successfully decoded)
    pub decoded: Option<DecodedFrame>,
}

/// Frame direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Frame received from meter
    Rx = 0,
    /// Frame sent to meter
    Tx = 1,
}

/// Decoded frame information
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    /// Frame sequence number (if present)
    pub sequence: Option<u8>,
    /// Source address
    pub source_addr: u8,
    /// Destination address
    pub dest_addr: u8,
    /// Frame type
    pub frame_type: FrameType,
}

/// Frame type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// Normal information frame
    Information = 0,
    /// Supervisory frame (ACK, NACK)
    Supervisory = 1,
    /// Unnumbered frame (UI, DISC, etc.)
    Unnumbered = 2,
    /// Unknown/Invalid
    Unknown = 3,
}

/// Protocol sniffer for HDLC/DLMS traffic
#[derive(Debug, Clone)]
pub struct ProtocolSniffer {
    /// Captured frames
    frames: Vec<CapturedFrame>,
    /// Maximum number of frames to store (0 = unlimited)
    max_frames: usize,
    /// Current frame buffer (for incomplete frames)
    buffer: Vec<u8>,
    /// Filter by client address (None = all)
    client_filter: Option<u8>,
    /// Decode APDU contents
    decode_apdu: bool,
}

impl ProtocolSniffer {
    /// Create a new protocol sniffer
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            max_frames: 1000,
            buffer: Vec::new(),
            client_filter: None,
            decode_apdu: false,
        }
    }

    /// Set maximum frames to store
    pub fn set_max_frames(&mut self, max: usize) {
        self.max_frames = max;
    }

    /// Set client address filter
    pub fn set_client_filter(&mut self, addr: Option<u8>) {
        self.client_filter = addr;
    }

    /// Enable/disable APDU decoding
    pub fn set_decode_apdu(&mut self, enabled: bool) {
        self.decode_apdu = enabled;
    }

    /// Process incoming raw bytes and capture frames
    pub fn process_bytes(&mut self, data: &[u8], direction: Direction) {
        for &byte in data {
            self.buffer.push(byte);

            // Check for frame boundary (0x7E flag)
            if byte == 0x7E {
                if self.buffer.len() > 3 {
                    // Complete frame - extract it (excluding the flag)
                    let frame_data = self.buffer[..self.buffer.len() - 1].to_vec();
                    self.capture_frame(frame_data, direction);
                }
                self.buffer.clear();
            }
        }
    }

    /// Capture a complete frame
    fn capture_frame(&mut self, mut data: Vec<u8>, direction: Direction) {
        // Remove escape sequences
        data = self.unescape(&data);

        // Apply client filter
        if let Some(filter_addr) = self.client_filter {
            if data.len() >= 4 {
                // Address is usually at position 1 or 2
                let addr = data.get(1).copied().unwrap_or(0);
                if addr != filter_addr {
                    return;
                }
            }
        }

        // Decode frame
        let decoded = self.decode_frame(&data);

        // Check frame limit
        if self.max_frames > 0 && self.frames.len() >= self.max_frames {
            self.frames.remove(0);
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.frames.push(CapturedFrame {
            timestamp,
            direction,
            data,
            decoded,
        });
    }

    /// Decode HDLC frame
    fn decode_frame(&self, data: &[u8]) -> Option<DecodedFrame> {
        if data.len() < 4 {
            return None;
        }

        // Basic HDLC parsing
        let dest_addr = data.get(1).copied().unwrap_or(0);
        let source_addr = data.get(2).copied().unwrap_or(0);

        // Determine frame type from control byte
        let control = data.get(3).copied().unwrap_or(0);
        let frame_type = if (control & 0x01) == 0 {
            FrameType::Information
        } else if (control & 0x02) == 0 {
            FrameType::Supervisory
        } else {
            FrameType::Unnumbered
        };

        Some(DecodedFrame {
            sequence: None,
            source_addr,
            dest_addr,
            frame_type,
        })
    }

    /// Remove HDLC escape sequences
    fn unescape(&self, data: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        let mut escaped = false;

        for &byte in data {
            if byte == 0x7D {
                escaped = true;
            } else if escaped {
                result.push(byte ^ 0x20);
                escaped = false;
            } else {
                result.push(byte);
            }
        }

        result
    }

    /// Get all captured frames
    pub fn frames(&self) -> &[CapturedFrame] {
        &self.frames
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Clear all captured frames
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Export captured frames to CSV format
    #[cfg(feature = "std")]
    pub fn export_csv(&self) -> String {
        let mut csv = String::from("timestamp,direction,dest,source,length\n");

        for frame in &self.frames {
            let direction_str = if frame.direction as u8 == Direction::Rx as u8 {
                "RX"
            } else {
                "TX"
            };

            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                frame.timestamp,
                direction_str,
                frame.decoded.as_ref().map(|d| d.dest_addr).unwrap_or(0),
                frame.decoded.as_ref().map(|d| d.source_addr).unwrap_or(0),
                frame.data.len()
            ));
        }

        csv
    }
}

impl Default for ProtocolSniffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sniffer_new() {
        let sniffer = ProtocolSniffer::new();
        assert_eq!(sniffer.frame_count(), 0);
    }

    #[test]
    fn test_unescape() {
        let sniffer = ProtocolSniffer::new();
        let escaped = vec![0x01, 0x7D, 0x21, 0x02]; // 0x21^0x20 = 0x01
        let unescaped = sniffer.unescape(&escaped);
        assert_eq!(unescaped, vec![0x01, 0x01, 0x02]);
    }

    #[test]
    fn test_process_bytes() {
        let mut sniffer = ProtocolSniffer::new();
        // Simulated HDLC frame with flag
        let frame = vec![0x01, 0x02, 0x03, 0x04, 0x7E];

        sniffer.process_bytes(&frame, Direction::Rx);
        assert_eq!(sniffer.frame_count(), 1);
    }

    #[test]
    fn test_set_client_filter() {
        let mut sniffer = ProtocolSniffer::new();
        sniffer.set_client_filter(Some(0x10));
        assert_eq!(sniffer.client_filter, Some(0x10));
    }

    #[test]
    fn test_clear() {
        let mut sniffer = ProtocolSniffer::new();
        let frame = vec![0x01, 0x02, 0x03, 0x04, 0x7E];
        sniffer.process_bytes(&frame, Direction::Rx);
        assert_eq!(sniffer.frame_count(), 1);

        sniffer.clear();
        assert_eq!(sniffer.frame_count(), 0);
    }

    #[test]
    fn test_export_csv() {
        let sniffer = ProtocolSniffer::new();
        let csv = sniffer.export_csv();
        assert!(csv.contains("timestamp,direction"));
    }
}
