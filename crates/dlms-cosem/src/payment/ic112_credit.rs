//!
//! Interface Class 112: Credit
//!
//! Reference: Blue Book Part 2 §5.112
//!
//! Credit manages credit amounts for prepaid meters.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// Credit status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CreditStatus {
    /// Credit available
    Available = 0,
    /// Low credit warning
    LowCredit = 1,
    /// Credit exhausted
    Exhausted = 2,
}

/// COSEM IC 112: Credit
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | credit_available | 2 | double-long | dynamic |
/// | credit_status | 3 | enum | dynamic |
/// | priority | 4 | unsigned | static |
/// | warning_threshold | 5 | double-long | static |
/// | limit | 6 | double-long | static |
/// | credit_type | 7 | enum | static |
/// | credit_reference | 8 | octet-string | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | credit | 1 | Add credit |
#[derive(Debug, Clone)]
pub struct Credit {
    logical_name: ObisCode,
    credit_available: i64,
    credit_status: CreditStatus,
    priority: u8,
    warning_threshold: i64,
    limit: i64,
    credit_type: u8,
    credit_reference: DlmsType,
}

impl Credit {
    /// Create a new Credit object
    pub fn new(
        logical_name: ObisCode,
        warning_threshold: i64,
        limit: i64,
        credit_type: u8,
    ) -> Self {
        Self {
            logical_name,
            credit_available: 0,
            credit_status: CreditStatus::Available,
            priority: 0,
            warning_threshold,
            limit,
            credit_type,
            credit_reference: DlmsType::OctetString(alloc::vec![]),
        }
    }

    pub const fn get_credit_available(&self) -> i64 {
        self.credit_available
    }

    pub const fn get_status(&self) -> CreditStatus {
        self.credit_status
    }
}

impl CosemClass for Credit {
    const CLASS_ID: u16 = 112;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        8
    }

    fn method_count() -> u8 {
        1
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Int64(self.credit_available)),
            3 => Ok(DlmsType::UInt8(self.credit_status as u8)),
            4 => Ok(DlmsType::UInt8(self.priority)),
            5 => Ok(DlmsType::Int64(self.warning_threshold)),
            6 => Ok(DlmsType::Int64(self.limit)),
            7 => Ok(DlmsType::UInt8(self.credit_type)),
            8 => Ok(self.credit_reference.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => Err(CosemError::ReadOnly),
            3 => Err(CosemError::ReadOnly),
            4 => Err(CosemError::ReadOnly),
            5 => {
                if let DlmsType::Int64(t) = value {
                    self.warning_threshold = t;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 21,
                        got: value.tag(),
                    })
                }
            }
            6 => {
                if let DlmsType::Int64(l) = value {
                    self.limit = l;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 21,
                        got: value.tag(),
                    })
                }
            }
            7 => Err(CosemError::ReadOnly),
            8 => {
                self.credit_reference = value;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                // credit
                if let DlmsType::Int64(amount) = params {
                    self.credit_available += amount;
                    // Update status based on credit
                    self.credit_status = if self.credit_available <= 0 {
                        CreditStatus::Exhausted
                    } else if self.credit_available <= self.warning_threshold {
                        CreditStatus::LowCredit
                    } else {
                        CreditStatus::Available
                    };
                    Ok(DlmsType::Null)
                } else {
                    Err(CosemError::InvalidParameter)
                }
            }
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credit_class_id() {
        let credit = Credit::new(
            ObisCode::new(0, 0, 112, 0, 0, 255),
            10,
            0,
            0,
        );
        assert_eq!(Credit::CLASS_ID, 112);
    }
}
