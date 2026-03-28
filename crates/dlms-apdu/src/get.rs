//! Get-Request and Get-Response APDUs
//!
//! Reference: IEC 62056-53 §8.4.1

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::vec::Vec;
use dlms_core::{DlmsType, DataAccessError};
use crate::types::{
    ApduError, InvokeId, AttributeDescriptor, AccessRequest, AccessResult,
    TAG_GET_REQUEST, TAG_GET_RESPONSE, TAG_SUBTYPE_NORMAL, TAG_SUBTYPE_NEXT,
    TAG_SUBTYPE_WITH_LIST, TAG_SUBTYPE_DATA, TAG_SUBTYPE_BLOCK, TAG_SUBTYPE_DATA_ACCESS_ERROR,
};
use crate::codec::{ApduEncoder, ApduDecoder};

// ============================================================
// Get-Request PDUs
// ============================================================

/// Get-Request normal (single attribute read)
#[derive(Debug, Clone, PartialEq)]
pub struct GetRequestNormal {
    pub invoke_id: InvokeId,
    pub request: AccessRequest,
}

impl GetRequestNormal {
    pub fn new(invoke_id: InvokeId, descriptor: AttributeDescriptor) -> Self {
        Self {
            invoke_id,
            request: AccessRequest::new(descriptor),
        }
    }

    pub fn with_selective_raw(
        invoke_id: InvokeId,
        descriptor: AttributeDescriptor,
        selective_data: Vec<u8>,
    ) -> Self {
        Self {
            invoke_id,
            request: AccessRequest::with_selective_raw(descriptor, selective_data),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_GET_REQUEST, TAG_SUBTYPE_NORMAL);
        enc.write_invoke_id(self.invoke_id);
        enc.write_attribute_descriptor(&self.request.descriptor);

        // Write access selector and selective access if present
        match &self.request.access_selector {
            crate::types::AccessSelector::None => {
                enc.write_byte(0x01); // Normal access (GET)
            }
            crate::types::AccessSelector::WithRawData(data) => {
                enc.write_byte(0x02); // Selective access
                enc.write_bytes(data);
            }
        }

        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_GET_REQUEST || subtype != TAG_SUBTYPE_NORMAL {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let descriptor = dec.read_attribute_descriptor()?;

        // Read access selector
        let access_selector_byte = dec.read_byte()?;
        let access_selector = match access_selector_byte {
            0x01 => crate::types::AccessSelector::None,
            0x02 => {
                // Selective access - for now, we'll skip parsing and store raw
                // In a real implementation, you'd parse the selective access here
                // For simplicity, we'll just skip to the end
                let remaining = &data[dec.position()..];
                crate::types::AccessSelector::WithRawData(remaining.to_vec())
            }
            _ => return Err(ApduError::InvalidData),
        };

        Ok(Self {
            invoke_id,
            request: AccessRequest {
                descriptor,
                access_selector,
            },
        })
    }
}

/// Get-Request next (block transfer continuation)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetRequestNext {
    pub invoke_id: InvokeId,
    pub block_number: u32,
}

impl GetRequestNext {
    pub fn new(invoke_id: InvokeId, block_number: u32) -> Self {
        Self { invoke_id, block_number }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_GET_REQUEST, TAG_SUBTYPE_NEXT);
        enc.write_invoke_id(self.invoke_id);
        enc.write_u32(self.block_number);
        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_GET_REQUEST || subtype != TAG_SUBTYPE_NEXT {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let block_number = dec.read_u32()?;

        Ok(Self { invoke_id, block_number })
    }
}

/// Single request in Get-Request-With-List
#[derive(Debug, Clone, PartialEq)]
pub struct GetRequestListItem {
    pub descriptor: AttributeDescriptor,
    pub access_selector: crate::types::AccessSelector,
}

/// Get-Request with list (multiple attributes)
#[derive(Debug, Clone, PartialEq)]
pub struct GetRequestWithList {
    pub invoke_id: InvokeId,
    pub requests: Vec<GetRequestListItem>,
}

impl GetRequestWithList {
    pub fn new(invoke_id: InvokeId, requests: Vec<GetRequestListItem>) -> Self {
        Self { invoke_id, requests }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_GET_REQUEST, TAG_SUBTYPE_WITH_LIST);
        enc.write_invoke_id(self.invoke_id);

        // Number of requests (variable length)
        let count = self.requests.len();
        if count < 128 {
            enc.write_byte(count as u8);
        } else if count < 256 {
            enc.write_byte(0x81);
            enc.write_byte(count as u8);
        } else {
            return Err(ApduError::InvalidLength);
        }

        // Encode each request
        for req in &self.requests {
            enc.write_attribute_descriptor(&req.descriptor);
            match &req.access_selector {
                crate::types::AccessSelector::None => {
                    enc.write_byte(0x01);
                }
                crate::types::AccessSelector::WithRawData(data) => {
                    enc.write_byte(0x02);
                    enc.write_bytes(data);
                }
            }
        }

        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_GET_REQUEST || subtype != TAG_SUBTYPE_WITH_LIST {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;

        // Read number of requests
        let first = dec.read_byte()?;
        let count = if first < 128 {
            first as usize
        } else if first == 0x81 {
            dec.read_byte()? as usize
        } else {
            return Err(ApduError::InvalidLength);
        };

        let mut requests = Vec::with_capacity(count);
        for _ in 0..count {
            let descriptor = dec.read_attribute_descriptor()?;
            let access_selector_byte = dec.read_byte()?;
            let access_selector = match access_selector_byte {
                0x01 => crate::types::AccessSelector::None,
                0x02 => {
                    let remaining = &data[dec.position()..];
                    crate::types::AccessSelector::WithRawData(remaining.to_vec())
                }
                _ => return Err(ApduError::InvalidData),
            };
            requests.push(GetRequestListItem { descriptor, access_selector });
        }

        Ok(Self { invoke_id, requests })
    }
}

/// Enum for all Get-Request types
#[derive(Debug, Clone, PartialEq)]
pub enum GetRequest {
    Normal(GetRequestNormal),
    Next(GetRequestNext),
    WithList(GetRequestWithList),
}

impl GetRequest {
    pub fn invoke_id(&self) -> InvokeId {
        match self {
            Self::Normal(r) => r.invoke_id,
            Self::Next(r) => r.invoke_id,
            Self::WithList(r) => r.invoke_id,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        match self {
            Self::Normal(r) => r.encode(),
            Self::Next(r) => Ok(r.encode()),
            Self::WithList(r) => r.encode(),
        }
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        if data.len() < 2 {
            return Err(ApduError::TooShort);
        }

        let subtype = data[1];
        match subtype {
            TAG_SUBTYPE_NORMAL => Ok(Self::Normal(GetRequestNormal::decode(data)?)),
            TAG_SUBTYPE_NEXT => Ok(Self::Next(GetRequestNext::decode(data)?)),
            TAG_SUBTYPE_WITH_LIST => Ok(Self::WithList(GetRequestWithList::decode(data)?)),
            _ => Err(ApduError::InvalidTag(subtype)),
        }
    }
}

// ============================================================
// Get-Response PDUs
// ============================================================

/// Get-Response normal (data)
#[derive(Debug, Clone, PartialEq)]
pub struct GetResponseNormal {
    pub invoke_id: InvokeId,
    pub result: AccessResult,
}

impl GetResponseNormal {
    pub fn success(invoke_id: InvokeId, data: DlmsType) -> Self {
        Self {
            invoke_id,
            result: AccessResult::Success(data),
        }
    }

    pub fn error(invoke_id: InvokeId, error: DataAccessError) -> Self {
        Self {
            invoke_id,
            result: AccessResult::Error(error),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_GET_RESPONSE, TAG_SUBTYPE_DATA);
        enc.write_invoke_id(self.invoke_id);

        match &self.result {
            AccessResult::Success(data) => {
                // Result = 0 (success)
                enc.write_byte(0);
                enc.write_dlms_value(data)?;
            }
            AccessResult::Error(err) => {
                // Result = error code
                enc.write_byte(*err as u8);
            }
        }

        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_GET_RESPONSE || subtype != TAG_SUBTYPE_DATA {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let result_byte = dec.read_byte()?;

        let result = if result_byte == 0 {
            // Success - read the data
            let value = dec.read_dlms_value()?;
            AccessResult::Success(value)
        } else {
            // Error
            let error = DataAccessError::from_code(result_byte)
                .ok_or(ApduError::InvalidData)?;
            AccessResult::Error(error)
        };

        Ok(Self { invoke_id, result })
    }
}

/// Get-Response with block (for block transfer)
#[derive(Debug, Clone, PartialEq)]
pub struct GetResponseBlock {
    pub invoke_id: InvokeId,
    pub block_number: u32,
    pub last_block: bool,
    pub data: Vec<u8>,
}

impl GetResponseBlock {
    pub fn new(invoke_id: InvokeId, block_number: u32, last_block: bool, data: Vec<u8>) -> Self {
        Self {
            invoke_id,
            block_number,
            last_block,
            data,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_GET_RESPONSE, TAG_SUBTYPE_BLOCK);
        enc.write_invoke_id(self.invoke_id);
        enc.write_u32(self.block_number);

        // Status: bit 7 = last block flag
        let status = if self.last_block { 0x80 } else { 0x00 };
        enc.write_byte(status);

        // Data length (uint32)
        enc.write_u32(self.data.len() as u32);
        enc.write_bytes(&self.data);

        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_GET_RESPONSE || subtype != TAG_SUBTYPE_BLOCK {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let block_number = dec.read_u32()?;
        let status = dec.read_byte()?;
        let last_block = (status & 0x80) != 0;

        let data_len = dec.read_u32()? as usize;
        let data = dec.read_bytes(data_len)?.to_vec();

        Ok(Self {
            invoke_id,
            block_number,
            last_block,
            data,
        })
    }
}

/// Get-Response with data access error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetResponseError {
    pub invoke_id: InvokeId,
    pub error: DataAccessError,
}

impl GetResponseError {
    pub fn new(invoke_id: InvokeId, error: DataAccessError) -> Self {
        Self { invoke_id, error }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_GET_RESPONSE, TAG_SUBTYPE_DATA_ACCESS_ERROR);
        enc.write_invoke_id(self.invoke_id);
        enc.write_byte(self.error.clone() as u8);
        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_GET_RESPONSE || subtype != TAG_SUBTYPE_DATA_ACCESS_ERROR {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let error_code = dec.read_byte()?;
        let error = DataAccessError::from_code(error_code)
            .ok_or(ApduError::InvalidData)?;

        Ok(Self { invoke_id, error })
    }
}

/// Enum for all Get-Response types
#[derive(Debug, Clone, PartialEq)]
pub enum GetResponse {
    Data(GetResponseNormal),
    Block(GetResponseBlock),
    DataAccessError(GetResponseError),
}

impl GetResponse {
    pub fn invoke_id(&self) -> InvokeId {
        match self {
            Self::Data(r) => r.invoke_id,
            Self::Block(r) => r.invoke_id,
            Self::DataAccessError(r) => r.invoke_id,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Data(r) => r.encode().unwrap(),
            Self::Block(r) => r.encode(),
            Self::DataAccessError(r) => r.encode(),
        }
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        if data.len() < 2 {
            return Err(ApduError::TooShort);
        }

        let subtype = data[1];
        match subtype {
            TAG_SUBTYPE_DATA => Ok(Self::Data(GetResponseNormal::decode(data)?)),
            TAG_SUBTYPE_BLOCK => Ok(Self::Block(GetResponseBlock::decode(data)?)),
            TAG_SUBTYPE_DATA_ACCESS_ERROR => Ok(Self::DataAccessError(GetResponseError::decode(data)?)),
            _ => Err(ApduError::InvalidTag(subtype)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use dlms_core::ObisCode;

    #[test]
    fn test_get_request_normal_encode() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let req = GetRequestNormal::new(InvokeId::new(1), desc);
        let encoded = req.encode().unwrap();

        assert_eq!(encoded[0], TAG_GET_REQUEST);
        assert_eq!(encoded[1], TAG_SUBTYPE_NORMAL);
        assert_eq!(encoded[2], 1); // invoke_id
    }

    #[test]
    fn test_get_request_normal_roundtrip() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let req = GetRequestNormal::new(InvokeId::new(42), desc);
        let encoded = req.encode().unwrap();
        let decoded = GetRequestNormal::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, req.invoke_id);
        assert_eq!(decoded.request.descriptor, req.request.descriptor);
    }

    #[test]
    fn test_get_request_next_encode() {
        let req = GetRequestNext::new(InvokeId::new(1), 5);
        let encoded = req.encode();

        assert_eq!(encoded[0], TAG_GET_REQUEST);
        assert_eq!(encoded[1], TAG_SUBTYPE_NEXT);
        assert_eq!(encoded[2], 1); // invoke_id
        // Block number should be 5 (big-endian u32)
        assert_eq!(encoded[3..7], [0, 0, 0, 5]);
    }

    #[test]
    fn test_get_request_next_roundtrip() {
        let req = GetRequestNext::new(InvokeId::new(1), 999);
        let encoded = req.encode();
        let decoded = GetRequestNext::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, req.invoke_id);
        assert_eq!(decoded.block_number, req.block_number);
    }

    #[test]
    fn test_get_response_normal_success() {
        let resp = GetResponseNormal::success(
            InvokeId::new(1),
            DlmsType::from_u32(12345),
        );
        let encoded = resp.encode().unwrap();

        assert_eq!(encoded[0], TAG_GET_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_DATA);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 0); // result = success
    }

    #[test]
    fn test_get_response_normal_error() {
        let resp = GetResponseNormal::error(
            InvokeId::new(1),
            DataAccessError::ReadWriteDenied,
        );
        let encoded = resp.encode().unwrap();

        assert_eq!(encoded[0], TAG_GET_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_DATA);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 1); // result = ReadWriteDenied
    }

    #[test]
    fn test_get_response_normal_roundtrip() {
        let resp = GetResponseNormal::success(
            InvokeId::new(42),
            DlmsType::from_u16(100),
        );
        let encoded = resp.encode().unwrap();
        let decoded = GetResponseNormal::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, resp.invoke_id);
        match decoded.result {
            AccessResult::Success(DlmsType::UInt16(100)) => {}
            _ => panic!("Unexpected result"),
        }
    }

    #[test]
    fn test_get_response_block_encode() {
        let data = vec![1, 2, 3, 4];
        let resp = GetResponseBlock::new(InvokeId::new(1), 0, true, data);
        let encoded = resp.encode();

        assert_eq!(encoded[0], TAG_GET_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_BLOCK);
        assert_eq!(encoded[2], 1); // invoke_id
        // Status with last_block flag set
        assert_eq!(encoded[7], 0x80);
    }

    #[test]
    fn test_get_response_block_roundtrip() {
        let data = vec![0xAA, 0xBB, 0xCC];
        let resp = GetResponseBlock::new(InvokeId::new(1), 5, false, data.clone());
        let encoded = resp.encode();
        let decoded = GetResponseBlock::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, resp.invoke_id);
        assert_eq!(decoded.block_number, resp.block_number);
        assert_eq!(decoded.last_block, resp.last_block);
        assert_eq!(decoded.data, resp.data);
    }

    #[test]
    fn test_get_response_error() {
        let resp = GetResponseError::new(InvokeId::new(1), DataAccessError::ObjectUndefined);
        let encoded = resp.encode();

        assert_eq!(encoded[0], TAG_GET_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_DATA_ACCESS_ERROR);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 2); // ObjectUndefined
    }

    #[test]
    fn test_get_request_decode() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let req = GetRequestNormal::new(InvokeId::new(1), desc);
        let encoded = req.encode().unwrap();

        let decoded = GetRequest::decode(&encoded).unwrap();
        match decoded {
            GetRequest::Normal(r) => {
                assert_eq!(r.invoke_id, req.invoke_id);
            }
            _ => panic!("Unexpected request type"),
        }
    }

    #[test]
    fn test_get_response_decode() {
        let resp = GetResponseNormal::success(
            InvokeId::new(1),
            DlmsType::from_u8(42),
        );
        let encoded = resp.encode().unwrap();

        let decoded = GetResponse::decode(&encoded).unwrap();
        match decoded {
            GetResponse::Data(r) => {
                assert_eq!(r.invoke_id, resp.invoke_id);
            }
            _ => panic!("Unexpected response type"),
        }
    }
}
