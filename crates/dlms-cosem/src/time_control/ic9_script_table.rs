//!
//! Interface Class 9: Script Table
//!
//! Reference: Blue Book Part 2 §5.9
//!
//! Script Table stores and executes scripts (sequences of actions).

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// Script action
#[derive(Debug, Clone)]
pub struct ScriptAction {
    /// Target OBIS code
    pub obis: ObisCode,
    /// Attribute or method ID
    pub id: u8,
    /// Method parameter (if method)
    pub parameter: DlmsType,
}

/// Script definition
#[derive(Debug, Clone)]
pub struct Script {
    /// Script ID
    pub script_id: u8,
    /// Actions to execute
    pub actions: alloc::vec::Vec<ScriptAction>,
}

/// COSEM IC 9: Script Table
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | scripts | 2 | array of structure | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | execute | 1 | Execute a script |
#[derive(Debug, Clone)]
pub struct ScriptTable {
    logical_name: ObisCode,
    scripts: DlmsType,
}

impl ScriptTable {
    /// Create a new Script Table object
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            scripts: DlmsType::Array(alloc::vec![]),
        }
    }
}

impl CosemClass for ScriptTable {
    const CLASS_ID: u16 = 9;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        2
    }

    fn method_count() -> u8 {
        1
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(self.scripts.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 => Err(CosemError::ReadOnly),
            2 => {
                self.scripts = value;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::Null), // execute
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_table_class_id() {
        let st = ScriptTable::new(ObisCode::new(0, 0, 10, 0, 0, 255));
        assert_eq!(ScriptTable::CLASS_ID, 9);
    }
}
