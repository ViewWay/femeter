//! Timer management traits
//!
//! Provides abstraction for software timers (one-shot and periodic).

use core::fmt;

/// Timer mode - one-shot or periodic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum TimerMode {
    /// Fire once and stop
    OneShot,
    /// Fire repeatedly at the specified interval
    Periodic,
}

/// Timer configuration
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct TimerConfig {
    /// Timer mode (one-shot or periodic)
    pub mode: TimerMode,
    /// Period in milliseconds
    pub period_ms: u32,
    /// Auto-start the timer
    pub auto_start: bool,
}

impl TimerConfig {
    /// Create a new one-shot timer config
    pub const fn one_shot(period_ms: u32) -> Self {
        Self {
            mode: TimerMode::OneShot,
            period_ms,
            auto_start: false,
        }
    }

    /// Create a new periodic timer config
    pub const fn periodic(period_ms: u32) -> Self {
        Self {
            mode: TimerMode::Periodic,
            period_ms,
            auto_start: false,
        }
    }

    /// Set auto-start flag
    pub const fn auto_start(mut self) -> Self {
        self.auto_start = true;
        self
    }
}

/// Timer handle returned when creating a timer
///
/// This handle allows control over the created timer.
pub trait TimerHandle: Send + Sync {
    /// Start the timer
    ///
    /// If the timer is already running, it will be reset.
    fn start(&self);

    /// Stop the timer
    ///
    /// Returns true if the timer was running.
    fn stop(&self) -> bool;

    /// Check if the timer is currently running
    fn is_running(&self) -> bool;

    /// Get the timer's period in milliseconds
    fn period(&self) -> u32;

    /// Get the timer mode
    fn mode(&self) -> TimerMode;

    /// Get remaining time until next expiration (in milliseconds)
    ///
    /// Returns None if the timer is not running or not supported.
    fn remaining(&self) -> Option<u32>;
}

/// Timer callback type
pub type TimerCallback = fn();

/// Timer-related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum TimerError {
    /// Maximum number of timers reached
    TimerLimit,
    /// Insufficient memory to create timer
    OutOfMemory,
    /// Invalid timer configuration
    InvalidConfig,
    /// Timer not found
    NotFound,
    /// Operation not allowed from ISR
    NotFromIsr,
}

#[cfg(feature = "std")]
impl fmt::Display for TimerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimerLimit => write!(f, "Timer limit reached"),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::InvalidConfig => write!(f, "Invalid configuration"),
            Self::NotFound => write!(f, "Timer not found"),
            Self::NotFromIsr => write!(f, "Operation not allowed from ISR"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TimerError {}

/// RTOS timer management trait
///
/// Abstracts software timer creation and control. Timers execute callbacks
/// either once (one-shot) or repeatedly (periodic).
///
/// # Example
///
/// ```ignore
/// fn timer_callback() {
///     // Handle timer expiration
/// }
///
/// let config = TimerConfig::periodic(1000).auto_start();
/// let handle = rtos.create_timer(timer_callback, config)?;
/// ```
pub trait RtosTimer: Sized {
    /// Timer handle type
    type Handle: TimerHandle;

    /// Create a new timer
    ///
    /// # Errors
    /// Returns an error if:
    /// - Maximum timer count reached
    /// - Insufficient memory
    /// - Invalid configuration
    fn create_timer(
        &self,
        callback: TimerCallback,
        config: TimerConfig,
    ) -> Result<Self::Handle, TimerError>;

    /// Get the number of active timers
    fn timer_count(&self) -> usize;

    /// Tick all timers (called by system tick handler)
    ///
    /// This method is called internally by the RTOS tick handler.
    /// Real RTOS implementations will call this from their tick ISR.
    #[doc(hidden)]
    fn tick_timers(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_config() {
        let config = TimerConfig::one_shot(1000);
        assert_eq!(config.mode, TimerMode::OneShot);
        assert_eq!(config.period_ms, 1000);
        assert!(!config.auto_start);

        let config = TimerConfig::periodic(500).auto_start();
        assert_eq!(config.mode, TimerMode::Periodic);
        assert_eq!(config.period_ms, 500);
        assert!(config.auto_start);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_timer_error_display() {
        assert_eq!(alloc::format!("{}", TimerError::TimerLimit), "Timer limit reached");
        assert_eq!(alloc::format!("{}", TimerError::NotFound), "Timer not found");
    }
}
