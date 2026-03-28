//!
//! Interface Class 68: Arbitrator
//!
//! Reference: Blue Book Part 2 §5.68
//!
//! Arbitrator manages arbitration between competing control sources.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// COSEM IC 68: Arbitrator
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | sources | 2 | array of structure | static |
/// | winner_source | 3 | unsigned | dynamic |
/// | permission | 4 | boolean | dynamic |
///
/// Methods: None
#[derive(Debug, Clone)]
pub struct Arbitrator {
    logical_name: ObisCode,
    sources: DlmsType,
    winner_source: u8,
    permission: bool,
}

impl Arbitrator {
    /// Create a new Arbitrator object
    pub fn new(logical_name: ObisCode, sources: DlmsType) -> Self {
        Self {
            logical_name,
            sources,
            winner_source: 0,
            permission: true,
        }
    }

    pub const fn get_winner_source(&self) -> u8 {
        self.winner_source
    }

    pub const fn has_permission(&self) -> bool {
        self.permission
    }
}

impl CosemClass for Arbitrator {
    const CLASS_ID: u16 = 68;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        4
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.sources.clone()),
            3 => Ok(DlmsType::UInt8(self.winner_source)),
            4 => Ok(DlmsType::Boolean(self.permission)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.sources = value;
                Ok(())
            }
            3 => Err(CosemError::ReadOnly),
            4 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrator_class_id() {
        let arb = Arbitrator::new(
            ObisCode::new(0, 0, 68, 0, 0, 255),
            DlmsType::Array(alloc::vec![]),
        );
        assert_eq!(Arbitrator::CLASS_ID, 68);
    }
}
