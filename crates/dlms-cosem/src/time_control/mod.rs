//!
//! Group 3: Time & Event Control Interface Classes (13 ICs)
//!
//! This module contains interface classes for time management and event control:
//! - IC 8: Clock
//! - IC 9: Script Table
//! - IC 10: Schedule
//! - IC 11: Special Days Table
//! - IC 20: Activity Calendar
//! - IC 21: Register Monitor
//! - IC 22: Single Action Schedule
//! - IC 24: Display
//! - IC 32: Disconnector
//! - IC 70: Disconnect Control
//! - IC 71: Limiter
//! - IC 65: Parameter Monitor
//! - IC 67: Sensor Manager
//! - IC 68: Arbitrator

pub mod ic8_clock;
pub mod ic9_script_table;
pub mod ic10_schedule;
pub mod ic11_special_days_table;
pub mod ic20_activity_calendar;
pub mod ic21_register_monitor;
pub mod ic22_single_action_schedule;
pub mod ic24_display;
pub mod ic32_disconnector;
pub mod ic70_disconnect_control;
pub mod ic71_limiter;
pub mod ic65_parameter_monitor;
pub mod ic67_sensor_manager;
pub mod ic68_arbitrator;

// Re-export commonly used types
pub use ic8_clock::Clock;
pub use ic24_display::Display;
pub use ic32_disconnector::Disconnector;
pub use ic70_disconnect_control::DisconnectControl;
pub use ic71_limiter::Limiter;
