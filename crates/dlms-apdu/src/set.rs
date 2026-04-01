//! Set-Request and Set-Response APDUs
//!
//! Reference: IEC 62056-53 §8.4.2

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use crate::codec::{ApduDecoder, ApduEncoder};
use crate::types::{
    AccessResult, ApduError, AttributeDescriptor, InvokeId, TAG_SET_REQUEST, TAG_SET_RESPONSE,
    TAG_SUBTYPE_BLOCK, TAG_SUBTYPE_DATA, TAG_SUBTYPE_DATA_ACCESS_ERROR, TAG_SUBTYPE_NORMAL,
    TAG_SUBTYPE_WITH_LIST,
};
use alloc::vec::Vec;
use dlms_core::{DataAccessError, DlmsType};

// ============================================================
// Set-Request PDUs
// ============================================================

/// Single item in Set-Request
#[derive(Debug, Clone, PartialEq)]
pub struct SetRequestItem {
    pub descriptor: AttributeDescriptor,
    pub value: DlmsType,
}

/// Set-Request normal (single attribute write)
#[derive(Debug, Clone, PartialEq)]
pub struct SetRequestNormal {
    pub invoke_id: InvokeId,
    pub item: SetRequestItem,
}

impl SetRequestNormal {
    pub fn new(invoke_id: InvokeId, descriptor: AttributeDescriptor, value: DlmsType) -> Self {
        Self {
            invoke_id,
            item: SetRequestItem { descriptor, value },
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_SET_REQUEST, TAG_SUBTYPE_NORMAL);
        enc.write_invoke_id(self.invoke_id);
        enc.write_attribute_descriptor(&self.item.descriptor);
        enc.write_byte(0x02); // Access selector = SET
        enc.write_dlms_value(&self.item.value)?;
        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_SET_REQUEST || subtype != TAG_SUBTYPE_NORMAL {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let descriptor = dec.read_attribute_descriptor()?;
        let _access_selector = dec.read_byte()?; // Should be 0x02 for SET
        let value = dec.read_dlms_value()?;

        Ok(Self {
            invoke_id,
            item: SetRequestItem { descriptor, value },
        })
    }
}

/// Set-Request with list (multiple attributes)
#[derive(Debug, Clone, PartialEq)]
pub struct SetRequestWithList {
    pub invoke_id: InvokeId,
    pub items: Vec<SetRequestItem>,
}

impl SetRequestWithList {
    pub fn new(invoke_id: InvokeId, items: Vec<SetRequestItem>) -> Self {
        Self { invoke_id, items }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_SET_REQUEST, TAG_SUBTYPE_WITH_LIST);
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
            enc.write_attribute_descriptor(&item.descriptor);
            enc.write_byte(0x02); // Access selector = SET
            enc.write_dlms_value(&item.value)?;
        }

        Ok(enc.into_bytes())
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_SET_REQUEST || subtype != TAG_SUBTYPE_WITH_LIST {
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
            let descriptor = dec.read_attribute_descriptor()?;
            let _access_selector = dec.read_byte()?;
            let value = dec.read_dlms_value()?;
            items.push(SetRequestItem { descriptor, value });
        }

        Ok(Self { invoke_id, items })
    }
}

/// Enum for all Set-Request types
#[derive(Debug, Clone, PartialEq)]
pub enum SetRequest {
    Normal(SetRequestNormal),
    WithList(SetRequestWithList),
}

impl SetRequest {
    pub fn invoke_id(&self) -> InvokeId {
        match self {
            Self::Normal(r) => r.invoke_id,
            Self::WithList(r) => r.invoke_id,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        match self {
            Self::Normal(r) => r.encode(),
            Self::WithList(r) => r.encode(),
        }
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        if data.len() < 2 {
            return Err(ApduError::TooShort);
        }

        let subtype = data[1];
        match subtype {
            TAG_SUBTYPE_NORMAL => Ok(Self::Normal(SetRequestNormal::decode(data)?)),
            TAG_SUBTYPE_WITH_LIST => Ok(Self::WithList(SetRequestWithList::decode(data)?)),
            _ => Err(ApduError::InvalidTag(subtype)),
        }
    }
}

// ============================================================
// Set-Response PDUs
// ============================================================

/// Set-Response normal (data)
#[derive(Debug, Clone, PartialEq)]
pub struct SetResponseNormal {
    pub invoke_id: InvokeId,
    pub result: AccessResult,
}

impl SetResponseNormal {
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
        enc.write_tag(TAG_SET_RESPONSE, TAG_SUBTYPE_DATA);
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
        if tag_type != TAG_SET_RESPONSE || subtype != TAG_SUBTYPE_DATA {
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
            let error = DataAccessError::from_code(result_byte).ok_or(ApduError::InvalidData)?;
            AccessResult::Error(error)
        };

        Ok(Self { invoke_id, result })
    }
}

/// Set-Response with block (for block transfer)
#[derive(Debug, Clone, PartialEq)]
pub struct SetResponseBlock {
    pub invoke_id: InvokeId,
    pub block_number: u32,
    pub last_block: bool,
    pub data: Vec<u8>,
}

impl SetResponseBlock {
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
        enc.write_tag(TAG_SET_RESPONSE, TAG_SUBTYPE_BLOCK);
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
        if tag_type != TAG_SET_RESPONSE || subtype != TAG_SUBTYPE_BLOCK {
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

/// Set-Response with data access error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetResponseError {
    pub invoke_id: InvokeId,
    pub error: DataAccessError,
}

impl SetResponseError {
    pub fn new(invoke_id: InvokeId, error: DataAccessError) -> Self {
        Self { invoke_id, error }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_SET_RESPONSE, TAG_SUBTYPE_DATA_ACCESS_ERROR);
        enc.write_invoke_id(self.invoke_id);
        enc.write_byte(self.error as u8);
        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != TAG_SET_RESPONSE || subtype != TAG_SUBTYPE_DATA_ACCESS_ERROR {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let error_code = dec.read_byte()?;
        let error = DataAccessError::from_code(error_code).ok_or(ApduError::InvalidData)?;

        Ok(Self { invoke_id, error })
    }
}

/// Enum for all Set-Response types
#[derive(Debug, Clone, PartialEq)]
pub enum SetResponse {
    Data(SetResponseNormal),
    Block(SetResponseBlock),
    DataAccessError(SetResponseError),
}

impl SetResponse {
    pub fn invoke_id(&self) -> InvokeId {
        match self {
            Self::Data(r) => r.invoke_id,
            Self::Block(r) => r.invoke_id,
            Self::DataAccessError(r) => r.invoke_id,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApduError> {
        match self {
            Self::Data(r) => r.encode(),
            Self::Block(r) => Ok(r.encode()),
            Self::DataAccessError(r) => Ok(r.encode()),
        }
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        if data.len() < 2 {
            return Err(ApduError::TooShort);
        }

        let subtype = data[1];
        match subtype {
            TAG_SUBTYPE_DATA => Ok(Self::Data(SetResponseNormal::decode(data)?)),
            TAG_SUBTYPE_BLOCK => Ok(Self::Block(SetResponseBlock::decode(data)?)),
            TAG_SUBTYPE_DATA_ACCESS_ERROR => {
                Ok(Self::DataAccessError(SetResponseError::decode(data)?))
            }
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
    fn test_set_request_normal_encode() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let req = SetRequestNormal::new(InvokeId::new(1), desc, DlmsType::from_u32(100));
        let encoded = req.encode().unwrap();

        assert_eq!(encoded[0], TAG_SET_REQUEST);
        assert_eq!(encoded[1], TAG_SUBTYPE_NORMAL);
        assert_eq!(encoded[2], 1); // invoke_id
    }

    #[test]
    fn test_set_request_normal_roundtrip() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let req = SetRequestNormal::new(InvokeId::new(42), desc, DlmsType::from_u16(12345));
        let encoded = req.encode().unwrap();
        let decoded = SetRequestNormal::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, req.invoke_id);
        assert_eq!(decoded.item.descriptor, req.item.descriptor);
        assert_eq!(decoded.item.value, req.item.value);
    }

    #[test]
    fn test_set_request_with_list_encode() {
        let items = vec![
            SetRequestItem {
                descriptor: AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2),
                value: DlmsType::from_u8(10),
            },
            SetRequestItem {
                descriptor: AttributeDescriptor::new(3, ObisCode::new(1, 0, 2, 8, 0, 255), 2),
                value: DlmsType::from_u8(20),
            },
        ];
        let req = SetRequestWithList::new(InvokeId::new(1), items);
        let encoded = req.encode().unwrap();

        assert_eq!(encoded[0], TAG_SET_REQUEST);
        assert_eq!(encoded[1], TAG_SUBTYPE_WITH_LIST);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 2); // count
    }

    #[test]
    fn test_set_response_normal_success() {
        let resp = SetResponseNormal::success(InvokeId::new(1), DlmsType::from_u32(0));
        let encoded = resp.encode().unwrap();

        assert_eq!(encoded[0], TAG_SET_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_DATA);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 0); // result = success
    }

    #[test]
    fn test_set_response_normal_error() {
        let resp = SetResponseNormal::error(InvokeId::new(1), DataAccessError::ReadWriteDenied);
        let encoded = resp.encode().unwrap();

        assert_eq!(encoded[0], TAG_SET_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_DATA);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 1); // result = ReadWriteDenied
    }

    #[test]
    fn test_set_response_normal_roundtrip() {
        let resp = SetResponseNormal::success(InvokeId::new(42), DlmsType::Null);
        let encoded = resp.encode().unwrap();
        let decoded = SetResponseNormal::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, resp.invoke_id);
        match decoded.result {
            AccessResult::Success(DlmsType::Null) => {}
            _ => panic!("Unexpected result"),
        }
    }

    #[test]
    fn test_set_response_block_encode() {
        let data = vec![1, 2, 3, 4];
        let resp = SetResponseBlock::new(InvokeId::new(1), 0, true, data);
        let encoded = resp.encode();

        assert_eq!(encoded[0], TAG_SET_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_BLOCK);
        assert_eq!(encoded[2], 1); // invoke_id
                                   // Status with last_block flag set
        assert_eq!(encoded[7], 0x80);
    }

    #[test]
    fn test_set_response_error() {
        let resp = SetResponseError::new(InvokeId::new(1), DataAccessError::TypeUnmatched);
        let encoded = resp.encode();

        assert_eq!(encoded[0], TAG_SET_RESPONSE);
        assert_eq!(encoded[1], TAG_SUBTYPE_DATA_ACCESS_ERROR);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], 8); // TypeUnmatched
    }

    #[test]
    fn test_set_request_decode() {
        let desc = AttributeDescriptor::new(3, ObisCode::new(1, 0, 1, 8, 0, 255), 2);
        let req = SetRequestNormal::new(InvokeId::new(1), desc, DlmsType::from_u8(99));
        let encoded = req.encode().unwrap();

        let decoded = SetRequest::decode(&encoded).unwrap();
        match decoded {
            SetRequest::Normal(r) => {
                assert_eq!(r.invoke_id, req.invoke_id);
            }
            _ => panic!("Unexpected request type"),
        }
    }

    #[test]
    fn test_set_response_decode() {
        let resp = SetResponseNormal::success(InvokeId::new(1), DlmsType::Null);
        let encoded = resp.encode().unwrap();

        let decoded = SetResponse::decode(&encoded).unwrap();
        match decoded {
            SetResponse::Data(r) => {
                assert_eq!(r.invoke_id, resp.invoke_id);
            }
            _ => panic!("Unexpected response type"),
        }
    }
}
