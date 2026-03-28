//!
//! Interface Class 70: Disconnect Control
//!
//! Reference: Blue Book Part 2 §5.70
//!
//! Disconnect Control manages the connection/disconnection of the meter.

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// Disconnect control state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DisconnectState {
    Disconnected = 0,
    Connected = 1,
    ReadyForReconnection = 2,
}

/// Control mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ControlMode {
    /// Direct control
    Direct = 0,
    /// Mode 1
    Mode1 = 1,
    /// Mode 2
    Mode2 = 2,
}

/// COSEM IC 70: Disconnect Control
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | output_state | 2 | boolean | dynamic |
/// | control_state | 3 | enum | dynamic |
/// | control_mode | 4 | enum | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | remote_reconnect | 1 | Remote reconnect |
/// | remote_disconnect | 2 | Remote disconnect |
#[derive(Debug, Clone)]
pub struct DisconnectControl {
    logical_name: ObisCode,
    output_state: bool,
    control_state: DisconnectState,
    control_mode: ControlMode,
}

impl DisconnectControl {
    /// Create a new Disconnect Control object
    pub const fn new(logical_name: ObisCode, control_mode: ControlMode) -> Self {
        Self {
            logical_name,
            output_state: false,
            control_state: DisconnectState::Disconnected,
            control_mode,
        }
    }

    pub const fn get_control_state(&self) -> DisconnectState {
        self.control_state
    }
}

impl CosemClass for DisconnectControl {
    const CLASS_ID: u16 = 70;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        4
    }

    fn method_count() -> u8 {
        2
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Boolean(self.output_state)),
            3 => Ok(DlmsType::UInt8(self.control_state as u8)),
            4 => Ok(DlmsType::UInt8(self.control_mode as u8)),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, _value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 3 => Err(CosemError::ReadOnly),
            2 => Err(CosemError::ReadOnly),
            4 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                // remote_reconnect
                self.output_state = true;
                self.control_state = DisconnectState::Connected;
                Ok(DlmsType::Null)
            }
            2 => {
                // remote_disconnect
                self.output_state = false;
                self.control_state = DisconnectState::Disconnected;
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
    fn test_disconnect_control_class_id() {
        let dc = DisconnectControl::new(
            ObisCode::new(0, 0, 96, 3, 10, 255),
            ControlMode::Direct,
        );
        assert_eq!(DisconnectControl::CLASS_ID, 70);
        assert_eq!(DisconnectControl::VERSION, 1);
        assert_eq!(dc.get_control_state(), DisconnectState::Disconnected);
    }

    #[test]
    fn test_disconnect_control_methods() {
        let mut dc = DisconnectControl::new(
            ObisCode::new(0, 0, 96, 3, 10, 255),
            ControlMode::Direct,
        );
        dc.execute_method(2, DlmsType::Null).unwrap();
        assert!(!dc.output_state);
        assert_eq!(dc.control_state, DisconnectState::Disconnected);

        dc.execute_method(1, DlmsType::Null).unwrap();
        assert!(dc.output_state);
        assert_eq!(dc.control_state, DisconnectState::Connected);
    }
}
