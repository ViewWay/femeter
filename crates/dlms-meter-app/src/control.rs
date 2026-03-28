//! Relay control for load disconnect/reconnect functionality
//!
//! This module provides:
//! - Relay control for load disconnection
//! - Reconnect control (manual and automatic)
//! - Relay status monitoring
//! - Emergency disconnect handling

#![no_std]

extern crate alloc;

use dlms_core::{errors::CosemError, types::DlmsType};

/// Relay state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RelayState {
    /// Relay closed (load connected)
    Closed = 0,
    /// Relay open (load disconnected)
    Open = 1,
    /// Relay in intermediate state
    Intermediate = 2,
}

impl RelayState {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Closed),
            1 => Some(Self::Open),
            2 => Some(Self::Intermediate),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Check if load is connected
    pub fn is_connected(self) -> bool {
        self == Self::Closed
    }
}

/// Control mode for relay operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ControlMode {
    /// Manual control only
    Manual = 0,
    /// Automatic control based on credit/limits
    Automatic = 1,
    /// Both manual and automatic
    Both = 2,
}

impl ControlMode {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Manual),
            1 => Some(Self::Automatic),
            2 => Some(Self::Both),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Output state for control signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OutputState {
    /// Output disabled
    Disabled = 0,
    /// Output enabled (active)
    Enabled = 1,
}

impl OutputState {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Disabled),
            1 => Some(Self::Enabled),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Check if enabled
    pub fn is_enabled(self) -> bool {
        self == Self::Enabled
    }
}

/// Relay control for load disconnection
#[derive(Debug, PartialEq)]
pub struct RelayControl {
    /// Current relay state
    state: RelayState,
    /// Control mode
    mode: ControlMode,
    /// Output state for control signal
    output_state: OutputState,
    /// Number of disconnect operations
    disconnect_count: u32,
    /// Number of reconnect operations
    reconnect_count: u32,
    /// Emergency disconnect flag
    emergency_disconnect: bool,
    /// Manual disconnect override flag
    manual_override: bool,
    /// Automatic reconnect enabled
    auto_reconnect: bool,
    /// Minimum time between operations (seconds)
    min_operation_interval_s: u32,
    /// Time of last operation
    last_operation_time: u32,
}

impl RelayControl {
    /// Create a new relay control
    pub fn new() -> Self {
        Self {
            state: RelayState::Closed,
            mode: ControlMode::Both,
            output_state: OutputState::Disabled,
            disconnect_count: 0,
            reconnect_count: 0,
            emergency_disconnect: false,
            manual_override: false,
            auto_reconnect: false,
            min_operation_interval_s: 5, // 5 second minimum
            last_operation_time: 0,
        }
    }

    /// Get current relay state
    pub fn state(&self) -> RelayState {
        self.state
    }

    /// Get control mode
    pub fn mode(&self) -> ControlMode {
        self.mode
    }

    /// Set control mode
    pub fn set_mode(&mut self, mode: ControlMode) {
        self.mode = mode;
    }

    /// Get output state
    pub fn output_state(&self) -> OutputState {
        self.output_state
    }

    /// Check if load is currently connected
    pub fn is_connected(&self) -> bool {
        self.state.is_connected()
    }

    /// Get disconnect count
    pub fn disconnect_count(&self) -> u32 {
        self.disconnect_count
    }

    /// Get reconnect count
    pub fn reconnect_count(&self) -> u32 {
        self.reconnect_count
    }

    /// Disconnect the load
    pub fn disconnect(&mut self, current_time: u32) -> Result<(), CosemError> {
        if self.state == RelayState::Open {
            return Err(CosemError::AccessDenied); // Already disconnected
        }

        // Check minimum interval
        if current_time < self.last_operation_time + self.min_operation_interval_s {
            return Err(CosemError::TemporaryFailure);
        }

        self.state = RelayState::Open;
        self.disconnect_count = self.disconnect_count.saturating_add(1);
        self.last_operation_time = current_time;

        Ok(())
    }

    /// Reconnect the load
    pub fn reconnect(&mut self, current_time: u32) -> Result<(), CosemError> {
        if self.state == RelayState::Closed {
            return Err(CosemError::AccessDenied); // Already connected
        }

        // Check if emergency disconnect is active
        if self.emergency_disconnect {
            return Err(CosemError::AccessDenied);
        }

        // Check minimum interval
        if current_time < self.last_operation_time + self.min_operation_interval_s {
            return Err(CosemError::TemporaryFailure);
        }

        self.state = RelayState::Closed;
        self.reconnect_count = self.reconnect_count.saturating_add(1);
        self.last_operation_time = current_time;

        Ok(())
    }

    /// Perform emergency disconnect
    pub fn emergency_disconnect(&mut self, current_time: u32) -> Result<(), CosemError> {
        self.emergency_disconnect = true;
        self.disconnect(current_time)
    }

    /// Clear emergency disconnect
    pub fn clear_emergency(&mut self) {
        self.emergency_disconnect = false;
    }

    /// Check if emergency disconnect is active
    pub fn is_emergency_active(&self) -> bool {
        self.emergency_disconnect
    }

    /// Set manual override
    pub fn set_manual_override(&mut self, override_state: bool) {
        self.manual_override = override_state;
    }

    /// Get manual override state
    pub fn manual_override(&self) -> bool {
        self.manual_override
    }

    /// Enable/disable automatic reconnect
    pub fn set_auto_reconnect(&mut self, enabled: bool) {
        self.auto_reconnect = enabled;
    }

    /// Check if auto reconnect is enabled
    pub fn auto_reconnect(&self) -> bool {
        self.auto_reconnect
    }

    /// Attempt automatic reconnect (if conditions allow)
    pub fn try_auto_reconnect(&mut self, current_time: u32) -> Result<(), CosemError> {
        if !self.auto_reconnect {
            return Err(CosemError::AccessDenied);
        }

        if self.emergency_disconnect {
            return Err(CosemError::AccessDenied);
        }

        self.reconnect(current_time)
    }

    /// Set control output state
    pub fn set_output(&mut self, state: OutputState) {
        self.output_state = state;
    }

    /// Get operation counts as DLMS structure
    pub fn operation_counts_dlms(&self) -> DlmsType {
        DlmsType::Structure(alloc::vec![
            DlmsType::UInt32(self.disconnect_count),
            DlmsType::UInt32(self.reconnect_count),
        ])
    }

    /// Reset operation counters
    pub fn reset_counters(&mut self) {
        self.disconnect_count = 0;
        self.reconnect_count = 0;
    }

    /// Set minimum operation interval
    pub fn set_min_interval(&mut self, interval_s: u32) {
        self.min_operation_interval_s = interval_s;
    }

    /// Get minimum operation interval
    pub fn min_interval(&self) -> u32 {
        self.min_operation_interval_s
    }
}

impl Default for RelayControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_control_new() {
        let control = RelayControl::new();
        assert_eq!(control.state(), RelayState::Closed);
        assert!(control.is_connected());
        assert_eq!(control.disconnect_count(), 0);
    }

    #[test]
    fn test_disconnect() {
        let mut control = RelayControl::new();

        assert!(control.disconnect(100).is_ok());
        assert_eq!(control.state(), RelayState::Open);
        assert!(!control.is_connected());
        assert_eq!(control.disconnect_count(), 1);
    }

    #[test]
    fn test_reconnect() {
        let mut control = RelayControl::new();

        control.disconnect(100).unwrap();
        assert!(control.reconnect(200).is_ok());
        assert_eq!(control.state(), RelayState::Closed);
        assert!(control.is_connected());
        assert_eq!(control.reconnect_count(), 1);
    }

    #[test]
    fn test_double_disconnect() {
        let mut control = RelayControl::new();

        control.disconnect(100).unwrap();
        assert!(control.disconnect(200).is_err()); // Already disconnected
    }

    #[test]
    fn test_emergency_disconnect() {
        let mut control = RelayControl::new();

        control.emergency_disconnect(100).unwrap();
        assert!(control.is_emergency_active());
        assert_eq!(control.state(), RelayState::Open);

        // Cannot reconnect while emergency is active
        assert!(control.reconnect(200).is_err());

        control.clear_emergency();
        assert!(!control.is_emergency_active());
        assert!(control.reconnect(300).is_ok());
    }

    #[test]
    fn test_auto_reconnect() {
        let mut control = RelayControl::new();

        control.set_auto_reconnect(true);
        control.disconnect(100).unwrap();

        assert!(control.try_auto_reconnect(200).is_ok());
        assert!(control.is_connected());
    }

    #[test]
    fn test_auto_reconnect_disabled() {
        let mut control = RelayControl::new();

        // Auto reconnect disabled by default
        control.disconnect(100).unwrap();

        assert!(control.try_auto_reconnect(200).is_err());
        assert!(!control.is_connected());
    }

    #[test]
    fn test_min_interval() {
        let mut control = RelayControl::new();
        control.set_min_interval(10);

        control.disconnect(100).unwrap();
        // Try to reconnect before interval
        assert!(control.reconnect(105).is_err());

        // After interval
        assert!(control.reconnect(115).is_ok());
    }

    #[test]
    fn test_output_state() {
        let mut control = RelayControl::new();

        control.set_output(OutputState::Enabled);
        assert_eq!(control.output_state(), OutputState::Enabled);
        assert!(control.output_state().is_enabled());
    }

    #[test]
    fn test_manual_override() {
        let mut control = RelayControl::new();

        control.set_manual_override(true);
        assert!(control.manual_override());
    }

    #[test]
    fn test_reset_counters() {
        let mut control = RelayControl::new();

        control.disconnect(100).unwrap();
        control.reconnect(200).unwrap();
        control.disconnect(300).unwrap();

        control.reset_counters();
        assert_eq!(control.disconnect_count(), 0);
        assert_eq!(control.reconnect_count(), 0);
    }

    #[test]
    fn test_relay_state_conversion() {
        assert_eq!(RelayState::from_u8(0), Some(RelayState::Closed));
        assert_eq!(RelayState::from_u8(1), Some(RelayState::Open));
        assert_eq!(RelayState::from_u8(2), Some(RelayState::Intermediate));
        assert_eq!(RelayState::from_u8(99), None);

        assert_eq!(RelayState::Closed.code(), 0);
        assert!(RelayState::Closed.is_connected());
        assert!(!RelayState::Open.is_connected());
    }

    #[test]
    fn test_control_mode_conversion() {
        assert_eq!(ControlMode::from_u8(0), Some(ControlMode::Manual));
        assert_eq!(ControlMode::from_u8(1), Some(ControlMode::Automatic));
        assert_eq!(ControlMode::from_u8(2), Some(ControlMode::Both));
    }

    #[test]
    fn test_output_state_conversion() {
        assert_eq!(OutputState::from_u8(0), Some(OutputState::Disabled));
        assert_eq!(OutputState::from_u8(1), Some(OutputState::Enabled));
        assert!(OutputState::Enabled.is_enabled());
        assert!(!OutputState::Disabled.is_enabled());
    }

    #[test]
    fn test_operation_counts_dlms() {
        let control = RelayControl {
            disconnect_count: 5,
            reconnect_count: 3,
            ..RelayControl::new()
        };

        let dlms = control.operation_counts_dlms();
        if let DlmsType::Structure(items) = dlms {
            assert_eq!(items[0], DlmsType::UInt32(5));
            assert_eq!(items[1], DlmsType::UInt32(3));
        } else {
            panic!("Expected Structure");
        }
    }
}
