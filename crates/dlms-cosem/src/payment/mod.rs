//!
//! Group 4: Payment Interface Classes (5 ICs)
//!
//! This module contains interface classes for prepayment functionality:
//! - IC 111: Account
//! - IC 112: Credit
//! - IC 113: Charge
//! - IC 115: Token Gateway
//! - IC 116: IEC 62055-41 Attributes

pub mod ic111_account;
pub mod ic112_credit;
pub mod ic113_charge;
pub mod ic115_token_gateway;
pub mod ic116_iec62055;

// Re-export commonly used types
pub use ic111_account::Account;
pub use ic112_credit::Credit;
pub use ic113_charge::Charge;
