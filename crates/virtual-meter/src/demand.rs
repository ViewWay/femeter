//! 需量测量 (Demand Measurement)
//!
//! 滑差窗口需量计算, 最大需量记录

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemandWindow {
    Min5 = 5,
    Min10 = 10,
    Min15 = 15,
    Min30 = 30,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseDemand {
    pub p: f64,
    pub q: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandRecord {
    pub timestamp: DateTime<Utc>,
    pub p_a: f64,
    pub p_b: f64,
    pub p_c: f64,
    p_total: f64,
    pub q_a: f64,
    pub q_b: f64,
    pub q_c: f64,
    q_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxDemandRecord {
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandExceedEvent {
    pub threshold: f64,
    pub actual: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandMeter {
    window: DemandWindow,
    /// 功率采样历史 (samples at ~1s intervals)
    p_samples: Vec<f64>,
    q_samples: Vec<f64>,
    /// 当前需量
    current_p: f64,
    current_q: f64,
    /// 各相当前需量
    phase_a: PhaseDemand,
    phase_b: PhaseDemand,
    phase_c: PhaseDemand,
    p_samples_a: Vec<f64>,
    p_samples_b: Vec<f64>,
    p_samples_c: Vec<f64>,
    q_samples_a: Vec<f64>,
    q_samples_b: Vec<f64>,
    q_samples_c: Vec<f64>,
    /// 最大需量
    max_p: MaxDemandRecord,
    max_q: MaxDemandRecord,
    /// 需量超限阈值 (kW), None=不限
    threshold: Option<f64>,
    pub events: Vec<DemandExceedEvent>,
}

impl Default for DemandMeter {
    fn default() -> Self {
        Self {
            window: DemandWindow::Min15,
            p_samples: Vec::new(),
            q_samples: Vec::new(),
            current_p: 0.0,
            current_q: 0.0,
            phase_a: PhaseDemand::default(),
            phase_b: PhaseDemand::default(),
            phase_c: PhaseDemand::default(),
            p_samples_a: Vec::new(),
            p_samples_b: Vec::new(),
            p_samples_c: Vec::new(),
            q_samples_a: Vec::new(),
            q_samples_b: Vec::new(),
            q_samples_c: Vec::new(),
            max_p: MaxDemandRecord {
                value: 0.0,
                timestamp: Utc::now(),
            },
            max_q: MaxDemandRecord {
                value: 0.0,
                timestamp: Utc::now(),
            },
            threshold: None,
            events: Vec::new(),
        }
    }
}

impl DemandMeter {
    pub fn new(window: DemandWindow) -> Self {
        Self {
            window,
            ..Self::default()
        }
    }

    pub fn window(&self) -> DemandWindow {
        self.window
    }
    pub fn set_window(&mut self, w: DemandWindow) {
        self.window = w;
        self.p_samples.clear();
        self.q_samples.clear();
        self.p_samples_a.clear();
        self.p_samples_b.clear();
        self.p_samples_c.clear();
        self.q_samples_a.clear();
        self.q_samples_b.clear();
        self.q_samples_c.clear();
    }
    pub fn set_threshold(&mut self, t: Option<f64>) {
        self.threshold = t;
    }
    pub fn current_p(&self) -> f64 {
        self.current_p
    }
    pub fn current_q(&self) -> f64 {
        self.current_q
    }
    pub fn max_p(&self) -> &MaxDemandRecord {
        &self.max_p
    }
    pub fn max_q(&self) -> &MaxDemandRecord {
        &self.max_q
    }

    /// 添加功率采样 (调用频率 ~1s)
    pub fn sample(&mut self, p_a: f64, p_b: f64, p_c: f64, q_a: f64, q_b: f64, q_c: f64) {
        let max_samples = self.window as usize * 60;
        let p_total = p_a + p_b + p_c;
        let q_total = q_a + q_b + q_c;

        self.p_samples.push(p_total);
        self.q_samples.push(q_total);
        self.p_samples_a.push(p_a);
        self.p_samples_b.push(p_b);
        self.p_samples_c.push(p_c);
        self.q_samples_a.push(q_a);
        self.q_samples_b.push(q_b);
        self.q_samples_c.push(q_c);

        // trim
        if self.p_samples.len() > max_samples {
            let excess = self.p_samples.len() - max_samples;
            self.p_samples.drain(..excess);
            self.q_samples.drain(..excess);
            self.p_samples_a.drain(..excess);
            self.p_samples_b.drain(..excess);
            self.p_samples_c.drain(..excess);
            self.q_samples_a.drain(..excess);
            self.q_samples_b.drain(..excess);
            self.q_samples_c.drain(..excess);
        }

        // calculate demand = average power over window
        let n = self.p_samples.len() as f64;
        if n > 0.0 {
            let sum_p: f64 = self.p_samples.iter().sum();
            let sum_q: f64 = self.q_samples.iter().sum();
            self.current_p = sum_p / n;
            self.current_q = sum_q / n;
            self.phase_a = PhaseDemand {
                p: self.p_samples_a.iter().sum::<f64>() / n,
                q: self.q_samples_a.iter().sum::<f64>() / n,
            };
            self.phase_b = PhaseDemand {
                p: self.p_samples_b.iter().sum::<f64>() / n,
                q: self.q_samples_b.iter().sum::<f64>() / n,
            };
            self.phase_c = PhaseDemand {
                p: self.p_samples_c.iter().sum::<f64>() / n,
                q: self.q_samples_c.iter().sum::<f64>() / n,
            };

            if self.current_p > self.max_p.value {
                self.max_p = MaxDemandRecord {
                    value: self.current_p,
                    timestamp: Utc::now(),
                };
            }
            if self.current_q > self.max_q.value {
                self.max_q = MaxDemandRecord {
                    value: self.current_q,
                    timestamp: Utc::now(),
                };
            }

            // threshold check (in kW)
            if let Some(th) = self.threshold {
                if self.current_p > th {
                    self.events.push(DemandExceedEvent {
                        threshold: th,
                        actual: self.current_p,
                        timestamp: Utc::now(),
                    });
                }
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Self {
            window: self.window,
            threshold: self.threshold,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demand_basic() {
        let mut dm = DemandMeter::new(DemandWindow::Min5);
        for _ in 0..10 {
            dm.sample(1000.0, 1000.0, 1000.0, 200.0, 200.0, 200.0);
        }
        assert!((dm.current_p() - 3000.0).abs() < 0.01);
        assert!((dm.current_q() - 600.0).abs() < 0.01);
    }

    #[test]
    fn test_max_demand() {
        let mut dm = DemandMeter::new(DemandWindow::Min5);
        dm.sample(100.0, 100.0, 100.0, 0.0, 0.0, 0.0);
        assert!(dm.max_p().value > 0.0);
    }
}
