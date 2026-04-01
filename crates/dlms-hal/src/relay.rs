//! Relay HAL trait
//!
//! Provides interface for relay control (load switch).

use crate::HalResult;

/// Relay HAL trait for relay control
///
/// This trait is object-safe and can be used with `dyn RelayHal`.
pub trait RelayHal {
    /// Close relay (connect circuit, enable load)
    fn close(&mut self) -> HalResult<()>;

    /// Open relay (disconnect circuit, disable load)
    fn open(&mut self) -> HalResult<()>;

    /// Check if relay is closed
    fn is_closed(&mut self) -> HalResult<bool>;

    /// Check if relay is open
    fn is_open(&mut self) -> HalResult<bool> {
        self.is_closed().map(|c| !c)
    }

    /// Toggle relay state
    fn toggle(&mut self) -> HalResult<()> {
        let closed = self.is_closed()?;
        if closed {
            self.open()
        } else {
            self.close()
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::HalError;

    struct MockRelay {
        closed: bool,
        initialized: bool,
    }

    impl MockRelay {
        fn new() -> Self {
            Self {
                closed: false,
                initialized: true,
            }
        }

        fn closed() -> Self {
            Self {
                closed: true,
                initialized: true,
            }
        }

        fn uninitialized() -> Self {
            Self {
                closed: false,
                initialized: false,
            }
        }
    }

    impl RelayHal for MockRelay {
        fn close(&mut self) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.closed = true;
            Ok(())
        }

        fn open(&mut self) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.closed = false;
            Ok(())
        }

        fn is_closed(&mut self) -> HalResult<bool> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            Ok(self.closed)
        }
    }

    #[test]
    fn test_relay_close() {
        let mut relay = MockRelay::new();
        assert!(!relay.is_closed().unwrap());

        relay.close().unwrap();
        assert!(relay.is_closed().unwrap());
        assert!(!relay.is_open().unwrap());
    }

    #[test]
    fn test_relay_open() {
        let mut relay = MockRelay::closed();
        assert!(relay.is_closed().unwrap());

        relay.open().unwrap();
        assert!(!relay.is_closed().unwrap());
        assert!(relay.is_open().unwrap());
    }

    #[test]
    fn test_relay_toggle() {
        let mut relay = MockRelay::new();
        assert!(!relay.is_closed().unwrap());

        relay.toggle().unwrap();
        assert!(relay.is_closed().unwrap());

        relay.toggle().unwrap();
        assert!(!relay.is_closed().unwrap());
    }

    #[test]
    fn test_relay_not_initialized() {
        let mut relay = MockRelay::uninitialized();
        assert_eq!(relay.is_closed().unwrap_err(), HalError::NotInitialized);
        assert_eq!(relay.close().unwrap_err(), HalError::NotInitialized);
        assert_eq!(relay.open().unwrap_err(), HalError::NotInitialized);
    }

    #[test]
    fn test_relay_object_safe() {
        let mut relay: std::boxed::Box<dyn RelayHal> = std::boxed::Box::new(MockRelay::new());
        assert!(!relay.is_closed().unwrap());
        relay.close().unwrap();
        assert!(relay.is_closed().unwrap());
    }
}
