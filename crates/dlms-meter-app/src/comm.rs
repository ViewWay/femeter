//! Communication manager for HDLC connection and push notifications
//!
//! This module provides:
//! - HDLC connection management
//! - Push notification setup
//! - Connection status monitoring
//! - Communication event handling

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use alloc::vec;
use dlms_core::{errors::CosemError, obis::ObisCode};

/// Communication port type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PortType {
    /// RS-485 serial port
    Rs485 = 0,
    /// TCP/IP port
    TcpIp = 1,
    /// UDP port
    Udp = 2,
    /// PLC (Power Line Communication)
    Plc = 3,
    /// Wireless (MBUS, Zigbee, etc.)
    Wireless = 4,
}

impl PortType {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Rs485),
            1 => Some(Self::TcpIp),
            2 => Some(Self::Udp),
            3 => Some(Self::Plc),
            4 => Some(Self::Wireless),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectionState {
    /// Not connected
    Disconnected = 0,
    /// Connection in progress
    Connecting = 1,
    /// Connected
    Connected = 2,
    /// Disconnecting
    Disconnecting = 3,
    /// Connection error
    Error = 4,
}

impl ConnectionState {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Disconnected),
            1 => Some(Self::Connecting),
            2 => Some(Self::Connected),
            3 => Some(Self::Disconnecting),
            4 => Some(Self::Error),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Check if connection is active
    pub fn is_active(self) -> bool {
        matches!(self, Self::Connected | Self::Connecting)
    }
}

/// Push notification trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PushTrigger {
    /// Periodic push (time-based)
    Periodic = 0,
    /// Event-based push
    Event = 1,
    /// Threshold exceeded
    Threshold = 2,
    /// Manual push
    Manual = 3,
}

impl PushTrigger {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Periodic),
            1 => Some(Self::Event),
            2 => Some(Self::Threshold),
            3 => Some(Self::Manual),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Communication statistics
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommStats {
    /// Number of bytes sent
    pub bytes_sent: u32,
    /// Number of bytes received
    pub bytes_received: u32,
    /// Number of frames sent
    pub frames_sent: u32,
    /// Number of frames received
    pub frames_received: u32,
    /// Number of frames with errors
    pub frames_error: u32,
    /// Number of connection attempts
    pub connection_attempts: u32,
}

impl CommStats {
    /// Create zero stats
    pub const fn zero() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            frames_sent: 0,
            frames_received: 0,
            frames_error: 0,
            connection_attempts: 0,
        }
    }
}

impl Default for CommStats {
    fn default() -> Self {
        Self::zero()
    }
}

/// Push notification configuration
#[derive(Debug, Clone, PartialEq)]
pub struct PushConfig {
    /// Enable/disable push
    pub enabled: bool,
    /// Push destination address
    pub destination: Vec<u8>,
    /// Push trigger type
    pub trigger: PushTrigger,
    /// Push interval in seconds (for periodic push)
    pub interval_s: u32,
    /// OBIS codes to include in push
    pub objects: Vec<ObisCode>,
}

impl PushConfig {
    /// Create a new push configuration
    pub fn new() -> Self {
        Self {
            enabled: false,
            destination: Vec::new(),
            trigger: PushTrigger::Periodic,
            interval_s: 3600, // 1 hour default
            objects: Vec::new(),
        }
    }

    /// Add an object to push
    pub fn add_object(&mut self, obis: ObisCode) {
        if self.objects.len() < 32 {
            self.objects.push(obis);
        }
    }
}

impl Default for PushConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Communication manager
#[derive(Debug, PartialEq)]
pub struct CommManager {
    /// Communication port type
    port_type: PortType,
    /// Current connection state
    conn_state: ConnectionState,
    /// Push configuration
    push_config: PushConfig,
    /// Communication statistics
    stats: CommStats,
    /// Last push time (seconds since boot)
    last_push_time: u32,
    /// Client address (for HDLC)
    client_address: u16,
    /// Server address (for HDLC)
    server_address: u16,
}

impl CommManager {
    /// Create a new communication manager
    pub fn new(port_type: PortType) -> Self {
        Self {
            port_type,
            conn_state: ConnectionState::Disconnected,
            push_config: PushConfig::new(),
            stats: CommStats::zero(),
            last_push_time: 0,
            client_address: 0x01, // Default client address
            server_address: 0x01, // Default server address
        }
    }

    /// Get port type
    pub fn port_type(&self) -> PortType {
        self.port_type
    }

    /// Get connection state
    pub fn connection_state(&self) -> ConnectionState {
        self.conn_state
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.conn_state == ConnectionState::Connected
    }

    /// Initiate connection
    pub fn connect(&mut self) -> Result<(), CosemError> {
        if self.conn_state.is_active() {
            return Err(CosemError::AccessDenied);
        }

        self.conn_state = ConnectionState::Connecting;
        self.stats.connection_attempts += 1;

        // In real implementation, would initiate physical connection
        // For now, transition directly to connected
        self.conn_state = ConnectionState::Connected;

        Ok(())
    }

    /// Disconnect
    pub fn disconnect(&mut self) -> Result<(), CosemError> {
        if self.conn_state == ConnectionState::Disconnected {
            return Err(CosemError::AccessDenied);
        }

        self.conn_state = ConnectionState::Disconnecting;

        // In real implementation, would close connection
        self.conn_state = ConnectionState::Disconnected;

        Ok(())
    }

    /// Update connection state (called by lower layer)
    pub fn set_connection_state(&mut self, state: ConnectionState) {
        self.conn_state = state;
    }

    /// Get push configuration
    pub fn push_config(&self) -> &PushConfig {
        &self.push_config
    }

    /// Set push configuration
    pub fn set_push_config(&mut self, config: PushConfig) {
        self.push_config = config;
    }

    /// Enable/disable push
    pub fn set_push_enabled(&mut self, enabled: bool) {
        self.push_config.enabled = enabled;
    }

    /// Check if push is enabled
    pub fn push_enabled(&self) -> bool {
        self.push_config.enabled
    }

    /// Add object to push list
    pub fn add_push_object(&mut self, obis: ObisCode) {
        self.push_config.add_object(obis);
    }

    /// Clear push objects
    pub fn clear_push_objects(&mut self) {
        self.push_config.objects.clear();
    }

    /// Get push objects
    pub fn push_objects(&self) -> &[ObisCode] {
        &self.push_config.objects
    }

    /// Set push interval
    pub fn set_push_interval(&mut self, interval_s: u32) {
        self.push_config.interval_s = interval_s;
    }

    /// Get push interval
    pub fn push_interval(&self) -> u32 {
        self.push_config.interval_s
    }

    /// Set push trigger
    pub fn set_push_trigger(&mut self, trigger: PushTrigger) {
        self.push_config.trigger = trigger;
    }

    /// Get push trigger
    pub fn push_trigger(&self) -> PushTrigger {
        self.push_config.trigger
    }

    /// Check if push is needed
    pub fn needs_push(&self, current_time: u32) -> bool {
        if !self.push_config.enabled {
            return false;
        }

        if self.push_config.trigger != PushTrigger::Periodic {
            return false;
        }

        let elapsed = current_time.saturating_sub(self.last_push_time);
        elapsed >= self.push_config.interval_s
    }

    /// Record a push operation
    pub fn record_push(&mut self, current_time: u32, bytes_sent: u32) {
        self.last_push_time = current_time;
        self.stats.bytes_sent += bytes_sent;
        self.stats.frames_sent += 1;
    }

    /// Get communication statistics
    pub fn stats(&self) -> CommStats {
        self.stats
    }

    /// Record received data
    pub fn record_receive(&mut self, bytes: u32) {
        self.stats.bytes_received += bytes;
        self.stats.frames_received += 1;
    }

    /// Record error
    pub fn record_error(&mut self) {
        self.stats.frames_error += 1;
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = CommStats::zero();
    }

    /// Get client address
    pub fn client_address(&self) -> u16 {
        self.client_address
    }

    /// Set client address
    pub fn set_client_address(&mut self, addr: u16) {
        self.client_address = addr;
    }

    /// Get server address
    pub fn server_address(&self) -> u16 {
        self.server_address
    }

    /// Set server address
    pub fn set_server_address(&mut self, addr: u16) {
        self.server_address = addr;
    }
}

impl Default for CommManager {
    fn default() -> Self {
        Self::new(PortType::Rs485)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comm_manager_new() {
        let manager = CommManager::new(PortType::TcpIp);
        assert_eq!(manager.port_type(), PortType::TcpIp);
        assert_eq!(manager.connection_state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_connect() {
        let mut manager = CommManager::new(PortType::Rs485);

        assert!(manager.connect().is_ok());
        assert_eq!(manager.connection_state(), ConnectionState::Connected);
        assert!(manager.is_connected());
    }

    #[test]
    fn test_disconnect() {
        let mut manager = CommManager::new(PortType::Rs485);

        manager.connect().unwrap();
        assert!(manager.disconnect().is_ok());
        assert_eq!(manager.connection_state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_double_connect() {
        let mut manager = CommManager::new(PortType::Rs485);

        manager.connect().unwrap();
        assert!(manager.connect().is_err()); // Already connected
    }

    #[test]
    fn test_push_config() {
        let mut manager = CommManager::new(PortType::Rs485);
        assert!(!manager.push_enabled());

        let config = PushConfig {
            enabled: true,
            destination: vec![0x01, 0x02, 0x03, 0x04],
            trigger: PushTrigger::Event,
            interval_s: 1800,
            objects: vec![],
        };

        manager.set_push_config(config);
        assert!(manager.push_enabled());
        assert_eq!(manager.push_trigger(), PushTrigger::Event);
        assert_eq!(manager.push_interval(), 1800);
    }

    #[test]
    fn test_push_objects() {
        let mut manager = CommManager::new(PortType::Rs485);

        manager.add_push_object(dlms_core::obis::TOTAL_ACTIVE_ENERGY_IMPORT);
        manager.add_push_object(dlms_core::obis::VOLTAGE_L1);

        assert_eq!(manager.push_objects().len(), 2);

        manager.clear_push_objects();
        assert_eq!(manager.push_objects().len(), 0);
    }

    #[test]
    fn test_needs_push() {
        let mut manager = CommManager::new(PortType::Rs485);
        manager.set_push_enabled(true);
        manager.set_push_interval(100); // Use 100 second interval for testing
        manager.set_push_trigger(PushTrigger::Periodic);

        // Initially needs push (last_push_time = 0, current_time = 100)
        assert!(manager.needs_push(100));

        manager.record_push(100, 100);
        assert!(!manager.needs_push(100)); // Just pushed, elapsed = 0

        // After interval
        assert!(manager.needs_push(250)); // elapsed = 150 >= 100
    }

    #[test]
    fn test_stats() {
        let mut manager = CommManager::new(PortType::Rs485);

        manager.record_receive(256);
        manager.record_push(100, 512);

        let stats = manager.stats();
        assert_eq!(stats.bytes_received, 256);
        assert_eq!(stats.bytes_sent, 512);
        assert_eq!(stats.frames_sent, 1);
        assert_eq!(stats.frames_received, 1);
    }

    #[test]
    fn test_connection_state_conversion() {
        assert_eq!(ConnectionState::from_u8(0), Some(ConnectionState::Disconnected));
        assert_eq!(ConnectionState::from_u8(1), Some(ConnectionState::Connecting));
        assert_eq!(ConnectionState::from_u8(2), Some(ConnectionState::Connected));

        assert!(ConnectionState::Connected.is_active());
        assert!(ConnectionState::Connecting.is_active());
        assert!(!ConnectionState::Disconnected.is_active());
    }

    #[test]
    fn test_port_type_conversion() {
        assert_eq!(PortType::from_u8(0), Some(PortType::Rs485));
        assert_eq!(PortType::from_u8(1), Some(PortType::TcpIp));
        assert_eq!(PortType::from_u8(2), Some(PortType::Udp));
        assert_eq!(PortType::from_u8(3), Some(PortType::Plc));
        assert_eq!(PortType::from_u8(4), Some(PortType::Wireless));
    }

    #[test]
    fn test_push_trigger_conversion() {
        assert_eq!(PushTrigger::from_u8(0), Some(PushTrigger::Periodic));
        assert_eq!(PushTrigger::from_u8(1), Some(PushTrigger::Event));
        assert_eq!(PushTrigger::from_u8(2), Some(PushTrigger::Threshold));
        assert_eq!(PushTrigger::from_u8(3), Some(PushTrigger::Manual));
    }

    #[test]
    fn test_addresses() {
        let mut manager = CommManager::new(PortType::Rs485);

        manager.set_client_address(0x1234);
        manager.set_server_address(0x5678);

        assert_eq!(manager.client_address(), 0x1234);
        assert_eq!(manager.server_address(), 0x5678);
    }

    #[test]
    fn test_reset_stats() {
        let mut manager = CommManager::new(PortType::Rs485);

        manager.record_receive(256);
        manager.record_error();

        assert_eq!(manager.stats().frames_error, 1);

        manager.reset_stats();
        assert_eq!(manager.stats().bytes_received, 0);
        assert_eq!(manager.stats().frames_error, 0);
    }
}
