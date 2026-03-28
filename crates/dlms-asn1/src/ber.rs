//! BER TLV encoder/decoder base

use alloc::vec::Vec;
use alloc::vec;

/// BER Tag wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BerTag {
    pub class: u8,   // 0=universal, 1=application, 2=context, 3=private
    pub constructed: bool,
    pub number: u32,
}

impl BerTag {
    pub const fn universal(number: u32) -> Self {
        Self { class: 0, constructed: false, number }
    }
    pub const fn application(number: u32) -> Self {
        Self { class: 1, constructed: false, number }
    }
    pub const fn context(number: u32) -> Self {
        Self { class: 2, constructed: false, number }
    }
    pub const fn context_constructed(number: u32) -> Self {
        Self { class: 2, constructed: true, number }
    }
    pub const fn application_constructed(number: u32) -> Self {
        Self { class: 1, constructed: true, number }
    }

    /// Encode tag to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let class_bits = (self.class & 0x03) << 6;
        let constr_bit = if self.constructed { 0x20 } else { 0x00 };

        if self.number < 31 {
            buf.push(class_bits | constr_bit | (self.number as u8));
        } else {
            buf.push(class_bits | constr_bit | 0x1F);
            // Encode number in base-128, high bit = more follows
            let mut n = self.number;
            let mut bytes = Vec::new();
            bytes.push((n & 0x7F) as u8);
            n >>= 7;
            while n > 0 {
                bytes.push(0x80 | (n & 0x7F) as u8);
                n >>= 7;
            }
            for b in bytes.into_iter().rev() {
                buf.push(b);
            }
        }
        buf
    }

    /// Decode tag from bytes, returns (tag, bytes_consumed)
    pub fn decode(data: &[u8]) -> Result<(Self, usize), BerError> {
        if data.is_empty() { return Err(BerError::UnexpectedEnd); }
        let first = data[0];
        let class = (first >> 6) & 0x03;
        let constructed = (first & 0x20) != 0;
        let number;

        if (first & 0x1F) < 31 {
            number = (first & 0x1F) as u32;
            Ok((Self { class, constructed, number }, 1))
        } else {
            let mut n: u32 = 0;
            let mut i = 1;
            loop {
                if i >= data.len() { return Err(BerError::UnexpectedEnd); }
                let b = data[i];
                n = (n << 7) | (b & 0x7F) as u32;
                i += 1;
                if (b & 0x80) == 0 { break; }
            }
            number = n;
            Ok((Self { class, constructed, number }, i))
        }
    }
}

/// BER encode error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BerError {
    UnexpectedEnd,
    InvalidTag,
    InvalidLength,
    InvalidData,
    BufferOverflow,
}

/// BER TLV encoder
pub struct BerEncoder {
    buf: Vec<u8>,
}

impl BerEncoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self { buf: Vec::with_capacity(cap) }
    }

    /// Write a TLV (tag-length-value)
    pub fn write_tlv(&mut self, tag: BerTag, value: &[u8]) {
        self.buf.extend(tag.encode());
        self.encode_length(value.len());
        self.buf.extend_from_slice(value);
    }

    /// Write a constructed TLV with a closure to fill the content
    pub fn write_constructed<F>(&mut self, tag: BerTag, f: F)
    where F: FnOnce(&mut BerEncoder)
    {
        let mut inner = BerEncoder::new();
        f(&mut inner);
        let content = inner.buf;
        self.buf.extend(tag.encode());
        self.encode_length(content.len());
        self.buf.extend(content);
    }

    /// Write raw bytes
    pub fn write_raw(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// Write a single byte
    pub fn write_byte(&mut self, b: u8) {
        self.buf.push(b);
    }

    /// Write an integer value (universal tag 0x02)
    pub fn write_integer(&mut self, v: i64) {
        let bytes = Self::encode_integer_bytes(v);
        self.write_tlv(BerTag::universal(0x02), &bytes);
    }

    /// Write an octet string (universal tag 0x04)
    pub fn write_octet_string(&mut self, data: &[u8]) {
        self.write_tlv(BerTag::universal(0x04), data);
    }

    /// Write a visible string (universal tag 0x1A = 26)
    pub fn write_visible_string(&mut self, data: &[u8]) {
        self.write_tlv(BerTag::universal(0x1A), data);
    }

    /// Write a boolean (universal tag 0x01)
    pub fn write_boolean(&mut self, v: bool) {
        self.write_tlv(BerTag::universal(0x01), &[if v { 0xFF } else { 0x00 }]);
    }

    /// Write a null (universal tag 0x05)
    pub fn write_null(&mut self) {
        self.buf.extend(BerTag::universal(0x05).encode());
        self.buf.push(0);
    }

    /// Write an object identifier (universal tag 0x06)
    pub fn write_oid(&mut self, components: &[u64]) {
        if components.len() < 2 { return; }
        let mut encoded = Vec::new();
        // First two components encoded as 40*first + second
        encoded.push((40 * components[0] + components[1]) as u8);
        for &c in &components[2..] {
            Self::encode_oid_component(&mut encoded, c);
        }
        self.write_tlv(BerTag::universal(0x06), &encoded);
    }

    fn encode_oid_component(buf: &mut Vec<u8>, c: u64) {
        if c < 128 {
            buf.push(c as u8);
        } else {
            let mut bytes = Vec::new();
            bytes.push((c & 0x7F) as u8);
            let mut n = c >> 7;
            while n > 0 {
                bytes.push(0x80 | (n & 0x7F) as u8);
                n >>= 7;
            }
            for b in bytes.into_iter().rev() {
                buf.push(b);
            }
        }
    }

    fn encode_integer_bytes(v: i64) -> Vec<u8> {
        if (0..=127).contains(&v) || (-128..0).contains(&v) {
            vec![v as u8]
        } else {
            // Find minimum bytes needed
            let mut bytes = Vec::new();
            let mut n = v;
            for _ in 0..8 {
                bytes.push((n & 0xFF) as u8);
                n >>= 8;
            }
            // Remove trailing bytes that are just sign extension
            while bytes.len() > 1 {
                // Safety: bytes.len() > 1 ensures both last() and indexing are valid
                let last = *bytes.last().expect("bytes has at least 2 elements");
                let prev = bytes[bytes.len() - 2];
                if (last == 0xFF && (prev & 0x80) != 0) || (last == 0x00 && (prev & 0x80) == 0) {
                    bytes.pop();
                } else {
                    break;
                }
            }
            bytes.reverse();
            bytes
        }
    }

    fn encode_length(&mut self, len: usize) {
        if len < 128 {
            self.buf.push(len as u8);
        } else if len <= 255 {
            self.buf.push(0x81);
            self.buf.push(len as u8);
        } else {
            self.buf.push(0x82);
            self.buf.push((len >> 8) as u8);
            self.buf.push((len & 0xFF) as u8);
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

impl Default for BerEncoder {
    fn default() -> Self { Self::new() }
}

/// BER TLV decoder
pub struct BerDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> BerDecoder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn position(&self) -> usize { self.pos }
    pub fn remaining(&self) -> usize { self.buf.len().saturating_sub(self.pos) }

    /// Read one TLV, returns (tag, value_slice)
    pub fn read_tlv(&mut self) -> Result<(BerTag, &'a [u8]), BerError> {
        let (tag, tag_len) = BerTag::decode(&self.buf[self.pos..])?;
        self.pos += tag_len;
        let len = self.decode_length()?;
        if self.pos + len > self.buf.len() {
            return Err(BerError::UnexpectedEnd);
        }
        let value = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Ok((tag, value))
    }

    /// Read just the next tag without consuming
    pub fn peek_tag(&mut self) -> Result<BerTag, BerError> {
        let (tag, _) = BerTag::decode(&self.buf[self.pos..])?;
        Ok(tag)
    }

    /// Expect a specific tag and return the value
    pub fn expect_tag(&mut self, expected: BerTag) -> Result<&'a [u8], BerError> {
        let (tag, value) = self.read_tlv()?;
        if tag != expected {
            return Err(BerError::InvalidTag);
        }
        Ok(value)
    }

    /// Read an integer (universal tag 0x02)
    pub fn read_integer(&mut self) -> Result<i64, BerError> {
        let (tag, value) = self.read_tlv()?;
        if tag != BerTag::universal(0x02) {
            return Err(BerError::InvalidTag);
        }
        if value.is_empty() { return Ok(0); }
        // Sign-extend from the first byte
        let mut result: i64 = 0;
        let negative = value[0] & 0x80 != 0;
        if negative {
            result = -1i64;
        }
        for &b in value {
            result = (result << 8) | (b as i64);
        }
        Ok(result)
    }

    /// Read an octet string (universal tag 0x04)
    pub fn read_octet_string(&mut self) -> Result<&'a [u8], BerError> {
        self.expect_tag(BerTag::universal(0x04))
    }

    /// Read a boolean
    pub fn read_boolean(&mut self) -> Result<bool, BerError> {
        let (tag, value) = self.read_tlv()?;
        if tag != BerTag::universal(0x01) || value.len() != 1 {
            return Err(BerError::InvalidTag);
        }
        Ok(value[0] != 0)
    }

    /// Read a visible string
    pub fn read_visible_string(&mut self) -> Result<&'a [u8], BerError> {
        self.expect_tag(BerTag::universal(0x1A))
    }

    fn decode_length(&mut self) -> Result<usize, BerError> {
        if self.pos >= self.buf.len() { return Err(BerError::UnexpectedEnd); }
        let first = self.buf[self.pos];
        self.pos += 1;
        if first < 128 {
            Ok(first as usize)
        } else if first == 0x81 {
            if self.pos >= self.buf.len() { return Err(BerError::UnexpectedEnd); }
            let len = self.buf[self.pos] as usize;
            self.pos += 1;
            Ok(len)
        } else if first == 0x82 {
            if self.pos + 1 >= self.buf.len() { return Err(BerError::UnexpectedEnd); }
            let len = ((self.buf[self.pos] as usize) << 8) | (self.buf[self.pos + 1] as usize);
            self.pos += 2;
            Ok(len)
        } else {
            Err(BerError::InvalidLength)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_encode_simple() {
        let tag = BerTag::universal(0x02); // INTEGER
        assert_eq!(tag.encode(), vec![0x02]);
    }

    #[test]
    fn test_tag_encode_context() {
        let tag = BerTag::context_constructed(0x01);
        assert_eq!(tag.encode(), vec![0xA1]);
    }

    #[test]
    fn test_tag_roundtrip() {
        let tags = [
            BerTag::universal(0x01),
            BerTag::application(0x05),
            BerTag::context_constructed(0x0A),
            BerTag::application_constructed(0x3C),
        ];
        for tag in &tags {
            let encoded = tag.encode();
            let (decoded, consumed) = BerTag::decode(&encoded).unwrap();
            assert_eq!(consumed, encoded.len());
            assert_eq!(decoded, *tag);
        }
    }

    #[test]
    fn test_integer_encoding() {
        let mut enc = BerEncoder::new();
        enc.write_integer(0);
        assert_eq!(enc.to_bytes(), &[0x02, 0x01, 0x00]);

        let mut enc = BerEncoder::new();
        enc.write_integer(127);
        assert_eq!(enc.to_bytes(), &[0x02, 0x01, 0x7F]);

        let mut enc = BerEncoder::new();
        enc.write_integer(128);
        assert_eq!(enc.to_bytes(), &[0x02, 0x02, 0x00, 0x80]);

        let mut enc = BerEncoder::new();
        enc.write_integer(-1);
        assert_eq!(enc.to_bytes(), &[0x02, 0x01, 0xFF]);
    }

    #[test]
    fn test_integer_roundtrip() {
        let values = [0i64, 1, 127, 128, 255, 256, -1, -128, -129, 32767, -32768];
        for v in &values {
            let mut enc = BerEncoder::new();
            enc.write_integer(*v);
            let mut dec = BerDecoder::new(enc.to_bytes());
            let decoded = dec.read_integer().unwrap();
            assert_eq!(decoded, *v, "Roundtrip failed for {}", v);
        }
    }

    #[test]
    fn test_octet_string() {
        let mut enc = BerEncoder::new();
        enc.write_octet_string(b"hello");
        let mut dec = BerDecoder::new(enc.to_bytes());
        let result = dec.read_octet_string().unwrap();
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_constructed() {
        let mut enc = BerEncoder::new();
        enc.write_constructed(BerTag::context_constructed(0x01), |inner| {
            inner.write_integer(42);
            inner.write_octet_string(b"test");
        });
        let bytes = enc.into_bytes();
        assert_eq!(bytes[0], 0xA1); // context, constructed, tag 1
        let mut dec = BerDecoder::new(&bytes);
        let (tag, content) = dec.read_tlv().unwrap();
        assert_eq!(tag, BerTag::context_constructed(0x01));
        // Decode content
        let mut inner = BerDecoder::new(content);
        let v = inner.read_integer().unwrap();
        assert_eq!(v, 42);
    }

    #[test]
    fn test_oid() {
        let mut enc = BerEncoder::new();
        enc.write_oid(&[2, 16, 776, 1, 1]);
        let bytes = enc.into_bytes();
        assert_eq!(bytes[0], 0x06); // OID tag
        // Decode it back
        let mut dec = BerDecoder::new(&bytes);
        let (tag, value) = dec.read_tlv().unwrap();
        assert_eq!(tag.number, 0x06);
        assert_eq!(value[0], 2 * 40 + 16); // 96
    }
}
