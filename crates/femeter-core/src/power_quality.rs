/* ================================================================== */
/*                                                                    */
/*  power_quality.rs — 智能电能质量分析                                 */
/*                                                                    */
/*  谐波分析(THD 2~50次)、电压暂降/暂升(GB/T 30137)、                 */
/*  电压波动与闪变(IEC 61000-4-15简化)、三相不平衡(GB/T 15543)、      */
/*  功率因数校正建议、电能质量事件记录。                                 */
/*                                                                    */
/*  嵌入式友好: 无堆分配, 所有分析基于固定大小数组。                      */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

/// 最大谐波次数
pub const MAX_HARMONIC_ORDER: usize = 50;
/// 电压暂降阈值 (GB/T 30137): 电压降至额定值的 10%~90%
pub const SAG_THRESHOLD_PU: f32 = 0.90;
/// 电压暂升阈值: 电压升至额定值的 110%
pub const SWELL_THRESHOLD_PU: f32 = 1.10;
/// 三相不平衡度限值 (GB/T 15543): 正常 ≤2%, 短时 ≤4%
pub const UNBALANCE_NORMAL_LIMIT: f32 = 0.02;
pub const UNBALANCE_SHORT_TERM_LIMIT: f32 = 0.04;
/// 功率因数合格阈值
pub const PF_GOOD_THRESHOLD: f32 = 0.90;
/// 功率因数优秀阈值
pub const PF_EXCELLENT_THRESHOLD: f32 = 0.95;
/// 闪变短期严重度限值 (IEC 61000-4-15)
pub const FLICKER_PST_LIMIT: f32 = 1.0;
/// 闪变长期严重度限值
pub const FLICKER_PLT_LIMIT: f32 = 0.65;

// ── 谐波分析 ──

/// 谐波分析结果 (固定大小, 嵌入式友好)
#[derive(Clone, Copy, Debug)]
pub struct HarmonicAnalysis {
    /// 各次谐波含量 (%) — [0] 未使用, [1]=基波, [2..50]=2~49次
    pub harmonics: [f32; MAX_HARMONIC_ORDER],
    /// 总谐波畸变率 THD (%)
    pub thd: f32,
}

impl Default for HarmonicAnalysis {
    fn default() -> Self {
        Self {
            harmonics: [0.0; MAX_HARMONIC_ORDER],
            thd: 0.0,
        }
    }
}

/// 计算总谐波畸变率 (THD)
///
/// THD = sqrt(sum(H_n^2)) / H_1 × 100%
///
/// # Arguments
/// * `harmonics` - 谐波幅值数组, [0]=基波(或DC), [1..]=各次谐波
/// * `fundamental_idx` - 基波在数组中的索引
/// * `count` - 有效谐波数量
pub fn calculate_thd(harmonics: &[f32], fundamental_idx: usize, count: usize) -> f32 {
    if fundamental_idx >= count || count == 0 {
        return 0.0;
    }
    let fundamental = harmonics[fundamental_idx].abs();
    if fundamental < 1e-10 {
        return 0.0;
    }
    let mut sum_sq: f32 = 0.0;
    for (i, &h) in harmonics.iter().enumerate().take(count) {
        if i != fundamental_idx {
            sum_sq += h * h;
        }
    }
    (sum_sq.sqrt() / fundamental) * 100.0
}

/// 执行完整谐波分析 (2~50次)
///
/// 接受归一化谐波幅值 (相对于基波), 计算 THD。
pub fn analyze_harmonics(harmonic_amplitudes: &[f32; MAX_HARMONIC_ORDER]) -> HarmonicAnalysis {
    let mut result = HarmonicAnalysis::default();
    let fundamental = harmonic_amplitudes[0]; // 基波
    if fundamental.abs() < 1e-10 {
        return result;
    }
    for (i, &amp) in harmonic_amplitudes.iter().enumerate() {
        result.harmonics[i] = if fundamental.abs() > 1e-10 {
            (amp.abs() / fundamental.abs()) * 100.0
        } else {
            0.0
        };
    }
    result.thd = calculate_thd(harmonic_amplitudes, 0, MAX_HARMONIC_ORDER);
    result
}

// ── 电压暂降/暂升检测 (GB/T 30137) ──

/// 电压事件类型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum VoltageEventType {
    #[default]
    Normal = 0,
    Sag = 1,          // 电压暂降
    Swell = 2,        // 电压暂升
    Interruption = 3, // 电压中断 (<10% Un)
    Overvoltage = 4,  // 持续过压
    Undervoltage = 5, // 持续欠压
}

/// 电压暂降/暂升事件
#[derive(Clone, Copy, Debug, Default)]
pub struct VoltageEvent {
    pub event_type: VoltageEventType,
    /// 暂降/暂升开始时间 (相对 ms)
    pub start_ms: u32,
    /// 持续时间 ms
    pub duration_ms: u32,
    /// 残余电压 (p.u., 即 Un 的百分比 / 100)
    pub residual_pu: f32,
    /// 相别: 0=A, 1=B, 2=C
    pub phase: u8,
}

/// 电压事件检测器
#[derive(Clone, Debug)]
pub struct VoltageEventDetector {
    /// 额定电压 (0.01V 单位)
    pub rated_voltage: u16,
    /// 各相当前状态
    pub phase_state: [VoltageEventType; 3],
    /// 各相事件开始时间
    pub phase_start: [u32; 3],
    /// 事件缓冲区
    pub events: [Option<VoltageEvent>; 16],
    pub event_count: usize,
}

impl VoltageEventDetector {
    pub fn new(rated_voltage_u16: u16) -> Self {
        Self {
            rated_voltage: rated_voltage_u16,
            phase_state: [VoltageEventType::Normal; 3],
            phase_start: [0; 3],
            events: [None; 16],
            event_count: 0,
        }
    }

    /// 检测三相电压事件
    ///
    /// # Arguments
    /// * `voltages` - [A, B, C] 相电压 (0.01V)
    /// * `timestamp_ms` - 当前时间戳 ms
    pub fn check(&mut self, voltages: [u16; 3], timestamp_ms: u32) -> Option<VoltageEvent> {
        let mut result = None;
        for (phase, &v) in voltages.iter().enumerate() {
            let pu = v as f32 / self.rated_voltage as f32;
            let new_type = if pu < 0.10 {
                VoltageEventType::Interruption
            } else if pu < SAG_THRESHOLD_PU {
                VoltageEventType::Sag
            } else if pu > SWELL_THRESHOLD_PU && pu < 1.20 {
                VoltageEventType::Swell
            } else if pu >= 1.20 {
                VoltageEventType::Overvoltage
            } else if pu < 0.90 {
                VoltageEventType::Undervoltage
            } else {
                VoltageEventType::Normal
            };

            if new_type != VoltageEventType::Normal
                && self.phase_state[phase] == VoltageEventType::Normal
            {
                // 事件开始
                self.phase_state[phase] = new_type;
                self.phase_start[phase] = timestamp_ms;
            } else if new_type == VoltageEventType::Normal
                && self.phase_state[phase] != VoltageEventType::Normal
            {
                // 事件结束 — 记录
                let event = VoltageEvent {
                    event_type: self.phase_state[phase],
                    start_ms: self.phase_start[phase],
                    duration_ms: timestamp_ms - self.phase_start[phase],
                    residual_pu: 0.0, // 已恢复
                    phase: phase as u8,
                };
                if self.event_count < self.events.len() {
                    self.events[self.event_count] = Some(event);
                    result = Some(event);
                    self.event_count += 1;
                }
                self.phase_state[phase] = VoltageEventType::Normal;
            }
        }
        result
    }
}

// ── 电压波动与闪变 (IEC 61000-4-15 简化) ──

/// 闪变分析器 (简化模型)
///
/// 基于半波 RMS 电压变化计算瞬时闪变视感度,
/// 简化 IEC 61000-4-15 的多级滤波链。
#[derive(Clone, Debug)]
pub struct FlickerAnalyzer {
    /// 电压 RMS 窗口 (半周期, 10ms)
    pub rms_window: [f32; 10],
    pub rms_pos: usize,
    pub rms_count: usize,
    /// 短期闪变严重度 Pst 累加 (2min 窗口)
    pub pst_sum: f32,
    pub pst_count: u32,
    /// 长期闪变严重度 Plt 累加 (12个Pst)
    pub plt_values: [f32; 12],
    pub plt_pos: usize,
    pub plt_count: u32,
}

impl FlickerAnalyzer {
    pub fn new() -> Self {
        Self {
            rms_window: [0.0; 10],
            rms_pos: 0,
            rms_count: 0,
            pst_sum: 0.0,
            pst_count: 0,
            plt_values: [0.0; 12],
            plt_pos: 0,
            plt_count: 0,
        }
    }

    /// 输入一个半波 RMS 电压值 (p.u.)
    pub fn feed_half_cycle_rms(&mut self, rms_pu: f32) {
        self.rms_window[self.rms_pos] = rms_pu;
        self.rms_pos = (self.rms_pos + 1) % self.rms_window.len();
        if self.rms_count < self.rms_window.len() {
            self.rms_count += 1;
        }
    }

    /// 计算瞬时闪变视感度 d (简化)
    pub fn instantaneous_flicker(&self) -> f32 {
        if self.rms_count < 2 {
            return 0.0;
        }
        let mean = {
            let mut s = 0.0f32;
            for i in 0..self.rms_count {
                s += self.rms_window[i];
            }
            s / self.rms_count as f32
        };
        if mean < 1e-10 {
            return 0.0;
        }
        // 简化: 相对电压变化 d = ΔU / U_mean
        let mut max_delta = 0.0f32;
        for i in 0..self.rms_count {
            let d = ((self.rms_window[i] - mean) / mean).abs();
            if d > max_delta {
                max_delta = d;
            }
        }
        max_delta
    }

    /// 获取短期闪变严重度 Pst (简化)
    pub fn pst(&self) -> f32 {
        if self.pst_count == 0 {
            return 0.0;
        }
        self.pst_sum / self.pst_count as f32
    }

    /// 获取长期闪变严重度 Plt (立方根平均)
    pub fn plt(&self) -> f32 {
        if self.plt_count == 0 {
            return 0.0;
        }
        let mut sum = 0.0f32;
        for i in 0..self.plt_count as usize {
            sum += self.plt_values[i].powf(3.0);
        }
        (sum / self.plt_count as f32).powf(1.0 / 3.0)
    }

    /// 完成一个 Pst 评估周期 (约 2min), 调用后累加到 Plt
    pub fn complete_pst_period(&mut self, pst_value: f32) {
        self.pst_sum += pst_value;
        self.pst_count += 1;
        self.plt_values[self.plt_pos] = pst_value;
        self.plt_pos = (self.plt_pos + 1) % self.plt_values.len();
        if self.plt_count < 12 {
            self.plt_count += 1;
        }
    }
}

impl Default for FlickerAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ── 三相不平衡度 (GB/T 15543) ──

/// 三相不平衡分析结果
#[derive(Clone, Copy, Debug, Default)]
pub struct UnbalanceResult {
    /// 电压不平衡度 (%)
    pub voltage_unbalance: f32,
    /// 电流不平衡度 (%)
    pub current_unbalance: f32,
    /// 最大偏差相
    pub max_deviation_phase: u8,
    /// 是否超标
    pub is_abnormal: bool,
}

/// 计算三相不平衡度 (对称分量法简化)
///
/// GB/T 15543 定义: ε = max(|U_a - U_avg|, |U_b - U_avg|, |U_c - U_avg|) / U_avg
pub fn calculate_unbalance(values: [f32; 3]) -> UnbalanceResult {
    let avg = (values[0] + values[1] + values[2]) / 3.0;
    if avg < 1e-10 {
        return UnbalanceResult::default();
    }
    let devs = [
        (values[0] - avg).abs(),
        (values[1] - avg).abs(),
        (values[2] - avg).abs(),
    ];
    let max_dev = devs[0].max(devs[1]).max(devs[2]);
    let unbalance = max_dev / avg;
    let mut max_phase = 0u8;
    if devs[1] > devs[0] && devs[1] >= devs[2] {
        max_phase = 1;
    } else if devs[2] > devs[0] && devs[2] > devs[1] {
        max_phase = 2;
    }
    UnbalanceResult {
        voltage_unbalance: unbalance * 100.0,
        current_unbalance: unbalance * 100.0, // 复用, 调用者区分
        max_deviation_phase: max_phase,
        is_abnormal: unbalance > UNBALANCE_NORMAL_LIMIT,
    }
}

// ── 功率因数校正建议 ──

/// 功率因数校正建议
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum PfCorrectionAdvice {
    /// 功率因数优秀, 无需校正
    #[default]
    Excellent = 0,
    /// 功率因数良好, 建议监控
    Good = 1,
    /// 功率因数偏低, 建议安装无功补偿
    CompensationNeeded = 2,
    /// 功率因数严重偏低, 需立即补偿
    UrgentCompensation = 3,
    /// 容性过补偿
    OverCompensated = 4,
}

/// 功率因数分析结果
#[derive(Clone, Copy, Debug)]
pub struct PfAnalysis {
    /// 总功率因数
    pub pf_total: f32,
    /// A 相功率因数
    pub pf_a: f32,
    /// B 相功率因数
    pub pf_b: f32,
    /// C 相功率因数
    pub pf_c: f32,
    /// 建议
    pub advice: PfCorrectionAdvice,
    /// 建议补偿容量 (kvar, 近似)
    pub suggested_kvar: f32,
}

impl Default for PfAnalysis {
    fn default() -> Self {
        Self {
            pf_total: 0.0,
            pf_a: 0.0,
            pf_b: 0.0,
            pf_c: 0.0,
            advice: PfCorrectionAdvice::default(),
            suggested_kvar: 0.0,
        }
    }
}

/// 分析功率因数并给出校正建议
///
/// # Arguments
/// * `pf_values` - [total, A, B, C] 功率因数 (0.001 单位, 如 950 = 0.950)
/// * `active_power_kw` - 有功功率 (kW)
pub fn analyze_power_factor(pf_values: [u16; 4], active_power_kw: f32) -> PfAnalysis {
    let pf_total = pf_values[0] as f32 / 1000.0;
    let pf_a = pf_values[1] as f32 / 1000.0;
    let pf_b = pf_values[2] as f32 / 1000.0;
    let pf_c = pf_values[3] as f32 / 1000.0;

    let (advice, suggested_kvar) = if pf_total < 0.70 {
        // 容性或严重滞后
        let q = if pf_total > 1e-10 {
            active_power_kw * (1.0 / pf_total - 1.0).sqrt()
        } else {
            0.0
        };
        (PfCorrectionAdvice::UrgentCompensation, q)
    } else if pf_total < PF_GOOD_THRESHOLD {
        let q = if pf_total > 1e-10 {
            active_power_kw * (1.0 / pf_total - 1.0).sqrt()
        } else {
            0.0
        };
        (PfCorrectionAdvice::CompensationNeeded, q)
    } else if pf_total > 1.0 && pf_total < 1.05 {
        // 轻微过补偿
        (PfCorrectionAdvice::OverCompensated, 0.0)
    } else if pf_total >= PF_EXCELLENT_THRESHOLD {
        (PfCorrectionAdvice::Excellent, 0.0)
    } else {
        (PfCorrectionAdvice::Good, 0.0)
    };

    PfAnalysis {
        pf_total,
        pf_a,
        pf_b,
        pf_c,
        advice,
        suggested_kvar,
    }
}

// ── 电能质量事件记录 ──

/// 电能质量事件类别
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum PowerQualityEventType {
    #[default]
    None = 0,
    VoltageSag = 1,
    VoltageSwell = 2,
    VoltageInterruption = 3,
    HarmonicExceed = 4,
    UnbalanceExceed = 5,
    FlickerExceed = 6,
    LowPowerFactor = 7,
    FrequencyDeviation = 8,
}

/// 电能质量事件记录
#[derive(Clone, Copy, Debug, Default)]
pub struct PowerQualityEvent {
    pub event_type: PowerQualityEventType,
    /// 事件发生时间戳 (unix s)
    pub timestamp: u32,
    /// 关联数值 (如 THD%, 不平衡度%, 电压 p.u.)
    pub value: f32,
    /// 相别
    pub phase: u8,
    /// 持续时间 ms
    pub duration_ms: u32,
}

/// 电能质量监测器 — 综合分析入口
#[derive(Clone, Debug)]
pub struct PowerQualityMonitor {
    pub voltage_detector: VoltageEventDetector,
    pub flicker: FlickerAnalyzer,
    /// THD 超限阈值 (%)
    pub thd_limit: f32,
    /// 事件日志
    pub events: [Option<PowerQualityEvent>; 32],
    pub event_count: usize,
}

impl PowerQualityMonitor {
    pub fn new(rated_voltage: u16) -> Self {
        Self {
            voltage_detector: VoltageEventDetector::new(rated_voltage),
            flicker: FlickerAnalyzer::new(),
            thd_limit: 5.0, // THD ≤5% 为合格 (GB/T 14549)
            events: [None; 32],
            event_count: 0,
        }
    }

    /// 记录事件
    fn record_event(&mut self, event: PowerQualityEvent) {
        if self.event_count < self.events.len() {
            self.events[self.event_count] = Some(event);
            self.event_count += 1;
        }
    }

    /// 综合电能质量检查
    pub fn check(
        &mut self,
        voltages: [u16; 3],
        _currents: [u16; 3],
        pf_total: u16,
        frequency: u16,
        timestamp: u32,
    ) -> Vec<PowerQualityEvent> {
        let mut found = Vec::new();

        // 1. 电压暂降/暂升
        if let Some(ve) = self
            .voltage_detector
            .check(voltages, timestamp.wrapping_mul(1000))
        {
            let pqe = PowerQualityEvent {
                event_type: match ve.event_type {
                    VoltageEventType::Sag => PowerQualityEventType::VoltageSag,
                    VoltageEventType::Swell => PowerQualityEventType::VoltageSwell,
                    VoltageEventType::Interruption => PowerQualityEventType::VoltageInterruption,
                    _ => PowerQualityEventType::None,
                },
                timestamp,
                value: voltages[ve.phase as usize] as f32
                    / self.voltage_detector.rated_voltage as f32,
                phase: ve.phase,
                duration_ms: ve.duration_ms,
            };
            self.record_event(pqe);
            found.push(pqe);
        }

        // 2. 三相不平衡
        let v_vals = [voltages[0] as f32, voltages[1] as f32, voltages[2] as f32];
        let ub = calculate_unbalance(v_vals);
        if ub.is_abnormal {
            let pqe = PowerQualityEvent {
                event_type: PowerQualityEventType::UnbalanceExceed,
                timestamp,
                value: ub.voltage_unbalance,
                phase: ub.max_deviation_phase,
                duration_ms: 0,
            };
            self.record_event(pqe);
            found.push(pqe);
        }

        // 3. 低功率因数
        let pf = pf_total as f32 / 1000.0;
        if pf < PF_GOOD_THRESHOLD && pf > 0.01 {
            let pqe = PowerQualityEvent {
                event_type: PowerQualityEventType::LowPowerFactor,
                timestamp,
                value: pf * 100.0,
                phase: 0,
                duration_ms: 0,
            };
            self.record_event(pqe);
            found.push(pqe);
        }

        // 4. 频率偏差 (50Hz ±0.5Hz)
        let freq = frequency as f32 / 100.0;
        if (freq - 50.0).abs() > 0.5 {
            let pqe = PowerQualityEvent {
                event_type: PowerQualityEventType::FrequencyDeviation,
                timestamp,
                value: freq,
                phase: 0,
                duration_ms: 0,
            };
            self.record_event(pqe);
            found.push(pqe);
        }

        found
    }
}

// ══════════════════════════════════════════════════════════════════
//  单元测试
// ══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thd_zero_harmonics() {
        let h = [0.0; MAX_HARMONIC_ORDER];
        assert!((calculate_thd(&h, 0, MAX_HARMONIC_ORDER) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_thd_only_fundamental() {
        let mut h = [0.0; MAX_HARMONIC_ORDER];
        h[0] = 220.0;
        assert!((calculate_thd(&h, 0, 10) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_thd_known_value() {
        let mut h = [0.0; MAX_HARMONIC_ORDER];
        h[0] = 220.0; // 基波
        h[2] = 22.0; // 3次谐波 10%
        let thd = calculate_thd(&h, 0, 10);
        assert!((thd - 10.0).abs() < 0.1, "THD should be ~10%, got {}", thd);
    }

    #[test]
    fn test_thd_multiple_harmonics() {
        let mut h = [0.0; MAX_HARMONIC_ORDER];
        h[0] = 100.0;
        h[1] = 20.0; // 2次
        h[2] = 15.0; // 3次
        h[4] = 10.0; // 5次
        let thd = calculate_thd(&h, 0, 10);
        let expected =
            (20.0_f32.powi(2) + 15.0_f32.powi(2) + 10.0_f32.powi(2)).sqrt() / 100.0 * 100.0;
        assert!((thd - expected).abs() < 0.1);
    }

    #[test]
    fn test_analyze_harmonics_full() {
        let mut h = [0.0; MAX_HARMONIC_ORDER];
        h[0] = 220.0;
        h[2] = 11.0; // 3次 5%
        h[4] = 4.4; // 5次 2%
        let result = analyze_harmonics(&h);
        assert!((result.harmonics[2] - 5.0).abs() < 0.1);
        assert!((result.harmonics[4] - 2.0).abs() < 0.1);
        assert!((result.harmonics[0] - 100.0).abs() < 0.1); // 基波 100%
        assert!(result.thd > 0.0);
    }

    #[test]
    fn test_analyze_harmonics_zero_fundamental() {
        let h = [0.0; MAX_HARMONIC_ORDER];
        let result = analyze_harmonics(&h);
        assert!((result.thd - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_voltage_event_normal() {
        let mut det = VoltageEventDetector::new(22000);
        let ev = det.check([22000, 22000, 22000], 1000);
        assert!(ev.is_none());
    }

    #[test]
    fn test_voltage_sag_detection() {
        let mut det = VoltageEventDetector::new(22000);
        // 暂降: 19800V = 90% → 刚好在边界
        det.check([19500, 22000, 22000], 1000);
        // 恢复正常
        let ev = det.check([22000, 22000, 22000], 2000);
        assert!(ev.is_some());
        let e = ev.unwrap();
        assert_eq!(e.event_type, VoltageEventType::Sag);
        assert_eq!(e.duration_ms, 1000);
        assert_eq!(e.phase, 0);
    }

    #[test]
    fn test_voltage_swell_detection() {
        let mut det = VoltageEventDetector::new(22000);
        det.check([24400, 22000, 22000], 1000); // 111% → swell
        let ev = det.check([22000, 22000, 22000], 3000);
        assert!(ev.is_some());
        assert_eq!(ev.unwrap().event_type, VoltageEventType::Swell);
    }

    #[test]
    fn test_voltage_interruption() {
        let mut det = VoltageEventDetector::new(22000);
        det.check([0, 22000, 22000], 1000); // 0V = 中断
        let ev = det.check([22000, 22000, 22000], 1500);
        assert!(ev.is_some());
        assert_eq!(ev.unwrap().event_type, VoltageEventType::Interruption);
    }

    #[test]
    fn test_voltage_event_buffer_overflow() {
        let mut det = VoltageEventDetector::new(22000);
        for i in 0..20 {
            det.check([19500, 22000, 22000], (i * 1000) as u32);
            det.check([22000, 22000, 22000], (i * 1000 + 500) as u32);
        }
        assert_eq!(det.event_count, 16); // buffer limited
    }

    #[test]
    fn test_flicker_new() {
        let f = FlickerAnalyzer::new();
        assert!((f.pst() - 0.0).abs() < 1e-6);
        assert!((f.plt() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_flicker_feed_and_instantaneous() {
        let mut f = FlickerAnalyzer::new();
        for _ in 0..10 {
            f.feed_half_cycle_rms(1.0);
        }
        assert!((f.instantaneous_flicker() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_flicker_with_variation() {
        let mut f = FlickerAnalyzer::new();
        for i in 0..10 {
            let v = if i % 2 == 0 { 1.05 } else { 0.95 };
            f.feed_half_cycle_rms(v);
        }
        let d = f.instantaneous_flicker();
        assert!(d > 0.0, "should detect flicker variation");
    }

    #[test]
    fn test_flicker_pst_period() {
        let mut f = FlickerAnalyzer::new();
        f.complete_pst_period(0.8);
        f.complete_pst_period(1.2);
        assert!((f.pst() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_flicker_plt() {
        let mut f = FlickerAnalyzer::new();
        for i in 0..12 {
            f.complete_pst_period(0.5 + i as f32 * 0.05);
        }
        let plt = f.plt();
        assert!(plt > 0.0);
    }

    #[test]
    fn test_unbalance_balanced() {
        let ub = calculate_unbalance([220.0, 220.0, 220.0]);
        assert!((ub.voltage_unbalance - 0.0).abs() < 0.1);
        assert!(!ub.is_abnormal);
    }

    #[test]
    fn test_unbalance_imbalanced() {
        let ub = calculate_unbalance([220.0, 215.0, 225.0]);
        assert!(ub.voltage_unbalance > 0.0);
        assert!(ub.voltage_unbalance < 5.0);
    }

    #[test]
    fn test_unbalance_severe() {
        let ub = calculate_unbalance([220.0, 180.0, 220.0]);
        assert!(ub.voltage_unbalance > 5.0);
        assert!(ub.is_abnormal);
    }

    #[test]
    fn test_unbalance_zero_values() {
        let ub = calculate_unbalance([0.0, 0.0, 0.0]);
        assert!((ub.voltage_unbalance - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_pf_analysis_excellent() {
        let pf = analyze_power_factor([980, 980, 980, 980], 10.0);
        assert_eq!(pf.advice, PfCorrectionAdvice::Excellent);
        assert!((pf.pf_total - 0.98).abs() < 0.001);
    }

    #[test]
    fn test_pf_analysis_compensation_needed() {
        let pf = analyze_power_factor([850, 850, 850, 850], 10.0);
        assert_eq!(pf.advice, PfCorrectionAdvice::CompensationNeeded);
        assert!(pf.suggested_kvar > 0.0);
    }

    #[test]
    fn test_pf_analysis_urgent() {
        let pf = analyze_power_factor([600, 600, 600, 600], 10.0);
        assert_eq!(pf.advice, PfCorrectionAdvice::UrgentCompensation);
        assert!(pf.suggested_kvar > 0.0);
    }

    #[test]
    fn test_pf_analysis_zero_power() {
        let pf = analyze_power_factor([950, 950, 950, 950], 0.0);
        assert!((pf.suggested_kvar - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_power_quality_monitor_new() {
        let m = PowerQualityMonitor::new(22000);
        assert_eq!(m.event_count, 0);
    }

    #[test]
    fn test_power_quality_monitor_normal() {
        let mut m = PowerQualityMonitor::new(22000);
        let events = m.check([22000, 22000, 22000], [5000, 5000, 5000], 950, 5000, 1000);
        assert!(events.is_empty());
    }

    #[test]
    fn test_power_quality_monitor_unbalance_event() {
        let mut m = PowerQualityMonitor::new(22000);
        let events = m.check([22000, 18000, 22000], [5000, 5000, 5000], 950, 5000, 1000);
        assert!(!events.is_empty());
        assert_eq!(events[0].event_type, PowerQualityEventType::UnbalanceExceed);
    }

    #[test]
    fn test_power_quality_monitor_low_pf() {
        let mut m = PowerQualityMonitor::new(22000);
        let events = m.check([22000, 22000, 22000], [5000, 5000, 5000], 800, 5000, 1000);
        let pf_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == PowerQualityEventType::LowPowerFactor)
            .collect();
        assert!(!pf_events.is_empty());
    }

    #[test]
    fn test_power_quality_monitor_freq_deviation() {
        let mut m = PowerQualityMonitor::new(22000);
        let events = m.check([22000, 22000, 22000], [5000, 5000, 5000], 950, 5500, 1000);
        let freq_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == PowerQualityEventType::FrequencyDeviation)
            .collect();
        assert!(!freq_events.is_empty());
    }

    #[test]
    fn test_power_quality_event_struct_size() {
        // 确保嵌入式友好: PowerQualityEvent 应 ≤ 32 bytes
        assert!(
            core::mem::size_of::<PowerQualityEvent>() <= 32,
            "PowerQualityEvent too large: {}",
            core::mem::size_of::<PowerQualityEvent>()
        );
    }
}
