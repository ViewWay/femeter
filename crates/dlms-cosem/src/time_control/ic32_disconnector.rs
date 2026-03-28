//! Disconnector Interface (IC 32)
//!
//! The Disconnector interface represents a physical or logical disconnect mechanism
//! that can be used to control the supply of energy to a load.
//!
//! Reference: IEC 62056-6-2 (Blue Book Part 2) §7.5.32

use dlms_core::{
    errors::CosemError,
    obis::ObisCode,
    traits::CosemClass,
    types::DlmsType,
};

/// Disconnect Control state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum DisconnectControlState {
    /// Disconnector is closed (supply active)
    #[default]
    Closed,
    /// Disconnector is open (supply inactive)
    Open,
}

impl From<u8> for DisconnectControlState {
    fn from(value: u8) -> Self {
        match value {
            0 => DisconnectControlState::Closed,
            1 => DisconnectControlState::Open,
            _ => DisconnectControlState::Closed,
        }
    }
}

impl From<DisconnectControlState> for u8 {
    fn from(state: DisconnectControlState) -> Self {
        match state {
            DisconnectControlState::Closed => 0,
            DisconnectControlState::Open => 1,
        }
    }
}

/// Control state enumeration for disconnector control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum ControlState {
    /// Disconnector is ready and armed
    #[default]
    Ready,
    /// Disconnector has been disconnected
    Disconnected,
    /// Disconnector is in the process of reconnecting
    Reconnecting,
    /// Disconnector has reconnected
    Reconnected,
}

impl From<u8> for ControlState {
    fn from(value: u8) -> Self {
        match value {
            0 => ControlState::Ready,
            1 => ControlState::Disconnected,
            2 => ControlState::Reconnecting,
            3 => ControlState::Reconnected,
            _ => ControlState::Ready,
        }
    }
}

impl From<ControlState> for u8 {
    fn from(state: ControlState) -> Self {
        match state {
            ControlState::Ready => 0,
            ControlState::Disconnected => 1,
            ControlState::Reconnecting => 2,
            ControlState::Reconnected => 3,
        }
    }
}

/// Disconnector Interface Class (IC 32)
///
/// Attributes:
/// - 1: logical_name (octet-string)
/// - 2: disconnect_control (enum)
/// - 3: control_state (enum)
///
/// Methods:
/// - 1: disconnect
/// - 2: reconnect
/// - 3: arm
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct Disconnector {
    logical_name: ObisCode,
    disconnect_control: DisconnectControlState,
    control_state: ControlState,
}

impl Disconnector {
    /// Create a new Disconnector instance
    ///
    /// # Arguments
    ///
    /// * `logical_name` - OBIS code identifying this object
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            disconnect_control: DisconnectControlState::Closed,
            control_state: ControlState::Ready,
        }
    }

    /// Get the disconnect control state
    pub fn disconnect_control(&self) -> DisconnectControlState {
        self.disconnect_control
    }

    /// Get the control state
    pub fn control_state(&self) -> ControlState {
        self.control_state
    }

    /// Disconnect method - open the disconnector
    pub fn disconnect(&mut self) -> Result<(), CosemError> {
        self.disconnect_control = DisconnectControlState::Open;
        self.control_state = ControlState::Disconnected;
        Ok(())
    }

    /// Reconnect method - close the disconnector
    pub fn reconnect(&mut self) -> Result<(), CosemError> {
        self.disconnect_control = DisconnectControlState::Closed;
        self.control_state = ControlState::Reconnected;
        Ok(())
    }

    /// Arm method - prepare the disconnector for operation
    pub fn arm(&mut self) -> Result<(), CosemError> {
        self.control_state = ControlState::Ready;
        Ok(())
    }
}

impl CosemClass for Disconnector {
    const CLASS_ID: u16 = 32;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::Enum(u8::from(self.disconnect_control))),
            3 => Ok(DlmsType::Enum(u8::from(self.control_state))),
            _ => Err(CosemError::NotImplemented),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            2 => {
                if let DlmsType::Enum(state) = value {
                    self.disconnect_control = DisconnectControlState::from(state);
                    Ok(())
                } else {
                    Err(CosemError::NotImplemented)
                }
            }
            _ => Err(CosemError::NotImplemented),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                self.disconnect()?;
                Ok(DlmsType::Null)
            }
            2 => {
                self.reconnect()?;
                Ok(DlmsType::Null)
            }
            3 => {
                self.arm()?;
                Ok(DlmsType::Null)
            }
            _ => Err(CosemError::NotImplemented),
        }
    }

    fn attribute_count() -> u8 {
        3
    }

    fn method_count() -> u8 {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_disconnector() -> Disconnector {
        Disconnector::new(ObisCode::new(0, 0, 96, 1, 0, 255))
    }

    #[test]
    fn test_disconnector_creation() {
        let dis = create_test_disconnector();
        assert_eq!(dis.disconnect_control(), DisconnectControlState::Closed);
        assert_eq!(dis.control_state(), ControlState::Ready);
    }

    #[test]
    fn test_disconnect() {
        let mut dis = create_test_disconnector();
        dis.disconnect().unwrap();
        assert_eq!(dis.disconnect_control(), DisconnectControlState::Open);
        assert_eq!(dis.control_state(), ControlState::Disconnected);
    }

    #[test]
    fn test_reconnect() {
        let mut dis = create_test_disconnector();
        dis.disconnect().unwrap();
        dis.reconnect().unwrap();
        assert_eq!(dis.disconnect_control(), DisconnectControlState::Closed);
        assert_eq!(dis.control_state(), ControlState::Reconnected);
    }

    #[test]
    fn test_arm() {
        let mut dis = create_test_disconnector();
        dis.disconnect().unwrap();
        dis.arm().unwrap();
        assert_eq!(dis.control_state(), ControlState::Ready);
    }

    #[test]
    fn test_get_attributes() {
        let dis = create_test_disconnector();
        let ln = dis.get_attribute(1).unwrap();
        let dc = dis.get_attribute(2).unwrap();
        let cs = dis.get_attribute(3).unwrap();

        assert!(matches!(ln, DlmsType::OctetString(_)));
        assert!(matches!(dc, DlmsType::Enum(_)));
        assert!(matches!(cs, DlmsType::Enum(_)));
    }

    #[test]
    fn test_set_disconnect_control() {
        let mut dis = create_test_disconnector();
        dis.set_attribute(2, DlmsType::Enum(1)).unwrap();
        assert_eq!(dis.disconnect_control(), DisconnectControlState::Open);
    }

    #[test]
    fn test_execute_methods() {
        let mut dis = create_test_disconnector();

        // Test disconnect method
        let result = dis.execute_method(1, DlmsType::Null).unwrap();
        assert!(matches!(result, DlmsType::Null));
        assert_eq!(dis.control_state(), ControlState::Disconnected);

        // Test reconnect method
        let result = dis.execute_method(2, DlmsType::Null).unwrap();
        assert!(matches!(result, DlmsType::Null));
        assert_eq!(dis.control_state(), ControlState::Reconnected);

        // Test arm method
        let result = dis.execute_method(3, DlmsType::Null).unwrap();
        assert!(matches!(result, DlmsType::Null));
        assert_eq!(dis.control_state(), ControlState::Ready);
    }
}
