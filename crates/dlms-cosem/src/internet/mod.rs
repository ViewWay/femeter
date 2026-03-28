//!
//! Group 7: Internet Communication Interface Classes (9 ICs)
//!
//! This module contains interface classes for IP-based communication:
//! - IC 47: TCP-UDP Setup
//! - IC 48: IPv4 Setup
//! - IC 57: PPP Setup
//! - IC 58: GPRS Setup
//! - IC 59: SMS Setup
//! - IC 64: IPv6 Setup

pub mod ic47_tcp_udp_setup;
pub mod ic48_ipv4_setup;
pub mod ic57_ppp_setup;
pub mod ic58_gprs_setup;
pub mod ic59_sms_setup;
pub mod ic64_ipv6_setup;

// Re-export commonly used types
pub use ic47_tcp_udp_setup::TcpUdpSetup;
pub use ic48_ipv4_setup::IPv4Setup;
pub use ic57_ppp_setup::PppSetup;
pub use ic58_gprs_setup::GprsSetup;
pub use ic59_sms_setup::SmsSetup;
pub use ic64_ipv6_setup::IPv6Setup;
