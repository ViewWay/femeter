//! COSEM date/time types
//!
//! Re-exported from types.rs for convenience.

pub use crate::types::{clock_status, CosemDate, CosemDateTime, CosemTime, ScalerUnit};

/// Not-specified sentinel values
pub const YEAR_NOT_SPECIFIED: u16 = 0xFFFF;
pub const MONTH_NOT_SPECIFIED: u8 = 0xFF;
pub const DAY_NOT_SPECIFIED: u8 = 0xFF;
pub const DAY_OF_WEEK_NOT_SPECIFIED: u8 = 0xFF;
pub const HOUR_NOT_SPECIFIED: u8 = 0xFF;
pub const MINUTE_NOT_SPECIFIED: u8 = 0xFF;
pub const SECOND_NOT_SPECIFIED: u8 = 0xFF;
pub const HUNDREDTHS_NOT_SPECIFIED: u8 = 0xFF;
pub const DEVIATION_NOT_SPECIFIED: i16 = -32768;

/// Special date values
pub const MONTH_DST_END: u8 = 0xFD;
pub const MONTH_DST_BEGIN: u8 = 0xFE;
pub const DAY_SECOND_LAST: u8 = 0xFD;
pub const DAY_LAST: u8 = 0xFE;

impl CosemDate {
    /// Create a date with all fields not specified
    pub fn unspecified() -> Self {
        Self {
            year: YEAR_NOT_SPECIFIED,
            month: MONTH_NOT_SPECIFIED,
            day: DAY_NOT_SPECIFIED,
            day_of_week: DAY_OF_WEEK_NOT_SPECIFIED,
        }
    }

    /// Check if year is specified
    pub fn is_year_specified(&self) -> bool {
        self.year != YEAR_NOT_SPECIFIED
    }
}

impl CosemTime {
    /// Create a time with all fields not specified
    pub fn unspecified() -> Self {
        Self {
            hour: HOUR_NOT_SPECIFIED,
            minute: MINUTE_NOT_SPECIFIED,
            second: SECOND_NOT_SPECIFIED,
            hundredths: HUNDREDTHS_NOT_SPECIFIED,
        }
    }
}

impl CosemDateTime {
    /// Create a datetime with all fields not specified
    pub fn unspecified() -> Self {
        Self {
            date: CosemDate::unspecified(),
            time: CosemTime::unspecified(),
            deviation: DEVIATION_NOT_SPECIFIED,
            clock_status: 0,
        }
    }

    /// Check if deviation is specified
    pub fn is_deviation_specified(&self) -> bool {
        self.deviation != DEVIATION_NOT_SPECIFIED
    }
}
