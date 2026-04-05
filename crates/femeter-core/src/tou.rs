/* ================================================================== */
/*                                                                    */
/*  tou.rs — TOU Time-of-Use 费率引擎                                  */
/*                                                                    */
/*  支持 T1~T8 八费率，日时段表，周计划，季节日历，节假日                */
/*  对应 DLMS IC19 Activity Calendar / Tariff Schedule                 */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

/// 费率号 (T1~T8)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum TariffRate {
    T1 = 0,
    T2 = 1,
    T3 = 2,
    T4 = 3,
    T5 = 4,
    T6 = 5,
    T7 = 6,
    T8 = 7,
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

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::T1),
            1 => Some(Self::T2),
            2 => Some(Self::T3),
            3 => Some(Self::T4),
            4 => Some(Self::T5),
            5 => Some(Self::T6),
            6 => Some(Self::T7),
            7 => Some(Self::T8),
            _ => None,
        }
    }
}

impl Default for TariffRate {
    fn default() -> Self {
        Self::T1
    }
}

/// 时间段定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSegment {
    /// 开始小时 (0-23)
    pub start_hour: u8,
    /// 开始分钟 (0-59)
    pub start_min: u8,
    /// 费率
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

    /// 从午夜开始的分钟数
    pub fn minutes_from_midnight(&self) -> u16 {
        self.start_hour as u16 * 60 + self.start_min as u16
    }
}

/// 日时段表 (最多 14 个时段)
#[derive(Debug, Clone)]
pub struct DayProfile {
    /// 日时段表 ID
    pub id: u8,
    /// 时间段列表（必须按时间排序）
    pub segments: Vec<TimeSegment>,
}

impl DayProfile {
    pub fn new(id: u8, segments: Vec<TimeSegment>) -> Self {
        Self { id, segments }
    }

    pub fn single_rate(id: u8, rate: TariffRate) -> Self {
        Self {
            id,
            segments: vec![TimeSegment::new(0, 0, rate)],
        }
    }

    /// 根据分钟数查找当前费率
    pub fn rate_at(&self, minutes: u16) -> TariffRate {
        let mut result = TariffRate::T1; // 默认费率
        
        for seg in &self.segments {
            if minutes >= seg.minutes_from_midnight() {
                result = seg.rate;
            } else {
                break;
            }
        }
        
        result
    }

    /// 验证时段表有效性
    pub fn validate(&self) -> Result<(), TouError> {
        if self.segments.is_empty() {
            return Err(TouError::EmptyDayProfile);
        }

        if self.segments.len() > 14 {
            return Err(TouError::TooManySegments);
        }

        // 检查时间排序
        for i in 1..self.segments.len() {
            if self.segments[i].minutes_from_midnight() <= self.segments[i - 1].minutes_from_midnight() {
                return Err(TouError::SegmentsNotSorted);
            }
        }

        Ok(())
    }
}

/// 周计划
#[derive(Debug, Clone, Copy)]
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

    pub fn weekday_weekend(id: u8, weekday_id: u8, weekend_id: u8) -> Self {
        Self {
            id,
            day_monday: weekday_id,
            day_tuesday: weekday_id,
            day_wednesday: weekday_id,
            day_thursday: weekday_id,
            day_friday: weekday_id,
            day_saturday: weekend_id,
            day_sunday: weekend_id,
        }
    }

    /// 根据星期几获取日时段表 ID (weekday: 1=周一, 7=周日)
    pub fn day_profile_id(&self, weekday: u8) -> u8 {
        match weekday {
            1 => self.day_monday,
            2 => self.day_tuesday,
            3 => self.day_wednesday,
            4 => self.day_thursday,
            5 => self.day_friday,
            6 => self.day_saturday,
            7 | _ => self.day_sunday,
        }
    }
}

/// 季节定义
#[derive(Debug, Clone, Copy)]
pub struct SeasonProfile {
    pub id: u8,
    /// 开始月份 (1-12)
    pub start_month: u8,
    /// 开始日期 (1-31)
    pub start_day: u8,
    /// 对应的周计划 ID
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

    /// 全年统一
    pub fn全年统一(id: u8, week_profile_id: u8) -> Self {
        Self {
            id,
            start_month: 1,
            start_day: 1,
            week_profile_id,
        }
    }
}

/// 节假日定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Holiday {
    pub month: u8,
    pub day: u8,
}

impl Holiday {
    pub fn new(month: u8, day: u8) -> Self {
        Self { month, day }
    }
}

/// 特殊日类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialDayType {
    /// 普通节假日
    Holiday,
    /// 调休工作日
    Workday,
}

/// 特殊日定义
#[derive(Debug, Clone, Copy)]
pub struct SpecialDay {
    pub month: u8,
    pub day: u8,
    pub day_type: SpecialDayType,
    pub day_profile_id: u8,
}

impl SpecialDay {
    pub fn holiday(month: u8, day: u8, profile_id: u8) -> Self {
        Self {
            month,
            day,
            day_type: SpecialDayType::Holiday,
            day_profile_id: profile_id,
        }
    }

    pub fn workday(month: u8, day: u8, profile_id: u8) -> Self {
        Self {
            month,
            day,
            day_type: SpecialDayType::Workday,
            day_profile_id: profile_id,
        }
    }
}

/// 年日历
#[derive(Debug, Clone, Default)]
pub struct ActivityCalendar {
    /// 季节列表
    pub seasons: Vec<SeasonProfile>,
    /// 日时段表列表
    pub day_profiles: Vec<DayProfile>,
    /// 周计划列表
    pub week_profiles: Vec<WeekProfile>,
    /// 特殊日列表
    pub special_days: Vec<SpecialDay>,
    /// 默认日时段表 ID（未匹配时使用）
    pub default_day_profile_id: u8,
}

impl ActivityCalendar {
    pub fn new() -> Self {
        Self::default()
    }

    /// 查找指定日期的季节
    fn find_season(&self, month: u8, day: u8) -> Option<&SeasonProfile> {
        // 将日期转换为年进度值 (0-365)
        let date_val = month as u16 * 100 + day as u16;
        
        // 查找当前日期所属的季节
        let mut current_season: Option<&SeasonProfile> = None;
        
        for season in &self.seasons {
            let season_val = season.start_month as u16 * 100 + season.start_day as u16;
            
            if date_val >= season_val {
                current_season = Some(season);
            } else if current_season.is_some() {
                // 已经过了当前季节的开始日期，且还没到下一个季节
                break;
            }
        }
        
        current_season
    }

    /// 查找日时段表
    fn find_day_profile(&self, id: u8) -> Option<&DayProfile> {
        self.day_profiles.iter().find(|d| d.id == id)
    }

    /// 查找周计划
    fn find_week_profile(&self, id: u8) -> Option<&WeekProfile> {
        self.week_profiles.iter().find(|w| w.id == id)
    }

    /// 获取指定日期和时间的费率
    /// 
    /// 参数：
    /// - month: 月份 (1-12)
    /// - day: 日期 (1-31)
    /// - weekday: 星期 (1=周一, 7=周日)
    /// - hour: 小时 (0-23)
    /// - minute: 分钟 (0-59)
    pub fn get_rate(&self, month: u8, day: u8, weekday: u8, hour: u8, minute: u8) -> TariffRate {
        // 1. 检查特殊日
        for special in &self.special_days {
            if special.month == month && special.day == day {
                if let Some(dp) = self.find_day_profile(special.day_profile_id) {
                    let minutes = hour as u16 * 60 + minute as u16;
                    return dp.rate_at(minutes);
                }
            }
        }

        // 2. 查找季节
        let week_profile_id = if let Some(season) = self.find_season(month, day) {
            season.week_profile_id
        } else {
            // 没有匹配的季节，使用默认日时段表
            if let Some(dp) = self.find_day_profile(self.default_day_profile_id) {
                let minutes = hour as u16 * 60 + minute as u16;
                return dp.rate_at(minutes);
            }
            return TariffRate::T1;
        };

        // 3. 查找周计划
        let day_profile_id = if let Some(wp) = self.find_week_profile(week_profile_id) {
            wp.day_profile_id(weekday)
        } else {
            self.default_day_profile_id
        };

        // 4. 查找日时段表并计算费率
        if let Some(dp) = self.find_day_profile(day_profile_id) {
            let minutes = hour as u16 * 60 + minute as u16;
            dp.rate_at(minutes)
        } else {
            TariffRate::T1
        }
    }
}

/// TOU 费率引擎
#[derive(Debug, Clone)]
pub struct TouEngine {
    /// 年日历
    pub calendar: ActivityCalendar,
    /// 当前激活费率
    pub active_rate: TariffRate,
    /// 上一次费率
    pub last_rate: TariffRate,
    /// 费率切换次数
    pub rate_change_count: u32,
}

impl Default for TouEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TouEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            calendar: ActivityCalendar::new(),
            active_rate: TariffRate::T1,
            last_rate: TariffRate::T1,
            rate_change_count: 0,
        };
        engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);
        engine
    }

    /// 计算当前费率
    pub fn calculate_rate(&mut self, month: u8, day: u8, weekday: u8, hour: u8, minute: u8) -> TariffRate {
        let rate = self.calendar.get_rate(month, day, weekday, hour, minute);
        
        if rate != self.active_rate {
            self.last_rate = self.active_rate;
            self.active_rate = rate;
            self.rate_change_count += 1;
        }
        
        rate
    }

    /// 获取当前费率
    pub fn current_rate(&self) -> TariffRate {
        self.active_rate
    }

    /// 加载预设方案
    pub fn load_preset(&mut self, preset: TouPreset) {
        match preset {
            TouPreset::SingleRate => {
                let dp = DayProfile::single_rate(1, TariffRate::T1);
                let wp = WeekProfile::uniform(1, 1);
                let season = SeasonProfile::全年统一(1, 1);
                
                self.calendar = ActivityCalendar {
                    seasons: vec![season],
                    day_profiles: vec![dp],
                    week_profiles: vec![wp],
                    special_days: Vec::new(),
                    default_day_profile_id: 1,
                };
                self.active_rate = TariffRate::T1;
            }
            
            TouPreset::TwoRatePeakValley => {
                let dp = DayProfile::new(
                    1,
                    vec![
                        TimeSegment::new(0, 0, TariffRate::T2),  // 谷 00:00-08:00
                        TimeSegment::new(8, 0, TariffRate::T1),  // 峰 08:00-22:00
                        TimeSegment::new(22, 0, TariffRate::T2), // 谷 22:00-24:00
                    ],
                );
                let wp = WeekProfile::uniform(1, 1);
                let season = SeasonProfile::全年统一(1, 1);
                
                self.calendar = ActivityCalendar {
                    seasons: vec![season],
                    day_profiles: vec![dp],
                    week_profiles: vec![wp],
                    special_days: Vec::new(),
                    default_day_profile_id: 1,
                };
            }
            
            TouPreset::ThreeRatePeakFlatValley => {
                let dp = DayProfile::new(
                    1,
                    vec![
                        TimeSegment::new(0, 0, TariffRate::T3),  // 谷 00:00-06:00
                        TimeSegment::new(6, 0, TariffRate::T2),  // 平 06:00-08:00
                        TimeSegment::new(8, 0, TariffRate::T1),  // 峰 08:00-11:00
                        TimeSegment::new(11, 0, TariffRate::T2), // 平 11:00-17:00
                        TimeSegment::new(17, 0, TariffRate::T1), // 峰 17:00-21:00
                        TimeSegment::new(21, 0, TariffRate::T3), // 谷 21:00-24:00
                    ],
                );
                let wp = WeekProfile::uniform(1, 1);
                let season = SeasonProfile::全年统一(1, 1);
                
                self.calendar = ActivityCalendar {
                    seasons: vec![season],
                    day_profiles: vec![dp],
                    week_profiles: vec![wp],
                    special_days: Vec::new(),
                    default_day_profile_id: 1,
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
                let season = SeasonProfile::全年统一(1, 1);
                
                self.calendar = ActivityCalendar {
                    seasons: vec![season],
                    day_profiles: vec![dp],
                    week_profiles: vec![wp],
                    special_days: Vec::new(),
                    default_day_profile_id: 1,
                };
            }
        }
        
        self.last_rate = self.active_rate;
        self.rate_change_count = 0;
    }

    /// 添加节假日
    pub fn add_holiday(&mut self, month: u8, day: u8, day_profile_id: u8) {
        self.calendar.special_days.push(SpecialDay::holiday(month, day, day_profile_id));
    }

    /// 添加特殊工作日
    pub fn add_workday(&mut self, month: u8, day: u8, day_profile_id: u8) {
        self.calendar.special_days.push(SpecialDay::workday(month, day, day_profile_id));
    }
}

/// 预设费率方案
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouPreset {
    /// 单一费率
    SingleRate,
    /// 两费率（峰谷）
    TwoRatePeakValley,
    /// 三费率（峰平谷）
    ThreeRatePeakFlatValley,
    /// 四费率（尖峰平谷）
    FourRatePeakFlatValleySharp,
}

/// 分相电能（按费率累计）
#[derive(Debug, Clone, Default)]
pub struct TariffEnergy {
    /// 有功电能 [费率][相]
    pub active: [[u64; 3]; 8],
    /// 无功电能 [费率][相]
    pub reactive: [[u64; 3]; 8],
    /// 总有功电能 [费率]
    pub active_total: [u64; 8],
    /// 总无功电能 [费率]
    pub reactive_total: [u64; 8],
}

impl TariffEnergy {
    pub fn new() -> Self {
        Self::default()
    }

    /// 累计电能
    pub fn accumulate(&mut self, rate: TariffRate, phase: usize, p_wh: u64, q_varh: u64) {
        let ri = rate.index();
        if phase < 3 {
            self.active[ri][phase] = self.active[ri][phase].saturating_add(p_wh);
            self.reactive[ri][phase] = self.reactive[ri][phase].saturating_add(q_varh);
        }
        self.active_total[ri] = self.active_total[ri].saturating_add(p_wh);
        self.reactive_total[ri] = self.reactive_total[ri].saturating_add(q_varh);
    }

    /// 获取指定费率的总有功电能
    pub fn get_active_total(&self, rate: TariffRate) -> u64 {
        self.active_total[rate.index()]
    }

    /// 获取所有费率的总有功电能之和
    pub fn get_all_active_total(&self) -> u64 {
        self.active_total.iter().fold(0u64, |sum, &v| sum.saturating_add(v))
    }

    /// 重置
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// TOU 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TouError {
    /// 日时段表为空
    EmptyDayProfile,
    /// 时段数过多（>14）
    TooManySegments,
    /// 时段未排序
    SegmentsNotSorted,
    /// 无效费率
    InvalidTariff,
    /// 无效时间
    InvalidTime,
}

/* ================================================================== */
/*  单元测试                                                           */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== TariffRate 测试 ====================

    #[test]
    fn test_tariff_rate_from_index() {
        assert_eq!(TariffRate::from_index(0), TariffRate::T1);
        assert_eq!(TariffRate::from_index(3), TariffRate::T4);
        assert_eq!(TariffRate::from_index(7), TariffRate::T8);
        assert_eq!(TariffRate::from_index(99), TariffRate::T8); // 超出范围
    }

    #[test]
    fn test_tariff_rate_label() {
        assert_eq!(TariffRate::T1.label(), "尖(Sharp)");
        assert_eq!(TariffRate::T4.label(), "谷(Valley)");
    }

    #[test]
    fn test_tariff_rate_from_u8() {
        assert_eq!(TariffRate::from_u8(0), Some(TariffRate::T1));
        assert_eq!(TariffRate::from_u8(7), Some(TariffRate::T8));
        assert_eq!(TariffRate::from_u8(8), None);
    }

    // ==================== TimeSegment 测试 ====================

    #[test]
    fn test_time_segment_minutes() {
        let seg = TimeSegment::new(8, 30, TariffRate::T1);
        assert_eq!(seg.minutes_from_midnight(), 510);
    }

    #[test]
    fn test_time_segment_midnight() {
        let seg = TimeSegment::new(0, 0, TariffRate::T1);
        assert_eq!(seg.minutes_from_midnight(), 0);
    }

    // ==================== DayProfile 测试 ====================

    #[test]
    fn test_day_profile_single_rate() {
        let dp = DayProfile::single_rate(1, TariffRate::T2);
        assert_eq!(dp.id, 1);
        assert_eq!(dp.segments.len(), 1);
    }

    #[test]
    fn test_day_profile_rate_at() {
        let dp = DayProfile::new(
            1,
            vec![
                TimeSegment::new(0, 0, TariffRate::T4),
                TimeSegment::new(6, 0, TariffRate::T3),
                TimeSegment::new(8, 0, TariffRate::T2),
            ],
        );

        // 00:00-06:00 谷
        assert_eq!(dp.rate_at(0), TariffRate::T4);
        assert_eq!(dp.rate_at(300), TariffRate::T4); // 05:00

        // 06:00-08:00 平
        assert_eq!(dp.rate_at(360), TariffRate::T3); // 06:00
        assert_eq!(dp.rate_at(400), TariffRate::T3); // 06:40

        // 08:00+ 峰
        assert_eq!(dp.rate_at(480), TariffRate::T2); // 08:00
        assert_eq!(dp.rate_at(1000), TariffRate::T2); // 16:40
    }

    #[test]
    fn test_day_profile_validate_empty() {
        let dp = DayProfile::new(1, vec![]);
        assert_eq!(dp.validate(), Err(TouError::EmptyDayProfile));
    }

    #[test]
    fn test_day_profile_validate_too_many() {
        let segments: Vec<TimeSegment> = (0..15)
            .map(|i| TimeSegment::new(i, 0, TariffRate::T1))
            .collect();
        let dp = DayProfile::new(1, segments);
        assert_eq!(dp.validate(), Err(TouError::TooManySegments));
    }

    #[test]
    fn test_day_profile_validate_not_sorted() {
        let dp = DayProfile::new(
            1,
            vec![
                TimeSegment::new(8, 0, TariffRate::T1),
                TimeSegment::new(6, 0, TariffRate::T2), // 时间倒序
            ],
        );
        assert_eq!(dp.validate(), Err(TouError::SegmentsNotSorted));
    }

    #[test]
    fn test_day_profile_validate_ok() {
        let dp = DayProfile::new(
            1,
            vec![
                TimeSegment::new(0, 0, TariffRate::T1),
                TimeSegment::new(6, 0, TariffRate::T2),
            ],
        );
        assert!(dp.validate().is_ok());
    }

    // ==================== WeekProfile 测试 ====================

    #[test]
    fn test_week_profile_uniform() {
        let wp = WeekProfile::uniform(1, 5);
        assert_eq!(wp.day_monday, 5);
        assert_eq!(wp.day_sunday, 5);
    }

    #[test]
    fn test_week_profile_day_id() {
        let wp = WeekProfile::weekday_weekend(1, 1, 2);
        assert_eq!(wp.day_profile_id(1), 1); // 周一
        assert_eq!(wp.day_profile_id(5), 1); // 周五
        assert_eq!(wp.day_profile_id(6), 2); // 周六
        assert_eq!(wp.day_profile_id(7), 2); // 周日
    }

    // ==================== ActivityCalendar 测试 ====================

    #[test]
    fn test_calendar_single_rate() {
        let mut engine = TouEngine::new();
        engine.load_preset(TouPreset::SingleRate);

        // 任何时间都应该是 T1
        assert_eq!(engine.calculate_rate(4, 5, 1, 10, 30), TariffRate::T1);
        assert_eq!(engine.calculate_rate(12, 25, 7, 23, 59), TariffRate::T1);
    }

    #[test]
    fn test_calendar_four_rate() {
        let mut engine = TouEngine::new();
        engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);

        // 谷 00:00-06:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 3, 0), TariffRate::T4);
        
        // 尖 10:00-12:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 10, 30), TariffRate::T1);
        
        // 平 12:00-17:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 14, 0), TariffRate::T3);
        
        // 峰 17:00-18:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 17, 30), TariffRate::T2);
    }

    #[test]
    fn test_calendar_special_day() {
        let mut engine = TouEngine::new();
        engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);
        
        // 添加国庆节（10月1日）使用特殊日时段表（全谷）
        let holiday_dp = DayProfile::single_rate(2, TariffRate::T4);
        engine.calendar.day_profiles.push(holiday_dp);
        engine.add_holiday(10, 1, 2);

        // 10月1日任何时间都是谷
        assert_eq!(engine.calculate_rate(10, 1, 1, 10, 0), TariffRate::T4);
        
        // 10月2日恢复正常
        assert_eq!(engine.calculate_rate(10, 2, 1, 10, 0), TariffRate::T1);
    }

    // ==================== TouEngine 测试 ====================

    #[test]
    fn test_engine_new() {
        let engine = TouEngine::new();
        assert_eq!(engine.rate_change_count, 0);
    }

    #[test]
    fn test_engine_rate_change_count() {
        let mut engine = TouEngine::new();
        engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);

        // 初始费率
        engine.calculate_rate(4, 5, 1, 3, 0); // T4
        assert!(engine.rate_change_count >= 1);

        let count_before = engine.rate_change_count;
        
        // 相同费率，不增加计数
        engine.calculate_rate(4, 5, 1, 4, 0); // T4
        assert_eq!(engine.rate_change_count, count_before);

        // 切换费率
        engine.calculate_rate(4, 5, 1, 10, 0); // T1
        assert_eq!(engine.rate_change_count, count_before + 1);
    }

    #[test]
    fn test_engine_current_rate() {
        let mut engine = TouEngine::new();
        engine.calculate_rate(4, 5, 1, 10, 30);
        assert_eq!(engine.current_rate(), TariffRate::T1);
    }

    // ==================== TariffEnergy 测试 ====================

    #[test]
    fn test_tariff_energy_new() {
        let te = TariffEnergy::new();
        assert_eq!(te.get_active_total(TariffRate::T1), 0);
    }

    #[test]
    fn test_tariff_energy_accumulate() {
        let mut te = TariffEnergy::new();
        te.accumulate(TariffRate::T1, 0, 100, 10);
        te.accumulate(TariffRate::T1, 0, 50, 5);
        
        assert_eq!(te.active[0][0], 150);
        assert_eq!(te.reactive[0][0], 15);
        assert_eq!(te.get_active_total(TariffRate::T1), 150);
    }

    #[test]
    fn test_tariff_energy_multi_phase() {
        let mut te = TariffEnergy::new();
        te.accumulate(TariffRate::T1, 0, 100, 0);
        te.accumulate(TariffRate::T1, 1, 200, 0);
        te.accumulate(TariffRate::T1, 2, 300, 0);
        
        assert_eq!(te.get_active_total(TariffRate::T1), 600);
    }

    #[test]
    fn test_tariff_energy_multi_rate() {
        let mut te = TariffEnergy::new();
        te.accumulate(TariffRate::T1, 0, 100, 0);
        te.accumulate(TariffRate::T2, 0, 200, 0);
        te.accumulate(TariffRate::T3, 0, 300, 0);
        
        assert_eq!(te.get_active_total(TariffRate::T1), 100);
        assert_eq!(te.get_active_total(TariffRate::T2), 200);
        assert_eq!(te.get_active_total(TariffRate::T3), 300);
        assert_eq!(te.get_all_active_total(), 600);
    }

    #[test]
    fn test_tariff_energy_reset() {
        let mut te = TariffEnergy::new();
        te.accumulate(TariffRate::T1, 0, 100, 0);
        te.reset();
        
        assert_eq!(te.get_active_total(TariffRate::T1), 0);
    }

    #[test]
    fn test_tariff_energy_saturating() {
        let mut te = TariffEnergy::new();
        te.accumulate(TariffRate::T1, 0, u64::MAX, 0);
        te.accumulate(TariffRate::T1, 0, 100, 0); // 不会溢出
        
        assert_eq!(te.get_active_total(TariffRate::T1), u64::MAX);
    }

    // ==================== 预设方案测试 ====================

    #[test]
    fn test_preset_single_rate() {
        let mut engine = TouEngine::new();
        engine.load_preset(TouPreset::SingleRate);
        
        for hour in 0..24 {
            assert_eq!(
                engine.calculate_rate(4, 5, 1, hour, 0),
                TariffRate::T1
            );
        }
    }

    #[test]
    fn test_preset_two_rate() {
        let mut engine = TouEngine::new();
        engine.load_preset(TouPreset::TwoRatePeakValley);
        
        // 谷 00:00-08:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 7, 59), TariffRate::T2);
        
        // 峰 08:00-22:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 8, 0), TariffRate::T1);
        assert_eq!(engine.calculate_rate(4, 5, 1, 21, 59), TariffRate::T1);
        
        // 谷 22:00-24:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 22, 0), TariffRate::T2);
    }

    #[test]
    fn test_preset_three_rate() {
        let mut engine = TouEngine::new();
        engine.load_preset(TouPreset::ThreeRatePeakFlatValley);
        
        // 谷 00:00-06:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 5, 0), TariffRate::T3);
        
        // 峰 08:00-11:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 9, 0), TariffRate::T1);
        
        // 平 12:00-17:00
        assert_eq!(engine.calculate_rate(4, 5, 1, 14, 0), TariffRate::T2);
    }

    #[test]
    fn test_season_profile() {
        let season = SeasonProfile::new(1, 6, 1, 1); // 6月1日开始
        assert_eq!(season.start_month, 6);
        assert_eq!(season.start_day, 1);
    }

    #[test]
    fn test_holiday() {
        let h = Holiday::new(10, 1);
        assert_eq!(h.month, 10);
        assert_eq!(h.day, 1);
    }

    #[test]
    fn test_special_day_types() {
        let h = SpecialDay::holiday(1, 1, 1);
        assert_eq!(h.day_type, SpecialDayType::Holiday);
        
        let w = SpecialDay::workday(1, 1, 1);
        assert_eq!(w.day_type, SpecialDayType::Workday);
    }
}
