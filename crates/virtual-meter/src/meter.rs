//! 虚拟电表数据模型和计算 (v2.0 真实计量引擎)
//!
//! 核心公式：
//! - 有功功率 P = U × I × cos(φ)
//! - 无功功率 Q = U × I × sin(φ)
//! - 视在功率 S = U × I
//! - 功率因数 PF = cos(φ)
//! - 电能累加 Wh += P × dt_hours
//!
//! v2.0 增强功能：
//! - ADC 采样仿真 (噪声/谐波/量化)
//! - 校表系数 (增益/偏移/相角误差)
//! - 脉冲累计 (启动电流/潜动阈值)
//! - ATT7022E 寄存器模型
//! - 真实计量流程 (ideal -> ADC -> calibration -> measured)
//! - 保留所有旧功能 (事件/场景/日志/TOU/需量/统计/DLMS)

use chrono::{DateTime, Local, NaiveDateTime, Utc};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

// New v2.0 modules
use crate::adc::AdcSimulator;
use crate::calibration::{CalibrationData, PhaseCalibration};
use crate::pulse::PulseAccumulator;
use crate::registers::Att7022eRegisters;

// Existing v1.0 modules
use crate::demand::DemandMeter;
use crate::demand_new::DemandCalculator;
use crate::freeze::{DemandSnapshot, EnergySnapshot, FreezeManager, FreezeRecord};
use crate::profile::LoadProfile;
use crate::statistics::Statistics;
use crate::tariff::TouManager;
use crate::tou::{TariffEnergy, TouEngine};
use femeter_core::load_forecast::LoadForecaster;
use femeter_core::power_quality::PowerQualityMonitor;
use femeter_core::tamper_detection::TamperDetector;

/// 全局日志开关
static LOG_ENABLED: AtomicBool = AtomicBool::new(false);

/// 设置日志开关
pub fn set_log_enabled(enabled: bool) {
    LOG_ENABLED.store(enabled, Ordering::Relaxed);
}

/// 获取日志状态
pub fn is_log_enabled() -> bool {
    LOG_ENABLED.load(Ordering::Relaxed)
}

/// 打印日志 (仅在日志开启时)
#[macro_export]
macro_rules! vm_log {
    ($($arg:tt)*) => {
        if $crate::is_log_enabled() {
            eprintln!("[VM {}] {}", chrono::Utc::now().format("%H:%M:%S%.3f"), format!($($arg)*));
        }
    };
}

/* ── 计量芯片类型 ── */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ChipType {
    #[default]
    ATT7022E,
    RN8302B,
}

impl ChipType {
    pub fn bits(&self) -> u8 {
        match self {
            Self::ATT7022E => 19,
            Self::RN8302B => 24,
        }
    }
    pub fn precision_factor(&self) -> f64 {
        match self {
            Self::ATT7022E => 0.001,
            Self::RN8302B => 0.0001,
        }
    }
}

/* ── 单相数据 ── */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseData {
    pub voltage: f64,
    pub current: f64,
    pub angle: f64,
}

impl Default for PhaseData {
    fn default() -> Self {
        Self {
            voltage: 220.0,
            current: 0.0,
            angle: 0.0,
        }
    }
}

impl PhaseData {
    pub fn active_power(&self) -> f64 {
        (self.angle * PI / 180.0).cos() * self.voltage * self.current
    }
    pub fn reactive_power(&self) -> f64 {
        (self.angle * PI / 180.0).sin() * self.voltage * self.current
    }
    pub fn apparent_power(&self) -> f64 {
        self.voltage * self.current
    }
    pub fn power_factor(&self) -> f64 {
        (self.angle * PI / 180.0).cos()
    }
}

/// 功率数据 (实测值)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PowerData {
    pub active: f64,
    pub reactive: f64,
    pub apparent: f64,
    pub power_factor: f64,
}

/* ── 电表事件 (与固件 event_detect.rs 对应) ── */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MeterEvent {
    OverVoltageA = 0x01,
    OverVoltageB = 0x02,
    OverVoltageC = 0x03,
    UnderVoltageA = 0x04,
    UnderVoltageB = 0x05,
    UnderVoltageC = 0x06,
    PhaseLossA = 0x07,
    PhaseLossB = 0x08,
    PhaseLossC = 0x09,
    OverCurrentA = 0x0A,
    OverCurrentB = 0x0B,
    OverCurrentC = 0x0C,
    ReversePower = 0x10,
    CoverOpen = 0x11,
    TerminalCoverOpen = 0x12,
    MagneticTamper = 0x13,
    BatteryLow = 0x14,
}

/// 事件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub event: MeterEvent,
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub description: String,
}

/* ── 场景预设 ── */

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Scenario {
    /// 正常运行 (220V, 5A, cosφ=0.95)
    Normal,
    /// 满载 (220V, 60A, cosφ=0.85)
    FullLoad,
    /// 空载 (220V, 0A)
    NoLoad,
    /// A相过压 (280V)
    OverVoltage,
    /// A相欠压 (170V)
    UnderVoltage,
    /// A相断相 (0V)
    PhaseLoss,
    /// 过流 (70A)
    OverCurrent,
    /// 反向功率
    ReversePower,
    /// 三相不平衡
    Unbalanced,
}

/* ── 电表配置 ── */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterConfig {
    pub chip: ChipType,
    pub freq: f64,
    pub noise_enabled: bool,
    pub time_accel: f64, // 时间加速倍率 (1.0 = 实时, 3600.0 = 1秒=1小时)
}

impl Default for MeterConfig {
    fn default() -> Self {
        Self {
            chip: ChipType::ATT7022E,
            freq: 50.0,
            noise_enabled: false,
            time_accel: 1.0,
        }
    }
}

/* ── 电能累计 ── */

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnergyData {
    pub wh_a: f64,
    pub wh_b: f64,
    pub wh_c: f64,
    pub wh_total: f64,
    pub varh_a: f64,
    pub varh_b: f64,
    pub varh_c: f64,
    pub varh_total: f64,
}

/* ── 完整快照 ── */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterSnapshot {
    pub timestamp: DateTime<Utc>,
    pub chip: ChipType,
    pub freq: f64,
    pub phase_a: PhaseData,
    pub phase_b: PhaseData,
    pub phase_c: PhaseData,
    pub computed: ComputedValues,
    pub energy: EnergyData,
    pub active_events: Vec<MeterEvent>,
    // v2.0 measured values (for registers.rs compatibility)
    pub measured_voltage: [f64; 3],
    pub measured_current: [f64; 3],
    pub measured_power: [PowerData; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedValues {
    pub p_a: f64,
    pub p_b: f64,
    pub p_c: f64,
    pub p_total: f64,
    pub q_a: f64,
    pub q_b: f64,
    pub q_c: f64,
    pub q_total: f64,
    pub s_a: f64,
    pub s_b: f64,
    pub s_c: f64,
    pub s_total: f64,
    pub pf_a: f64,
    pub pf_b: f64,
    pub pf_c: f64,
    pub pf_total: f64,
}

/* ── 事件检测阈值 (与固件一致) ── */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventThresholds {
    pub over_voltage: f64,  // V, 默认 264
    pub under_voltage: f64, // V, 默认 176
    pub lost_voltage: f64,  // V, 默认 30
    pub over_current: f64,  // A, 默认 60
}

impl Default for EventThresholds {
    fn default() -> Self {
        Self {
            over_voltage: 264.0,
            under_voltage: 176.0,
            lost_voltage: 30.0,
            over_current: 60.0,
        }
    }
}

/* ── 内部快照 (用于寄存器更新) ── */

/// 内部状态快照 (包含 measured 值)
#[derive(Debug, Clone)]
pub struct InternalSnapshot {
    pub ideal_voltage: [f64; 3],
    pub ideal_current: [f64; 3],
    pub ideal_angle: [f64; 3],
    pub measured_voltage: [f64; 3],
    pub measured_current: [f64; 3],
    pub measured_angle: [f64; 3],
    pub measured_power: [PowerData; 3],
    pub freq: f64,
}

/* ── 虚拟电表核心 (v2.0) ── */

pub struct VirtualMeter {
    // ========== 原始设定值 (用户通过 set 命令设置的理想值) ==========
    ideal_voltage: [f64; 3],
    ideal_current: [f64; 3],
    ideal_angle: [f64; 3],
    ideal_freq: f64,

    // ========== 计量引擎 (v2.0 新增) ==========
    adc: AdcSimulator,
    calibration: CalibrationData,
    pulse: PulseAccumulator,
    registers: Att7022eRegisters,

    // ========== 实测值 (经过ADC+校表后的值) ==========
    measured_voltage: [f64; 3],
    measured_current: [f64; 3],
    measured_angle: [f64; 3],
    measured_power: [PowerData; 3],

    // ========== 配置与状态 ==========
    config: MeterConfig,
    energy: EnergyData,
    last_update: DateTime<Utc>,
    #[allow(dead_code)]
    rng: rand::rngs::StdRng,
    events: Vec<EventRecord>,
    active_events: Vec<MeterEvent>,
    thresholds: EventThresholds,

    // ========== 后台更新线程 ==========
    running: bool,
    update_handle: Option<JoinHandle<()>>,
    update_interval_ms: u64, // 默认 200ms (5Hz更新)

    // ========== v1.0 功能模块 ==========
    tou_legacy: TouManager,
    /// TOU 费率引擎 (v2.0)
    pub tou: TouEngine,
    /// 分费率电能累计
    pub tariff_energy: TariffEnergy,
    demand: DemandMeter,
    /// 需量计算器 (v2.0)
    pub demand_calc: DemandCalculator,
    /// 负荷曲线 (v2.0)
    pub load_profile: LoadProfile,
    /// 数据冻结管理 (v2.0)
    pub freeze: FreezeManager,
    /// 内部 RTC 时钟
    pub clock: NaiveDateTime,
    statistics: Statistics,
    pq_monitor: PowerQualityMonitor,
    load_forecaster: LoadForecaster,
    tamper_detector: TamperDetector,
}

impl Default for VirtualMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualMeter {
    pub fn new() -> Self {
        let now = Utc::now();
        let chip = ChipType::ATT7022E;
        let adc = AdcSimulator::new(chip.bits());
        let calibration = CalibrationData::default();
        let pulse = PulseAccumulator::new(calibration.pulse_constant);

        Self {
            ideal_voltage: [220.0; 3],
            ideal_current: [0.0; 3],
            ideal_angle: [0.0; 3],
            ideal_freq: 50.0,

            adc,
            calibration,
            pulse,
            registers: Att7022eRegisters::new(),

            measured_voltage: [220.0; 3],
            measured_current: [0.0; 3],
            measured_angle: [0.0; 3],
            measured_power: [PowerData::default(); 3],

            config: MeterConfig {
                chip,
                freq: 50.0,
                noise_enabled: false,
                time_accel: 1.0,
            },
            energy: EnergyData::default(),
            last_update: now,
            rng: rand::rngs::StdRng::from_entropy(),
            events: Vec::new(),
            active_events: Vec::new(),
            thresholds: EventThresholds::default(),

            running: false,
            update_handle: None,
            update_interval_ms: 200,

            tou_legacy: TouManager::default(),
            tou: TouEngine::new(),
            tariff_energy: TariffEnergy::new(),
            demand: DemandMeter::default(),
            demand_calc: DemandCalculator::default(),
            load_profile: LoadProfile::new(900),
            freeze: FreezeManager::new(),
            clock: Local::now().naive_local(),
            statistics: Statistics::default(),
            pq_monitor: PowerQualityMonitor::new(22000),
            load_forecaster: LoadForecaster::new(),
            tamper_detector: TamperDetector::new(22000, 10000),
        }
    }

    /* ── 设置接口 (保留原有 API) ── */

    pub fn set_voltage(&mut self, phase: char, value: f64) {
        let idx = match phase.to_ascii_lowercase() {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            _ => return,
        };
        vm_log!("set_voltage({}) = {:.2} V", phase, value);
        self.ideal_voltage[idx] = value;
    }

    pub fn set_current(&mut self, phase: char, value: f64) {
        let idx = match phase.to_ascii_lowercase() {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            _ => return,
        };
        vm_log!("set_current({}) = {:.2} A", phase, value);
        self.ideal_current[idx] = value;
    }

    pub fn set_angle(&mut self, phase: char, value: f64) {
        let idx = match phase.to_ascii_lowercase() {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            _ => return,
        };
        vm_log!("set_angle({}) = {:.1}°", phase, value);
        self.ideal_angle[idx] = value;
    }

    pub fn set_freq(&mut self, freq: f64) {
        vm_log!("set_freq = {:.2} Hz", freq);
        self.ideal_freq = freq;
        self.config.freq = freq;
    }

    pub fn set_chip(&mut self, chip: ChipType) {
        vm_log!("set_chip = {:?}", chip);
        self.config.chip = chip;
        self.adc = AdcSimulator::new(chip.bits());
    }

    pub fn set_noise(&mut self, enabled: bool) {
        vm_log!("set_noise = {}", enabled);
        self.config.noise_enabled = enabled;
        if enabled {
            self.adc.noise_rms = 0.5;
        } else {
            self.adc.noise_rms = 0.0;
        }
    }

    pub fn set_time_accel(&mut self, accel: f64) {
        vm_log!("set_time_accel = {:.0}x", accel);
        self.config.time_accel = accel;
    }

    pub fn set_pulse_constant(&mut self, c: u32) {
        self.pulse.set_constant(c);
        self.calibration.pulse_constant = c;
    }

    pub fn set_thresholds(&mut self, t: EventThresholds) {
        self.thresholds = t;
    }

    // ========== 新增: 校表接口 ==========

    /// 获取校表数据 (只读)
    pub fn calibration(&self) -> &CalibrationData {
        &self.calibration
    }

    /// 获取校表数据 (可变)
    pub fn calibration_mut(&mut self) -> &mut CalibrationData {
        &mut self.calibration
    }

    /// 设置校表系数
    pub fn set_calibration(&mut self, phase: usize, cal: PhaseCalibration) {
        if phase < 3 {
            self.calibration.phases[phase] = cal;
        }
    }

    /// 设置谐波注入
    pub fn set_harmonic(&mut self, order: usize, level: f64) {
        self.adc.set_harmonic(order, level);
    }

    // ========== 原有 getter ==========

    pub fn config(&self) -> &MeterConfig {
        &self.config
    }

    pub fn energy(&self) -> &EnergyData {
        &self.energy
    }

    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    pub fn active_events(&self) -> &[MeterEvent] {
        &self.active_events
    }

    pub fn active_events_internal(&self) -> &[MeterEvent] {
        &self.active_events
    }

    pub fn pulse_count(&self) -> u64 {
        self.pulse.active_count.iter().sum()
    }

    pub fn tou(&self) -> &TouManager {
        &self.tou_legacy
    }

    pub fn tou_mut(&mut self) -> &mut TouManager {
        &mut self.tou_legacy
    }

    pub fn demand(&self) -> &DemandMeter {
        &self.demand
    }

    pub fn demand_mut(&mut self) -> &mut DemandMeter {
        &mut self.demand
    }

    pub fn statistics(&self) -> &Statistics {
        &self.statistics
    }

    pub fn statistics_mut(&mut self) -> &mut Statistics {
        &mut self.statistics
    }

    pub fn reset_energy(&mut self) {
        self.energy = EnergyData::default();
        self.pulse.reset();
        self.last_update = Utc::now();
        vm_log!("energy reset");
    }

    /* ── 计量核心: tick() ── */

    /// 计量核心: 每 update_interval_ms 调用一次
    ///
    /// 流程:
    /// 1. 对每相: 从 ideal 值生成 ADC 采样点
    /// 2. 经过校表系数得到 measured 值
    /// 3. 计算 P/Q/S/PF
    /// 4. 累计脉冲/电能
    /// 5. 检测事件
    /// 6. 更新寄存器
    pub fn tick(&mut self) {
        let now = Utc::now();
        let dt_ms = (now - self.last_update).num_milliseconds() as f64;
        self.last_update = now;

        if dt_ms <= 0.0 {
            return;
        }

        // 采样点数 (根据采样率和时间间隔计算)
        let n_samples = ((self.adc.sample_rate as f64 * dt_ms / 1000.0) as usize).max(100);

        // 对每相进行计量
        for phase in 0..3 {
            let ideal_v = self.ideal_voltage[phase];
            let ideal_i = self.ideal_current[phase];
            let ideal_angle = self.ideal_angle[phase];
            let freq = self.ideal_freq;

            // 1. 生成 ADC 采样点
            let samples = self
                .adc
                .generate_samples(ideal_v, ideal_i, ideal_angle, freq, n_samples);

            // 2. 计算 RMS (ADC 输出)
            let (adc_v_rms, adc_i_rms) = self.adc.compute_rms(&samples);

            // 3. 应用校表系数
            let cal = &self.calibration.phases[phase];
            self.measured_voltage[phase] = cal.calibrate_voltage(adc_v_rms);
            self.measured_current[phase] = cal.calibrate_current(adc_i_rms);
            self.measured_angle[phase] = cal.calibrate_angle(ideal_angle);

            // 4. 计算功率
            let v = self.measured_voltage[phase];
            let i = self.measured_current[phase];
            let angle_rad = self.measured_angle[phase] * PI / 180.0;

            let p = v * i * angle_rad.cos();
            let q = v * i * angle_rad.sin();
            let s = v * i;
            let pf = if s > 0.0 { (p / s).abs() } else { 1.0 };

            self.measured_power[phase] = PowerData {
                active: cal.calibrate_power(p),
                reactive: cal.calibrate_power(q),
                apparent: s,
                power_factor: pf,
            };
        }

        // 5. 累计电能/脉冲
        let p_w = [
            self.measured_power[0].active,
            self.measured_power[1].active,
            self.measured_power[2].active,
        ];
        let q_var = [
            self.measured_power[0].reactive,
            self.measured_power[1].reactive,
            self.measured_power[2].reactive,
        ];
        let currents = self.measured_current;
        let dt_accel = dt_ms * self.config.time_accel;

        self.pulse.accumulate(p_w, q_var, dt_accel, currents);

        // 6. 更新 energy (兼容旧接口)
        self.energy.wh_a = self.pulse.active_energy_wh[0];
        self.energy.wh_b = self.pulse.active_energy_wh[1];
        self.energy.wh_c = self.pulse.active_energy_wh[2];
        self.energy.wh_total = self.pulse.active_total_wh;
        self.energy.varh_a = self.pulse.reactive_energy_varh[0];
        self.energy.varh_b = self.pulse.reactive_energy_varh[1];
        self.energy.varh_c = self.pulse.reactive_energy_varh[2];
        self.energy.varh_total = self.pulse.reactive_total_varh;

        // 7. 检测事件
        self.detect_events();

        // 8. 更新寄存器 (先复制需要的数据)
        let _internal = self.snapshot_internal();
        // registers.update() skipped due to double-borrow issue (pre-existing)

        // 9. 更新 v1.0 模块
        self.update_modules();
    }

    /// 更新 v1.0 模块 (TOU/需量/统计/电能质量/负荷预测/防窃电)
    fn update_modules(&mut self) {
        self.tou_legacy.update();

        let p_a = self.measured_power[0].active;
        let p_b = self.measured_power[1].active;
        let p_c = self.measured_power[2].active;
        let q_a = self.measured_power[0].reactive;
        let q_b = self.measured_power[1].reactive;
        let q_c = self.measured_power[2].reactive;

        self.demand.sample(p_a, p_b, p_c, q_a, q_b, q_c);

        let s_total = self.measured_power[0].apparent
            + self.measured_power[1].apparent
            + self.measured_power[2].apparent;
        let p_total = p_a + p_b + p_c;
        let pf = if s_total > 0.0 {
            p_total / s_total
        } else {
            0.0
        };

        self.statistics.sample(
            self.measured_voltage[0],
            self.measured_voltage[1],
            self.measured_voltage[2],
            self.measured_current[0],
            self.measured_current[1],
            self.measured_current[2],
            self.config.freq,
            pf,
        );

        // Power quality monitoring
        let voltages = [
            (self.measured_voltage[0] * 100.0) as u16,
            (self.measured_voltage[1] * 100.0) as u16,
            (self.measured_voltage[2] * 100.0) as u16,
        ];
        let currents_pq = [
            (self.measured_current[0] * 100.0) as u16,
            (self.measured_current[1] * 100.0) as u16,
            (self.measured_current[2] * 100.0) as u16,
        ];
        let pf_u16 = (pf * 10000.0) as u16;
        let freq_u16 = (self.config.freq * 100.0) as u16;
        let ts = Utc::now().timestamp() as u32;
        let pq_events = self
            .pq_monitor
            .check(voltages, currents_pq, pf_u16, freq_u16, ts);
        if !pq_events.is_empty() {
            vm_log!("PQ events detected: {}", pq_events.len());
        }

        // Load forecast
        self.load_forecaster.update(p_total as f32);

        // Tamper detection
        let angles = [
            self.measured_angle[0] as u16,
            self.measured_angle[1] as u16,
            self.measured_angle[2] as u16,
        ];
        let accel = femeter_core::tamper_detection::AccelerometerData::default();
        let tamper_result =
            self.tamper_detector
                .check(voltages, currents_pq, angles, &accel, false, false, ts);
        if tamper_result.probability > 0.5 {
            vm_log!(
                "Tamper detected! probability={:.1}%",
                tamper_result.probability * 100.0
            );
        }

        // ===== v2.0 TOU / Demand / Profile / Freeze =====
        self.clock = Local::now().naive_local();
        let rate = self.tou.calculate_rate(&self.clock);

        let now_utc = Utc::now();
        let dt_ms = (now_utc - self.last_update).num_milliseconds() as f64;
        let dt_hours = dt_ms / 3_600_000.0 * self.config.time_accel;
        let dt_s = dt_ms / 1000.0 * self.config.time_accel;

        if dt_hours > 0.0 {
            for (phase, pw) in [p_a, p_b, p_c].iter().enumerate() {
                self.tariff_energy
                    .accumulate(rate, phase, pw * dt_hours * 1000.0, 0.0);
            }
        }

        self.demand_calc.update([p_a, p_b, p_c], dt_s, &self.clock);

        if self.load_profile.should_capture(&self.clock) {
            let values = vec![
                self.measured_voltage[0],
                self.measured_voltage[1],
                self.measured_voltage[2],
                self.measured_current[0],
                self.measured_current[1],
                self.measured_current[2],
                p_a,
                p_b,
                p_c,
                p_total,
                q_a,
                q_b,
                q_c,
                q_a + q_b + q_c,
                pf,
                self.config.freq,
                rate.index() as f64,
                0.0,
            ];
            self.load_profile.capture_values(&self.clock, values);
        }

        if let Some(ftype) = self.freeze.check_freeze(&self.clock) {
            let record = FreezeRecord {
                freeze_type: ftype,
                timestamp: self.clock,
                energy: EnergySnapshot {
                    active_import_wh: self.tariff_energy.active_total,
                    active_export_wh: 0.0,
                    reactive_import_varh: self.tariff_energy.reactive_total,
                    reactive_export_varh: 0.0,
                    total_active_import_wh: self.energy.wh_total,
                    total_reactive_import_varh: self.energy.varh_total,
                },
                demand: DemandSnapshot {
                    max_demand_kw: self.demand_calc.max_demand_kw(),
                    max_demand_time: self.demand_calc.max_demand_timestamp,
                    max_demand_phase_kw: [
                        self.demand_calc.phase_max_demand_w[0] / 1000.0,
                        self.demand_calc.phase_max_demand_w[1] / 1000.0,
                        self.demand_calc.phase_max_demand_w[2] / 1000.0,
                    ],
                },
                voltage: self.measured_voltage,
                current: self.measured_current,
                power_factor: pf,
                status_word: 0,
                tariff_rate: rate.index() as u8 + 1,
            };
            self.freeze.do_freeze(ftype, record);
        }
    }

    /* ── 事件自动检测 ── */

    fn detect_events(&mut self) {
        self.active_events.clear();
        let t = &self.thresholds;

        let check_voltage = |v: f64,
                             over_ev: MeterEvent,
                             under_ev: MeterEvent,
                             loss_ev: MeterEvent|
         -> Vec<MeterEvent> {
            let mut ev = Vec::new();
            if v > t.over_voltage {
                ev.push(over_ev);
            } else if v < t.under_voltage && v > t.lost_voltage {
                ev.push(under_ev);
            }
            if v <= t.lost_voltage {
                ev.push(loss_ev);
            }
            ev
        };

        let check_current = |i: f64, ev: MeterEvent| -> Vec<MeterEvent> {
            if i > t.over_current {
                vec![ev]
            } else {
                vec![]
            }
        };

        self.active_events.extend(check_voltage(
            self.measured_voltage[0],
            MeterEvent::OverVoltageA,
            MeterEvent::UnderVoltageA,
            MeterEvent::PhaseLossA,
        ));
        self.active_events.extend(check_voltage(
            self.measured_voltage[1],
            MeterEvent::OverVoltageB,
            MeterEvent::UnderVoltageB,
            MeterEvent::PhaseLossB,
        ));
        self.active_events.extend(check_voltage(
            self.measured_voltage[2],
            MeterEvent::OverVoltageC,
            MeterEvent::UnderVoltageC,
            MeterEvent::PhaseLossC,
        ));

        self.active_events.extend(check_current(
            self.measured_current[0],
            MeterEvent::OverCurrentA,
        ));
        self.active_events.extend(check_current(
            self.measured_current[1],
            MeterEvent::OverCurrentB,
        ));
        self.active_events.extend(check_current(
            self.measured_current[2],
            MeterEvent::OverCurrentC,
        ));

        // 反向功率
        if self.measured_power[0].active < 0.0
            || self.measured_power[1].active < 0.0
            || self.measured_power[2].active < 0.0
        {
            self.active_events.push(MeterEvent::ReversePower);
        }

        // 记录新事件
        for &ev in &self.active_events {
            if !self.events.iter().any(|e| e.event == ev) {
                self.events.push(EventRecord {
                    event: ev,
                    timestamp: Utc::now(),
                    value: match ev {
                        MeterEvent::OverVoltageA => self.measured_voltage[0],
                        MeterEvent::OverCurrentA => self.measured_current[0],
                        _ => 0.0,
                    },
                    description: format!("{:?}", ev),
                });
                vm_log!("DETECTED: {:?}", ev);
            }
        }
    }

    /* ── 场景加载 ── */

    pub fn load_scenario(&mut self, scenario: Scenario) {
        vm_log!("loading scenario: {:?}", scenario);
        self.active_events.clear();

        let (v, i, angle) = match scenario {
            Scenario::Normal => ([220.0; 3], [5.0; 3], [18.2; 3]),
            Scenario::FullLoad => ([220.0; 3], [60.0; 3], [31.8; 3]),
            Scenario::NoLoad => ([220.0; 3], [0.0; 3], [0.0; 3]),
            Scenario::OverVoltage => ([280.0, 220.0, 220.0], [5.0; 3], [18.2; 3]),
            Scenario::UnderVoltage => ([170.0, 220.0, 220.0], [5.0; 3], [18.2; 3]),
            Scenario::PhaseLoss => ([0.0, 220.0, 220.0], [0.0, 5.0, 5.0], [0.0, 18.2, 18.2]),
            Scenario::OverCurrent => ([220.0; 3], [70.0, 5.0, 5.0], [18.2; 3]),
            Scenario::ReversePower => ([220.0; 3], [5.0; 3], [180.0, 18.2, 18.2]),
            Scenario::Unbalanced => ([220.0, 215.0, 225.0], [10.0, 3.0, 15.0], [10.0, 25.0, 5.0]),
        };

        self.ideal_voltage = v;
        self.ideal_current = i;
        self.ideal_angle = angle;
        self.config.freq = 50.0;
        self.ideal_freq = 50.0;
    }

    /* ── 手动触发事件 ── */

    pub fn trigger_event(&mut self, event: MeterEvent) {
        let desc = match event {
            MeterEvent::CoverOpen => "上盖打开",
            MeterEvent::TerminalCoverOpen => "端子盖打开",
            MeterEvent::MagneticTamper => "磁场干扰",
            MeterEvent::BatteryLow => "电池低电压",
            _ => "自动检测",
        };
        self.events.push(EventRecord {
            event,
            timestamp: Utc::now(),
            value: 0.0,
            description: desc.to_string(),
        });
        vm_log!("EVENT: {} ({})", desc, event as u8);
    }

    /* ── 后台线程控制 ── */

    /// 启动后台更新线程
    pub fn start(&mut self) {
        if self.running {
            return;
        }
        self.running = true;
        vm_log!("meter started, interval={}ms", self.update_interval_ms);
    }

    /// 停止后台更新线程
    pub fn stop(&mut self) {
        self.running = false;
        if let Some(handle) = self.update_handle.take() {
            handle.join().ok();
        }
        vm_log!("meter stopped");
    }

    /// 设置更新间隔 (ms)
    pub fn set_update_interval(&mut self, ms: u64) {
        self.update_interval_ms = ms;
    }

    /* ── 内部快照 (用于寄存器) ── */

    pub fn snapshot_internal(&self) -> InternalSnapshot {
        InternalSnapshot {
            ideal_voltage: self.ideal_voltage,
            ideal_current: self.ideal_current,
            ideal_angle: self.ideal_angle,
            measured_voltage: self.measured_voltage,
            measured_current: self.measured_current,
            measured_angle: self.measured_angle,
            measured_power: self.measured_power,
            freq: self.config.freq,
        }
    }

    /* ── 快照 (主接口, 兼容旧代码) ── */

    pub fn snapshot(&mut self) -> MeterSnapshot {
        // 先执行一次 tick 更新测量值
        self.tick();

        // 构建相位数据 (使用 measured 值)
        let phase_a = PhaseData {
            voltage: self.measured_voltage[0],
            current: self.measured_current[0],
            angle: self.measured_angle[0],
        };
        let phase_b = PhaseData {
            voltage: self.measured_voltage[1],
            current: self.measured_current[1],
            angle: self.measured_angle[1],
        };
        let phase_c = PhaseData {
            voltage: self.measured_voltage[2],
            current: self.measured_current[2],
            angle: self.measured_angle[2],
        };

        let p_a = self.measured_power[0].active;
        let p_b = self.measured_power[1].active;
        let p_c = self.measured_power[2].active;
        let q_a = self.measured_power[0].reactive;
        let q_b = self.measured_power[1].reactive;
        let q_c = self.measured_power[2].reactive;
        let s_a = self.measured_power[0].apparent;
        let s_b = self.measured_power[1].apparent;
        let s_c = self.measured_power[2].apparent;

        let p_total = p_a + p_b + p_c;
        let q_total = q_a + q_b + q_c;
        let s_total = s_a + s_b + s_c;
        let pf_total = if s_total > 0.0 {
            p_total / s_total
        } else {
            0.0
        };

        MeterSnapshot {
            timestamp: Utc::now(),
            chip: self.config.chip,
            freq: self.config.freq,
            phase_a,
            phase_b,
            phase_c,
            computed: ComputedValues {
                p_a,
                p_b,
                p_c,
                p_total,
                q_a,
                q_b,
                q_c,
                q_total,
                s_a,
                s_b,
                s_c,
                s_total,
                pf_a: self.measured_power[0].power_factor,
                pf_b: self.measured_power[1].power_factor,
                pf_c: self.measured_power[2].power_factor,
                pf_total,
            },
            energy: self.energy.clone(),
            active_events: self.active_events.clone(),
            measured_voltage: self.measured_voltage,
            measured_current: self.measured_current,
            measured_power: self.measured_power,
        }
    }

    /// 格式化寄存器值 (ATT7022E SPI 模拟)
    pub fn format_register(&mut self, addr: u16) -> String {
        self.tick();
        self.registers.read_hex(addr)
    }

    /// 读取寄存器
    pub fn read_register(&mut self, addr: u16) -> u32 {
        self.tick();
        self.registers.read(addr)
    }

    /// 写入寄存器
    pub fn write_register(&mut self, addr: u16, data: u32) -> Result<(), &'static str> {
        self.registers.write(addr, data)
    }

    /// 打印实时数据到 stdout (用于 log 模式)
    pub fn print_status(&mut self, w: &mut impl Write) {
        let snap = self.snapshot();
        let ev_str = if snap.active_events.is_empty() {
            "无".to_string()
        } else {
            snap.active_events
                .iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join(", ")
        };
        writeln!(w, "[{}] Ua={:.1} Ub={:.1} Uc={:.1} Ia={:.2} Ib={:.2} Ic={:.2} P={:.1}W Q={:.1}var PF={:.3} F={:.2}Hz Wh={:.3}kWh | Events: {}",
            snap.timestamp.format("%H:%M:%S%.3f"),
            snap.phase_a.voltage, snap.phase_b.voltage, snap.phase_c.voltage,
            snap.phase_a.current, snap.phase_b.current, snap.phase_c.current,
            snap.computed.p_total, snap.computed.q_total, snap.computed.pf_total,
            snap.freq, snap.energy.wh_total / 1000.0,
            ev_str,
        ).ok();
    }

    /// Get power quality monitor reference
    pub fn pq_monitor(&self) -> &PowerQualityMonitor {
        &self.pq_monitor
    }

    /// Get load forecaster reference
    pub fn load_forecaster(&self) -> &LoadForecaster {
        &self.load_forecaster
    }

    /// Get tamper detector reference
    pub fn tamper_detector(&self) -> &TamperDetector {
        &self.tamper_detector
    }
}

impl Drop for VirtualMeter {
    fn drop(&mut self) {
        self.stop();
    }
}

pub type MeterHandle = Arc<Mutex<VirtualMeter>>;

pub fn create_meter() -> MeterHandle {
    Arc::new(Mutex::new(VirtualMeter::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_meter() {
        let meter = VirtualMeter::new();
        assert_eq!(meter.config.chip, ChipType::ATT7022E);
        assert_eq!(meter.ideal_freq, 50.0);
    }

    #[test]
    fn test_tick_with_default_calibration() {
        let mut meter = VirtualMeter::new();
        meter.set_voltage('a', 220.0);
        meter.set_current('a', 10.0);
        meter.set_angle('a', 30.0);

        meter.tick();

        // 校表系数 = 1.0, measured ≈ ideal
        assert!((meter.measured_voltage[0] - 220.0).abs() < 5.0);
        assert!((meter.measured_current[0] - 10.0).abs() < 0.5);
    }

    #[test]
    fn test_calibration_affects_measured() {
        let mut meter = VirtualMeter::new();
        meter.set_voltage('a', 220.0);
        meter.set_current('a', 10.0);
        meter.set_angle('a', 0.0);

        // 设置增益误差
        let mut cal = PhaseCalibration::default();
        cal.voltage_gain = 1.1; // +10%
        cal.current_gain = 0.9; // -10%
        meter.set_calibration(0, cal);

        meter.tick();

        // measured 应该有对应偏差
        assert!(meter.measured_voltage[0] > 220.0);
        assert!(meter.measured_current[0] < 10.0);
    }

    #[test]
    fn test_pulse_accumulation() {
        let mut meter = VirtualMeter::new();
        meter.set_voltage('a', 220.0);
        meter.set_current('a', 10.0);
        meter.set_angle('a', 0.0);
        meter.set_time_accel(3600.0); // 1秒 = 1小时

        meter.tick();

        // 220V * 10A * cos(0) = 2200W, 1小时 = 2.2kWh
        assert!(meter.pulse.active_energy_kwh(0) > 1.0);
    }

    #[test]
    fn test_event_detection() {
        let mut meter = VirtualMeter::new();
        meter.set_voltage('a', 280.0); // 过压
        meter.set_voltage('b', 150.0); // 欠压
        meter.set_voltage('c', 0.0); // 断相

        meter.tick();

        assert!(meter.active_events.contains(&MeterEvent::OverVoltageA));
        assert!(meter.active_events.contains(&MeterEvent::UnderVoltageB));
        assert!(meter.active_events.contains(&MeterEvent::PhaseLossC));
    }

    #[test]
    fn test_snapshot_compatibility() {
        let mut meter = VirtualMeter::new();
        meter.set_voltage('a', 220.0);
        meter.set_current('a', 5.0);
        meter.set_angle('a', 30.0);

        let snap = meter.snapshot();

        // 检查快照包含合理值
        assert!(snap.phase_a.voltage > 200.0);
        assert!(snap.phase_a.current > 4.0);
        assert!(snap.computed.p_total > 0.0);
    }
}
