//! RLRQ (Release Request) / RLRE (Release Response) encode/decode

use alloc::vec::Vec;
use crate::ber::{BerEncoder, BerDecoder, BerTag, BerError};

/// Release request reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseReason {
    Normal = 0,
    Urgent = 1,
    UserDefined = 2,
    Unknown = 255,
}

/// RLRQ (Release Request)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rlrq {
    pub reason: ReleaseReason,
}

impl Rlrq {
    pub fn normal() -> Self {
        Self { reason: ReleaseReason::Normal }
    }

    pub fn encode(&self, enc: &mut BerEncoder) {
        enc.write_constructed(BerTag::application_constructed(0x02), |inner| {
            inner.write_constructed(BerTag::context_constructed(0x00), |ctx| {
                ctx.write_integer(self.reason as i64);
            });
        });
    }

    pub fn decode(dec: &mut BerDecoder) -> Result<Self, BerError> {
        let (tag, content) = dec.read_tlv()?;
        if tag != BerTag::application_constructed(0x02) {
            return Err(BerError::InvalidTag);
        }
        let mut inner = BerDecoder::new(content);
        let mut reason = ReleaseReason::Normal;

        if inner.remaining() > 0 {
            let (_, field_content) = inner.read_tlv()?;
            let mut f = BerDecoder::new(field_content);
            let v = f.read_integer()?;
            reason = match v {
                0 => ReleaseReason::Normal,
                1 => ReleaseReason::Urgent,
                2 => ReleaseReason::UserDefined,
                _ => ReleaseReason::Unknown,
            };
        }

        Ok(Self { reason })
    }
}

/// RLRE (Release Response)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rlre {
    pub reason: ReleaseReason,
}

impl Rlre {
    pub fn normal() -> Self {
        Self { reason: ReleaseReason::Normal }
    }

    pub fn encode(&self, enc: &mut BerEncoder) {
        enc.write_constructed(BerTag::application_constructed(0x03), |inner| {
            inner.write_constructed(BerTag::context_constructed(0x00), |ctx| {
                ctx.write_integer(self.reason as i64);
            });
        });
    }

    pub fn decode(dec: &mut BerDecoder) -> Result<Self, BerError> {
        let (tag, content) = dec.read_tlv()?;
        if tag != BerTag::application_constructed(0x03) {
            return Err(BerError::InvalidTag);
        }
        let mut inner = BerDecoder::new(content);
        let mut reason = ReleaseReason::Normal;

        if inner.remaining() > 0 {
            let (_, field_content) = inner.read_tlv()?;
            let mut f = BerDecoder::new(field_content);
            let v = f.read_integer()?;
            reason = match v {
                0 => ReleaseReason::Normal,
                1 => ReleaseReason::Urgent,
                2 => ReleaseReason::UserDefined,
                _ => ReleaseReason::Unknown,
            };
        }

        Ok(Self { reason })
    }
}

pub fn encode_rlrq(rlrq: &Rlrq) -> Vec<u8> {
    let mut enc = BerEncoder::new();
    rlrq.encode(&mut enc);
    enc.into_bytes()
}

pub fn decode_rlrq(data: &[u8]) -> Result<Rlrq, BerError> {
    let mut dec = BerDecoder::new(data);
    Rlrq::decode(&mut dec)
}

pub fn encode_rlre(rlre: &Rlre) -> Vec<u8> {
    let mut enc = BerEncoder::new();
    rlre.encode(&mut enc);
    enc.into_bytes()
}

pub fn decode_rlre(data: &[u8]) -> Result<Rlre, BerError> {
    let mut dec = BerDecoder::new(data);
    Rlre::decode(&mut dec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rlrq_roundtrip() {
        let rlrq = Rlrq::normal();
        let bytes = encode_rlrq(&rlrq);
        let decoded = decode_rlrq(&bytes).unwrap();
        assert_eq!(decoded.reason, ReleaseReason::Normal);
    }

    #[test]
    fn test_rlre_roundtrip() {
        let rlre = Rlre::normal();
        let bytes = encode_rlre(&rlre);
        let decoded = decode_rlre(&bytes).unwrap();
        assert_eq!(decoded.reason, ReleaseReason::Normal);
    }

    #[test]
    fn test_rlrq_urgent() {
        let rlrq = Rlrq { reason: ReleaseReason::Urgent };
        let bytes = encode_rlrq(&rlrq);
        let decoded = decode_rlrq(&bytes).unwrap();
        assert_eq!(decoded.reason, ReleaseReason::Urgent);
    }
}
