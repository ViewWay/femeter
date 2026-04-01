//! AARQ (Association Request) encode/decode

use crate::ber::{BerDecoder, BerEncoder, BerError, BerTag};
use crate::context::ApplicationContextName;
use crate::initiate::{decode_initiate_request, encode_initiate_request, InitiateRequest};
use alloc::vec::Vec;

/// AARQ (Application Association Request)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aarq {
    pub application_context_name: ApplicationContextName,
    pub user_information: InitiateRequest,
    pub authentication_mechanism: Option<u8>,
}

impl Aarq {
    pub fn new_ln_no_cipher(initiate: InitiateRequest) -> Self {
        Self {
            application_context_name: ApplicationContextName::LogicalNameNoCiphering,
            user_information: initiate,
            authentication_mechanism: None,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = BerEncoder::new();
        // AARQ = APPLICATION 0 CONSTRUCTED
        enc.write_constructed(BerTag::application_constructed(0x00), |inner| {
            // [0] application-context-name (directly inline the OID)
            inner.write_constructed(BerTag::context_constructed(0x00), |ctx| {
                ctx.write_oid(self.application_context_name.oid());
            });
            // [30] user-information
            inner.write_constructed(BerTag::context_constructed(0x1E), |ctx| {
                let initiate_bytes = encode_initiate_request(&self.user_information);
                ctx.write_octet_string(&initiate_bytes);
            });
        });
        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, BerError> {
        let mut dec = BerDecoder::new(data);
        let (tag, content) = dec.read_tlv()?;
        if tag != BerTag::application_constructed(0x00) {
            return Err(BerError::InvalidTag);
        }
        let mut inner = BerDecoder::new(content);
        let mut app_ctx = ApplicationContextName::LogicalNameNoCiphering;
        let mut user_info: Option<InitiateRequest> = None;

        while inner.remaining() > 0 {
            let (field_tag, field_content) = inner.read_tlv()?;
            match field_tag.number {
                0x00 => {
                    // application-context-name: contains OID TLV
                    let mut f = BerDecoder::new(field_content);
                    let (oid_tag, oid_data) = f.read_tlv()?;
                    if oid_tag != BerTag::universal(0x06) {
                        return Err(BerError::InvalidTag);
                    }
                    // Decode OID
                    if oid_data.len() >= 2 {
                        let mut components = Vec::new();
                        components.push((oid_data[0] / 40) as u64);
                        components.push((oid_data[0] % 40) as u64);
                        let mut i = 1;
                        while i < oid_data.len() {
                            let mut n: u64 = 0;
                            while i < oid_data.len() {
                                let b = oid_data[i];
                                n = (n << 7) | ((b & 0x7F) as u64);
                                i += 1;
                                if (b & 0x80) == 0 {
                                    break;
                                }
                            }
                            components.push(n);
                        }
                        app_ctx = match components[..] {
                            [2, 16, 776, 1, 1] => ApplicationContextName::LogicalNameNoCiphering,
                            [2, 16, 776, 1, 2] => ApplicationContextName::LogicalNameWithCiphering,
                            [2, 16, 776, 2, 1] => ApplicationContextName::ShortNameNoCiphering,
                            [2, 16, 776, 2, 2] => ApplicationContextName::ShortNameWithCiphering,
                            _ => ApplicationContextName::Custom(components),
                        }
                    }
                }
                0x1E => {
                    // user-information: contains OCTET STRING
                    let mut f = BerDecoder::new(field_content);
                    let inner_data = f.read_octet_string()?;
                    user_info = Some(decode_initiate_request(inner_data)?);
                }
                _ => {} // skip unknown
            }
        }

        Ok(Self {
            application_context_name: app_ctx,
            user_information: user_info.ok_or(BerError::InvalidData)?,
            authentication_mechanism: None,
        })
    }
}

pub fn encode_aarq(aarq: &Aarq) -> Vec<u8> {
    aarq.encode()
}

pub fn decode_aarq(data: &[u8]) -> Result<Aarq, BerError> {
    Aarq::decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conformance::ConformanceBlock;

    #[test]
    fn test_aarq_roundtrip() {
        let req = InitiateRequest::new(ConformanceBlock::standard_meter(), 1024);
        let aarq = Aarq::new_ln_no_cipher(req);
        let bytes = encode_aarq(&aarq);
        let decoded = decode_aarq(&bytes).unwrap();
        assert_eq!(
            decoded.application_context_name.oid(),
            ApplicationContextName::LogicalNameNoCiphering.oid()
        );
        assert_eq!(decoded.user_information.proposed_max_pdu_size, 1024);
    }
}
