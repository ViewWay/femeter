//! 分时费率 (Time-of-Use / TOU)
//!
//! 4 个费率: 尖/峰/平/谷 (Sharp/Peak/Normal/Valley)
//! 日时段表, 多套费率表, 自动费率判断

use chrono::{Local, Datelike, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TariffType {
    Sharp,   // 尖
    Peak,    // 峰
    Normal,  // 平
    Valley,  // 谷
}

impl TariffType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sharp => "尖", Self::Peak => "峰",
            Self::Normal => "平", Self::Valley => "谷",
        }
    }
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(Self::Sharp), 2 => Some(Self::Peak),
            3 => Some(Self::Normal), 4 => Some(Self::Valley),
            _ => None,
        }
    }
    pub fn to_byte(&self) -> u8 {
        match self { Self::Sharp => 1, Self::Peak => 2, Self::Normal => 3, Self::Valley => 4 }
    }
}

/// 一个时段: 起始分钟 ~ 结束分钟
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePeriod {
    pub start: u32,  // minutes from 00:00
    pub end: u32,
    pub tariff: TariffType,
}

impl TimePeriod {
    pub fn new(start_h: u32, start_m: u32, end_h: u32, end_m: u32, tariff: TariffType) -> Self {
        Self {
            start: start_h * 60 + start_m,
            end: end_h * 60 + end_m,
            tariff,
        }
    }
}

/// 一套日时段表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySchedule {
    pub periods: Vec<TimePeriod>,
}

impl DailySchedule {
    pub fn weekday_default() -> Self {
        Self {
            periods: vec![
                TimePeriod::new(0, 0, 6, 0, TariffType::Valley),
                TimePeriod::new(6, 0, 8, 0, TariffType::Normal),
                TimePeriod::new(8, 0, 11, 0, TariffType::Peak),
                TimePeriod::new(11, 0, 13, 0, TariffType::Sharp),
                TimePeriod::new(13, 0, 17, 0, TariffType::Peak),
                TimePeriod::new(17, 0, 21, 0, TariffType::Sharp),
                TimePeriod::new(21, 0, 23, 0, TariffType::Normal),
                TimePeriod::new(23, 0, 24, 0, TariffType::Valley),
            ],
        }
    }

    pub fn weekend_default() -> Self {
        Self {
            periods: vec![
                TimePeriod::new(0, 0, 7, 0, TariffType::Valley),
                TimePeriod::new(7, 0, 21, 0, TariffType::Normal),
                TimePeriod::new(21, 0, 24, 0, TariffType::Valley),
            ],
        }
    }

    pub fn holiday_default() -> Self {
        Self::weekend_default()
    }

    /// 根据 minutes-from-midnight 判断费率
    pub fn tariff_at(&self, minutes: u32) -> TariffType {
        for p in &self.periods {
            if p.start <= minutes && minutes < p.end {
                return p.tariff;
            }
        }
        TariffType::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleType {
    Weekday,
    Weekend,
    Holiday,
}

/// 费率切换事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TariffChangeEvent {
    pub from: TariffType,
    pub to: TariffType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 分时费率管理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouManager {
    pub weekday: DailySchedule,
    pub weekend: DailySchedule,
    pub holiday: DailySchedule,
    /// 各费率累计有功电能 (Wh)
    pub energy: std::collections::HashMap<TariffType, f64>,
    /// 费率切换事件
    pub events: Vec<TariffChangeEvent>,
    /// 当前费率
    current_tariff: TariffType,
}

impl Default for TouManager {
    fn default() -> Self {
        let mut energy = std::collections::HashMap::new();
        energy.insert(TariffType::Sharp, 0.0);
        energy.insert(TariffType::Peak, 0.0);
        energy.insert(TariffType::Normal, 0.0);
        energy.insert(TariffType::Valley, 0.0);
        Self {
            weekday: DailySchedule::weekday_default(),
            weekend: DailySchedule::weekend_default(),
            holiday: DailySchedule::holiday_default(),
            energy,
            events: Vec::new(),
            current_tariff: TariffType::Normal,
        }
    }
}

impl TouManager {
    pub fn current_tariff(&self) -> TariffType { self.current_tariff }

    /// 根据本地时间判断当前费率
    pub fn update(&mut self) {
        let now = Local::now();
        let minutes = now.hour() * 60 + now.minute();
        let dow = now.weekday();
        let schedule = match dow {
            chrono::Weekday::Sat | chrono::Weekday::Sun => &self.weekend,
            _ => &self.weekday,
        };
        let new_tariff = schedule.tariff_at(minutes);
        if new_tariff != self.current_tariff {
            self.events.push(TariffChangeEvent {
                from: self.current_tariff,
                to: new_tariff,
                timestamp: chrono::Utc::now(),
            });
            self.current_tariff = new_tariff;
        }
    }

    /// 累加电能到当前费率
    pub fn accumulate(&mut self, wh: f64) {
        *self.energy.entry(self.current_tariff).or_insert(0.0) += wh;
    }

    pub fn reset(&mut self) {
        for v in self.energy.values_mut() { *v = 0.0; }
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_schedule() {
        let sched = DailySchedule::weekday_default();
        assert_eq!(sched.tariff_at(0), TariffType::Valley);
        assert_eq!(sched.tariff_at(7 * 60 + 30), TariffType::Normal);
        assert_eq!(sched.tariff_at(8 * 60 + 30), TariffType::Peak);
        assert_eq!(sched.tariff_at(12 * 60), TariffType::Sharp);
        assert_eq!(sched.tariff_at(23 * 60 + 30), TariffType::Valley);
    }

    #[test]
    fn test_tou_manager_accumulate() {
        let mut mgr = TouManager::default();
        mgr.current_tariff = TariffType::Peak;
        mgr.accumulate(100.0);
        assert_eq!(mgr.energy[&TariffType::Peak], 100.0);
        assert_eq!(mgr.energy[&TariffType::Sharp], 0.0);
    }

    #[test]
    fn test_tariff_type_conversion() {
        assert_eq!(TariffType::from_byte(1), Some(TariffType::Sharp));
        assert_eq!(TariffType::from_byte(5), None);
        assert_eq!(TariffType::Peak.to_byte(), 2);
    }
}
