//! 脉冲累计
//!
//! 模拟电能表脉冲输出：
//! - 有功/无功电能累计
//! - 脉冲常数 (imp/kWh)
//! - 脉冲频率计算
//! - LED/光耦脉冲输出模拟

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// 脉冲累加器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseAccumulator {
    /// 脉冲常数 (imp/kWh)
    pub constant: u32,
    /// 有功脉冲计数 A/B/C
    pub active_count: [u64; 3],
    /// 无功脉冲计数 A/B/C
    pub reactive_count: [u64; 3],
    /// 累计有功电能 (Wh)
    pub active_energy_wh: [f64; 3],
    /// 累计无功电能 (varh)
    pub reactive_energy_varh: [f64; 3],
    /// 合相有功电能 (Wh)
    pub active_total_wh: f64,
    /// 合相无功电能 (varh)
    pub reactive_total_varh: f64,
    /// 上次脉冲时间 (用于计算频率)
    #[serde(skip)]
    pub last_active_pulse_time: [Option<Instant>; 3],
    #[serde(skip)]
    pub last_reactive_pulse_time: [Option<Instant>; 3],
    /// 累计脉冲小数部分 (用于精确累计)
    #[serde(skip)]
    active_pulse_fraction: [f64; 3],
    #[serde(skip)]
    reactive_pulse_fraction: [f64; 3],
    /// 启动电流阈值 (A), 低于此值不计电能
    pub start_current: f64,
    /// 潜动阈值 (W), 无电流时功率低于此值不计
    pub creep_threshold: f64,
}

impl Default for PulseAccumulator {
    fn default() -> Self {
        Self {
            constant: 3200, // 默认 3200 imp/kWh
            active_count: [0; 3],
            reactive_count: [0; 3],
            active_energy_wh: [0.0; 3],
            reactive_energy_varh: [0.0; 3],
            active_total_wh: 0.0,
            reactive_total_varh: 0.0,
            last_active_pulse_time: [None; 3],
            last_reactive_pulse_time: [None; 3],
            active_pulse_fraction: [0.0; 3],
            reactive_pulse_fraction: [0.0; 3],
            start_current: 0.04,  // 40mA (0.4% Ib, Ib=10A)
            creep_threshold: 1.0, // 1W
        }
    }
}

impl PulseAccumulator {
    /// 创建新的脉冲累加器
    pub fn new(constant: u32) -> Self {
        Self {
            constant,
            ..Default::default()
        }
    }

    /// 设置脉冲常数
    pub fn set_constant(&mut self, constant: u32) {
        self.constant = constant;
    }

    /// 设置启动电流
    pub fn set_start_current(&mut self, current_a: f64) {
        self.start_current = current_a;
    }

    /// 设置潜动阈值
    pub fn set_creep_threshold(&mut self, threshold_w: f64) {
        self.creep_threshold = threshold_w;
    }

    /// 输入瞬时功率，累计电能
    ///
    /// # 参数
    /// - `p_w`: 有功功率数组 [A, B, C] (W)
    /// - `q_var`: 无功功率数组 [A, B, C] (var)
    /// - `dt_ms`: 时间间隔 (ms)
    /// - `currents`: 电流数组 [A, B, C] (A), 用于启动电流检测
    pub fn accumulate(&mut self, p_w: [f64; 3], q_var: [f64; 3], dt_ms: f64, currents: [f64; 3]) {
        if dt_ms <= 0.0 || self.constant == 0 {
            return;
        }

        let dt_hours = dt_ms / 3_600_000.0;

        for phase in 0..3 {
            // 启动电流检测：低于阈值不计电能
            if currents[phase] < self.start_current && p_w[phase].abs() < self.creep_threshold {
                continue;
            }

            // 有功电能累计
            let p_wh = p_w[phase] * dt_hours;
            self.active_energy_wh[phase] += p_wh;
            self.active_total_wh += p_wh;

            // 无功电能累计
            let q_varh = q_var[phase] * dt_hours;
            self.reactive_energy_varh[phase] += q_varh;
            self.reactive_total_varh += q_varh;

            // 脉冲计数 (带小数累计)
            let pulses_per_wh = self.constant as f64 / 1000.0; // imp/Wh
            let active_pulses = p_wh * pulses_per_wh;
            let reactive_pulses = q_varh * pulses_per_wh;

            // 累计小数部分
            self.active_pulse_fraction[phase] += active_pulses;
            self.reactive_pulse_fraction[phase] += reactive_pulses;

            // 提取整数脉冲
            let active_int = self.active_pulse_fraction[phase].floor() as u64;
            let reactive_int = self.reactive_pulse_fraction[phase].floor() as u64;

            if active_int > 0 {
                self.active_count[phase] += active_int;
                self.active_pulse_fraction[phase] -= active_int as f64;
            }
            if reactive_int > 0 {
                self.reactive_count[phase] += reactive_int;
                self.reactive_pulse_fraction[phase] -= reactive_int as f64;
            }
        }
    }

    /// 获取有功脉冲频率 (Hz)
    ///
    /// 基于当前功率和脉冲常数计算
    pub fn active_pulse_frequency(&self, power_w: f64) -> f64 {
        if self.constant == 0 {
            return 0.0;
        }
        // f = P (kW) * constant (imp/kWh) / 3600
        (power_w / 1000.0) * self.constant as f64 / 3600.0
    }

    /// 获取无功脉冲频率 (Hz)
    pub fn reactive_pulse_frequency(&self, power_var: f64) -> f64 {
        if self.constant == 0 {
            return 0.0;
        }
        (power_var / 1000.0) * self.constant as f64 / 3600.0
    }

    /// 检查是否产生新的有功脉冲
    ///
    /// 用于模拟 LED 闪烁或光耦输出
    pub fn check_active_pulse(&mut self, phase: usize) -> bool {
        if phase >= 3 {
            return false;
        }

        let now = Instant::now();
        let count = self.active_count[phase];

        if count > 0 {
            if let Some(last) = self.last_active_pulse_time[phase] {
                // 检查是否经过了一个脉冲周期
                let _elapsed = now.duration_since(last);
                // 简化：每次调用返回 true 表示有脉冲
                self.last_active_pulse_time[phase] = Some(now);
                return true;
            } else {
                self.last_active_pulse_time[phase] = Some(now);
                return true;
            }
        }
        false
    }

    /// 检查是否产生新的无功脉冲
    pub fn check_reactive_pulse(&mut self, phase: usize) -> bool {
        if phase >= 3 {
            return false;
        }

        let count = self.reactive_count[phase];
        count > 0
    }

    /// 获取有功电能 (kWh)
    pub fn active_energy_kwh(&self, phase: usize) -> f64 {
        if phase >= 3 {
            0.0
        } else {
            self.active_energy_wh[phase] / 1000.0
        }
    }

    /// 获取无功电能 (kvarh)
    pub fn reactive_energy_kvarh(&self, phase: usize) -> f64 {
        if phase >= 3 {
            0.0
        } else {
            self.reactive_energy_varh[phase] / 1000.0
        }
    }

    /// 获取合相有功电能 (kWh)
    pub fn active_total_kwh(&self) -> f64 {
        self.active_total_wh / 1000.0
    }

    /// 获取合相无功电能 (kvarh)
    pub fn reactive_total_kvarh(&self) -> f64 {
        self.reactive_total_varh / 1000.0
    }

    /// 重置电能累计
    pub fn reset(&mut self) {
        self.active_count = [0; 3];
        self.reactive_count = [0; 3];
        self.active_energy_wh = [0.0; 3];
        self.reactive_energy_varh = [0.0; 3];
        self.active_total_wh = 0.0;
        self.reactive_total_varh = 0.0;
        self.active_pulse_fraction = [0.0; 3];
        self.reactive_pulse_fraction = [0.0; 3];
        self.last_active_pulse_time = [None; 3];
        self.last_reactive_pulse_time = [None; 3];
    }

    /// 从寄存器值恢复电能
    pub fn restore_from_register(&mut self, phase: usize, active_wh: f64, reactive_varh: f64) {
        if phase >= 3 {
            return;
        }
        self.active_energy_wh[phase] = active_wh;
        self.reactive_energy_varh[phase] = reactive_varh;
        // 重新计算脉冲数
        let pulses_per_wh = self.constant as f64 / 1000.0;
        self.active_count[phase] = (active_wh * pulses_per_wh) as u64;
        self.reactive_count[phase] = (reactive_varh * pulses_per_wh) as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulate() {
        let mut pulse = PulseAccumulator::new(3200);
        let currents = [5.0, 5.0, 5.0];

        // 1kW 负载，运行 1 小时 (通过 100 次 36 秒累计)
        for _ in 0..100 {
            pulse.accumulate([1000.0, 0.0, 0.0], [0.0, 0.0, 0.0], 36000.0, currents);
        }

        // 应该累计 1 kWh
        assert!((pulse.active_energy_kwh(0) - 1.0).abs() < 0.01);
        // 脉冲数应该是 3200
        assert_eq!(pulse.active_count[0], 3200);
    }

    #[test]
    fn test_pulse_frequency() {
        let pulse = PulseAccumulator::new(3200);

        // 1kW 负载
        let freq = pulse.active_pulse_frequency(1000.0);
        // f = 1 kW * 3200 imp/kWh / 3600 s/h = 0.889 Hz
        assert!((freq - 0.889).abs() < 0.01);
    }

    #[test]
    fn test_start_current() {
        let mut pulse = PulseAccumulator::new(3200);
        pulse.set_start_current(0.1); // 100mA
        pulse.set_creep_threshold(5.0); // 5W

        // 低于启动电流，功率也低于潜动阈值
        let currents_low = [0.05, 0.05, 0.05];
        pulse.accumulate([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1000.0, currents_low);

        assert_eq!(
            pulse.active_energy_wh[0], 0.0,
            "Energy should not accumulate with low current and low power"
        );

        // 高于启动电流
        let currents_high = [5.0, 5.0, 5.0];
        pulse.accumulate([100.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1000.0, currents_high);

        assert!(
            pulse.active_energy_wh[0] > 0.0,
            "Energy should accumulate with high current"
        );
    }

    #[test]
    fn test_creep_threshold() {
        let mut pulse = PulseAccumulator::new(3200);
        pulse.set_creep_threshold(5.0); // 5W

        // 无电流，功率低于潜动阈值
        let currents = [0.0, 0.0, 0.0];
        pulse.accumulate([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1000.0, currents);

        assert_eq!(pulse.active_energy_wh[0], 0.0);

        // 无电流，功率高于潜动阈值（异常情况，仍计入）
        pulse.accumulate([10.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1000.0, currents);

        assert!(pulse.active_energy_wh[0] > 0.0);
    }

    #[test]
    fn test_reset() {
        let mut pulse = PulseAccumulator::new(3200);
        let currents = [5.0, 5.0, 5.0];
        pulse.accumulate([1000.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1000.0, currents);

        assert!(pulse.active_energy_wh[0] > 0.0);

        pulse.reset();

        assert_eq!(pulse.active_energy_wh[0], 0.0);
        assert_eq!(pulse.active_count[0], 0);
    }

    #[test]
    fn test_total_energy() {
        let mut pulse = PulseAccumulator::new(3200);
        let currents = [5.0, 5.0, 5.0];
        pulse.accumulate([1000.0, 2000.0, 3000.0], [0.0, 0.0, 0.0], 1000.0, currents);

        let total = pulse.active_total_wh;
        let expected = 1000.0 + 2000.0 + 3000.0; // 6kW * 1s = 1.67 Wh
        assert!((total - expected / 3600.0).abs() < 0.01);
    }
}
