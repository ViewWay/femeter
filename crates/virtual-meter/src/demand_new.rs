//! 需量计算 (Demand Calculation)
//!
//! 支持区间需量和滑差需量 (Sliding Window)
//! 默认: 15分钟窗口, 1分钟滑差

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// 需量计算方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DemandMode {
    Block,
    Sliding { window_min: u8, slide_min: u8 },
}

impl Default for DemandMode {
    fn default() -> Self {
        Self::Sliding {
            window_min: 15,
            slide_min: 1,
        }
    }
}

/// 需量计算器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandCalculator {
    pub mode: DemandMode,
    pub interval_seconds: u32,
    pub window_seconds: u32,
    // Sliding window state
    pub sub_intervals: Vec<f64>,
    pub current_sub_index: usize,
    // Results
    pub current_demand_w: f64,
    pub max_demand_w: f64,
    pub max_demand_timestamp: Option<NaiveDateTime>,
    pub last_demand_reset: Option<NaiveDateTime>,
    // Phase
    pub phase_demand_w: [f64; 3],
    pub phase_max_demand_w: [f64; 3],
    // Internal accumulator for block mode
    block_accumulator_w: f64,
    block_accumulator_phase: [f64; 3],
    block_elapsed: f64,
    // Sliding: sub-interval accumulator
    sub_accumulator_w: f64,
    sub_accumulator_phase: [f64; 3],
    sub_elapsed: f64,
}

impl Default for DemandCalculator {
    fn default() -> Self {
        Self::new(DemandMode::default())
    }
}

impl DemandCalculator {
    pub fn new(mode: DemandMode) -> Self {
        let (window_seconds, interval_seconds) = match mode {
            DemandMode::Block => (900, 900),
            DemandMode::Sliding {
                window_min,
                slide_min,
            } => (window_min as u32 * 60, slide_min as u32 * 60),
        };
        Self {
            mode,
            interval_seconds,
            window_seconds,
            sub_intervals: Vec::new(),
            current_sub_index: 0,
            current_demand_w: 0.0,
            max_demand_w: 0.0,
            max_demand_timestamp: None,
            last_demand_reset: None,
            phase_demand_w: [0.0; 3],
            phase_max_demand_w: [0.0; 3],
            block_accumulator_w: 0.0,
            block_accumulator_phase: [0.0; 3],
            block_elapsed: 0.0,
            sub_accumulator_w: 0.0,
            sub_accumulator_phase: [0.0; 3],
            sub_elapsed: 0.0,
        }
    }

    pub fn update(&mut self, p_w: [f64; 3], dt_seconds: f64, now: &NaiveDateTime) {
        let p_total = p_w[0] + p_w[1] + p_w[2];

        match self.mode {
            DemandMode::Block => {
                self.block_accumulator_w += p_total * dt_seconds;
                for (i, p) in p_w.iter().enumerate().take(3) {
                    self.block_accumulator_phase[i] += *p * dt_seconds;
                }
                self.block_elapsed += dt_seconds;

                if self.block_elapsed >= self.window_seconds as f64 {
                    // demand = average power over window
                    self.current_demand_w = self.block_accumulator_w / self.block_elapsed;
                    for i in 0..3 {
                        self.phase_demand_w[i] =
                            self.block_accumulator_phase[i] / self.block_elapsed;
                    }
                    self.check_max(now);
                    self.block_accumulator_w = 0.0;
                    self.block_accumulator_phase = [0.0; 3];
                    self.block_elapsed = 0.0;
                }
            }
            DemandMode::Sliding { .. } => {
                self.sub_accumulator_w += p_total * dt_seconds;
                for (i, p) in p_w.iter().enumerate().take(3) {
                    self.sub_accumulator_phase[i] += *p * dt_seconds;
                }
                self.sub_elapsed += dt_seconds;

                if self.sub_elapsed >= self.interval_seconds as f64 {
                    // Compute sub-interval average power
                    let sub_avg = self.sub_accumulator_w / self.sub_elapsed;
                    let sub_avg_phase = [
                        self.sub_accumulator_phase[0] / self.sub_elapsed,
                        self.sub_accumulator_phase[1] / self.sub_elapsed,
                        self.sub_accumulator_phase[2] / self.sub_elapsed,
                    ];

                    self.sub_intervals.push(sub_avg);

                    let max_subs = (self.window_seconds / self.interval_seconds) as usize;
                    if self.sub_intervals.len() > max_subs {
                        self.sub_intervals.remove(0);
                    }

                    // Current demand = max of sub-interval averages in window
                    if !self.sub_intervals.is_empty() {
                        self.current_demand_w = self
                            .sub_intervals
                            .iter()
                            .cloned()
                            .fold(f64::NEG_INFINITY, f64::max);
                        self.phase_demand_w = sub_avg_phase;
                    }

                    self.check_max(now);

                    self.sub_accumulator_w = 0.0;
                    self.sub_accumulator_phase = [0.0; 3];
                    self.sub_elapsed = 0.0;
                }
            }
        }
    }

    fn check_max(&mut self, now: &NaiveDateTime) {
        if self.current_demand_w > self.max_demand_w {
            self.max_demand_w = self.current_demand_w;
            self.max_demand_timestamp = Some(*now);
            for i in 0..3 {
                if self.phase_demand_w[i] > self.phase_max_demand_w[i] {
                    self.phase_max_demand_w[i] = self.phase_demand_w[i];
                }
            }
        }
    }

    pub fn reset_max(&mut self, now: &NaiveDateTime) {
        self.max_demand_w = 0.0;
        self.max_demand_timestamp = None;
        self.phase_max_demand_w = [0.0; 3];
        self.last_demand_reset = Some(*now);
    }

    pub fn current_demand_kw(&self) -> f64 {
        self.current_demand_w / 1000.0
    }

    pub fn max_demand_kw(&self) -> f64 {
        self.max_demand_w / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn now() -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        )
    }

    #[test]
    fn test_sliding_demand() {
        let mut dc = DemandCalculator::new(DemandMode::Sliding {
            window_min: 15,
            slide_min: 1,
        });
        let mut t = now();
        // Feed 1000W for 15 sub-intervals (1 min each = 60s)
        for _ in 0..15 {
            dc.update([1000.0, 1000.0, 1000.0], 60.0, &t);
            t = t + chrono::Duration::seconds(60);
        }
        assert!((dc.current_demand_w - 3000.0).abs() < 0.01);
    }

    #[test]
    fn test_block_demand() {
        let mut dc = DemandCalculator::new(DemandMode::Block);
        let mut t = now();
        // Feed 3000W total for 900s
        for _ in 0..900 {
            dc.update([1000.0, 1000.0, 1000.0], 1.0, &t);
        }
        assert!((dc.current_demand_w - 3000.0).abs() < 0.01);
    }

    #[test]
    fn test_max_demand() {
        let mut dc = DemandCalculator::new(DemandMode::Sliding {
            window_min: 15,
            slide_min: 1,
        });
        let mut t = now();
        dc.update([1000.0, 1000.0, 1000.0], 60.0, &t);
        t = t + chrono::Duration::seconds(60);
        assert!(dc.max_demand_w > 0.0);
        assert!(dc.max_demand_timestamp.is_some());
    }

    #[test]
    fn test_reset_max() {
        let mut dc = DemandCalculator::new(DemandMode::Sliding {
            window_min: 1,
            slide_min: 1,
        });
        let t = now();
        dc.update([1000.0, 1000.0, 1000.0], 60.0, &t);
        dc.reset_max(&t);
        assert_eq!(dc.max_demand_w, 0.0);
    }
}
