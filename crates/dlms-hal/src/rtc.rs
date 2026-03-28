//! RTC (Real-Time Clock) HAL trait
//!
//! Provides interface for real-time clock functionality.

use crate::HalResult;

/// Date and time representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTime {
    /// Create a new DateTime
    ///
    /// # Arguments
    /// * `year` - Full year (e.g., 2024)
    /// * `month` - Month (1-12)
    /// * `day` - Day (1-31)
    /// * `hour` - Hour (0-23)
    /// * `minute` - Minute (0-59)
    /// * `second` - Second (0-59)
    pub fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// Create a default DateTime (2024-01-01 00:00:00)
    ///
    /// This creates a DateTime at midnight on January 1, 2024.
    pub fn midnight_2024() -> Self {
        Self {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
}

impl Default for DateTime {
    fn default() -> Self {
        Self::midnight_2024()
    }
}

/// RTC HAL trait for real-time clock
///
/// This trait is object-safe and can be used with `dyn RtcHal`.
pub trait RtcHal {
    /// Get current date and time
    fn get_datetime(&mut self) -> HalResult<DateTime>;

    /// Set date and time
    fn set_datetime(&mut self, dt: DateTime) -> HalResult<()>;

    /// Check if RTC is running
    fn is_running(&mut self) -> HalResult<bool> {
        Ok(self.get_datetime().is_ok())
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::HalError;

    struct MockRtc {
        datetime: Option<DateTime>,
        initialized: bool,
    }

    impl MockRtc {
        fn new() -> Self {
            Self {
                datetime: Some(DateTime::default()),
                initialized: true,
            }
        }

        fn with_datetime(dt: DateTime) -> Self {
            Self {
                datetime: Some(dt),
                initialized: true,
            }
        }

        fn uninitialized() -> Self {
            Self {
                datetime: None,
                initialized: false,
            }
        }
    }

    impl RtcHal for MockRtc {
        fn get_datetime(&mut self) -> HalResult<DateTime> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.datetime.ok_or(HalError::HardwareFault)
        }

        fn set_datetime(&mut self, dt: DateTime) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            self.datetime = Some(dt);
            Ok(())
        }
    }

    #[test]
    fn test_rtc_get_datetime() {
        let dt = DateTime::new(2024, 3, 15, 14, 30, 45);
        let mut rtc = MockRtc::with_datetime(dt);

        let result = rtc.get_datetime().unwrap();
        assert_eq!(result.year, 2024);
        assert_eq!(result.month, 3);
        assert_eq!(result.day, 15);
        assert_eq!(result.hour, 14);
        assert_eq!(result.minute, 30);
        assert_eq!(result.second, 45);
    }

    #[test]
    fn test_rtc_set_datetime() {
        let mut rtc = MockRtc::new();
        let dt = DateTime::new(2025, 12, 31, 23, 59, 59);

        rtc.set_datetime(dt).unwrap();
        let result = rtc.get_datetime().unwrap();

        assert_eq!(result, dt);
    }

    #[test]
    fn test_rtc_not_initialized() {
        let mut rtc = MockRtc::uninitialized();
        assert_eq!(
            rtc.get_datetime().unwrap_err(),
            HalError::NotInitialized
        );
        assert_eq!(
            rtc.set_datetime(DateTime::default()).unwrap_err(),
            HalError::NotInitialized
        );
    }

    #[test]
    fn test_rtc_is_running() {
        let mut rtc = MockRtc::new();
        assert!(rtc.is_running().unwrap());

        let mut rtc2 = MockRtc::uninitialized();
        assert!(!rtc2.is_running().unwrap());
    }

    #[test]
    fn test_datetime_default() {
        let dt = DateTime::default();
        assert_eq!(dt.year, 2024);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.minute, 0);
        assert_eq!(dt.second, 0);
    }

    #[test]
    fn test_datetime_equality() {
        let dt1 = DateTime::new(2024, 3, 15, 14, 30, 45);
        let dt2 = DateTime::new(2024, 3, 15, 14, 30, 45);
        let dt3 = DateTime::new(2024, 3, 15, 14, 30, 46);

        assert_eq!(dt1, dt2);
        assert_ne!(dt1, dt3);
    }

    #[test]
    fn test_rtc_object_safe() {
        let mut rtc: std::boxed::Box<dyn RtcHal> = std::boxed::Box::new(MockRtc::new());
        let dt = DateTime::new(2024, 6, 1, 12, 0, 0);
        rtc.set_datetime(dt).unwrap();
        let result = rtc.get_datetime().unwrap();
        assert_eq!(result, dt);
    }
}
