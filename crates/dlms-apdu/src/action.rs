//! Action-Request and Action-Response APDUs
//!
//! Reference: IEC 62056-53 §8.4.4

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use crate::codec::{ApduDecoder, ApduEncoder};
use crate::types::{
    ApduError, InvokeId, MethodDescriptor, TAG_ACTION_REQUEST, TAG_ACTION_RESPONSE,
    TAG_SUBTYPE_BLOCK, TAG_SUBTYPE_NEXT, TAG_SUBTYPE_NORMAL, TAG_SUBTYPE_WITH_LIST,
};
use alloc::vec::Vec;
use dlms_core::{DataAccessError, DlmsType};

// ============================================================
// Action-Request PDUs
// ============================================================

/// Action-Request normal (single method invocation)
#[derive(Debug, Clone, PartialEq)]
pub struct ActionRequestNormal {
    pub invoke_id: InvokeId,
    pub method: MethodDescriptor,
    pub parameters: Option<DlmsType>,
}

impl ActionRequestNormal {
    pub fn new(invoke_id: InvokeId, method: MethodDescriptor) -> Self {
        Self {
            invoke_id,
            method,
            parameters: None,
        }
    }

    pub fn with_parameters(
        invoke_id: InvokeId,
        method: MethodDescriptor,
        parameters: DlmsType,
    ) -> Self {
        Self {
            invoke_id,
            method,
            parameters: Some(parameters),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_ACTION_REQUEST, TAG_SUBTYPE_NORMAL);
        enc.write_invoke_id(self.invoke_id);
        enc.write_method_descriptor(&self.method);

        // Parameters (optional)
        match &self.parameters {
            None => {
                // No parameters - write empty octet string
                enc.write_byte(0x09); // octet-string tag
                enc.write_byte(0x00); // length = 0
            }
            Some(params) => {
                enc.write_dlms_value(params)?;
            }
        }

        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_ACTION_REQUEST || subtype != TAG_SUBTYPE_NORMAL {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let method = dec.read_method_descriptor()?;

        // Check if there are parameters remaining
        let parameters = if dec.remaining() > 0 {
            Some(dec.read_dlms_value()?)
        } else {
            None
        };

        Ok(Self {
            invoke_id,
            method,
            parameters,
        })
    }
}

/// Action-Request with list (multiple method invocations)
#[derive(Debug, Clone, PartialEq)]
pub struct ActionRequestListItem {
    pub method: MethodDescriptor,
    pub parameters: Option<DlmsType>,
}

/// Action-Request with list
#[derive(Debug, Clone, PartialEq)]
pub struct ActionRequestWithList {
    pub invoke_id: InvokeId,
    pub items: Vec<ActionRequestListItem>,
}

impl ActionRequestWithList {
    pub fn new(invoke_id: InvokeId, items: Vec<ActionRequestListItem>) -> Self {
        Self { invoke_id, items }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_ACTION_REQUEST, TAG_SUBTYPE_WITH_LIST);
        enc.write_invoke_id(self.invoke_id);

        // Number of items (variable length)
        let count = self.items.len();
        if count < 128 {
            enc.write_byte(count as u8);
        } else if count < 256 {
            enc.write_byte(0x81);
            enc.write_byte(count as u8);
        } else {
            return Err(ApduError::InvalidLength);
        }

        // Encode each item
        for item in &self.items {
            enc.write_method_descriptor(&item.method);
            match &item.parameters {
                None => {
                    enc.write_byte(0x09); // octet-string tag
                    enc.write_byte(0x00); // length = 0
                }
                Some(params) => {
                    enc.write_dlms_value(params)?;
                }
            }
        }

        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_ACTION_REQUEST || subtype != TAG_SUBTYPE_WITH_LIST {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;

        // Read number of items
        let first = dec.read_byte()?;
        let count = if first < 128 {
            first as usize
        } else if first == 0x81 {
            dec.read_byte()? as usize
        } else {
            return Err(ApduError::InvalidLength);
        };

        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            let method = dec.read_method_descriptor()?;

            // Check if there are parameters remaining
            let parameters = if dec.remaining() > 0 {
                // Peek at next byte to see if it's a tag
                let next = data[dec.position()];
                if next == 0x09 || next >= 0x01 {
                    Some(dec.read_dlms_value()?)
                } else {
                    None
                }
            } else {
                None
            };

            items.push(ActionRequestListItem { method, parameters });
        }

        Ok(Self { invoke_id, items })
    }
}

/// Action-Request next (block transfer continuation)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequestNext {
    pub invoke_id: InvokeId,
    pub block_number: u32,
}

impl ActionRequestNext {
    pub fn new(invoke_id: InvokeId, block_number: u32) -> Self {
        Self {
            invoke_id,
            block_number,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_ACTION_REQUEST, TAG_SUBTYPE_NEXT);
        enc.write_invoke_id(self.invoke_id);
        enc.write_u32(self.block_number);
        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_ACTION_REQUEST || subtype != TAG_SUBTYPE_NEXT {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let block_number = dec.read_u32()?;

        Ok(Self {
            invoke_id,
            block_number,
        })
    }
}

/// Enum for all Action-Request types
#[derive(Debug, Clone, PartialEq)]
pub enum ActionRequest {
    Normal(ActionRequestNormal),
    WithList(ActionRequestWithList),
    Next(ActionRequestNext),
}

impl ActionRequest {
    pub fn invoke_id(&self) -> InvokeId {
        match self {
            Self::Normal(r) => r.invoke_id,
            Self::WithList(r) => r.invoke_id,
            Self::Next(r) => r.invoke_id,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        match self {
            Self::Normal(r) => r.encode(),
            Self::WithList(r) => r.encode(),
            Self::Next(r) => Ok(r.encode()),
        }
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        if data.len() < 2 {
            return Err(ApduError::TooShort);
        }

        let subtype = data[1];
        match subtype {
            TAG_SUBTYPE_NORMAL => Ok(Self::Normal(ActionRequestNormal::decode(data)?)),
            TAG_SUBTYPE_WITH_LIST => Ok(Self::WithList(ActionRequestWithList::decode(data)?)),
            TAG_SUBTYPE_NEXT => Ok(Self::Next(ActionRequestNext::decode(data)?)),
            _ => Err(ApduError::InvalidTag(subtype)),
        }
    }
}

// ============================================================
// Action-Response PDUs
// ============================================================

/// Action-Response normal (data)
#[derive(Debug, Clone, PartialEq)]
pub struct ActionResponseNormal {
    pub invoke_id: InvokeId,
    pub result: Result<DlmsType, DataAccessError>,
}

impl ActionResponseNormal {
    pub fn success(invoke_id: InvokeId, data: DlmsType) -> Self {
        Self {
            invoke_id,
            result: Ok(data),
        }
    }

    pub fn error(invoke_id: InvokeId, error: DataAccessError) -> Self {
        Self {
            invoke_id,
            result: Err(error),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_ACTION_RESPONSE, TAG_SUBTYPE_NORMAL);
        enc.write_invoke_id(self.invoke_id);

        match &self.result {
            Ok(data) => {
                // Result = 0 (success)
                enc.write_byte(0);
                enc.write_dlms_value(data)?;
            }
            Err(err) => {
                // Result = error code
                enc.write_byte(*err as u8);
            }
        }

        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_ACTION_RESPONSE || subtype != TAG_SUBTYPE_NORMAL {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let result_byte = dec.read_byte()?;

        let result = if result_byte == 0 {
            // Success - read the return data
            let value = dec.read_dlms_value()?;
            Ok(value)
        } else {
            // Error
            let error = DataAccessError::from_code(result_byte).ok_or(ApduError::InvalidData)?;
            Err(error)
        };

        Ok(Self { invoke_id, result })
    }
}

/// Action-Response with list (multiple method results)
#[derive(Debug, Clone, PartialEq)]
pub struct ActionResponseListItem {
    pub result: Result<DlmsType, DataAccessError>,
}

/// Action-Response with list
#[derive(Debug, Clone, PartialEq)]
pub struct ActionResponseWithList {
    pub invoke_id: InvokeId,
    pub items: Vec<ActionResponseListItem>,
}

impl ActionResponseWithList {
    pub fn new(invoke_id: InvokeId, items: Vec<ActionResponseListItem>) -> Self {
        Self { invoke_id, items }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_ACTION_RESPONSE, TAG_SUBTYPE_WITH_LIST);
        enc.write_invoke_id(self.invoke_id);

        // Number of items
        let count = self.items.len();
        if count < 128 {
            enc.write_byte(count as u8);
        } else if count < 256 {
            enc.write_byte(0x81);
            enc.write_byte(count as u8);
        } else {
            return Err(ApduError::InvalidLength);
        }

        // Encode each item
        for item in &self.items {
            match &item.result {
                Ok(data) => {
                    enc.write_byte(0); // Success
                    enc.write_dlms_value(data)?;
                }
                Err(err) => {
                    enc.write_byte(*err as u8); // Error code
                }
            }
        }

        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_ACTION_RESPONSE || subtype != TAG_SUBTYPE_WITH_LIST {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;

        // Read number of items
        let first = dec.read_byte()?;
        let count = if first < 128 {
            first as usize
        } else if first == 0x81 {
            dec.read_byte()? as usize
        } else {
            return Err(ApduError::InvalidLength);
        };

        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            let result_byte = dec.read_byte()?;
            let result = if result_byte == 0 {
                let value = dec.read_dlms_value()?;
                Ok(value)
            } else {
                let error =
                    DataAccessError::from_code(result_byte).ok_or(ApduError::InvalidData)?;
                Err(error)
            };
            items.push(ActionResponseListItem { result });
        }

        Ok(Self { invoke_id, items })
    }
}

/// Action-Response with block (for block transfer)
#[derive(Debug, Clone, PartialEq)]
pub struct ActionResponseBlock {
    pub invoke_id: InvokeId,
    pub block_number: u32,
    pub last_block: bool,
    pub data: Vec<u8>,
}

impl ActionResponseBlock {
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
        enc.write_tag(TAG_ACTION_RESPONSE, TAG_SUBTYPE_BLOCK);
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
        if tag_type != TAG_ACTION_RESPONSE || subtype != TAG_SUBTYPE_BLOCK {
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

/// Enum for all Action-Response types
#[derive(Debug, Clone, PartialEq)]
pub enum ActionResponse {
    Normal(ActionResponseNormal),
    WithList(ActionResponseWithList),
    Block(ActionResponseBlock),
}

impl ActionResponse {
    pub fn invoke_id(&self) -> InvokeId {
        match self {
            Self::Normal(r) => r.invoke_id,
            Self::WithList(r) => r.invoke_id,
            Self::Block(r) => r.invoke_id,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        match self {
            Self::Normal(r) => r.encode(),
            Self::WithList(r) => r.encode(),
            Self::Block(r) => Ok(r.encode()),
        }
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        if data.len() < 2 {
            return Err(ApduError::TooShort);
        }

        let subtype = data[1];
        match subtype {
            TAG_SUBTYPE_NORMAL => Ok(Self::Normal(ActionResponseNormal::decode(data)?)),
            TAG_SUBTYPE_WITH_LIST => Ok(Self::WithList(ActionResponseWithList::decode(data)?)),
            TAG_SUBTYPE_BLOCK => Ok(Self::Block(ActionResponseBlock::decode(data)?)),
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
    fn test_action_request_normal_encode() {
        let method = MethodDescriptor::new(70, ObisCode::new(0, 0, 96, 3, 10, 255), 1);
        let req = ActionRequestNormal::new(InvokeId::new(1), method);
        let encoded = req.encode().unwrap();

        assert_eq!(encoded[0], TAG_ACTION_REQUEST);
        assert_eq!(encoded[1], TAG_SUBTYPE_NORMAL);
        assert_eq!(encoded[2], 1); // invoke_id
    }

    #[test]
    fn test_action_request_normal_with_parameters() {
        let method = MethodDescriptor::new(70, ObisCode::new(0, 0, 96, 3, 10, 255), 1);
        let params = DlmsType::from_u8(1);
        let req = ActionRequestNormal::with_parameters(InvokeId::new(1), method, params);
        let encoded = req.encode().unwrap();

        assert_eq!(encoded[0], TAG_ACTION_REQUEST);
        assert_eq!(encoded[1], TAG_SUBTYPE_NORMAL);
        assert_eq!(encoded[2], 1); // invoke_id
    }

    #[test]
    fn test_action_request_normal_roundtrip() {
        let method = MethodDescriptor::new(70, ObisCode::new(0, 0, 96, 3, 10, 255), 1);
        let req = ActionRequestNormal::new(InvokeId::new(42), method);
        let encoded = req.encode().unwrap();
        let decoded = ActionRequestNormal::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, req.invoke_id);
        assert_eq!(decoded.method, req.method);
    }

    #[test]
    fn test_action_response_normal_success() {
        let resp = ActionResponseNormal::success(InvokeId::new(1), DlmsType::Null);
        let encoded = resp.encode().unwrap();

        assert_eq!(encoded[0], TAG_ACTION_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_NORMAL);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 0); // result = success
    }

    #[test]
    fn test_action_response_normal_error() {
        let resp = ActionResponseNormal::error(InvokeId::new(1), DataAccessError::ReadWriteDenied);
        let encoded = resp.encode().unwrap();

        assert_eq!(encoded[0], TAG_ACTION_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_NORMAL);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 1); // result = ReadWriteDenied
    }

    #[test]
    fn test_action_response_normal_roundtrip() {
        let resp = ActionResponseNormal::success(InvokeId::new(42), DlmsType::from_u16(100));
        let encoded = resp.encode().unwrap();
        let decoded = ActionResponseNormal::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, resp.invoke_id);
        assert_eq!(decoded.result, resp.result);
    }

    #[test]
    fn test_action_request_next_encode() {
        let req = ActionRequestNext::new(InvokeId::new(1), 5);
        let encoded = req.encode();

        assert_eq!(encoded[0], TAG_ACTION_REQUEST);
        assert_eq!(encoded[1], TAG_SUBTYPE_NEXT);
        assert_eq!(encoded[2], 1); // invoke_id
                                   // Block number should be 5 (big-endian u32)
        assert_eq!(encoded[3..7], [0, 0, 0, 5]);
    }

    #[test]
    fn test_action_response_with_list_encode() {
        let items = vec![
            ActionResponseListItem {
                result: Ok(DlmsType::Null),
            },
            ActionResponseListItem {
                result: Ok(DlmsType::from_u8(42)),
            },
        ];
        let resp = ActionResponseWithList::new(InvokeId::new(1), items);
        let encoded = resp.encode().unwrap();

        assert_eq!(encoded[0], TAG_ACTION_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_WITH_LIST);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 2); // count
    }

    #[test]
    fn test_action_request_decode() {
        let method = MethodDescriptor::new(70, ObisCode::new(0, 0, 96, 3, 10, 255), 1);
        let req = ActionRequestNormal::new(InvokeId::new(1), method);
        let encoded = req.encode().unwrap();

        let decoded = ActionRequest::decode(&encoded).unwrap();
        match decoded {
            ActionRequest::Normal(r) => {
                assert_eq!(r.invoke_id, req.invoke_id);
            }
            _ => panic!("Unexpected request type"),
        }
    }

    #[test]
    fn test_action_response_decode() {
        let resp = ActionResponseNormal::success(InvokeId::new(1), DlmsType::Null);
        let encoded = resp.encode().unwrap();

        let decoded = ActionResponse::decode(&encoded).unwrap();
        match decoded {
            ActionResponse::Normal(r) => {
                assert_eq!(r.invoke_id, resp.invoke_id);
            }
            _ => panic!("Unexpected response type"),
        }
    }
}
