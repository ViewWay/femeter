//! APDU codec base (encoder/decoder)
//!
//! Provides encoding and decoding for APDU messages using the DLMS/COSEM
//! Application Layer protocol.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use crate::types::{ApduError, AttributeDescriptor, InvokeId, MethodDescriptor};
use alloc::vec::Vec;
use dlms_axdr::{AxdrDecoder, AxdrEncoder};
use dlms_core::{DlmsType, ObisCode};

/// APDU encoder
pub struct ApduEncoder {
    buf: Vec<u8>,
}

impl ApduEncoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
        }
    }

    /// Write APDU tag (2 bytes: type + subtype)
    pub fn write_tag(&mut self, tag_type: u8, subtype: u8) {
        self.buf.push(tag_type);
        self.buf.push(subtype);
    }

    /// Write invoke ID
    pub fn write_invoke_id(&mut self, id: InvokeId) {
        self.buf.push(id.get());
    }

    /// Write attribute descriptor (class + instance + attr_id)
    pub fn write_attribute_descriptor(&mut self, desc: &AttributeDescriptor) {
        // Class ID (2 bytes, big-endian)
        self.buf.extend_from_slice(&desc.class_id.to_be_bytes());
        // OBIS code (6 bytes)
        self.buf.extend_from_slice(&desc.instance.to_bytes());
        // Attribute ID (1 byte)
        self.buf.push(desc.attribute_id);
    }

    /// Write method descriptor (class + instance + method_id)
    pub fn write_method_descriptor(&mut self, desc: &MethodDescriptor) {
        // Class ID (2 bytes, big-endian)
        self.buf.extend_from_slice(&desc.class_id.to_be_bytes());
        // OBIS code (6 bytes)
        self.buf.extend_from_slice(&desc.instance.to_bytes());
        // Method ID (1 byte)
        self.buf.push(desc.method_id);
    }

    /// Write a DlmsType value using A-XDR encoding
    pub fn write_dlms_value(&mut self, value: &DlmsType) -> Result<(), ApduError> {
        let mut axdr = AxdrEncoder::new();
        axdr.encode(value).map_err(|_| ApduError::EncodeError)?;
        self.buf.extend_from_slice(axdr.to_bytes());
        Ok(())
    }

    /// Write raw bytes
    pub fn write_bytes(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// Write a single byte
    pub fn write_byte(&mut self, b: u8) {
        self.buf.push(b);
    }

    /// Write u16 big-endian
    pub fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Write u32 big-endian
    pub fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Get the encoded bytes
    pub fn to_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Consume and return the encoded bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// Reset the buffer
    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

impl Default for ApduEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// APDU decoder
pub struct ApduDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ApduDecoder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Current position
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Remaining bytes
    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    /// Check if at end
    pub fn is_empty(&self) -> bool {
        self.pos >= self.buf.len()
    }

    /// Read APDU tag (2 bytes: type + subtype)
    pub fn read_tag(&mut self) -> Result<(u8, u8), ApduError> {
        if self.pos + 2 > self.buf.len() {
            return Err(ApduError::TooShort);
        }
        let tag_type = self.buf[self.pos];
        let subtype = self.buf[self.pos + 1];
        self.pos += 2;
        Ok((tag_type, subtype))
    }

    /// Read invoke ID
    pub fn read_invoke_id(&mut self) -> Result<InvokeId, ApduError> {
        let id = self.read_byte()?;
        Ok(InvokeId::new(id))
    }

    /// Read attribute descriptor
    pub fn read_attribute_descriptor(&mut self) -> Result<AttributeDescriptor, ApduError> {
        if self.pos + 9 > self.buf.len() {
            return Err(ApduError::TooShort);
        }

        let class_id = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;

        let obis_bytes = [
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
            self.buf[self.pos + 4],
            self.buf[self.pos + 5],
        ];
        let instance = ObisCode::from_bytes(&obis_bytes);
        self.pos += 6;

        let attribute_id = self.buf[self.pos];
        self.pos += 1;

        Ok(AttributeDescriptor::new(class_id, instance, attribute_id))
    }

    /// Read method descriptor
    pub fn read_method_descriptor(&mut self) -> Result<MethodDescriptor, ApduError> {
        if self.pos + 9 > self.buf.len() {
            return Err(ApduError::TooShort);
        }

        let class_id = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;

        let obis_bytes = [
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
            self.buf[self.pos + 4],
            self.buf[self.pos + 5],
        ];
        let instance = ObisCode::from_bytes(&obis_bytes);
        self.pos += 6;

        let method_id = self.buf[self.pos];
        self.pos += 1;

        Ok(MethodDescriptor::new(class_id, instance, method_id))
    }

    /// Read a DlmsType value using A-XDR decoding
    pub fn read_dlms_value(&mut self) -> Result<DlmsType, ApduError> {
        let mut axdr = AxdrDecoder::new(&self.buf[self.pos..]);
        let value = axdr.decode().map_err(|_| ApduError::DecodeError)?;
        self.pos += axdr.position();
        Ok(value)
    }

    /// Read raw bytes
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], ApduError> {
        if self.pos + n > self.buf.len() {
            return Err(ApduError::UnexpectedEnd);
        }
        let slice = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    /// Read a single byte
    pub fn read_byte(&mut self) -> Result<u8, ApduError> {
        if self.pos >= self.buf.len() {
            return Err(ApduError::UnexpectedEnd);
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(b)
    }

    /// Read u16 big-endian
    pub fn read_u16(&mut self) -> Result<u16, ApduError> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    /// Read u32 big-endian
    pub fn read_u32(&mut self) -> Result<u32, ApduError> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Peek at next tag without consuming
    pub fn peek_tag(&self) -> Option<(u8, u8)> {
        if self.pos + 2 > self.buf.len() {
            return None;
        }
        Some((self.buf[self.pos], self.buf[self.pos + 1]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_write_tag() {
        let mut enc = ApduEncoder::new();
        enc.write_tag(0xC0, 0x01);
        assert_eq!(enc.to_bytes(), &[0xC0, 0x01]);
    }

    #[test]
    fn test_encoder_invoke_id() {
        let mut enc = ApduEncoder::new();
        enc.write_invoke_id(InvokeId::new(42));
        assert_eq!(enc.to_bytes(), &[42]);
    }

    #[test]
    fn test_encoder_attribute_descriptor() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let mut enc = ApduEncoder::new();
        enc.write_attribute_descriptor(&desc);

        let bytes = enc.to_bytes();
        assert_eq!(bytes.len(), 9);
        assert_eq!(bytes[0], 0); // class high byte
        assert_eq!(bytes[1], 3); // class low byte
        assert_eq!(&bytes[2..8], &[1, 0, 1, 8, 0, 255]); // OBIS
        assert_eq!(bytes[8], 2); // attribute id
    }

    #[test]
    fn test_decoder_read_tag() {
        let data = [0xC0, 0x01, 0xC4, 0x02];
        let mut dec = ApduDecoder::new(&data);

        let (tag_type, subtype) = dec.read_tag().unwrap();
        assert_eq!(tag_type, 0xC0);
        assert_eq!(subtype, 0x01);

        let (tag_type, subtype) = dec.read_tag().unwrap();
        assert_eq!(tag_type, 0xC4);
        assert_eq!(subtype, 0x02);
    }

    #[test]
    fn test_decoder_attribute_descriptor() {
        let data = [
            0x00, 0x03, // class = 3
            0x01, 0x00, 0x01, 0x08, 0x00, 0xFF, // OBIS
            0x02, // attribute_id = 2
        ];

        let mut dec = ApduDecoder::new(&data);
        let desc = dec.read_attribute_descriptor().unwrap();

        assert_eq!(desc.class_id, 3);
        assert_eq!(desc.instance, ObisCode::new(1, 0, 1, 8, 0, 255));
        assert_eq!(desc.attribute_id, 2);
    }

    #[test]
    fn test_decoder_too_short() {
        let mut dec = ApduDecoder::new(&[0xC0]); // Only 1 byte
        assert!(dec.read_tag().is_err());
    }
}
