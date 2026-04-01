//!
//! Interface Class 111: Account
//!
//! Reference: Blue Book Part 2 §5.111
//!
//! Account manages the prepaid account balance and status.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// Account status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AccountStatus {
    /// New account
    New = 0,
    /// Active
    Active = 1,
    /// Suspended
    Suspended = 2,
    /// Closed
    Closed = 3,
}

/// COSEM IC 111: Account
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | account_id | 2 | octet-string | static |
/// | current_status | 3 | enum | dynamic |
/// | current_credit | 4 | double-long | dynamic |
/// | credit_status | 5 | enum | dynamic |
/// | priority | 6 | unsigned | static |
/// | activated_date | 7 | date-time | dynamic |
/// | closed_date | 8 | date-time | dynamic |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | credit | 1 | Add credit |
/// | debit | 2 | Debit credit |
/// | close_account | 3 | Close the account |
#[derive(Debug, Clone)]
pub struct Account {
    logical_name: ObisCode,
    account_id: DlmsType,
    current_status: AccountStatus,
    current_credit: i64,
    credit_status: u8,
    priority: u8,
    activated_date: DlmsType,
    closed_date: DlmsType,
}

impl Account {
    /// Create a new Account object
    pub fn new(logical_name: ObisCode, account_id: DlmsType) -> Self {
        Self {
            logical_name,
            account_id,
            current_status: AccountStatus::New,
            current_credit: 0,
            credit_status: 0,
            priority: 0,
            activated_date: DlmsType::Null,
            closed_date: DlmsType::Null,
        }
    }

    pub const fn get_current_credit(&self) -> i64 {
        self.current_credit
    }

    pub const fn get_status(&self) -> AccountStatus {
        self.current_status
    }
}

impl CosemClass for Account {
    const CLASS_ID: u16 = 111;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        8
    }

    fn method_count() -> u8 {
        3
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.account_id.clone()),
            3 => Ok(DlmsType::UInt8(self.current_status as u8)),
            4 => Ok(DlmsType::Int64(self.current_credit)),
            5 => Ok(DlmsType::UInt8(self.credit_status)),
            6 => Ok(DlmsType::UInt8(self.priority)),
            7 => Ok(self.activated_date.clone()),
            8 => Ok(self.closed_date.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, _value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => Err(CosemError::ReadOnly),
            3 => Err(CosemError::ReadOnly),
            4 => Err(CosemError::ReadOnly),
            5 => Err(CosemError::ReadOnly),
            6 => Err(CosemError::ReadOnly),
            7 => Err(CosemError::ReadOnly),
            8 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                // credit
                if let DlmsType::Int64(amount) = params {
                    self.current_credit += amount;
                    Ok(DlmsType::Null)
                } else {
                    Err(CosemError::InvalidParameter)
                }
            }
            2 => {
                // debit
                if let DlmsType::Int64(amount) = params {
                    self.current_credit -= amount;
                    Ok(DlmsType::Null)
                } else {
                    Err(CosemError::InvalidParameter)
                }
            }
            3 => {
                // close_account
                self.current_status = AccountStatus::Closed;
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
    fn test_account_class_id() {
        let acc = Account::new(
            ObisCode::new(0, 0, 111, 0, 0, 255),
            DlmsType::OctetString(alloc::vec![]),
        );
        assert_eq!(Account::CLASS_ID, 111);
        assert_eq!(acc.get_status(), AccountStatus::New);
    }

    #[test]
    fn test_account_credit() {
        let mut acc = Account::new(
            ObisCode::new(0, 0, 111, 0, 0, 255),
            DlmsType::OctetString(alloc::vec![]),
        );
        acc.execute_method(1, DlmsType::Int64(1000)).unwrap();
        assert_eq!(acc.get_current_credit(), 1000);
    }
}
