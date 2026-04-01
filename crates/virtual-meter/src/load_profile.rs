//! 负荷曲线 (Load Profile)
//!
//! 环形缓冲区, 可配置冻结间隔, CSV 导出

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 冻结间隔 (分钟)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FreezeInterval {
    Min1 = 1,
    Min5 = 5,
    Min15 = 15,
    Min30 = 30,
    Min60 = 60,
}

/// 一条冻结记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeRecord {
    pub timestamp: DateTime<Utc>,
    pub wh_active: f64,
    pub wh_reactive: f64,
    pub va: f64, pub vb: f64, pub vc: f64,
    pub ia: f64, pub ib: f64, pub ic: f64,
    pub max_demand: f64,
    pub tariff: u8,  // 1=Sharp,2=Peak,3=Normal,4=Valley
}

/// 环形缓冲负荷曲线
const MAX_RECORDS: usize = 4032;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadProfile {
    interval: FreezeInterval,
    buf: Vec<FreezeRecord>,
    write_idx: usize,
    last_freeze: DateTime<Utc>,
}

impl Default for LoadProfile {
    fn default() -> Self {
        Self {
            interval: FreezeInterval::Min15,
            buf: Vec::with_capacity(MAX_RECORDS),
            write_idx: 0,
            last_freeze: DateTime::<Utc>::MIN_UTC,
        }
    }
}

impl LoadProfile {
    pub fn new(interval: FreezeInterval) -> Self {
        Self { interval, ..Self::default() }
    }

    pub fn interval(&self) -> FreezeInterval { self.interval }
    pub fn set_interval(&mut self, i: FreezeInterval) { self.interval = i; }
    pub fn len(&self) -> usize { self.buf.len() }
    pub fn is_empty(&self) -> bool { self.buf.is_empty() }
    pub fn capacity() -> usize { MAX_RECORDS }

    /// 检查是否需要冻结, 并执行
    pub fn try_freeze(&mut self, record: &FreezeRecord) -> bool {
        let interval_secs = self.interval as u32 * 60;
        let elapsed = (record.timestamp - self.last_freeze).num_seconds().unsigned_abs() as u32;
        if elapsed < interval_secs { return false; }

        if self.buf.len() < MAX_RECORDS {
            self.buf.push(record.clone());
        } else {
            self.buf[self.write_idx] = record.clone();
        }
        self.write_idx = (self.write_idx + 1) % MAX_RECORDS;
        self.last_freeze = record.timestamp;
        true
    }

    /// 按时间范围查询
    pub fn query_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&FreezeRecord> {
        self.buf.iter().filter(|r| r.timestamp >= start && r.timestamp <= end).collect()
    }

    /// CSV 导出
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("timestamp,wh_active,wh_reactive,va,vb,vc,ia,ib,ic,max_demand,tariff\n");
        for r in &self.buf {
            csv.push_str(&format!("{},{},{},{},{},{},{},{},{},{},{}\n",
                r.timestamp.to_rfc3339(), r.wh_active, r.wh_reactive,
                r.va, r.vb, r.vc, r.ia, r.ib, r.ic, r.max_demand, r.tariff));
        }
        csv
    }

    pub fn records(&self) -> &[FreezeRecord] { &self.buf }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.write_idx = 0;
        self.last_freeze = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freeze_interval() {
        let mut lp = LoadProfile::new(FreezeInterval::Min1);
        let base = Utc::now();
        let r1 = FreezeRecord {
            timestamp: base, wh_active: 1.0, wh_reactive: 0.0,
            va: 220.0, vb: 220.0, vc: 220.0,
            ia: 5.0, ib: 5.0, ic: 5.0, max_demand: 0.0, tariff: 3,
        };
        assert!(lp.try_freeze(&r1));
        // same timestamp, should not freeze again
        let r2 = FreezeRecord { timestamp: base, ..r1.clone() };
        assert!(!lp.try_freeze(&r2));
        assert_eq!(lp.len(), 1);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut lp = LoadProfile::new(FreezeInterval::Min1);
        let base = Utc::now();
        for i in 0..(MAX_RECORDS + 10) {
            let r = FreezeRecord {
                timestamp: base + chrono::Duration::minutes(i as i64),
                wh_active: i as f64, wh_reactive: 0.0,
                va: 220.0, vb: 220.0, vc: 220.0,
                ia: 5.0, ib: 5.0, ic: 5.0, max_demand: 0.0, tariff: 3,
            };
            lp.try_freeze(&r);
        }
        assert_eq!(lp.len(), MAX_RECORDS);
    }

    #[test]
    fn test_csv_export() {
        let lp = LoadProfile::new(FreezeInterval::Min15);
        let csv = lp.to_csv();
        assert!(csv.contains("timestamp,wh_active"));
    }
}
