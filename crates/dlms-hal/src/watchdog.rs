//! Watchdog HAL trait
//!
//! Provides interface for watchdog timer control.

use crate::{HalError, HalResult};

/// Watchdog HAL trait for watchdog timer control
///
/// This trait is object-safe and can be used with `dyn WatchdogHal`.
pub trait WatchdogHal {
    /// Feed (pet) the watchdog to reset the timer
    fn feed(&mut self) -> HalResult<()>;

    /// Start the watchdog timer
    ///
    /// # Arguments
    /// * `timeout_ms` - Timeout in milliseconds
    fn start(&mut self, timeout_ms: u32) -> HalResult<()>;

    /// Stop the watchdog timer
    ///
    /// # Note
    /// Many watchdogs cannot be stopped once started
    fn stop(&mut self) -> HalResult<()> {
        Err(HalError::NotImplemented)
    }

    /// Check if watchdog is running
    fn is_running(&mut self) -> HalResult<bool>;

    /// Get the current timeout value
    fn get_timeout(&mut self) -> HalResult<u32>;
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::HalError;

    struct MockWatchdog {
        running: bool,
        timeout_ms: u32,
        feed_count: usize,
    }

    impl MockWatchdog {
        fn new() -> Self {
            Self {
                running: false,
                timeout_ms: 0,
                feed_count: 0,
            }
        }
    }

    impl WatchdogHal for MockWatchdog {
        fn feed(&mut self) -> HalResult<()> {
            if !self.running {
                return Err(HalError::NotInitialized);
            }
            self.feed_count += 1;
            Ok(())
        }

        fn start(&mut self, timeout_ms: u32) -> HalResult<()> {
            if timeout_ms == 0 {
                return Err(HalError::InvalidParam);
            }
            self.running = true;
            self.timeout_ms = timeout_ms;
            Ok(())
        }

        fn stop(&mut self) -> HalResult<()> {
            self.running = false;
            Ok(())
        }

        fn is_running(&mut self) -> HalResult<bool> {
            Ok(self.running)
        }

        fn get_timeout(&mut self) -> HalResult<u32> {
            if !self.running {
                return Err(HalError::NotInitialized);
            }
            Ok(self.timeout_ms)
        }
    }

    #[test]
    fn test_watchdog_start() {
        let mut wdt = MockWatchdog::new();
        assert!(!wdt.is_running().unwrap());

        wdt.start(1000).unwrap();
        assert!(wdt.is_running().unwrap());
        assert_eq!(wdt.get_timeout().unwrap(), 1000);
    }

    #[test]
    fn test_watchdog_stop() {
        let mut wdt = MockWatchdog::new();
        wdt.start(1000).unwrap();
        assert!(wdt.is_running().unwrap());

        wdt.stop().unwrap();
        assert!(!wdt.is_running().unwrap());
    }

    #[test]
    fn test_watchdog_feed() {
        let mut wdt = MockWatchdog::new();
        wdt.start(1000).unwrap();

        wdt.feed().unwrap();
        wdt.feed().unwrap();
        wdt.feed().unwrap();
    }

    #[test]
    fn test_watchdog_feed_without_start() {
        let mut wdt = MockWatchdog::new();
        assert_eq!(wdt.feed().unwrap_err(), HalError::NotInitialized);
    }

    #[test]
    fn test_watchdog_invalid_timeout() {
        let mut wdt = MockWatchdog::new();
        assert_eq!(wdt.start(0).unwrap_err(), HalError::InvalidParam);
    }

    #[test]
    fn test_watchdog_get_timeout_without_start() {
        let mut wdt = MockWatchdog::new();
        assert_eq!(wdt.get_timeout().unwrap_err(), HalError::NotInitialized);
    }

    #[test]
    fn test_watchdog_multiple_timeouts() {
        let mut wdt = MockWatchdog::new();

        wdt.start(100).unwrap();
        assert_eq!(wdt.get_timeout().unwrap(), 100);

        wdt.stop().unwrap();
        wdt.start(1000).unwrap();
        assert_eq!(wdt.get_timeout().unwrap(), 1000);
    }

    #[test]
    fn test_watchdog_object_safe() {
        let mut wdt: std::boxed::Box<dyn WatchdogHal> = std::boxed::Box::new(MockWatchdog::new());
        wdt.start(500).unwrap();
        assert!(wdt.is_running().unwrap());
        wdt.feed().unwrap();
    }
}
