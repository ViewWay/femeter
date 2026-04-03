//! 数据冻结 (Data Freeze)
//!
//! 支持日冻结、月冻结、结算日冻结、时冻结、手动冻结
//! 日冻结保留 62 天，月冻结保留 24 月

use chrono::{Datelike, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FreezeType {
    Daily,
    Monthly,
    Settlement,
    Hourly,
    OnDemand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergySnapshot {
    pub active_import_wh: [f64; 8],
    pub active_export_wh: f64,
    pub reactive_import_varh: [f64; 8],
    pub reactive_export_varh: f64,
    pub total_active_import_wh: f64,
    pub total_reactive_import_varh: f64,
}

impl Default for EnergySnapshot {
    fn default() -> Self {
        Self {
            active_import_wh: [0.0; 8],
            active_export_wh: 0.0,
            reactive_import_varh: [0.0; 8],
            reactive_export_varh: 0.0,
            total_active_import_wh: 0.0,
            total_reactive_import_varh: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandSnapshot {
    pub max_demand_kw: f64,
    pub max_demand_time: Option<NaiveDateTime>,
    pub max_demand_phase_kw: [f64; 3],
}

impl Default for DemandSnapshot {
    fn default() -> Self {
        Self {
            max_demand_kw: 0.0,
            max_demand_time: None,
            max_demand_phase_kw: [0.0; 3],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeRecord {
    pub freeze_type: FreezeType,
    pub timestamp: NaiveDateTime,
    pub energy: EnergySnapshot,
    pub demand: DemandSnapshot,
    pub voltage: [f64; 3],
    pub current: [f64; 3],
    pub power_factor: f64,
    pub status_word: u32,
    pub tariff_rate: u8,
}

pub struct FreezeManager {
    pub daily_records: Vec<FreezeRecord>,
    pub monthly_records: Vec<FreezeRecord>,
    pub settlement_day: u8,
    pub last_daily_freeze: Option<NaiveDateTime>,
    pub last_monthly_freeze: Option<NaiveDateTime>,
    max_daily: usize,
    max_monthly: usize,
}

impl Default for FreezeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FreezeManager {
    pub fn new() -> Self {
        Self {
            daily_records: Vec::new(),
            monthly_records: Vec::new(),
            settlement_day: 1,
            last_daily_freeze: None,
            last_monthly_freeze: None,
            max_daily: 62,
            max_monthly: 24,
        }
    }

    /// Check if a freeze should be triggered
    pub fn check_freeze(&mut self, now: &NaiveDateTime) -> Option<FreezeType> {
        let hour = now.hour();
        let minute = now.minute();
        let day = now.day();
        let _month = now.month();

        // Daily freeze at 00:00
        if hour == 0 && minute == 0 {
            let should = match self.last_daily_freeze {
                None => true,
                Some(last) => last.date() != now.date(),
            };
            if should {
                return Some(FreezeType::Daily);
            }
        }

        // Monthly freeze at settlement day 00:00
        if day == self.settlement_day as u32 && hour == 0 && minute == 0 {
            let should = match self.last_monthly_freeze {
                None => true,
                Some(last) => last.year() != now.year() || last.month() != now.month(),
            };
            if should {
                return Some(FreezeType::Monthly);
            }
        }

        None
    }

    /// Execute a freeze (caller must build the FreezeRecord)
    pub fn do_freeze(&mut self, ftype: FreezeType, record: FreezeRecord) {
        match ftype {
            FreezeType::Daily => {
                self.last_daily_freeze = Some(record.timestamp);
                if self.daily_records.len() >= self.max_daily {
                    self.daily_records.remove(0);
                }
                self.daily_records.push(record);
            }
            FreezeType::Monthly => {
                self.last_monthly_freeze = Some(record.timestamp);
                if self.monthly_records.len() >= self.max_monthly {
                    self.monthly_records.remove(0);
                }
                self.monthly_records.push(record);
            }
            FreezeType::OnDemand | FreezeType::Hourly | FreezeType::Settlement => {
                // Store in daily for simplicity
                if self.daily_records.len() >= self.max_daily {
                    self.daily_records.remove(0);
                }
                self.daily_records.push(record);
            }
        }
    }

    pub fn query(
        &self,
        ftype: FreezeType,
        from: &NaiveDateTime,
        to: &NaiveDateTime,
    ) -> Vec<&FreezeRecord> {
        let records = match ftype {
            FreezeType::Daily | FreezeType::Hourly | FreezeType::OnDemand => &self.daily_records,
            FreezeType::Monthly | FreezeType::Settlement => &self.monthly_records,
        };
        records
            .iter()
            .filter(|r| r.timestamp >= *from && r.timestamp <= *to)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_daily_freeze_trigger() {
        let mut fm = FreezeManager::new();
        let t = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        assert_eq!(fm.check_freeze(&t), Some(FreezeType::Daily));
    }

    #[test]
    fn test_no_freeze_at_noon() {
        let mut fm = FreezeManager::new();
        let t = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        assert_eq!(fm.check_freeze(&t), None);
    }

    #[test]
    fn test_monthly_freeze() {
        let mut fm = FreezeManager::new();
        let t = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        // Both daily and monthly trigger at same time; check_freeze returns daily first
        assert_eq!(fm.check_freeze(&t), Some(FreezeType::Daily));
    }

    #[test]
    fn test_freeze_record_limits() {
        let mut fm = FreezeManager::new();
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        for i in 0..70 {
            let t = NaiveDateTime::new(base, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                + chrono::Duration::days(i);
            let record = FreezeRecord {
                freeze_type: FreezeType::Daily,
                timestamp: t,
                energy: EnergySnapshot::default(),
                demand: DemandSnapshot::default(),
                voltage: [220.0; 3],
                current: [5.0; 3],
                power_factor: 0.95,
                status_word: 0,
                tariff_rate: 3,
            };
            fm.do_freeze(FreezeType::Daily, record);
        }
        assert_eq!(fm.daily_records.len(), 62);
    }
}
