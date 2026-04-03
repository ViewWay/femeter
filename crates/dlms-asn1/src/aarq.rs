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
                0x00 | 0x01 => {
                    // Accept both tag 0 (standard) and tag 1 (some implementations)
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
                0x06 | 0x1E => {
                    // Accept context-6 (some impls) and context-30 (standard)
                    // user-information: may be wrapped in OCTET STRING or direct BER
                    let raw_data = if field_content.len() >= 2 && field_content[0] == 0x04 {
                        // OCTET STRING wrapper (standard per DLMS)
                        let mut f = BerDecoder::new(field_content);
                        f.read_octet_string()?
                    } else if field_content.len() >= 2 && field_content[0] == 0xA1 {
                        // Direct BER context-1 constructed (non-standard)
                        field_content
                    } else {
                        field_content
                    };
                    // Try BER decode first, then raw decode
                    user_info = Some(if raw_data.len() >= 2 && raw_data[0] == 0xA1 {
                        decode_initiate_request(raw_data)?
                    } else {
                        // Raw initiate request: version(1) + conformance(4) + max_pdu(2) + ...
                        if raw_data.len() < 6 {
                            return Err(BerError::InvalidData);
                        }
                        crate::InitiateRequest {
                            proposed_dlms_version: raw_data[0],
                            proposed_conformance: crate::ConformanceBlock::from_bytes(
                                &raw_data[1..4],
                            )
                            .ok_or(BerError::InvalidData)?,
                            proposed_max_pdu_size: u16::from_be_bytes([raw_data[4], raw_data[5]]),
                            client_max_receive_pdu_size: None,
                        }
                    });
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

#[cfg(test)]
mod python_compat_tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn test_decode_python_dlms_cosem_aarq() {
        // AARQ generated by Python dlms-cosem library
        let aarq_bytes: Vec<u8> = vec![
            0x60, 0x29, 0xA1, 0x09, 0x06, 0x07, 0x60, 0x85, 0x74, 0x05, 0x08, 0x01, 0x01, 0xA6,
            0x0A, 0x04, 0x08, 0x75, 0x74, 0x69, 0x0B, 0x1A, 0x70, 0x5C, 0xA8, 0xBE, 0x10, 0x04,
            0x0E, 0x01, 0x00, 0x00, 0x00, 0x06, 0x5F, 0x1F, 0x04, 0x00, 0x20, 0x52, 0x5F, 0xFF,
            0xFF,
        ];
        let result = decode_aarq(&aarq_bytes);
        match &result {
            Ok(_aarq) => (),
            Err(e) => panic!("Decode failed: {:?}", e),
        }
        assert!(result.is_ok(), "Failed to decode Python AARQ");
    }
}
