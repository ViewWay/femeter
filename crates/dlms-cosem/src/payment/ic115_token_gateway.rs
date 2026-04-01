//!
//! Interface Class 115: Token Gateway
//!
//! Reference: Blue Book Part 2 §5.115
//!
//! Token Gateway processes token-based credit transfers for prepaid meters.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// Token gateway status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenGatewayStatus {
    /// Idle
    Idle = 0,
    /// Processing token
    Processing = 1,
    /// Token accepted
    Accepted = 2,
    /// Token rejected
    Rejected = 3,
}

/// COSEM IC 115: Token Gateway
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | token_status | 2 | enum | dynamic |
/// | token_id | 3 | octet-string | dynamic |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | transfer_token | 1 | Transfer a token |
#[derive(Debug, Clone)]
pub struct TokenGateway {
    logical_name: ObisCode,
    token_status: TokenGatewayStatus,
    token_id: DlmsType,
}

impl TokenGateway {
    /// Create a new Token Gateway object
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            token_status: TokenGatewayStatus::Idle,
            token_id: DlmsType::OctetString(alloc::vec![]),
        }
    }

    pub const fn get_status(&self) -> TokenGatewayStatus {
        self.token_status
    }
}

impl CosemClass for TokenGateway {
    const CLASS_ID: u16 = 115;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        3
    }

    fn method_count() -> u8 {
        1
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt8(self.token_status as u8)),
            3 => Ok(self.token_id.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, _value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => Err(CosemError::ReadOnly),
            3 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                // transfer_token
                self.token_status = TokenGatewayStatus::Processing;
                // Process token (placeholder - actual validation would go here)
                self.token_status = TokenGatewayStatus::Accepted;
                if let DlmsType::OctetString(data) = &params {
                    self.token_id = DlmsType::OctetString(data.clone());
                }
                Ok(DlmsType::Null)
            }
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_gateway_class_id() {
        let tg = TokenGateway::new(ObisCode::new(0, 0, 115, 0, 0, 255));
        assert_eq!(TokenGateway::CLASS_ID, 115);
        assert_eq!(tg.get_status(), TokenGatewayStatus::Idle);
    }
}
