//!
//! Interface Class 71: Limiter
//!
//! Reference: Blue Book Part 2 §5.71
//!
//! Limiter controls power consumption by limiting load.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// Limiter status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LimiterStatus {
    /// Below limit
    BelowLimit = 0,
    /// Approaching limit
    ApproachingLimit = 1,
    /// Exceeded limit
    ExceededLimit = 2,
    /// Emergency
    Emergency = 3,
}

/// COSEM IC 71: Limiter
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | threshold_active | 2 | CHOICE | static |
/// | threshold_normal | 3 | CHOICE | static |
/// | threshold_emergency | 4 | CHOICE | static |
/// | min_over_threshold_active | 5 | double-long-unsigned | static |
/// | min_over_threshold_normal | 6 | double-long-unsigned | static |
/// | min_over_threshold_emergency | 7 | double-long-unsigned | static |
/// | limiter_status | 8 | unsigned | dynamic |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | set_limit | 1 | Set a limit |
/// | set_thresholds | 2 | Set thresholds |
/// | set_emergency_profile | 3 | Set emergency profile |
#[derive(Debug, Clone)]
pub struct Limiter {
    logical_name: ObisCode,
    threshold_active: DlmsType,
    threshold_normal: DlmsType,
    threshold_emergency: DlmsType,
    min_over_threshold_active: u32,
    min_over_threshold_normal: u32,
    min_over_threshold_emergency: u32,
    limiter_status: LimiterStatus,
}

impl Limiter {
    /// Create a new Limiter object
    pub fn new(
        logical_name: ObisCode,
        threshold_active: DlmsType,
        threshold_normal: DlmsType,
        threshold_emergency: DlmsType,
    ) -> Self {
        Self {
            logical_name,
            threshold_active,
            threshold_normal,
            threshold_emergency,
            min_over_threshold_active: 0,
            min_over_threshold_normal: 0,
            min_over_threshold_emergency: 0,
            limiter_status: LimiterStatus::BelowLimit,
        }
    }

    pub const fn get_status(&self) -> LimiterStatus {
        self.limiter_status
    }
}

impl CosemClass for Limiter {
    const CLASS_ID: u16 = 71;
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
            2 => Ok(self.threshold_active.clone()),
            3 => Ok(self.threshold_normal.clone()),
            4 => Ok(self.threshold_emergency.clone()),
            5 => Ok(DlmsType::UInt32(self.min_over_threshold_active)),
            6 => Ok(DlmsType::UInt32(self.min_over_threshold_normal)),
            7 => Ok(DlmsType::UInt32(self.min_over_threshold_emergency)),
            8 => Ok(DlmsType::UInt8(self.limiter_status as u8)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.threshold_active = value;
                Ok(())
            }
            3 => {
                self.threshold_normal = value;
                Ok(())
            }
            4 => {
                self.threshold_emergency = value;
                Ok(())
            }
            5 => {
                if let DlmsType::UInt32(v) = value {
                    self.min_over_threshold_active = v;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 6,
                        got: value.tag(),
                    })
                }
            }
            6 => {
                if let DlmsType::UInt32(v) = value {
                    self.min_over_threshold_normal = v;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 6,
                        got: value.tag(),
                    })
                }
            }
            7 => {
                if let DlmsType::UInt32(v) = value {
                    self.min_over_threshold_emergency = v;
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 6,
                        got: value.tag(),
                    })
                }
            }
            8 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 | 2 | 3 => Ok(DlmsType::Null),
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_class_id() {
        let lim = Limiter::new(
            ObisCode::new(0, 0, 17, 0, 0, 255),
            DlmsType::Null,
            DlmsType::Null,
            DlmsType::Null,
        );
        assert_eq!(Limiter::CLASS_ID, 71);
        assert_eq!(Limiter::method_count(), 3);
    }
}
