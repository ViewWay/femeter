//! InitiateRequest / InitiateResponse

use crate::ber::{BerDecoder, BerEncoder, BerError, BerTag};
use crate::conformance::ConformanceBlock;
use alloc::vec::Vec;

/// InitiateRequest PDU (client → server)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitiateRequest {
    pub proposed_conformance: ConformanceBlock,
    pub proposed_max_pdu_size: u16,
    pub proposed_dlms_version: u8,
    pub client_max_receive_pdu_size: Option<u16>,
}

impl InitiateRequest {
    pub fn new(conformance: ConformanceBlock, max_pdu: u16) -> Self {
        Self {
            proposed_conformance: conformance,
            proposed_max_pdu_size: max_pdu,
            proposed_dlms_version: 6, // DLMS version 6
            client_max_receive_pdu_size: None,
        }
    }

    pub fn encode(&self, enc: &mut BerEncoder) {
        enc.write_constructed(BerTag::context_constructed(0x01), |inner| {
            // Use BER encoding but COSEM uses a simplified format:
            // Octet string containing: DLMS version, conformance, max_pdu
            let mut content = Vec::new();
            content.push(self.proposed_dlms_version);
            content.extend_from_slice(&self.proposed_conformance.to_bytes());
            content.push((self.proposed_max_pdu_size >> 8) as u8);
            content.push((self.proposed_max_pdu_size & 0xFF) as u8);
            inner.write_octet_string(&content);
        });
    }

    pub fn decode(dec: &mut BerDecoder) -> Result<Self, BerError> {
        let (tag, content) = dec.read_tlv()?;
        if tag != BerTag::context_constructed(0x01) {
            return Err(BerError::InvalidTag);
        }
        let mut inner = BerDecoder::new(content);
        let data = inner.read_octet_string()?;
        if data.len() < 6 {
            return Err(BerError::InvalidData);
        }
        let dlms_version = data[0];
        let conformance = ConformanceBlock::from_bytes(&data[1..4]).ok_or(BerError::InvalidData)?;
        let max_pdu = u16::from_be_bytes([data[4], data[5]]);
        Ok(Self {
            proposed_conformance: conformance,
            proposed_max_pdu_size: max_pdu,
            proposed_dlms_version: dlms_version,
            client_max_receive_pdu_size: None,
        })
    }
}

/// InitiateResponse PDU (server → client)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitiateResponse {
    pub negotiated_conformance: ConformanceBlock,
    pub negotiated_max_pdu_size: u16,
    pub negotiated_dlms_version: u8,
    pub server_max_receive_pdu_size: Option<u16>,
    pub vaa_name: u16,
}

impl InitiateResponse {
    pub fn new(conformance: ConformanceBlock, max_pdu: u16) -> Self {
        Self {
            negotiated_conformance: conformance,
            negotiated_max_pdu_size: max_pdu,
            negotiated_dlms_version: 6,
            server_max_receive_pdu_size: None,
            vaa_name: 0x0007,
        }
    }

    pub fn encode(&self, enc: &mut BerEncoder) {
        enc.write_constructed(BerTag::context_constructed(0x01), |inner| {
            let mut content = Vec::new();
            content.push(self.negotiated_dlms_version);
            content.extend_from_slice(&self.negotiated_conformance.to_bytes());
            content.push((self.negotiated_max_pdu_size >> 8) as u8);
            content.push((self.negotiated_max_pdu_size & 0xFF) as u8);
            content.push((self.vaa_name >> 8) as u8);
            content.push((self.vaa_name & 0xFF) as u8);
            inner.write_octet_string(&content);
        });
    }

    pub fn decode(dec: &mut BerDecoder) -> Result<Self, BerError> {
        let (tag, content) = dec.read_tlv()?;
        if tag != BerTag::context_constructed(0x01) {
            return Err(BerError::InvalidTag);
        }
        let mut inner = BerDecoder::new(content);
        let data = inner.read_octet_string()?;
        if data.len() < 8 {
            return Err(BerError::InvalidData);
        }
        let dlms_version = data[0];
        let conformance = ConformanceBlock::from_bytes(&data[1..4]).ok_or(BerError::InvalidData)?;
        let max_pdu = u16::from_be_bytes([data[4], data[5]]);
        let vaa_name = u16::from_be_bytes([data[6], data[7]]);
        Ok(Self {
            negotiated_conformance: conformance,
            negotiated_max_pdu_size: max_pdu,
            negotiated_dlms_version: dlms_version,
            server_max_receive_pdu_size: None,
            vaa_name,
        })
    }
}

/// Convenience functions
pub fn encode_initiate_request(req: &InitiateRequest) -> Vec<u8> {
    let mut enc = BerEncoder::new();
    req.encode(&mut enc);
    enc.into_bytes()
}

pub fn decode_initiate_request(data: &[u8]) -> Result<InitiateRequest, BerError> {
    let mut dec = BerDecoder::new(data);
    InitiateRequest::decode(&mut dec)
}

pub fn encode_initiate_response(resp: &InitiateResponse) -> Vec<u8> {
    let mut enc = BerEncoder::new();
    resp.encode(&mut enc);
    enc.into_bytes()
}

pub fn decode_initiate_response(data: &[u8]) -> Result<InitiateResponse, BerError> {
    let mut dec = BerDecoder::new(data);
    InitiateResponse::decode(&mut dec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initiate_request_roundtrip() {
        let req = InitiateRequest::new(ConformanceBlock::standard_meter(), 1024);
        let bytes = encode_initiate_request(&req);
        let decoded = decode_initiate_request(&bytes).unwrap();
        assert_eq!(decoded.proposed_dlms_version, 6);
        assert_eq!(decoded.proposed_max_pdu_size, 1024);
        assert_eq!(
            decoded.proposed_conformance,
            ConformanceBlock::standard_meter()
        );
    }

    #[test]
    fn test_initiate_response_roundtrip() {
        let resp = InitiateResponse::new(ConformanceBlock::standard_meter(), 2048);
        let bytes = encode_initiate_response(&resp);
        let decoded = decode_initiate_response(&bytes).unwrap();
        assert_eq!(decoded.negotiated_max_pdu_size, 2048);
        assert_eq!(decoded.vaa_name, 0x0007);
    }
}
