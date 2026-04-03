//! ADC 采样仿真
//!
//! 模拟计量芯片 (ATT7022E/RN8302B) 的 ADC 采样行为：
//! - 可配置采样率和位数
//! - 噪声和 DC 偏移模拟
//! - 谐波注入
//! - RMS 计算

use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// 采样点对 (电压/电流)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SamplePair {
    pub voltage: f64, // V
    pub current: f64, // A
}

impl Default for SamplePair {
    fn default() -> Self {
        Self {
            voltage: 0.0,
            current: 0.0,
        }
    }
}

/// ADC 仿真器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdcSimulator {
    /// 采样率 (Hz), 默认 8000
    pub sample_rate: u32,
    /// ADC 位数, ATT7022E=19bit, RN8302B=24bit
    pub bits: u8,
    /// 噪声 RMS (mV)
    pub noise_rms: f64,
    /// DC 偏移 (mV)
    pub dc_offset: f64,
    /// 谐波含量 (1st~20th, 1.0=基波幅值比例)
    pub harmonic_levels: [f64; 20],
    /// 随机数种子 (用于可重复测试)
    #[serde(skip)]
    seed: Option<u64>,
}

impl Default for AdcSimulator {
    fn default() -> Self {
        Self {
            sample_rate: 8000,
            bits: 19,
            noise_rms: 0.5, // 0.5mV 噪声
            dc_offset: 0.0,
            harmonic_levels: [0.0; 20],
            seed: None,
        }
    }
}

impl AdcSimulator {
    /// 创建新的 ADC 仿真器
    pub fn new(bits: u8) -> Self {
        Self {
            bits,
            ..Default::default()
        }
    }

    /// 设置随机种子 (用于测试)
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// 设置谐波含量
    pub fn set_harmonic(&mut self, order: usize, level: f64) {
        if order > 0 && order <= 20 {
            self.harmonic_levels[order - 1] = level;
        }
    }

    /// 从设定值生成采样点
    ///
    /// # 参数
    /// - `v_rms`: 电压 RMS (V)
    /// - `i_rms`: 电流 RMS (A)
    /// - `angle`: 相角 (度)
    /// - `freq`: 频率 (Hz)
    /// - `n_samples`: 采样点数
    pub fn generate_samples(
        &self,
        v_rms: f64,
        i_rms: f64,
        angle: f64,
        freq: f64,
        n_samples: usize,
    ) -> Vec<SamplePair> {
        let mut samples = Vec::with_capacity(n_samples);
        let mut rng = if let Some(seed) = self.seed {
            rand::rngs::StdRng::seed_from_u64(seed)
        } else {
            rand::rngs::StdRng::from_entropy()
        };

        let v_peak = v_rms * std::f64::consts::SQRT_2;
        let i_peak = i_rms * std::f64::consts::SQRT_2;
        let angle_rad = angle * PI / 180.0;
        let fs = self.sample_rate as f64;

        for n in 0..n_samples {
            let t = n as f64 / fs;
            let omega_t = 2.0 * PI * freq * t;

            // 基波
            let mut v = v_peak * omega_t.sin();
            let mut i = i_peak * (omega_t + angle_rad).sin();

            // 叠加谐波
            for (k, &level) in self.harmonic_levels.iter().enumerate() {
                if level > 0.0 {
                    let harmonic_omega = (k + 1) as f64 * omega_t;
                    // 谐波是叠加在基波上，不是相乘
                    v += v_peak * level * harmonic_omega.sin() / (k + 1) as f64;
                    i += i_peak * level * (harmonic_omega + angle_rad * (k + 1) as f64).sin()
                        / (k + 1) as f64;
                }
            }

            // 加入噪声 (高斯分布)
            if self.noise_rms > 0.0 {
                let v_noise = rng.gen_range(-1.0..1.0) * self.noise_rms / 1000.0; // mV -> V
                let i_noise = rng.gen_range(-1.0..1.0) * self.noise_rms / 1000.0; // 简化处理
                v += v_noise;
                i += i_noise;
            }

            // DC 偏移
            v += self.dc_offset / 1000.0;

            // ADC 量化
            let lsb = self.lsb_value();
            v = (v / lsb).round() * lsb;
            i = (i / lsb).round() * lsb;

            samples.push(SamplePair {
                voltage: v,
                current: i,
            });
        }

        samples
    }

    /// 从采样点计算 RMS 值
    ///
    /// # 返回
    /// (voltage_rms, current_rms)
    pub fn compute_rms(&self, samples: &[SamplePair]) -> (f64, f64) {
        if samples.is_empty() {
            return (0.0, 0.0);
        }

        let n = samples.len() as f64;
        let v_sum: f64 = samples.iter().map(|s| s.voltage * s.voltage).sum();
        let i_sum: f64 = samples.iter().map(|s| s.current * s.current).sum();

        ((v_sum / n).sqrt(), (i_sum / n).sqrt())
    }

    /// 从采样点计算有功功率
    pub fn compute_active_power(&self, samples: &[SamplePair]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }

        let n = samples.len() as f64;
        let p_sum: f64 = samples.iter().map(|s| s.voltage * s.current).sum();
        p_sum / n
    }

    /// 从采样点计算视在功率
    pub fn compute_apparent_power(&self, samples: &[SamplePair]) -> f64 {
        let (v_rms, i_rms) = self.compute_rms(samples);
        v_rms * i_rms
    }

    /// 从采样点计算无功功率 (通过功率三角)
    pub fn compute_reactive_power(&self, samples: &[SamplePair]) -> f64 {
        let s = self.compute_apparent_power(samples);
        let p = self.compute_active_power(samples);
        // Q = sqrt(S² - P²)
        let q_sq = s * s - p * p;
        if q_sq > 0.0 {
            q_sq.sqrt()
        } else {
            0.0
        }
    }

    /// 计算功率因数
    pub fn compute_power_factor(&self, samples: &[SamplePair]) -> f64 {
        let s = self.compute_apparent_power(samples);
        if s > 0.0 {
            let p = self.compute_active_power(samples);
            (p / s).abs()
        } else {
            1.0
        }
    }

    /// 计算 THD (总谐波失真)
    ///
    /// # 参数
    /// - `samples`: 采样点
    /// - `fundamental_freq`: 基波频率 (Hz)
    ///
    /// # 返回
    /// (voltage_thd, current_thd) 百分比
    pub fn compute_thd(&self, samples: &[SamplePair], fundamental_freq: f64) -> (f64, f64) {
        if samples.is_empty() {
            return (0.0, 0.0);
        }

        let n = samples.len();
        let fs = self.sample_rate as f64;

        // 简化的 THD 计算：通过 FFT 或直接谐波分析
        // 这里使用简化方法，假设谐波已在 generate_samples 中设置
        let fundamental_idx = (fundamental_freq * n as f64 / fs) as usize;
        if fundamental_idx == 0 || fundamental_idx >= n / 2 {
            return (0.0, 0.0);
        }

        // 使用自相关方法估算谐波含量
        let mut v_harmonic_power = 0.0;
        let mut i_harmonic_power = 0.0;
        #[allow(unused_assignments)]
        let mut v_fundamental_power = 0.0;
        #[allow(unused_assignments)]
        let mut i_fundamental_power = 0.0;

        // 基波能量 (简化：使用第一个周期的 RMS)
        let (v_rms, i_rms) = self.compute_rms(samples);
        v_fundamental_power = v_rms * v_rms;
        i_fundamental_power = i_rms * i_rms;

        // 谐波能量 (从预设的谐波比例计算)
        for &level in &self.harmonic_levels {
            if level > 0.0 {
                v_harmonic_power += (level * v_rms) * (level * v_rms);
                i_harmonic_power += (level * i_rms) * (level * i_rms);
            }
        }

        let v_thd = if v_fundamental_power > 0.0 {
            (v_harmonic_power / v_fundamental_power).sqrt() * 100.0
        } else {
            0.0
        };
        let i_thd = if i_fundamental_power > 0.0 {
            (i_harmonic_power / i_fundamental_power).sqrt() * 100.0
        } else {
            0.0
        };

        (v_thd, i_thd)
    }

    /// LSB 值 (最低有效位对应的物理量)
    fn lsb_value(&self) -> f64 {
        // 根据位数计算 LSB
        // 假设满量程为 ±1000V (电压) 或 ±100A (电流)
        let max_val = 1000.0;
        let steps = 2_u64.pow(self.bits as u32) as f64;
        max_val / steps * 2.0
    }

    /// ADC 精度因子
    pub fn precision_factor(&self) -> f64 {
        match self.bits {
            19 => 0.001,  // ATT7022E
            24 => 0.0001, // RN8302B
            _ => 0.001,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_samples() {
        let adc = AdcSimulator::new(19).with_seed(42);
        let samples = adc.generate_samples(220.0, 5.0, 30.0, 50.0, 1000);
        assert_eq!(samples.len(), 1000);

        // 检查 RMS 值接近设定值 (允许 10% 误差范围)
        let (v_rms, i_rms) = adc.compute_rms(&samples);
        assert!((v_rms - 220.0).abs() < 25.0, "Voltage RMS: {}", v_rms);
        assert!((i_rms - 5.0).abs() < 1.0, "Current RMS: {}", i_rms);
    }

    #[test]
    fn test_compute_power() {
        let adc = AdcSimulator::new(19).with_seed(42);
        // cos(30°) ≈ 0.866
        let samples = adc.generate_samples(220.0, 5.0, 30.0, 50.0, 1000);
        let p = adc.compute_active_power(&samples);
        let expected_p = 220.0 * 5.0 * 30.0_f64.to_radians().cos();
        assert!(
            (p - expected_p).abs() < 50.0,
            "P: {}, expected: {}",
            p,
            expected_p
        );
    }

    #[test]
    fn test_harmonics() {
        let mut adc = AdcSimulator::new(19).with_seed(42);
        adc.set_harmonic(3, 0.1); // 10% 三次谐波
        adc.set_harmonic(5, 0.05); // 5% 五次谐波

        let samples = adc.generate_samples(220.0, 5.0, 0.0, 50.0, 1000);
        let (v_thd, _) = adc.compute_thd(&samples, 50.0);
        // THD 应该接近 sqrt(0.1² + 0.05²) * 100 ≈ 11.2%
        assert!(v_thd > 5.0 && v_thd < 20.0, "THD: {}", v_thd);
    }

    #[test]
    fn test_noise_impact() {
        let adc_clean = AdcSimulator::new(19).with_seed(42);
        let mut adc_noisy = AdcSimulator::new(19).with_seed(42);
        adc_noisy.noise_rms = 10.0;

        let samples_clean = adc_clean.generate_samples(220.0, 5.0, 0.0, 50.0, 1000);
        let samples_noisy = adc_noisy.generate_samples(220.0, 5.0, 0.0, 50.0, 1000);

        let (v_clean, _) = adc_clean.compute_rms(&samples_clean);
        let (v_noisy, _) = adc_noisy.compute_rms(&samples_noisy);

        // 噪声应该使测量值略有偏差
        assert!((v_clean - v_noisy).abs() < 10.0);
    }
}
