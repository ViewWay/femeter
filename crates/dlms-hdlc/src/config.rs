//! HDLC configuration parameters
//!
//! Reference: Green Book Ed.9 §8.4.4.2

/// HDLC configuration for parameter negotiation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdlcConfig {
    /// Window size (max outstanding I-frames, default 1)
    pub window_size: u8,
    /// Maximum transmit information field length (default 128)
    pub max_info_tx: u16,
    /// Maximum receive information field length (default 128)
    pub max_info_rx: u16,
    /// Inter-octet timeout (ms, default 250)
    pub inactivity_timeout: u16,
}

impl Default for HdlcConfig {
    fn default() -> Self {
        Self {
            window_size: 1,
            max_info_tx: 128,
            max_info_rx: 128,
            inactivity_timeout: 250,
        }
    }
}

impl HdlcConfig {
    pub fn new(window_size: u8, max_info_tx: u16, max_info_rx: u16) -> Self {
        Self {
            window_size,
            max_info_tx,
            max_info_rx,
            inactivity_timeout: 250,
        }
    }

    /// Encode SNRM negotiation payload
    /// Format: [tag][length][value] pairs
    #[allow(clippy::vec_init_then_push)]
    pub fn encode_snrm_payload(&self) -> alloc::vec::Vec<u8> {
        let mut payload = alloc::vec::Vec::new();
        payload.push(0x05);
        payload.push(0x01);
        payload.push(self.window_size);
        payload.push(0x06);
        payload.push(0x02);
        payload.push((self.max_info_tx >> 8) as u8);
        payload.push((self.max_info_tx & 0xFF) as u8);
        payload.push(0x07);
        payload.push(0x02);
        payload.push((self.max_info_rx >> 8) as u8);
        payload.push((self.max_info_rx & 0xFF) as u8);
        payload
    }

    /// Parse UA negotiation response
    pub fn parse_ua_payload(data: &[u8]) -> Option<Self> {
        let mut config = Self::default();
        let mut i = 0;
        while i + 2 <= data.len() {
            let tag = data[i];
            let len = data[i + 1] as usize;
            if i + 2 + len > data.len() { break; }
            match tag {
                0x05 if len == 1 => config.window_size = data[i + 2],
                0x06 if len == 2 => config.max_info_tx = u16::from_be_bytes([data[i + 2], data[i + 3]]),
                0x07 if len == 2 => config.max_info_rx = u16::from_be_bytes([data[i + 2], data[i + 3]]),
                _ => {}
            }
            i += 2 + len;
        }
        Some(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HdlcConfig::default();
        assert_eq!(config.window_size, 1);
        assert_eq!(config.max_info_tx, 128);
        assert_eq!(config.max_info_rx, 128);
    }

    #[test]
    fn test_snrm_payload_roundtrip() {
        let config = HdlcConfig::new(2, 256, 512);
        let payload = config.encode_snrm_payload();
        let parsed = HdlcConfig::parse_ua_payload(&payload).unwrap();
        assert_eq!(parsed.window_size, 2);
        assert_eq!(parsed.max_info_tx, 256);
        assert_eq!(parsed.max_info_rx, 512);
    }
}
