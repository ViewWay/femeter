//! AARE (Association Response) encode/decode

use alloc::vec::Vec;
use crate::ber::{BerEncoder, BerDecoder, BerTag, BerError};
use crate::context::ApplicationContextName;
use crate::initiate::{InitiateResponse, encode_initiate_response, decode_initiate_response};

/// Association result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationResult {
    Accepted = 0,
    RejectedPermanent = 1,
    RejectedTransient = 2,
}

/// Result source diagnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Diagnostic {
    Null = 0,
    NoReason = 1,
    ApplicationContextNameNotSupported = 2,
    AuthenticationFailed = 5,
    AuthenticationRequired = 6,
}

/// AARE (Application Association Response)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aare {
    pub application_context_name: ApplicationContextName,
    pub result: AssociationResult,
    pub result_source_diagnostic: Diagnostic,
    pub user_information: Option<InitiateResponse>,
}

impl Aare {
    pub fn accepted_ln_no_cipher(response: InitiateResponse) -> Self {
        Self {
            application_context_name: ApplicationContextName::LogicalNameNoCiphering,
            result: AssociationResult::Accepted,
            result_source_diagnostic: Diagnostic::Null,
            user_information: Some(response),
        }
    }

    pub fn rejected(reason: Diagnostic) -> Self {
        Self {
            application_context_name: ApplicationContextName::LogicalNameNoCiphering,
            result: AssociationResult::RejectedPermanent,
            result_source_diagnostic: reason,
            user_information: None,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = BerEncoder::new();
        // AARE = APPLICATION 1 CONSTRUCTED
        enc.write_constructed(BerTag::application_constructed(0x01), |inner| {
            // [0] application-context-name
            inner.write_constructed(BerTag::context_constructed(0x00), |ctx| {
                ctx.write_oid(self.application_context_name.oid());
            });

            // [1] result - ASN.1 ENUMERATED
            inner.write_constructed(BerTag::context_constructed(0x01), |ctx| {
                // ENUMERATED = universal tag 10
                ctx.write_tlv(BerTag::universal(0x0A), &[self.result as u8]);
            });

            // [2] result-source-diagnostic
            inner.write_constructed(BerTag::context_constructed(0x02), |ctx| {
                ctx.write_tlv(BerTag::universal(0x0A), &[self.result_source_diagnostic as u8]);
            });

            // [30] user-information (optional)
            if let Some(ref resp) = self.user_information {
                inner.write_constructed(BerTag::context_constructed(0x1E), |ctx| {
                    let bytes = encode_initiate_response(resp);
                    ctx.write_octet_string(&bytes);
                });
            }
        });
        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, BerError> {
        let mut dec = BerDecoder::new(data);
        let (tag, content) = dec.read_tlv()?;
        if tag != BerTag::application_constructed(0x01) {
            return Err(BerError::InvalidTag);
        }
        let mut inner = BerDecoder::new(content);
        let mut app_ctx = ApplicationContextName::LogicalNameNoCiphering;
        let mut result = AssociationResult::Accepted;
        let mut diagnostic = Diagnostic::Null;
        let mut user_info: Option<InitiateResponse> = None;

        while inner.remaining() > 0 {
            let (field_tag, field_content) = inner.read_tlv()?;
            match field_tag.number {
                0x00 => {
                    let mut f = BerDecoder::new(field_content);
                    let (oid_tag, oid_data) = f.read_tlv()?;
                    if oid_tag == BerTag::universal(0x06) && oid_data.len() >= 2 {
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
                                if (b & 0x80) == 0 { break; }
                            }
                            components.push(n);
                        }
                        app_ctx = match components[..] {
                            [2, 16, 776, 1, 1] => ApplicationContextName::LogicalNameNoCiphering,
                            [2, 16, 776, 1, 2] => ApplicationContextName::LogicalNameWithCiphering,
                            [2, 16, 776, 2, 1] => ApplicationContextName::ShortNameNoCiphering,
                            [2, 16, 776, 2, 2] => ApplicationContextName::ShortNameWithCiphering,
                            _ => ApplicationContextName::Custom(components),
                        };
                    }
                }
                0x01 => {
                    let mut f = BerDecoder::new(field_content);
                    let (_, enum_content) = f.read_tlv()?;
                    if !enum_content.is_empty() {
                        result = match enum_content[0] {
                            0 => AssociationResult::Accepted,
                            1 => AssociationResult::RejectedPermanent,
                            2 => AssociationResult::RejectedTransient,
                            _ => AssociationResult::RejectedPermanent,
                        };
                    }
                }
                0x02 => {
                    let mut f = BerDecoder::new(field_content);
                    let (_, enum_content) = f.read_tlv()?;
                    if !enum_content.is_empty() {
                        diagnostic = match enum_content[0] {
                            0 => Diagnostic::Null,
                            1 => Diagnostic::NoReason,
                            2 => Diagnostic::ApplicationContextNameNotSupported,
                            5 => Diagnostic::AuthenticationFailed,
                            6 => Diagnostic::AuthenticationRequired,
                            _ => Diagnostic::NoReason,
                        };
                    }
                }
                0x1E => {
                    let mut f = BerDecoder::new(field_content);
                    let inner_data = f.read_octet_string()?;
                    user_info = Some(decode_initiate_response(inner_data)?);
                }
                _ => {}
            }
        }

        Ok(Self {
            application_context_name: app_ctx,
            result,
            result_source_diagnostic: diagnostic,
            user_information: user_info,
        })
    }
}

pub fn encode_aare(aare: &Aare) -> Vec<u8> {
    aare.encode()
}

pub fn decode_aare(data: &[u8]) -> Result<Aare, BerError> {
    Aare::decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conformance::ConformanceBlock;

    #[test]
    fn test_aare_accepted_roundtrip() {
        let resp = InitiateResponse::new(ConformanceBlock::standard_meter(), 2048);
        let aare = Aare::accepted_ln_no_cipher(resp);
        let bytes = encode_aare(&aare);
        let decoded = decode_aare(&bytes).unwrap();
        assert_eq!(decoded.result, AssociationResult::Accepted);
        assert!(decoded.user_information.is_some());
        assert_eq!(decoded.user_information.unwrap().negotiated_max_pdu_size, 2048);
    }

    #[test]
    fn test_aare_rejected() {
        let aare = Aare::rejected(Diagnostic::AuthenticationFailed);
        let bytes = encode_aare(&aare);
        let decoded = decode_aare(&bytes).unwrap();
        assert_eq!(decoded.result, AssociationResult::RejectedPermanent);
        assert!(decoded.user_information.is_none());
    }
}
