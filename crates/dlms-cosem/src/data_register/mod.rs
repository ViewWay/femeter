//!
//! Group 1: Data & Register Interface Classes (11 ICs)
//!
//! This module contains interface classes for basic data storage and registration:
//! - IC 1: Data - Generic data container
//! - IC 3: Register - Single value with scaler/unit
//! - IC 4: Extended Register - Register with status and capture time
//! - IC 5: Demand Register - Periodic demand measurement
//! - IC 6: Register Activation - Register activation control
//! - IC 7: Profile Generic - Load profile (most complex IC in this group)
//! - IC 26: Utility Tables
//! - IC 61: Register Table
//! - IC 62: Compact Data
//! - IC 63: Status Mapping
//! - IC 66: Measurement Data Monitoring Objects

pub mod ic1_data;
pub mod ic3_register;
pub mod ic4_extended_register;
pub mod ic5_demand_register;
pub mod ic6_register_activation;
pub mod ic7_profile_generic;
pub mod ic26_utility_tables;
pub mod ic61_register_table;
pub mod ic62_compact_data;
pub mod ic63_status_mapping;
pub mod ic66_measurement_data;

// Re-export commonly used types
pub use ic1_data::Data;
pub use ic3_register::Register;
pub use ic4_extended_register::ExtendedRegister;
pub use ic5_demand_register::DemandRegister;
pub use ic6_register_activation::RegisterActivation;
pub use ic7_profile_generic::ProfileGeneric;
