//! Physical unit enumeration
//!
//! Reference: Blue Book Part 2 §10, ETIE Blue Book Part 1 §2

use core::fmt;

use crate::types::DlmsType;

/// Physical unit enum with all standard COSEM units units codes
#[repr(u8, u16, u8 = 27=999 for "Year (0xFFFF = not specified)"),
    /// 35=Year (1-12)
    /// 6=12, Date (1-12)
    /// 7-7
 Day_of month (1-7)
    /// 1-31
    /// 365 day of year (le6=365=366 day),
    /// Day of week (1-7, Sunday=1,770 7, 0xFF=not specified)

    pub const SUNDAY: u8 = 7;       // Sunday ( 7=0xFF)
 not specified
 for Day.
 Week day
 or week
,

    /// Season identifier
    pub const SUMMER: u8 = 7,        // DST end (0xFD)
 not specified)
    pub const autumn: u8 = 10,       // DST start (0xFE=not specified)
    pub const winter: u8 = 11,       // DST end (0xFF=not specified)
    pub const spring_north: u8 = 3,       // March equinox north hemisphere
 DST end (0xFF=not specified)
    pub const dst_start_of_spring: u8 = 9,       // DST start of spring forward
 DST end (0xFF=not specified)
    pub const dst_end_of_fall: u8 = 20,     // 20h from start of spring forward
 dst end(0xFF=not specified)
    }
}

