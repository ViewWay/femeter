//! Conformance block (24-bit feature flags)
//!
//! Reference: Green Book Ed.9 §8.3

/// 24-bit conformance block for DLMS/COSEM
///
/// Indicates which DLMS services and features are supported.
/// Reference: Green Book Ed.9 §8.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConformanceBlock {
    pub bits: [u8; 3],
}

impl ConformanceBlock {
    pub const fn new(bits: [u8; 3]) -> Self {
        Self { bits }
    }

    pub const fn empty() -> Self {
        Self { bits: [0, 0, 0] }
    }

    // Bit positions (byte index, bit mask) — MSB first
    // Byte 0, bit 7 = General Protection
    // Byte 0, bit 6 = General Block Transfer
    // Byte 0, bit 5 = Delta Value Encoding
    // ... etc

    /// Check if a specific bit is set
    pub const fn get_bit(&self, byte: usize, bit: u8) -> bool {
        (self.bits[byte] & (1 << bit)) != 0
    }

    /// Set a specific bit
    pub fn set_bit(&mut self, byte: usize, bit: u8) {
        self.bits[byte] |= 1 << bit;
    }

    // Named feature flags
    pub fn general_protection(&self) -> bool {
        self.get_bit(0, 7)
    }
    pub fn general_block_transfer(&self) -> bool {
        self.get_bit(0, 6)
    }
    pub fn delta_value_encoding(&self) -> bool {
        self.get_bit(0, 5)
    }
    pub fn attribute_0_supported(&self) -> bool {
        self.get_bit(0, 4)
    }

    pub fn priority_management(&self) -> bool {
        self.get_bit(1, 7)
    }
    pub fn attribute_and_method(&self) -> bool {
        self.get_bit(1, 6)
    }
    pub fn block_transfer_with_action(&self) -> bool {
        self.get_bit(1, 5)
    }
    pub fn block_transfer_with_set(&self) -> bool {
        self.get_bit(1, 4)
    }
    pub fn block_transfer_with_get(&self) -> bool {
        self.get_bit(1, 3)
    }

    pub fn multiple_references(&self) -> bool {
        self.get_bit(2, 7)
    }
    pub fn data_notification(&self) -> bool {
        self.get_bit(2, 6)
    }
    pub fn access(&self) -> bool {
        self.get_bit(2, 5)
    }
    pub fn get(&self) -> bool {
        self.get_bit(2, 4)
    }
    pub fn set(&self) -> bool {
        self.get_bit(2, 3)
    }
    pub fn selective_access(&self) -> bool {
        self.get_bit(2, 2)
    }
    pub fn event_notification(&self) -> bool {
        self.get_bit(2, 1)
    }
    pub fn action(&self) -> bool {
        self.get_bit(2, 0)
    }

    /// Standard conformance for a typical meter
    pub fn standard_meter() -> Self {
        let mut b = Self::empty();
        b.set_bit(1, 3); // block transfer with get
        b.set_bit(2, 4); // get
        b.set_bit(2, 3); // set
        b.set_bit(2, 2); // selective access
        b.set_bit(2, 1); // event notification
        b.set_bit(2, 0); // action
        b
    }

    /// Encode as BER bit-string (tag + length + unused-bits + data)
    /// Encode as BER bit-string (tag + length + unused-bits + data)
    pub fn encode_ber(&self) -> [u8; 6] {
        [0x04, 0x03, 0x00, self.bits[0], self.bits[1], self.bits[2]]
    }

    /// Encode to raw bytes (just the 3 conformance bytes)
    pub fn to_bytes(&self) -> [u8; 3] {
        self.bits
    }

    /// Decode from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 3 {
            return None;
        }
        Some(Self {
            bits: [bytes[0], bytes[1], bytes[2]],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conformance_empty() {
        let c = ConformanceBlock::empty();
        assert!(!c.get());
        assert!(!c.set());
    }

    #[test]
    fn test_conformance_standard() {
        let c = ConformanceBlock::standard_meter();
        assert!(c.get());
        assert!(c.set());
        assert!(c.action());
        assert!(!c.general_protection());
    }

    #[test]
    fn test_conformance_roundtrip() {
        let c = ConformanceBlock::standard_meter();
        let bytes = c.to_bytes();
        let decoded = ConformanceBlock::from_bytes(&bytes).unwrap();
        assert_eq!(c, decoded);
    }
}
