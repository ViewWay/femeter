//! 统计记录
//!
//! 电压/电流/频率/功率因数 min/max/avg (按日/月)

use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatRecord {
    pub min: f64,
    pub max: f64,
    pub sum: f64,
    pub count: u64,
}

impl StatRecord {
    pub fn update(&mut self, value: f64) {
        if self.count == 0 {
            self.min = value;
            self.max = value;
        } else {
            self.min = self.min.min(value);
            self.max = self.max.max(value);
        }
        self.sum += value;
        self.count += 1;
    }
    pub fn avg(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayStatistics {
    pub date: String, // YYYY-MM-DD
    pub va: StatRecord,
    pub vb: StatRecord,
    pub vc: StatRecord,
    pub ia: StatRecord,
    pub ib: StatRecord,
    pub ic: StatRecord,
    pub freq: StatRecord,
    pub pf: StatRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthStatistics {
    pub month: String, // YYYY-MM
    pub va: StatRecord,
    pub vb: StatRecord,
    pub vc: StatRecord,
    pub ia: StatRecord,
    pub ib: StatRecord,
    pub ic: StatRecord,
    pub freq: StatRecord,
    pub pf: StatRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Statistics {
    pub daily: Vec<DayStatistics>,
    pub monthly: Vec<MonthStatistics>,
    last_date: String,
    last_month: String,
}

impl Statistics {
    /// 采样更新
    #[allow(clippy::too_many_arguments)]
    pub fn sample(
        &mut self,
        va: f64,
        vb: f64,
        vc: f64,
        ia: f64,
        ib: f64,
        ic: f64,
        freq: f64,
        pf: f64,
    ) {
        let now = Utc::now();
        let date = now.format("%Y-%m-%d").to_string();
        let month = now.format("%Y-%m").to_string();

        // Ensure daily entry
        if self.last_date != date {
            self.last_date = date.clone();
            self.daily.push(DayStatistics {
                date: date.clone(),
                va: StatRecord::default(),
                vb: StatRecord::default(),
                vc: StatRecord::default(),
                ia: StatRecord::default(),
                ib: StatRecord::default(),
                ic: StatRecord::default(),
                freq: StatRecord::default(),
                pf: StatRecord::default(),
            });
        }
        if let Some(day) = self.daily.last_mut() {
            day.va.update(va);
            day.vb.update(vb);
            day.vc.update(vc);
            day.ia.update(ia);
            day.ib.update(ib);
            day.ic.update(ic);
            day.freq.update(freq);
            day.pf.update(pf);
        }

        // Ensure monthly entry
        if self.last_month != month {
            self.last_month = month.clone();
            self.monthly.push(MonthStatistics {
                month: month.clone(),
                va: StatRecord::default(),
                vb: StatRecord::default(),
                vc: StatRecord::default(),
                ia: StatRecord::default(),
                ib: StatRecord::default(),
                ic: StatRecord::default(),
                freq: StatRecord::default(),
                pf: StatRecord::default(),
            });
        }
        if let Some(mon) = self.monthly.last_mut() {
            mon.va.update(va);
            mon.vb.update(vb);
            mon.vc.update(vc);
            mon.ia.update(ia);
            mon.ib.update(ib);
            mon.ic.update(ic);
            mon.freq.update(freq);
            mon.pf.update(pf);
        }
    }

    pub fn reset(&mut self) {
        self.daily.clear();
        self.monthly.clear();
        self.last_date.clear();
        self.last_month.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_record() {
        let mut r = StatRecord::default();
        r.update(220.0);
        r.update(221.0);
        r.update(222.0);
        assert_eq!(r.min, 220.0);
        assert_eq!(r.max, 222.0);
        assert!((r.avg() - 221.0).abs() < 0.001);
    }

    #[test]
    fn test_statistics_sample() {
        let mut stats = Statistics::default();
        stats.sample(220.0, 220.0, 220.0, 5.0, 5.0, 5.0, 50.0, 0.95);
        stats.sample(221.0, 221.0, 221.0, 6.0, 6.0, 6.0, 50.1, 0.96);
        assert_eq!(stats.daily.len(), 1);
        let day = &stats.daily[0];
        assert!((day.va.avg() - 220.5).abs() < 0.001);
    }

    #[test]
    fn test_statistics_reset() {
        let mut stats = Statistics::default();
        stats.sample(220.0, 220.0, 220.0, 5.0, 5.0, 5.0, 50.0, 0.95);
        stats.reset();
        assert!(stats.daily.is_empty());
    }
}
