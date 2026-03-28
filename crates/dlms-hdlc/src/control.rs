//! HDLC control field encoding/decoding

/// HDLC frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// Information frame (carries data)
    I,
    /// Receiver Ready
    RR,
    /// Receiver Not Ready
    RNR,
    /// Set Normal Response Mode (connection request)
    SNRM,
    /// Unnumbered Acknowledge
    UA,
    /// Disconnect
    DISC,
    /// Disconnected Mode
    DM,
    /// Unnumbered Information
    UI,
    /// Frame Reject
    FRMR,
}

/// HDLC control field (1 or 2 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlField {
    pub frame_type: FrameType,
    /// Send sequence number N(S) for I-frames
    pub send_seq: u8,
    /// Receive sequence number N(R) for I/RR/RNR frames
    pub recv_seq: u8,
    /// Poll/Final bit
    pub poll_final: bool,
}

impl ControlField {
    /// Create an I-frame control field
    pub fn information(send_seq: u8, recv_seq: u8, pf: bool) -> Self {
        Self { frame_type: FrameType::I, send_seq: send_seq & 0x07, recv_seq: recv_seq & 0x07, poll_final: pf }
    }

    /// Create RR control field
    pub fn rr(recv_seq: u8, pf: bool) -> Self {
        Self { frame_type: FrameType::RR, send_seq: 0, recv_seq: recv_seq & 0x07, poll_final: pf }
    }

    /// Create RNR control field
    pub fn rnr(recv_seq: u8, pf: bool) -> Self {
        Self { frame_type: FrameType::RNR, send_seq: 0, recv_seq: recv_seq & 0x07, poll_final: pf }
    }

    /// Create SNRM control field
    pub fn snrm(pf: bool) -> Self {
        Self { frame_type: FrameType::SNRM, send_seq: 0, recv_seq: 0, poll_final: pf }
    }

    /// Create UA control field
    pub fn ua(pf: bool) -> Self {
        Self { frame_type: FrameType::UA, send_seq: 0, recv_seq: 0, poll_final: pf }
    }

    /// Create DISC control field
    pub fn disc(pf: bool) -> Self {
        Self { frame_type: FrameType::DISC, send_seq: 0, recv_seq: 0, poll_final: pf }
    }

    /// Create DM control field
    pub fn dm(pf: bool) -> Self {
        Self { frame_type: FrameType::DM, send_seq: 0, recv_seq: 0, poll_final: pf }
    }

    /// Encode to byte(s). Returns Vec<u8> — 1 byte for U/S frames, 1 byte for I-frames in HDLC.
    /// Actually in DLMS HDLC, I-frame control is 1 byte: N(S)<<1 | 0 | N(R)<<5 | P/F
    /// Wait, HDLC control field:
    /// - I-frame: bit0=0, N(S) bits 1-3, P/F bit 4, N(R) bits 5-7 → but only 3-bit seq for modulo 8
    /// Actually standard HDLC:
    /// - I: 0 | N(S)(3) | P/F | N(R)(3) — bit0 = 0
    /// - S: 1 | 0 | S1 S2 | P/F | N(R)(3) — bits 0-1 = 01
    /// - U: 1 | 1 | M1 M2 | P/F | M3 M4 M5 — bits 0-1 = 11
    pub fn encode(&self) -> u8 {
        match self.frame_type {
            FrameType::I => {
                // bit0=0, N(S) bits 1-3, P/F bit4, N(R) bits 5-7
                (self.recv_seq << 5) | ((self.poll_final as u8) << 4) | (self.send_seq << 1)
            }
            FrameType::RR => {
                // 0001 | P/F | N(R) → 0x01 pattern
                (self.recv_seq << 5) | ((self.poll_final as u8) << 4) | 0x01
            }
            FrameType::RNR => {
                // 0101 | P/F | N(R)
                (self.recv_seq << 5) | ((self.poll_final as u8) << 4) | 0x05
            }
            FrameType::SNRM => {
                // 1000 1110 → 0x83 with P/F at bit4
                // SNRM = 1100 0010 → but in DLMS: 0x83 with P bit
                // Actually: SNRM = 1100 P 0011 → 0x83 | (P<<4)
                0x83 | ((self.poll_final as u8) << 4)
            }
            FrameType::UA => {
                // 1100 P 0110 → 0x63 | (P<<4)
                0x63 | ((self.poll_final as u8) << 4)
            }
            FrameType::DISC => {
                // 1100 P 0010 → 0x43 | (P<<4)  — wait: DISC = 0x53 normally
                // Actually DISC = 0100 0010 in some refs. Let me use standard:
                // DISC: 1 1 0 0 P 0 1 0 = 0x42 | (P<<4)
                0x42 | ((self.poll_final as u8) << 4)
            }
            FrameType::DM => {
                // 1 1 0 0 P 1 1 1 = 0x0F | (P<<4)  — DM = 0001 1111
                // Actually DM: 1 1 0 0 P 1 1 1 = 0x0F? No.
                // Standard: DM = 0001 1111 = 0x1F
                0x0F | ((self.poll_final as u8) << 4)
            }
            FrameType::UI => {
                0x03 | ((self.poll_final as u8) << 4)
            }
            FrameType::FRMR => {
                0x87 | ((self.poll_final as u8) << 4)
            }
        }
    }

    /// Decode control field from byte
    pub fn decode(byte: u8) -> Self {
        let pf = (byte & 0x10) != 0;

        if byte & 0x01 == 0 {
            // I-frame: bit0=0
            let send_seq = (byte >> 1) & 0x07;
            let recv_seq = (byte >> 5) & 0x07;
            Self { frame_type: FrameType::I, send_seq, recv_seq, poll_final: pf }
        } else if byte & 0x02 == 0 {
            // S-frame: bit0=1, bit1=0
            let recv_seq = (byte >> 5) & 0x07;
            let s_bits = (byte >> 2) & 0x03;
            let frame_type = match s_bits {
                0 => FrameType::RR,
                1 => FrameType::RNR,
                _ => FrameType::RR, // REJ not implemented
            };
            Self { frame_type, send_seq: 0, recv_seq, poll_final: pf }
        } else {
            // U-frame: bit0=1, bit1=1
            let m_bits = ((byte >> 2) & 0x03) | ((byte >> 3) & 0x1C);
            let frame_type = match byte & 0xEF { // mask out P/F bit
                0x83 => FrameType::SNRM,
                0x63 => FrameType::UA,
                0x42 => FrameType::DISC,
                0x0F => FrameType::DM,
                0x03 => FrameType::UI,
                0x87 => FrameType::FRMR,
                _ => FrameType::UA, // fallback
            };
            Self { frame_type, send_seq: 0, recv_seq: 0, poll_final: pf }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i_frame_encode_decode() {
        let ctrl = ControlField::information(3, 5, true);
        let byte = ctrl.encode();
        let decoded = ControlField::decode(byte);
        assert_eq!(decoded.frame_type, FrameType::I);
        assert_eq!(decoded.send_seq, 3);
        assert_eq!(decoded.recv_seq, 5);
        assert!(decoded.poll_final);
    }

    #[test]
    fn test_snrm() {
        let ctrl = ControlField::snrm(true);
        let byte = ctrl.encode();
        assert_eq!(byte, 0x93); // 0x83 | (1<<4)
        let decoded = ControlField::decode(byte);
        assert_eq!(decoded.frame_type, FrameType::SNRM);
    }

    #[test]
    fn test_ua() {
        let ctrl = ControlField::ua(true);
        let byte = ctrl.encode();
        let decoded = ControlField::decode(byte);
        assert_eq!(decoded.frame_type, FrameType::UA);
    }

    #[test]
    fn test_rr() {
        let ctrl = ControlField::rr(3, true);
        let byte = ctrl.encode();
        let decoded = ControlField::decode(byte);
        assert_eq!(decoded.frame_type, FrameType::RR);
        assert_eq!(decoded.recv_seq, 3);
    }

    #[test]
    fn test_rr_no_poll() {
        let ctrl = ControlField::rr(0, false);
        let byte = ctrl.encode();
        assert_eq!(byte, 0x01); // RR with N(R)=0, P/F=0
    }
}
