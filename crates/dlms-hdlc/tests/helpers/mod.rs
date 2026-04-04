//! Test helpers for dlms-hdlc integration tests
//!
//! Provides mock transport, common fixtures, and frame builders
//! to reduce boilerplate in HDLC tests.

#![allow(dead_code)]

use dlms_hdlc::connection::{ConnectionState, HdlcConnection};
use dlms_hdlc::control::ControlField;
use dlms_hdlc::frame::{HdlcFrame, HDLC_FLAG};

use dlms_hdlc::{HdlcAddress, HdlcConfig};

// ────────────────────────────────────────────────────────────────
// Frame Builders
// ────────────────────────────────────────────────────────────────

/// Convenience builder for HDLC frames
pub struct FrameBuilder {
    address: HdlcAddress,
    control: Option<ControlField>,
    information: Vec<u8>,
}

impl FrameBuilder {
    pub fn new() -> Self {
        Self {
            address: HdlcAddress::new(1, 1, 0),
            control: None,
            information: Vec::new(),
        }
    }

    pub fn address(mut self, client: u8, server_upper: u16, server_lower: u16) -> Self {
        self.address = HdlcAddress::new(client, server_upper, server_lower);
        self
    }

    pub fn client_address(self, addr: u8) -> Self {
        self.address(addr, 1, 0)
    }

    pub fn server_address(self, logical: u16) -> Self {
        self.address(1, logical, 0)
    }

    pub fn control(mut self, ctrl: ControlField) -> Self {
        self.control = Some(ctrl);
        self
    }

    pub fn snrm(self, poll: bool) -> Self {
        self.control(ControlField::snrm(poll))
    }

    pub fn ua(self, poll: bool) -> Self {
        self.control(ControlField::ua(poll))
    }

    pub fn disc(self, poll: bool) -> Self {
        self.control(ControlField::disc(poll))
    }

    pub fn dm(self, poll: bool) -> Self {
        self.control(ControlField::dm(poll))
    }

    pub fn rr(self, recv_seq: u8, poll: bool) -> Self {
        self.control(ControlField::rr(recv_seq, poll))
    }

    pub fn information(self, send_seq: u8, recv_seq: u8, poll: bool) -> Self {
        self.control(ControlField::information(send_seq, recv_seq, poll))
    }

    pub fn payload(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.information = data.into();
        self
    }

    /// Build the frame (must have called a control method)
    pub fn build(self) -> HdlcFrame {
        HdlcFrame::new(
            self.address,
            self.control.expect("FrameBuilder: must set control type"),
            self.information,
        )
    }

    /// Build and encode to bytes
    pub fn encode(self) -> Vec<u8> {
        let mut frame = self.build();
        frame.encode()
    }
}

impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────
// Connected Session Fixture
// ────────────────────────────────────────────────────────────────

/// Pre-connected client+server address pair ready for data exchange
pub struct ConnectedSession {
    pub client_conn: HdlcConnection,
    pub client_addr: HdlcAddress,
    pub server_addr: HdlcAddress,
}

impl ConnectedSession {
    /// Create a new connected session with default addresses
    pub fn new() -> Self {
        Self::with_addresses(HdlcAddress::new(1, 0x10, 0), HdlcAddress::new(1, 1, 0))
    }

    /// Create a connected session with specific addresses
    /// Create a connected session with specific addresses
    pub fn with_addresses(client_addr: HdlcAddress, server_addr: HdlcAddress) -> Self {
        let mut client_conn = HdlcConnection::new(client_addr, HdlcConfig::default());
        client_conn.connect().unwrap();
        let ua = HdlcFrame::new(server_addr, ControlField::ua(true), vec![]);
        client_conn.handle_ua(&ua).unwrap();
        assert_eq!(client_conn.state, ConnectionState::Connected);

        Self {
            client_conn,
            client_addr,
            server_addr,
        }
    }

    /// Send data through client connection, returns encoded I-frame
    pub fn send(&mut self, data: impl Into<Vec<u8>>) -> HdlcFrame {
        self.client_conn.send(data.into()).unwrap()
    }

    /// Receive a server I-frame through client connection
    pub fn receive(&mut self, frame: &HdlcFrame) -> Vec<u8> {
        self.client_conn.receive(frame).unwrap().to_vec()
    }

    /// Create a server I-frame response
    /// Create a server I-frame response with explicit sequence numbers.
    /// `server_send_seq` is the server's send sequence number.
    pub fn server_response(&self, server_send_seq: u8, data: impl Into<Vec<u8>>) -> HdlcFrame {
        HdlcFrame::new(
            self.client_addr,
            ControlField::information(
                server_send_seq,
                self.client_conn.send_seq, // server's recv_seq = client's send_seq
                false,
            ),
            data.into(),
        )
    }

    /// Disconnect the session
    pub fn disconnect(&mut self) -> HdlcFrame {
        self.client_conn.disconnect().unwrap()
    }

    /// Handle DM from server
    pub fn handle_dm(&mut self) {
        let dm = HdlcFrame::new(self.server_addr, ControlField::dm(true), vec![]);
        self.client_conn.handle_dm(&dm).unwrap();
    }
}

impl Default for ConnectedSession {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────
// Mock Transport
// ────────────────────────────────────────────────────────────────

/// Simulates a byte stream transport for HDLC frame exchange.
///
/// Usage:
/// ```
/// let mut transport = MockTransport::new();
/// transport.write_frame(&client_frame);
/// let server_frame = transport.read_frame().unwrap();
/// ```
pub struct MockTransport {
    buffer: Vec<u8>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Write raw bytes into the transport buffer
    pub fn write(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Write an encoded HDLC frame into the transport
    pub fn write_frame(&mut self, frame: &mut HdlcFrame) {
        self.buffer.extend(frame.encode());
    }

    /// Write multiple frames
    pub fn write_frames(&mut self, frames: &mut [HdlcFrame]) {
        for f in frames {
            self.write_frame(f);
        }
    }

    /// Read the next complete frame from the buffer.
    /// Returns the frame bytes (including flags) and advances the buffer.
    pub fn read_frame_bytes(&mut self) -> Option<Vec<u8>> {
        let start = self.buffer.iter().position(|&b| b == HDLC_FLAG)?;
        let end = self.buffer[start + 1..]
            .iter()
            .position(|&b| b == HDLC_FLAG)?;
        let end = start + 1 + end + 1;
        let frame_bytes = self.buffer[start..end].to_vec();
        self.buffer.drain(..end);
        Some(frame_bytes)
    }

    /// Read and decode the next frame
    pub fn read_frame(&mut self) -> Option<HdlcFrame> {
        let bytes = self.read_frame_bytes()?;
        HdlcFrame::decode(&bytes).ok()
    }

    /// Read all frames currently in the buffer
    pub fn read_all_frames(&mut self) -> Vec<HdlcFrame> {
        let mut frames = Vec::new();
        while let Some(f) = self.read_frame() {
            frames.push(f);
        }
        frames
    }

    /// Remaining unread bytes
    pub fn remaining(&self) -> &[u8] {
        &self.buffer
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────
// Arbitrary Payload Generator (for fuzz-like testing)
// ────────────────────────────────────────────────────────────────

/// Generate deterministic test payloads
pub struct PayloadGenerator {
    seed: u64,
}

impl PayloadGenerator {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Generate `len` bytes using simple LCG
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(len);
        for _ in 0..len {
            self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            data.push((self.seed >> 16) as u8);
        }
        data
    }

    /// Generate payload with specific byte patterns
    pub fn pattern(&self, pattern: u8, len: usize) -> Vec<u8> {
        vec![pattern; len]
    }

    /// Generate payload with all 256 byte values repeated
    pub fn all_byte_values(&self) -> Vec<u8> {
        (0u8..=255).collect()
    }

    /// Generate payload with HDLC-special bytes (0x7E, 0x7D) heavily
    pub fn hdlc_heavy(&self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| match i % 4 {
                0 => 0x7E,
                1 => 0x7D,
                _ => (i & 0xFF) as u8,
            })
            .collect()
    }
}

// ────────────────────────────────────────────────────────────────
// Assertions
// ────────────────────────────────────────────────────────────────

/// Assert that two frames encode/decode to the same content
pub fn assert_frame_roundtrip(frame: &mut HdlcFrame) {
    let bytes = frame.encode();
    let decoded = HdlcFrame::decode(&bytes).unwrap();
    assert_eq!(
        decoded.address, frame.address,
        "address mismatch after roundtrip"
    );
    assert_eq!(
        decoded.control.frame_type, frame.control.frame_type,
        "frame type mismatch after roundtrip"
    );
    assert_eq!(
        decoded.information, frame.information,
        "information field mismatch after roundtrip"
    );
}

/// Assert that frame bytes contain no unescaped flags/escapes between the delimiters
pub fn assert_no_unescaped_special(bytes: &[u8]) {
    let inner = &bytes[1..bytes.len() - 1];
    let mut i = 0;
    while i < inner.len() {
        assert_ne!(inner[i], 0x7E, "unescaped flag byte at position {}", i);
        if inner[i] == 0x7D {
            assert!(
                i + 1 < inner.len(),
                "escape byte at end of frame body (position {})",
                i
            );
            i += 2;
        } else {
            i += 1;
        }
    }
}

/// Assert that a given decode attempt fails (doesn't panic, returns Err)
pub fn assert_decode_fails(data: &[u8]) {
    let result = HdlcFrame::decode(data);
    assert!(
        result.is_err(),
        "expected decode to fail for bytes: {:02X?}",
        data
    );
}
