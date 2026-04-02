//!
//! Group 2: Management Interface Classes (13 ICs)
//!
//! This module contains interface classes for COSEM object management:
//! - IC 12: Association SN - Short name association
//! - IC 15: Association LN - Logical name association (MOST IMPORTANT)
//! - IC 17: SAP Assignment
//! - IC 18: Image Transfer
//! - IC 30: Data Protection
//! - IC 40: Push Setup
//! - IC 43: COSEM Logical Device
//! - IC 44: COSEM Physical Device
//! - IC 62: Security Policy
//! - IC 64: Security Setup
//! - IC 122: Function Control
//! - IC 123: Array Manager
//! - IC 124: Communication Port Protection

pub mod ic02_association_sn_secure;
pub mod ic122_function_control;
pub mod ic123_array_manager;
pub mod ic124_comm_port_protection;
pub mod ic12_association_sn;
pub mod ic15_association_ln;
pub mod ic17_sap_assignment;
pub mod ic18_image_transfer;
pub mod ic30_data_protection;
pub mod ic40_push_setup;
pub mod ic43_logical_device;
pub mod ic44_physical_device;
pub mod ic62_security_policy;
pub mod ic64_security_setup;

// Re-export commonly used types
pub use ic02_association_sn_secure::AssociationSnSecure;
pub use ic15_association_ln::AssociationLn;
pub use ic43_logical_device::LogicalDevice;
pub use ic44_physical_device::PhysicalDevice;
pub use ic62_security_policy::SecurityPolicy;
pub use ic64_security_setup::SecuritySetup;
