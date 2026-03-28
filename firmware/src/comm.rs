//! RS-485 HDLC communication driver

/// RS-485 communication state
pub struct CommDriver {
    server_address: u16,
    rx_buffer: [u8; 256],
    rx_len: usize,
    transmitting: bool,
}

impl CommDriver {
    pub fn new(server_address: u16) -> Self {
        Self {
            server_address,
            rx_buffer: [0u8; 256],
            rx_len: 0,
            transmitting: false,
        }
    }

    /// Feed a received byte. Returns true if complete frame received.
    pub fn feed_byte(&mut self, byte: u8) -> bool {
        if self.rx_len < self.rx_buffer.len() {
            self.rx_buffer[self.rx_len] = byte;
            self.rx_len += 1;
        }
        // HDLC flag byte 0x7E marks end of frame
        byte == 0x7E && self.rx_len > 1
    }

    /// Get received frame bytes
    pub fn rx_frame(&self) -> &[u8] {
        &self.rx_buffer[..self.rx_len]
    }

    /// Reset RX buffer for next frame
    pub fn rx_reset(&mut self) {
        self.rx_len = 0;
    }
}
