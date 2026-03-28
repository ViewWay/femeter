//!
//! Group 6: M-Bus Interface Classes (6 ICs)
//!
//! This module contains interface classes for M-Bus communication:
//! - IC 76: M-Bus Client
//! - IC 77: M-Bus Master Port Setup
//! - IC 78: M-Bus Slave Port Setup
//! - IC 79: M-Bus Slave Port Data
//! - IC 80: M-Bus Master Data
//! - IC 81: M-Bus Slave Port Identification

pub mod ic76_mbus_client;
pub mod ic77_mbus_master_port_setup;
pub mod ic78_mbus_slave_port_setup;
pub mod ic79_mbus_slave_port_data;
pub mod ic80_mbus_master_data;
pub mod ic81_mbus_slave_port_identification;

// Re-export commonly used types
pub use ic76_mbus_client::MBusClient;
pub use ic77_mbus_master_port_setup::MBusMasterPortSetup;
pub use ic78_mbus_slave_port_setup::MBusSlavePortSetup;
pub use ic79_mbus_slave_port_data::MBusSlavePortData;
pub use ic80_mbus_master_data::MBusMasterData;
pub use ic81_mbus_slave_port_identification::MBusSlavePortIdentification;
