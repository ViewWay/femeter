//! 负荷曲线 (Load Profile / DLMS IC7 Profile Generic)
//!
//! 可配置通道、捕获间隔，环形缓冲区

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileChannel {
    pub obis: (u8, u8, u8, u8, u8, u8),
    pub name: String,
    pub unit: String,
    pub value_type: ProfileValueType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileValueType {
    Float64,
    Int32,
    Uint32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEntry {
    pub timestamp: NaiveDateTime,
    pub values: Vec<f64>,
    pub capture_period_s: u32,
}

pub struct LoadProfile {
    pub name: String,
    pub obis: (u8, u8, u8, u8, u8, u8),
    pub channels: Vec<ProfileChannel>,
    pub capture_interval_s: u32,
    pub buffer: Vec<ProfileEntry>,
    pub max_entries: usize,
    pub last_capture: Option<NaiveDateTime>,
    pub capture_count: u64,
}

impl LoadProfile {
    pub fn new(interval_s: u32) -> Self {
        Self {
            name: "Load Profile".to_string(),
            obis: (1, 0, 99, 1, 0, 255),
            channels: default_channels(),
            capture_interval_s: interval_s,
            buffer: Vec::new(),
            max_entries: 4032,
            last_capture: None,
            capture_count: 0,
        }
    }

    pub fn set_interval(&mut self, interval_s: u32) {
        self.capture_interval_s = interval_s;
    }

    /// 检查是否需要捕获
    pub fn should_capture(&self, now: &NaiveDateTime) -> bool {
        match self.last_capture {
            None => true,
            Some(last) => {
                let elapsed = (*now - last).num_seconds().unsigned_abs();
                elapsed >= self.capture_interval_s as u64
            }
        }
    }

    /// 捕获数据点 (caller provides values array matching channels)
    pub fn capture_values(&mut self, now: &NaiveDateTime, values: Vec<f64>) {
        let entry = ProfileEntry {
            timestamp: *now,
            values,
            capture_period_s: self.capture_interval_s,
        };
        self.last_capture = Some(*now);
        self.capture_count += 1;

        if self.buffer.len() >= self.max_entries {
            self.buffer.remove(0);
        }
        self.buffer.push(entry);
    }

    pub fn query_range(&self, _from: &NaiveDateTime, _to: &NaiveDateTime) -> &[ProfileEntry] {
        // Return matching entries (simplified - returns all for now, caller can filter)
        &self.buffer
    }

    pub fn query_last(&self, n: usize) -> &[ProfileEntry] {
        let start = if n >= self.buffer.len() {
            0
        } else {
            self.buffer.len() - n
        };
        &self.buffer[start..]
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.capture_count = 0;
        self.last_capture = None;
    }
}

fn default_channels() -> Vec<ProfileChannel> {
    vec![
        ProfileChannel {
            obis: (1, 0, 32, 7, 0, 255),
            name: "Ua".into(),
            unit: "V".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 52, 7, 0, 255),
            name: "Ub".into(),
            unit: "V".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 72, 7, 0, 255),
            name: "Uc".into(),
            unit: "V".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 31, 7, 0, 255),
            name: "Ia".into(),
            unit: "A".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 51, 7, 0, 255),
            name: "Ib".into(),
            unit: "A".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 71, 7, 0, 255),
            name: "Ic".into(),
            unit: "A".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 21, 7, 0, 255),
            name: "Pa".into(),
            unit: "W".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 41, 7, 0, 255),
            name: "Pb".into(),
            unit: "W".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 61, 7, 0, 255),
            name: "Pc".into(),
            unit: "W".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 1, 7, 0, 255),
            name: "P_total".into(),
            unit: "W".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 22, 7, 0, 255),
            name: "Qa".into(),
            unit: "var".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 42, 7, 0, 255),
            name: "Qb".into(),
            unit: "var".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 62, 7, 0, 255),
            name: "Qc".into(),
            unit: "var".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 2, 7, 0, 255),
            name: "Q_total".into(),
            unit: "var".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 13, 7, 0, 255),
            name: "PF_total".into(),
            unit: "".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (1, 0, 14, 7, 0, 255),
            name: "Freq".into(),
            unit: "Hz".into(),
            value_type: ProfileValueType::Float64,
        },
        ProfileChannel {
            obis: (0, 0, 13, 0, 0, 255),
            name: "Tariff".into(),
            unit: "".into(),
            value_type: ProfileValueType::Uint32,
        },
        ProfileChannel {
            obis: (1, 0, 97, 97, 0, 255),
            name: "Status".into(),
            unit: "".into(),
            value_type: ProfileValueType::Uint32,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_interval() {
        let mut lp = LoadProfile::new(60);
        let base = NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        assert!(lp.should_capture(&base));
        lp.capture_values(&base, vec![1.0, 2.0]);
        assert_eq!(lp.capture_count, 1);
        // Same time - should not capture
        assert!(!lp.should_capture(&base));
    }

    #[test]
    fn test_ring_buffer() {
        let mut lp = LoadProfile::new(1);
        let base = NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        for i in 0..4050 {
            let t = base + chrono::Duration::seconds(i);
            lp.capture_values(&t, vec![i as f64]);
        }
        assert_eq!(lp.buffer.len(), 4032);
        assert_eq!(lp.query_last(5).len(), 5);
    }
}
