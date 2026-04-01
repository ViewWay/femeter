//! HDLC connection management state machine
//!
//! Reference: Green Book Ed.9 §8.4.4

use crate::address::HdlcAddress;
use crate::config::HdlcConfig;
use crate::control::{ControlField, FrameType};
use crate::frame::HdlcFrame;
use alloc::vec::Vec;
use dlms_core::errors::HdlcError;

/// HDLC connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

/// HDLC connection manager (client side)
pub struct HdlcConnection {
    pub state: ConnectionState,
    pub address: HdlcAddress,
    pub config: HdlcConfig,
    pub send_seq: u8,
    pub recv_seq: u8,
}

impl HdlcConnection {
    pub fn new(address: HdlcAddress, config: HdlcConfig) -> Self {
        Self {
            state: ConnectionState::Disconnected,
            address,
            config,
            send_seq: 0,
            recv_seq: 0,
        }
    }

    /// Initiate connection (SNRM)
    pub fn connect(&mut self) -> Result<HdlcFrame, HdlcError> {
        if self.state != ConnectionState::Disconnected {
            return Err(HdlcError::ConnectionError);
        }
        self.state = ConnectionState::Connecting;
        self.send_seq = 0;
        self.recv_seq = 0;

        // SNRM frame with configuration negotiation payload
        let info = self.config.encode_snrm_payload();
        Ok(HdlcFrame::new(self.address, ControlField::snrm(true), info))
    }

    /// Handle UA response to SNRM
    pub fn handle_ua(&mut self, _frame: &HdlcFrame) -> Result<(), HdlcError> {
        if self.state != ConnectionState::Connecting {
            return Err(HdlcError::ConnectionError);
        }
        // Parse negotiated parameters from frame information field
        // For now just transition to connected
        self.state = ConnectionState::Connected;
        Ok(())
    }

    /// Disconnect (DISC)
    pub fn disconnect(&mut self) -> Result<HdlcFrame, HdlcError> {
        if self.state != ConnectionState::Connected {
            return Err(HdlcError::ConnectionError);
        }
        self.state = ConnectionState::Disconnecting;
        Ok(HdlcFrame::new(
            self.address,
            ControlField::disc(true),
            Vec::new(),
        ))
    }

    /// Handle DM response to DISC
    pub fn handle_dm(&mut self, _frame: &HdlcFrame) -> Result<(), HdlcError> {
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    /// Send data (I-frame)
    pub fn send(&mut self, data: Vec<u8>) -> Result<HdlcFrame, HdlcError> {
        if self.state != ConnectionState::Connected {
            return Err(HdlcError::ConnectionError);
        }
        let frame = HdlcFrame::new(
            self.address,
            ControlField::information(self.send_seq & 0x07, self.recv_seq & 0x07, false),
            data,
        );
        self.send_seq = (self.send_seq + 1) & 0x07;
        Ok(frame)
    }

    /// Receive data from I-frame
    pub fn receive<'a>(&mut self, frame: &'a HdlcFrame) -> Result<&'a [u8], HdlcError> {
        if self.state != ConnectionState::Connected {
            return Err(HdlcError::ConnectionError);
        }
        if frame.control.frame_type != FrameType::I {
            return Err(HdlcError::UnexpectedFrame);
        }
        // Verify sequence number
        if frame.control.send_seq != self.recv_seq {
            return Err(HdlcError::SegmentationError);
        }
        self.recv_seq = (self.recv_seq + 1) & 0x07;
        Ok(&frame.information)
    }

    /// Create RR (Receiver Ready) frame
    pub fn rr(&self) -> HdlcFrame {
        HdlcFrame::new(
            self.address,
            ControlField::rr(self.recv_seq & 0x07, true),
            Vec::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_connect_disconnect() {
        let addr = HdlcAddress::new(1, 1, 0);
        let config = HdlcConfig::default();
        let mut conn = HdlcConnection::new(addr, config);

        // Connect
        let snrm = conn.connect().unwrap();
        assert_eq!(snrm.control.frame_type, FrameType::SNRM);
        assert_eq!(conn.state, ConnectionState::Connecting);

        // UA response
        let ua = HdlcFrame::new(addr, ControlField::ua(true), Vec::new());
        conn.handle_ua(&ua).unwrap();
        assert_eq!(conn.state, ConnectionState::Connected);

        // Send data
        let iframe = conn.send(vec![1, 2, 3]).unwrap();
        assert_eq!(iframe.control.frame_type, FrameType::I);

        // Disconnect
        let disc = conn.disconnect().unwrap();
        assert_eq!(disc.control.frame_type, FrameType::DISC);
    }

    #[test]
    fn test_send_sequence() {
        let addr = HdlcAddress::new(1, 1, 0);
        let config = HdlcConfig::default();
        let mut conn = HdlcConnection::new(addr, config);

        let _ = conn.connect().unwrap();
        let ua = HdlcFrame::new(addr, ControlField::ua(true), Vec::new());
        conn.handle_ua(&ua).unwrap();

        let f1 = conn.send(vec![1]).unwrap();
        let f2 = conn.send(vec![2]).unwrap();
        assert_eq!(f1.control.send_seq, 0);
        assert_eq!(f2.control.send_seq, 1);
    }

    #[test]
    fn test_connect_when_connected_fails() {
        let addr = HdlcAddress::new(1, 1, 0);
        let config = HdlcConfig::default();
        let mut conn = HdlcConnection::new(addr, config);
        let _ = conn.connect().unwrap();
        let ua = HdlcFrame::new(addr, ControlField::ua(true), Vec::new());
        conn.handle_ua(&ua).unwrap();

        assert!(conn.connect().is_err());
    }
}
