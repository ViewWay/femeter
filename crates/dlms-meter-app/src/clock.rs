//! Clock manager for RTC synchronization and time management
//!
//! This module provides:
//! - Real-time clock management
//! - Timezone handling
//! - Daylight Saving Time (DST) support
//! - Time synchronization from network

extern crate alloc;

use dlms_core::{errors::CosemError, types::CosemDateTime, types::clock_status};

/// DST (Daylight Saving Time) mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DstMode {
    /// DST not active
    Standard = 0,
    /// DST active
    Daylight = 1,
}

impl DstMode {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Standard),
            1 => Some(Self::Daylight),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Timezone information
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timezone {
    /// Offset from UTC in minutes (-720 to +720)
    pub offset_minutes: i16,
    /// DST mode
    pub dst_mode: DstMode,
}

impl Timezone {
    /// Create a new timezone
    pub fn new(offset_minutes: i16, dst_mode: DstMode) -> Self {
        Self {
            offset_minutes: offset_minutes.clamp(-720, 720),
            dst_mode,
        }
    }

    /// Create UTC timezone
    pub const fn utc() -> Self {
        Self {
            offset_minutes: 0,
            dst_mode: DstMode::Standard,
        }
    }

    /// Get timezone offset in minutes
    pub fn offset_minutes(&self) -> i16 {
        self.offset_minutes
    }

    /// Get timezone offset for DST (adds 60 minutes if DST active)
    pub fn offset_with_dst(&self) -> i16 {
        if self.dst_mode == DstMode::Daylight {
            self.offset_minutes + 60
        } else {
            self.offset_minutes
        }
    }

    /// Check if DST is active
    pub fn is_dst_active(&self) -> bool {
        self.dst_mode == DstMode::Daylight
    }
}

/// Clock sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SyncStatus {
    /// Never synchronized
    Never = 0,
    /// Sync in progress
    InProgress = 1,
    /// Successfully synchronized
    Synced = 2,
    /// Sync failed
    Failed = 3,
    /// Sync invalid (drift detected)
    Invalid = 4,
}

impl SyncStatus {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Never),
            1 => Some(Self::InProgress),
            2 => Some(Self::Synced),
            3 => Some(Self::Failed),
            4 => Some(Self::Invalid),
            _ => None,
        }
    }

    /// Get numeric code
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Check if clock is considered synchronized
    pub fn is_synced(self) -> bool {
        matches!(self, Self::Synced)
    }
}

/// Clock statistics
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockStats {
    /// Last successful sync time (seconds since boot)
    pub last_sync_time: u32,
    /// Number of successful syncs
    pub sync_count: u32,
    /// Number of failed sync attempts
    pub fail_count: u32,
    /// Current drift in seconds (positive = fast, negative = slow)
    pub drift_seconds: i32,
}

impl ClockStats {
    /// Create zero stats
    pub const fn zero() -> Self {
        Self {
            last_sync_time: 0,
            sync_count: 0,
            fail_count: 0,
            drift_seconds: 0,
        }
    }
}

impl Default for ClockStats {
    fn default() -> Self {
        Self::zero()
    }
}

/// Clock manager for RTC management
#[derive(Debug, PartialEq)]
pub struct ClockManager {
    /// Current date-time
    current_time: CosemDateTime,
    /// Timezone configuration
    timezone: Timezone,
    /// Sync status
    sync_status: SyncStatus,
    /// Clock statistics
    stats: ClockStats,
    /// Base time reference (for monotonic time)
    base_time: u32,
    /// Sync interval in seconds (default 1 hour)
    sync_interval_s: u32,
    /// Last sync attempt time
    last_sync_attempt: u32,
}

impl ClockManager {
    /// Create a new clock manager with default time
    pub fn new() -> Self {
        Self {
            current_time: CosemDateTime {
                date: dlms_core::types::CosemDate {
                    year: 2024,
                    month: 1,
                    day: 1,
                    day_of_week: 1,
                },
                time: dlms_core::types::CosemTime {
                    hour: 0,
                    minute: 0,
                    second: 0,
                    hundredths: 0,
                },
                deviation: 0,
                clock_status: clock_status::INVALID,
            },
            timezone: Timezone::utc(),
            sync_status: SyncStatus::Never,
            stats: ClockStats::zero(),
            base_time: 0,
            sync_interval_s: 3600, // 1 hour
            last_sync_attempt: 0,
        }
    }

    /// Get current time
    pub fn current_time(&self) -> &CosemDateTime {
        &self.current_time
    }

    /// Set current time (manual setting)
    pub fn set_time(&mut self, dt: CosemDateTime) -> Result<(), CosemError> {
        // Validate date-time fields
        if dt.date.month == 0 || dt.date.month > 12 {
            return Err(CosemError::InvalidParameter);
        }
        if dt.date.day == 0 || dt.date.day > 31 {
            return Err(CosemError::InvalidParameter);
        }
        if dt.time.hour > 23 || dt.time.minute > 59 || dt.time.second > 59 {
            return Err(CosemError::InvalidParameter);
        }

        self.current_time = dt;
        // Clear invalid status when manually set
        self.current_time.clock_status &= !clock_status::INVALID;
        Ok(())
    }

    /// Get timezone
    pub fn timezone(&self) -> Timezone {
        self.timezone
    }

    /// Set timezone
    pub fn set_timezone(&mut self, tz: Timezone) {
        self.timezone = tz;
        self.current_time.deviation = tz.offset_with_dst();
        if tz.is_dst_active() {
            self.current_time.clock_status |= clock_status::DST_ACTIVE;
        } else {
            self.current_time.clock_status &= !clock_status::DST_ACTIVE;
        }
    }

    /// Get sync status
    pub fn sync_status(&self) -> SyncStatus {
        self.sync_status
    }

    /// Check if clock is synchronized
    pub fn is_synced(&self) -> bool {
        self.sync_status.is_synced()
    }

    /// Get clock statistics
    pub fn stats(&self) -> ClockStats {
        self.stats
    }

    /// Get sync interval
    pub fn sync_interval(&self) -> u32 {
        self.sync_interval_s
    }

    /// Set sync interval
    pub fn set_sync_interval(&mut self, interval_s: u32) {
        self.sync_interval_s = interval_s;
    }

    /// Synchronize clock from external source
    pub fn sync(&mut self, dt: CosemDateTime, current_monotonic: u32) -> Result<(), CosemError> {
        self.last_sync_attempt = current_monotonic;
        self.sync_status = SyncStatus::InProgress;

        // Validate received time
        if dt.date.year < 2000 || dt.date.year > 2100 {
            self.sync_status = SyncStatus::Failed;
            self.stats.fail_count += 1;
            return Err(CosemError::InvalidParameter);
        }

        // Calculate drift
        let old_time = self.time_to_seconds(&self.current_time);
        let new_time = self.time_to_seconds(&dt);
        self.stats.drift_seconds = new_time.saturating_sub(old_time) as i32;

        // Update time
        self.current_time = dt;
        self.current_time.clock_status &= !clock_status::INVALID;

        // Mark as synced
        self.sync_status = SyncStatus::Synced;
        self.stats.sync_count += 1;
        self.stats.last_sync_time = current_monotonic;

        Ok(())
    }

    /// Check if sync is needed
    pub fn needs_sync(&self, current_monotonic: u32) -> bool {
        if self.sync_status == SyncStatus::Never {
            return true;
        }

        let elapsed = current_monotonic.saturating_sub(self.last_sync_attempt);
        elapsed >= self.sync_interval_s
    }

    /// Update monotonic time (advance clock)
    pub fn tick(&mut self, elapsed_seconds: u32) {
        self.add_seconds(elapsed_seconds as i32);
    }

    /// Add seconds to current time
    pub fn add_seconds(&mut self, seconds: i32) {
        let total = self.time_to_seconds(&self.current_time) as i64 + seconds as i64;
        // Create new time rather than borrowing self.current_time mutably
        let mut new_time = self.current_time;
        self.seconds_to_time(total as u32, &mut new_time);
        self.current_time = new_time;
    }

    /// Convert CosemDateTime to seconds since epoch
    fn time_to_seconds(&self, dt: &CosemDateTime) -> u32 {
        // Simplified conversion - just hours/minutes/seconds
        dt.time.hour as u32 * 3600 + dt.time.minute as u32 * 60 + dt.time.second as u32
    }

    /// Convert seconds to CosemDateTime (time portion only)
    fn seconds_to_time(&self, seconds: u32, dt: &mut CosemDateTime) {
        let secs = seconds % 86400;
        dt.time.hour = (secs / 3600) as u8;
        dt.time.minute = ((secs % 3600) / 60) as u8;
        dt.time.second = (secs % 60) as u8;
    }

    /// Get time as string (for display)
    #[cfg(feature = "std")]
    pub fn time_string(&self) -> alloc::string::String {
        use alloc::format;
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.current_time.date.year,
            self.current_time.date.month,
            self.current_time.date.day,
            self.current_time.time.hour,
            self.current_time.time.minute,
            self.current_time.time.second
        )
    }

    /// Enable/disable DST
    pub fn set_dst(&mut self, enabled: bool) {
        self.timezone.dst_mode = if enabled {
            DstMode::Daylight
        } else {
            DstMode::Standard
        };
        self.current_time.deviation = self.timezone.offset_with_dst();
        if enabled {
            self.current_time.clock_status |= clock_status::DST_ACTIVE;
        } else {
            self.current_time.clock_status &= !clock_status::DST_ACTIVE;
        }
    }

    /// Get current deviation (timezone offset)
    pub fn deviation(&self) -> i16 {
        self.current_time.deviation
    }

    /// Check if DST is active
    pub fn is_dst_active(&self) -> bool {
        (self.current_time.clock_status & clock_status::DST_ACTIVE) != 0
    }

    /// Reset sync statistics
    pub fn reset_stats(&mut self) {
        self.stats = ClockStats::zero();
    }
}

impl Default for ClockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlms_core::types::{CosemDate, CosemTime};

    fn make_datetime(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> CosemDateTime {
        CosemDateTime {
            date: CosemDate {
                year,
                month,
                day,
                day_of_week: 1,
            },
            time: CosemTime {
                hour,
                minute,
                second,
                hundredths: 0,
            },
            deviation: 0,
            clock_status: 0,
        }
    }

    #[test]
    fn test_clock_manager_new() {
        let manager = ClockManager::new();
        assert_eq!(manager.sync_status(), SyncStatus::Never);
        assert_eq!(manager.current_time.date.year, 2024);
    }

    #[test]
    fn test_set_time() {
        let mut manager = ClockManager::new();
        let dt = make_datetime(2024, 6, 15, 12, 30, 45);

        assert!(manager.set_time(dt).is_ok());
        assert_eq!(manager.current_time.time.hour, 12);
        assert_eq!(manager.current_time.time.minute, 30);
    }

    #[test]
    fn test_set_invalid_time() {
        let mut manager = ClockManager::new();

        // Invalid month
        let dt = make_datetime(2024, 13, 15, 12, 0, 0);
        assert!(manager.set_time(dt).is_err());

        // Invalid hour
        let dt = make_datetime(2024, 6, 15, 25, 0, 0);
        assert!(manager.set_time(dt).is_err());
    }

    #[test]
    fn test_timezone() {
        let tz = Timezone::new(60, DstMode::Standard); // UTC+1
        assert_eq!(tz.offset_minutes(), 60);
        assert_eq!(tz.offset_with_dst(), 60);
        assert!(!tz.is_dst_active());

        let tz_dst = Timezone::new(60, DstMode::Daylight); // UTC+1 + DST = UTC+2
        assert_eq!(tz_dst.offset_with_dst(), 120);
        assert!(tz_dst.is_dst_active());
    }

    #[test]
    fn test_set_timezone() {
        let mut manager = ClockManager::new();
        let tz = Timezone::new(-300, DstMode::Standard); // UTC-5 (EST)

        manager.set_timezone(tz);
        assert_eq!(manager.deviation(), -300);
        assert!(!manager.is_dst_active());
    }

    #[test]
    fn test_dst() {
        let mut manager = ClockManager::new();

        manager.set_dst(true);
        assert!(manager.is_dst_active());
        assert_eq!(manager.current_time.clock_status & clock_status::DST_ACTIVE, clock_status::DST_ACTIVE);

        manager.set_dst(false);
        assert!(!manager.is_dst_active());
    }

    #[test]
    fn test_sync() {
        let mut manager = ClockManager::new();
        let dt = make_datetime(2024, 6, 15, 12, 0, 0);

        assert!(manager.sync(dt, 100).is_ok());
        assert_eq!(manager.sync_status(), SyncStatus::Synced);
        assert_eq!(manager.stats().sync_count, 1);
        assert!(manager.is_synced());
    }

    #[test]
    fn test_sync_invalid_year() {
        let mut manager = ClockManager::new();
        let dt = make_datetime(1999, 6, 15, 12, 0, 0);

        assert!(manager.sync(dt, 100).is_err());
        assert_eq!(manager.sync_status(), SyncStatus::Failed);
        assert_eq!(manager.stats().fail_count, 1);
    }

    #[test]
    fn test_needs_sync() {
        let mut manager = ClockManager::new();
        manager.set_sync_interval(3600);

        // Never synced - needs sync
        assert!(manager.needs_sync(0));

        // Sync and check
        manager.sync(make_datetime(2024, 6, 15, 12, 0, 0), 100).unwrap();
        assert!(!manager.needs_sync(100)); // Just synced
        assert!(!manager.needs_sync(3000)); // Within interval
        assert!(manager.needs_sync(4000)); // After interval
    }

    #[test]
    fn test_tick() {
        let mut manager = ClockManager::new();
        manager.set_time(make_datetime(2024, 6, 15, 12, 0, 0)).unwrap();

        manager.tick(90); // Add 90 seconds
        assert_eq!(manager.current_time.time.minute, 1);
        assert_eq!(manager.current_time.time.second, 30);
    }

    #[test]
    fn test_sync_status_conversion() {
        assert_eq!(SyncStatus::from_u8(0), Some(SyncStatus::Never));
        assert_eq!(SyncStatus::from_u8(1), Some(SyncStatus::InProgress));
        assert_eq!(SyncStatus::from_u8(2), Some(SyncStatus::Synced));
        assert_eq!(SyncStatus::from_u8(3), Some(SyncStatus::Failed));
        assert_eq!(SyncStatus::from_u8(4), Some(SyncStatus::Invalid));

        assert!(SyncStatus::Synced.is_synced());
        assert!(!SyncStatus::Failed.is_synced());
    }

    #[test]
    fn test_dst_mode_conversion() {
        assert_eq!(DstMode::from_u8(0), Some(DstMode::Standard));
        assert_eq!(DstMode::from_u8(1), Some(DstMode::Daylight));
        assert_eq!(DstMode::Standard.code(), 0);
        assert_eq!(DstMode::Daylight.code(), 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut manager = ClockManager::new();
        let dt = make_datetime(2024, 6, 15, 12, 0, 0);

        manager.sync(dt.clone(), 100).unwrap();
        manager.sync(dt, 200).unwrap();
        assert_eq!(manager.stats().sync_count, 2);

        manager.reset_stats();
        assert_eq!(manager.stats().sync_count, 0);
    }
}
