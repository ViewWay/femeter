//! # DLMS Hardware Abstraction Layer
//!
//! This crate provides hardware abstraction traits for DLMS/COSEM smart meter peripherals.
//! It follows embedded-hal patterns and adds DLMS-specific peripherals (Relay, Display, Modem).
//!
//! ## Features
//!
//! - **std**: Enables mock implementations for testing
//! - **stm32f4**: Enables STM32F4 register definitions (stub)
//! - **defmt-log**: Enables defmt formatting support
//!
//! ## Design Philosophy
//!
//! All traits are object-safe, allowing usage with `dyn` references for flexible hardware
//! abstraction in meter applications.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

// Modules for each peripheral
mod adc;
mod display;
mod flash;
mod gpio;
mod i2c;
mod modem;
mod relay;
mod rtc;
mod spi;
mod uart;
mod watchdog;

// Re-export all traits
pub use adc::AdcHal;
pub use display::DisplayHal;
pub use flash::FlashHal;
pub use gpio::{GpioDirection, GpioHal};
pub use i2c::I2cHal;
pub use modem::ModemHal;
pub use relay::RelayHal;
pub use rtc::{DateTime, RtcHal};
pub use spi::SpiHal;
pub use uart::UartHal;
pub use watchdog::WatchdogHal;

pub use error::{HalError, HalResult};

// Core error type
mod error {
    use core::fmt;

    /// Hardware Abstraction Layer error type
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
    pub enum HalError {
        /// Peripheral not initialized
        NotInitialized,
        /// Invalid parameter provided
        InvalidParam,
        /// Operation timed out
        Timeout,
        /// Hardware fault detected
        HardwareFault,
        /// Feature not implemented
        NotImplemented,
    }

    pub type HalResult<T> = Result<T, HalError>;

    #[cfg(feature = "std")]
    impl fmt::Display for HalError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::NotInitialized => write!(f, "Peripheral not initialized"),
                Self::InvalidParam => write!(f, "Invalid parameter"),
                Self::Timeout => write!(f, "Operation timed out"),
                Self::HardwareFault => write!(f, "Hardware fault"),
                Self::NotImplemented => write!(f, "Feature not implemented"),
            }
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for HalError {}
}

/// Combined HAL provider trait
///
/// This trait combines all peripheral traits into a single interface
/// for convenient access to all hardware resources.
pub trait HalProvider:
    AdcHal
    + GpioHal
    + UartHal
    + SpiHal
    + I2cHal
    + RtcHal
    + FlashHal
    + DisplayHal
    + RelayHal
    + ModemHal
    + WatchdogHal
{
}

// Blanket implementation for any type that implements all traits
impl<T> HalProvider for T where
    T: AdcHal
        + GpioHal
        + UartHal
        + SpiHal
        + I2cHal
        + RtcHal
        + FlashHal
        + DisplayHal
        + RelayHal
        + ModemHal
        + WatchdogHal
{
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    // ===== HalError Tests =====

    #[test]
    fn test_hal_error_variants_exist() {
        // Test that all error variants can be created
        let _ = HalError::NotInitialized;
        let _ = HalError::InvalidParam;
        let _ = HalError::Timeout;
        let _ = HalError::HardwareFault;
        let _ = HalError::NotImplemented;
    }

    #[test]
    fn test_hal_error_equality() {
        assert_eq!(HalError::NotInitialized, HalError::NotInitialized);
        assert_eq!(HalError::InvalidParam, HalError::InvalidParam);
        assert_ne!(HalError::Timeout, HalError::HardwareFault);
    }

    #[test]
    fn test_hal_result_type() {
        let ok_result: HalResult<u8> = Ok(42);
        let err_result: HalResult<u8> = Err(HalError::InvalidParam);

        assert!(ok_result.is_ok());
        assert!(err_result.is_err());
        assert_eq!(ok_result.unwrap(), 42);
        assert_eq!(err_result.unwrap_err(), HalError::InvalidParam);
    }

    #[test]
    fn test_hal_error_display() {
        use std::format;

        assert_eq!(
            format!("{}", HalError::NotInitialized),
            "Peripheral not initialized"
        );
        assert_eq!(format!("{}", HalError::InvalidParam), "Invalid parameter");
        assert_eq!(format!("{}", HalError::Timeout), "Operation timed out");
        assert_eq!(format!("{}", HalError::HardwareFault), "Hardware fault");
        assert_eq!(
            format!("{}", HalError::NotImplemented),
            "Feature not implemented"
        );
    }
}
