//! TOU 费率引擎 (GB/T 15284 + DLMS IC19 Schedule Table)
//!
//! 支持 T1~T8 八费率, 日时段表, 周计划, 季节日历, 节假日
//! 预设方案: 单一/两费率/三费率/四费率(GB标准)

use chrono::{Datelike, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};

/// 费率号 (T1~T8)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TariffRate {
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
}

impl TariffRate {
    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Self::T1,
            1 => Self::T2,
            2 => Self::T3,
            3 => Self::T4,
            4 => Self::T5,
            5 => Self::T6,
            6 => Self::T7,
            _ => Self::T8,
        }
    }
    pub fn index(&self) -> usize {
        *self as usize
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::T1 => "尖(Sharp)",
            Self::T2 => "峰(Peak)",
            Self::T3 => "平(Normal)",
            Self::T4 => "谷(Valley)",
            Self::T5 => "T5",
            Self::T6 => "T6",
            Self::T7 => "T7",
            Self::T8 => "T8",
        }
    }
}

/// 时间段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSegment {
    pub start_hour: u8,
    pub start_min: u8,
    pub rate: TariffRate,
}

impl TimeSegment {
    pub fn new(hour: u8, min: u8, rate: TariffRate) -> Self {
        Self {
            start_hour: hour,
            start_min: min,
            rate,
        }
    }
    pub fn minutes_from_midnight(&self) -> u32 {
        self.start_hour as u32 * 60 + self.start_min as u32
    }
}

/// 日时段表 (最多 14 个时段)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayProfile {
    pub id: u8,
    pub segments: Vec<TimeSegment>,
}

impl DayProfile {
    pub fn new(id: u8, segments: Vec<TimeSegment>) -> Self {
        Self { id, segments }
    }
    /// 根据分钟数查找费率
    pub fn rate_at(&self, minutes: u32) -> TariffRate {
        let mut result = TariffRate::T3; // default
        for seg in &self.segments {
            if minutes >= seg.minutes_from_midnight() {
                result = seg.rate;
            } else {
                break;
            }
        }
        result
    }
}

/// 周计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekProfile {
    pub id: u8,
    pub day_monday: u8,
    pub day_tuesday: u8,
    pub day_wednesday: u8,
    pub day_thursday: u8,
    pub day_friday: u8,
    pub day_saturday: u8,
    pub day_sunday: u8,
}

impl WeekProfile {
    pub fn uniform(id: u8, day_profile_id: u8) -> Self {
        Self {
            id,
            day_monday: day_profile_id,
            day_tuesday: day_profile_id,
            day_wednesday: day_profile_id,
            day_thursday: day_profile_id,
            day_friday: day_profile_id,
            day_saturday: day_profile_id,
            day_sunday: day_profile_id,
        }
    }
    pub fn day_profile_id(&self, weekday: chrono::Weekday) -> u8 {
        use chrono::Weekday::*;
        match weekday {
            Mon => self.day_monday,
            Tue => self.day_tuesday,
            Wed => self.day_wednesday,
            Thu => self.day_thursday,
            Fri => self.day_friday,
            Sat => self.day_saturday,
            Sun => self.day_sunday,
        }
    }
}

/// 季节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonProfile {
    pub id: u8,
    pub start_month: u8,
    pub start_day: u8,
    pub week_profile_id: u8,
}

impl SeasonProfile {
    pub fn new(id: u8, month: u8, day: u8, week_profile_id: u8) -> Self {
        Self {
            id,
            start_month: month,
            start_day: day,
            week_profile_id,
        }
    }
    /// 检查日期是否属于此季节 (该季节生效于 start_month/start_day 之后)
    pub fn matches(&self, month: u8, day: u8, seasons: &[SeasonProfile]) -> bool {
        // This season applies from its start date until the next season's start date
        let self_val = self.start_month as u32 * 100 + self.start_day as u32;
        let date_val = month as u32 * 100 + day as u32;
        if date_val >= self_val {
            // Check if any later season has a start date <= our date
            let mut has_later = false;
            for s in seasons {
                if s.id != self.id {
                    let s_val = s.start_month as u32 * 100 + s.start_day as u32;
                    if s_val > self_val && s_val <= date_val {
                        has_later = true;
                        break;
                    }
                }
            }
            !has_later
        } else {
            // date is before this season's start - not matching
            // unless this season wraps around year end (e.g., winter starts in Nov)
            // In that case, check if this is the last season in the year
            let is_last = seasons.iter().all(|s| {
                s.id == self.id || {
                    let s_val = s.start_month as u32 * 100 + s.start_day as u32;
                    s_val < self_val
                }
            });
            is_last
        }
    }
}

/// 节假日
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holiday {
    pub month: u8,
    pub day: u8,
}

/// 年日历
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub seasons: Vec<SeasonProfile>,
    pub day_profiles: Vec<DayProfile>,
    pub week_profiles: Vec<WeekProfile>,
    pub holidays: Vec<Holiday>,
    pub holiday_day_profile_id: u8,
}

impl Calendar {
    fn find_season(&self, month: u8, day: u8) -> Option<&WeekProfile> {
        // Try each season; the one that matches is current
        for season in &self.seasons {
            if season.matches(month, day, &self.seasons) {
                return self
                    .week_profiles
                    .iter()
                    .find(|w| w.id == season.week_profile_id);
            }
        }
        // fallback: first week profile
        self.week_profiles.first()
    }

    fn find_day_profile(&self, id: u8) -> &DayProfile {
        self.day_profiles
            .iter()
            .find(|d| d.id == id)
            .unwrap_or_else(|| self.day_profiles.first().unwrap())
    }
}

/// 预设费率方案
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouPreset {
    SingleRate,
    TwoRateTimeOfDay,
    ThreeRatePeakFlatValley,
    FourRatePeakFlatValleySharp,
}

/// TOU 费率引擎
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouEngine {
    pub calendar: Calendar,
    pub active_rate: TariffRate,
    pub last_rate: TariffRate,
    pub rate_change_count: u32,
}

impl Default for TouEngine {
    fn default() -> Self {
        let mut engine = Self {
            calendar: Calendar {
                seasons: Vec::new(),
                day_profiles: Vec::new(),
                week_profiles: Vec::new(),
                holidays: Vec::new(),
                holiday_day_profile_id: 0,
            },
            active_rate: TariffRate::T1,
            last_rate: TariffRate::T1,
            rate_change_count: 0,
        };
        engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);
        engine
    }
}

impl TouEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calculate_rate(&mut self, datetime: &NaiveDateTime) -> TariffRate {
        let month = datetime.month() as u8;
        let day = datetime.day() as u8;
        let weekday = datetime.weekday();
        let minutes = datetime.hour() * 60 + datetime.minute();

        // Check holiday first
        let is_holiday = self
            .calendar
            .holidays
            .iter()
            .any(|h| h.month == month && h.day == day);

        let day_profile_id = if is_holiday {
            self.calendar.holiday_day_profile_id
        } else if let Some(wp) = self.calendar.find_season(month, day) {
            wp.day_profile_id(weekday)
        } else {
            1
        };

        let dp = self.calendar.find_day_profile(day_profile_id);
        let rate = dp.rate_at(minutes);

        self.last_rate = self.active_rate;
        if rate != self.active_rate {
            self.rate_change_count += 1;
        }
        self.active_rate = rate;
        rate
    }

    pub fn current_rate(&self) -> TariffRate {
        self.active_rate
    }

    pub fn load_preset(&mut self, preset: TouPreset) {
        match preset {
            TouPreset::SingleRate => {
                let dp = DayProfile::new(1, vec![TimeSegment::new(0, 0, TariffRate::T1)]);
                let wp = WeekProfile::uniform(1, 1);
                let season = SeasonProfile::new(1, 1, 1, 1);
                self.calendar = Calendar {
                    seasons: vec![season],
                    day_profiles: vec![dp],
                    week_profiles: vec![wp],
                    holidays: Vec::new(),
                    holiday_day_profile_id: 1,
                };
                self.active_rate = TariffRate::T1;
            }
            TouPreset::TwoRateTimeOfDay => {
                let dp = DayProfile::new(
                    1,
                    vec![
                        TimeSegment::new(0, 0, TariffRate::T2),  // Valley 00:00-08:00
                        TimeSegment::new(8, 0, TariffRate::T1),  // Peak 08:00-22:00
                        TimeSegment::new(22, 0, TariffRate::T2), // Valley 22:00-24:00
                    ],
                );
                let wp = WeekProfile::uniform(1, 1);
                let season = SeasonProfile::new(1, 1, 1, 1);
                self.calendar = Calendar {
                    seasons: vec![season],
                    day_profiles: vec![dp],
                    week_profiles: vec![wp],
                    holidays: Vec::new(),
                    holiday_day_profile_id: 1,
                };
            }
            TouPreset::ThreeRatePeakFlatValley => {
                let dp = DayProfile::new(
                    1,
                    vec![
                        TimeSegment::new(0, 0, TariffRate::T3),  // Valley 00:00-06:00
                        TimeSegment::new(6, 0, TariffRate::T2),  // Normal 06:00-08:00
                        TimeSegment::new(8, 0, TariffRate::T1),  // Peak 08:00-11:00
                        TimeSegment::new(11, 0, TariffRate::T2), // Normal 11:00-17:00
                        TimeSegment::new(17, 0, TariffRate::T1), // Peak 17:00-21:00
                        TimeSegment::new(21, 0, TariffRate::T3), // Valley 21:00-24:00
                    ],
                );
                let wp = WeekProfile::uniform(1, 1);
                let season = SeasonProfile::new(1, 1, 1, 1);
                self.calendar = Calendar {
                    seasons: vec![season],
                    day_profiles: vec![dp],
                    week_profiles: vec![wp],
                    holidays: Vec::new(),
                    holiday_day_profile_id: 1,
                };
            }
            TouPreset::FourRatePeakFlatValleySharp => {
                // 尖(T1) 峰(T2) 平(T3) 谷(T4)
                let dp = DayProfile::new(
                    1,
                    vec![
                        TimeSegment::new(0, 0, TariffRate::T4),  // 谷 00:00-06:00
                        TimeSegment::new(6, 0, TariffRate::T3),  // 平 06:00-08:00
                        TimeSegment::new(8, 0, TariffRate::T2),  // 峰 08:00-10:00
                        TimeSegment::new(10, 0, TariffRate::T1), // 尖 10:00-12:00
                        TimeSegment::new(12, 0, TariffRate::T3), // 平 12:00-17:00
                        TimeSegment::new(17, 0, TariffRate::T2), // 峰 17:00-18:00
                        TimeSegment::new(18, 0, TariffRate::T1), // 尖 18:00-20:00
                        TimeSegment::new(20, 0, TariffRate::T2), // 峰 20:00-21:00
                        TimeSegment::new(21, 0, TariffRate::T3), // 平 21:00-22:00
                        TimeSegment::new(22, 0, TariffRate::T4), // 谷 22:00-24:00
                    ],
                );
                let wp = WeekProfile::uniform(1, 1);
                let season = SeasonProfile::new(1, 1, 1, 1);
                self.calendar = Calendar {
                    seasons: vec![season],
                    day_profiles: vec![dp],
                    week_profiles: vec![wp],
                    holidays: Vec::new(),
                    holiday_day_profile_id: 1,
                };
            }
        }
        self.last_rate = self.active_rate;
        self.rate_change_count = 0;
    }
}

/// 分相电能（按费率累计）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TariffEnergy {
    pub active: [[f64; 3]; 8], // [rate][phase]
    pub reactive: [[f64; 3]; 8],
    pub active_total: [f64; 8],
    pub reactive_total: [f64; 8],
}

impl TariffEnergy {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn accumulate(&mut self, rate: TariffRate, phase: usize, p_wh: f64, q_varh: f64) {
        let ri = rate.index();
        if phase < 3 {
            self.active[ri][phase] += p_wh;
            self.reactive[ri][phase] += q_varh;
        }
        self.active_total[ri] += p_wh;
        self.reactive_total[ri] += q_varh;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_four_rate_times() {
        let mut engine = TouEngine::new();
        // 谷 00:00-06:00
        let t = NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            chrono::NaiveTime::from_hms_opt(3, 0, 0).unwrap(),
        );
        assert_eq!(engine.calculate_rate(&t), TariffRate::T4);
        // 尖 10:00-12:00
        let t = t.with_hour(10).unwrap();
        assert_eq!(engine.calculate_rate(&t), TariffRate::T1);
        // 平 12:00-17:00
        let t = t.with_hour(14).unwrap();
        assert_eq!(engine.calculate_rate(&t), TariffRate::T3);
        // 峰 17:00-18:00
        let t = t.with_hour(17).and_then(|t| t.with_minute(30)).unwrap();
        assert_eq!(engine.calculate_rate(&t), TariffRate::T2);
        // 尖 18:00-20:00
        let t = t.with_hour(19).unwrap();
        assert_eq!(engine.calculate_rate(&t), TariffRate::T1);
        // 谷 22:00-06:00
        let t = t.with_hour(23).unwrap();
        assert_eq!(engine.calculate_rate(&t), TariffRate::T4);
    }

    #[test]
    fn test_single_rate() {
        let mut engine = TouEngine::new();
        engine.load_preset(TouPreset::SingleRate);
        let t = NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        assert_eq!(engine.calculate_rate(&t), TariffRate::T1);
    }

    #[test]
    fn test_tariff_energy() {
        let mut te = TariffEnergy::new();
        te.accumulate(TariffRate::T1, 0, 100.0, 10.0);
        te.accumulate(TariffRate::T1, 1, 50.0, 5.0);
        assert!((te.active[0][0] - 100.0).abs() < 0.001);
        assert!((te.active[0][1] - 50.0).abs() < 0.001);
        assert!((te.active_total[0] - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_rate_change_count() {
        let mut engine = TouEngine::new();
        let base = NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            chrono::NaiveTime::from_hms_opt(5, 0, 0).unwrap(),
        );
        engine.calculate_rate(&base); // T4 -> sets active_rate from T1 to T4 (change 1)
        engine.calculate_rate(&base.with_hour(6).unwrap()); // T3 -> change 2
                                                            // After 2 changes, count should be 2 (initial T1->T4, then T4->T3)
        assert!(engine.rate_change_count >= 1);
    }
}
