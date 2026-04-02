//! 虚拟电表数据模型和计算 (增强版)
//!
//! 核心公式：
//! - 有功功率 P = U × I × cos(φ)
//! - 无功功率 Q = U × I × sin(φ)
//! - 视在功率 S = U × I
//! - 功率因数 PF = cos(φ)
//! - 电能累加 Wh += P × dt_hours
//!
//! 增强功能：
//! - 日志开关 (log on/off)
//! - 事件模拟 (过压/欠压/断相/过流/反向功率)
//! - 场景预设 (正常/满载/空载/故障)
//! - DLMS 兼容数据接口
//! - 时间加速模拟

use chrono::{DateTime, Utc};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// New v1.0 modules
use crate::demand::DemandMeter;
use crate::statistics::Statistics;
use crate::tariff::TouManager;

/// 全局日志开关
static LOG_ENABLED: AtomicBool = AtomicBool::new(true);

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

/* ── 虚拟电表核心 ── */

pub struct VirtualMeter {
    phase_a: PhaseData,
    phase_b: PhaseData,
    phase_c: PhaseData,
    config: MeterConfig,
    energy: EnergyData,
    last_update: DateTime<Utc>,
    rng: rand::rngs::StdRng,
    events: Vec<EventRecord>,
    active_events: Vec<MeterEvent>,
    thresholds: EventThresholds,
    /// 脉冲常数 (imp/kWh)
    pulse_constant: u32,
    /// 累计脉冲数
    pulse_count: u64,
    /// 分时费率管理器
    tou: TouManager,
    /// 需量测量
    demand: DemandMeter,
    /// 统计记录
    statistics: Statistics,
}

impl Default for VirtualMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualMeter {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            phase_a: PhaseData::default(),
            phase_b: PhaseData::default(),
            phase_c: PhaseData::default(),
            config: MeterConfig::default(),
            energy: EnergyData::default(),
            last_update: now,
            rng: rand::rngs::StdRng::from_entropy(),
            events: Vec::new(),
            active_events: Vec::new(),
            thresholds: EventThresholds::default(),
            pulse_constant: 6400,
            pulse_count: 0,
            tou: TouManager::default(),
            demand: DemandMeter::default(),
            statistics: Statistics::default(),
        }
    }

    /* ── 设置接口 ── */

    pub fn set_voltage(&mut self, phase: char, value: f64) {
        let p = match phase.to_ascii_lowercase() {
            'a' => &mut self.phase_a,
            'b' => &mut self.phase_b,
            'c' => &mut self.phase_c,
            _ => return,
        };
        vm_log!("set_voltage({}) = {:.2} V", phase, value);
        p.voltage = value;
    }

    pub fn set_current(&mut self, phase: char, value: f64) {
        let p = match phase.to_ascii_lowercase() {
            'a' => &mut self.phase_a,
            'b' => &mut self.phase_b,
            'c' => &mut self.phase_c,
            _ => return,
        };
        vm_log!("set_current({}) = {:.2} A", phase, value);
        p.current = value;
    }

    pub fn set_angle(&mut self, phase: char, value: f64) {
        let p = match phase.to_ascii_lowercase() {
            'a' => &mut self.phase_a,
            'b' => &mut self.phase_b,
            'c' => &mut self.phase_c,
            _ => return,
        };
        vm_log!("set_angle({}) = {:.1}°", phase, value);
        p.angle = value;
    }

    pub fn set_freq(&mut self, freq: f64) {
        vm_log!("set_freq = {:.2} Hz", freq);
        self.config.freq = freq;
    }
    pub fn set_chip(&mut self, chip: ChipType) {
        vm_log!("set_chip = {:?}", chip);
        self.config.chip = chip;
    }
    pub fn set_noise(&mut self, enabled: bool) {
        vm_log!("set_noise = {}", enabled);
        self.config.noise_enabled = enabled;
    }
    pub fn set_time_accel(&mut self, accel: f64) {
        vm_log!("set_time_accel = {:.0}x", accel);
        self.config.time_accel = accel;
    }
    pub fn set_pulse_constant(&mut self, c: u32) {
        self.pulse_constant = c;
    }
    pub fn set_thresholds(&mut self, t: EventThresholds) {
        self.thresholds = t;
    }

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
    pub fn pulse_count(&self) -> u64 {
        self.pulse_count
    }

    // v1.0 new getters
    pub fn tou(&self) -> &TouManager {
        &self.tou
    }
    pub fn tou_mut(&mut self) -> &mut TouManager {
        &mut self.tou
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
        self.pulse_count = 0;
        self.last_update = Utc::now();
        vm_log!("energy reset");
    }

    /* ── 场景加载 ── */

    pub fn load_scenario(&mut self, scenario: Scenario) {
        vm_log!("loading scenario: {:?}", scenario);
        self.active_events.clear();
        match scenario {
            Scenario::Normal => {
                self.phase_a = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.phase_b = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.phase_c = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.config.freq = 50.0;
            }
            Scenario::FullLoad => {
                self.phase_a = PhaseData {
                    voltage: 220.0,
                    current: 60.0,
                    angle: 31.8,
                };
                self.phase_b = PhaseData {
                    voltage: 220.0,
                    current: 60.0,
                    angle: 31.8,
                };
                self.phase_c = PhaseData {
                    voltage: 220.0,
                    current: 60.0,
                    angle: 31.8,
                };
                self.config.freq = 49.8;
            }
            Scenario::NoLoad => {
                self.phase_a = PhaseData {
                    voltage: 220.0,
                    current: 0.0,
                    angle: 0.0,
                };
                self.phase_b = PhaseData {
                    voltage: 220.0,
                    current: 0.0,
                    angle: 0.0,
                };
                self.phase_c = PhaseData {
                    voltage: 220.0,
                    current: 0.0,
                    angle: 0.0,
                };
            }
            Scenario::OverVoltage => {
                self.phase_a = PhaseData {
                    voltage: 280.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.phase_b = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.phase_c = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
            }
            Scenario::UnderVoltage => {
                self.phase_a = PhaseData {
                    voltage: 170.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.phase_b = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.phase_c = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
            }
            Scenario::PhaseLoss => {
                self.phase_a = PhaseData {
                    voltage: 0.0,
                    current: 0.0,
                    angle: 0.0,
                };
                self.phase_b = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.phase_c = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
            }
            Scenario::OverCurrent => {
                self.phase_a = PhaseData {
                    voltage: 220.0,
                    current: 70.0,
                    angle: 18.2,
                };
                self.phase_b = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.phase_c = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
            }
            Scenario::ReversePower => {
                self.phase_a = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 180.0,
                };
                self.phase_b = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
                self.phase_c = PhaseData {
                    voltage: 220.0,
                    current: 5.0,
                    angle: 18.2,
                };
            }
            Scenario::Unbalanced => {
                self.phase_a = PhaseData {
                    voltage: 220.0,
                    current: 10.0,
                    angle: 10.0,
                };
                self.phase_b = PhaseData {
                    voltage: 215.0,
                    current: 3.0,
                    angle: 25.0,
                };
                self.phase_c = PhaseData {
                    voltage: 225.0,
                    current: 15.0,
                    angle: 5.0,
                };
            }
        }
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

    /* ── 事件自动检测 (与固件 event_detect.rs 逻辑一致) ── */

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
            self.phase_a.voltage,
            MeterEvent::OverVoltageA,
            MeterEvent::UnderVoltageA,
            MeterEvent::PhaseLossA,
        ));
        self.active_events.extend(check_voltage(
            self.phase_b.voltage,
            MeterEvent::OverVoltageB,
            MeterEvent::UnderVoltageB,
            MeterEvent::PhaseLossB,
        ));
        self.active_events.extend(check_voltage(
            self.phase_c.voltage,
            MeterEvent::OverVoltageC,
            MeterEvent::UnderVoltageC,
            MeterEvent::PhaseLossC,
        ));

        self.active_events.extend(check_current(
            self.phase_a.current,
            MeterEvent::OverCurrentA,
        ));
        self.active_events.extend(check_current(
            self.phase_b.current,
            MeterEvent::OverCurrentB,
        ));
        self.active_events.extend(check_current(
            self.phase_c.current,
            MeterEvent::OverCurrentC,
        ));

        // 反向功率
        if self.phase_a.active_power() < 0.0
            || self.phase_b.active_power() < 0.0
            || self.phase_c.active_power() < 0.0
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
                        MeterEvent::OverVoltageA => self.phase_a.voltage,
                        MeterEvent::OverCurrentA => self.phase_a.current,
                        _ => 0.0,
                    },
                    description: format!("{:?}", ev),
                });
                vm_log!("DETECTED: {:?}", ev);
            }
        }
    }

    /* ── 内部 ── */

    fn apply_noise(&mut self, value: f64) -> f64 {
        if !self.config.noise_enabled {
            return value;
        }
        let factor = self.config.chip.precision_factor();
        value * (1.0 + self.rng.gen_range(-factor..factor))
    }

    pub fn update_energy(&mut self) {
        let now = Utc::now();
        let dt_ms = (now - self.last_update).num_milliseconds() as f64;
        let dt_hours = dt_ms / 3_600_000.0 * self.config.time_accel;
        self.last_update = now;

        if dt_hours <= 0.0 {
            return;
        }

        let p_a = self.phase_a.active_power();
        let p_b = self.phase_b.active_power();
        let p_c = self.phase_c.active_power();
        let q_a = self.phase_a.reactive_power();
        let q_b = self.phase_b.reactive_power();
        let q_c = self.phase_c.reactive_power();

        self.energy.wh_a += p_a * dt_hours;
        self.energy.wh_b += p_b * dt_hours;
        self.energy.wh_c += p_c * dt_hours;
        self.energy.wh_total += (p_a + p_b + p_c) * dt_hours;
        self.energy.varh_a += q_a * dt_hours;
        self.energy.varh_b += q_b * dt_hours;
        self.energy.varh_c += q_c * dt_hours;
        self.energy.varh_total += (q_a + q_b + q_c) * dt_hours;

        // 脉冲累计
        let wh_delta = (p_a + p_b + p_c) * dt_hours;
        if self.pulse_constant > 0 {
            let pulses = (wh_delta * self.pulse_constant as f64) as u64;
            self.pulse_count += pulses;
        }
    }

    /* ── 快照 (主接口) ── */

    /// Update TOU and demand before snapshot
    fn update_modules(&mut self) {
        self.tou.update();
        let p_a = self.phase_a.active_power();
        let p_b = self.phase_b.active_power();
        let p_c = self.phase_c.active_power();
        let q_a = self.phase_a.reactive_power();
        let q_b = self.phase_b.reactive_power();
        let q_c = self.phase_c.reactive_power();
        self.demand.sample(p_a, p_b, p_c, q_a, q_b, q_c);
        let s_total = self.phase_a.apparent_power()
            + self.phase_b.apparent_power()
            + self.phase_c.apparent_power();
        let p_total = p_a + p_b + p_c;
        let pf = if s_total > 0.0 {
            p_total / s_total
        } else {
            0.0
        };
        self.statistics.sample(
            self.phase_a.voltage,
            self.phase_b.voltage,
            self.phase_c.voltage,
            self.phase_a.current,
            self.phase_b.current,
            self.phase_c.current,
            self.config.freq,
            pf,
        );
    }

    pub fn snapshot(&mut self) -> MeterSnapshot {
        self.update_energy();
        self.update_modules();
        self.detect_events();

        let p_a = self.apply_noise(self.phase_a.active_power());
        let p_b = self.apply_noise(self.phase_b.active_power());
        let p_c = self.apply_noise(self.phase_c.active_power());
        let q_a = self.apply_noise(self.phase_a.reactive_power());
        let q_b = self.apply_noise(self.phase_b.reactive_power());
        let q_c = self.apply_noise(self.phase_c.reactive_power());
        let s_a = self.apply_noise(self.phase_a.apparent_power());
        let s_b = self.apply_noise(self.phase_b.apparent_power());
        let s_c = self.apply_noise(self.phase_c.apparent_power());

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
            freq: self.apply_noise(self.config.freq),
            phase_a: self.phase_a.clone(),
            phase_b: self.phase_b.clone(),
            phase_c: self.phase_c.clone(),
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
                pf_a: self.phase_a.power_factor(),
                pf_b: self.phase_b.power_factor(),
                pf_c: self.phase_c.power_factor(),
                pf_total,
            },
            energy: self.energy.clone(),
            active_events: self.active_events.clone(),
        }
    }

    /// 格式化寄存器值 (ATT7022E SPI 模拟)
    pub fn format_register(&mut self, addr: u16) -> String {
        let snap = self.snapshot();
        let value: u32 = match addr {
            0x00 => (snap.phase_a.voltage * 1000.0) as u32,
            0x01 => (snap.phase_b.voltage * 1000.0) as u32,
            0x02 => (snap.phase_c.voltage * 1000.0) as u32,
            0x03 => (snap.phase_a.current * 1000.0) as u32,
            0x04 => (snap.phase_b.current * 1000.0) as u32,
            0x05 => (snap.phase_c.current * 1000.0) as u32,
            0x06 => (snap.computed.p_a * 100.0) as u32,
            0x07 => (snap.computed.p_b * 100.0) as u32,
            0x08 => (snap.computed.p_c * 100.0) as u32,
            0x09 => (snap.computed.p_total * 100.0) as u32,
            0x0A => (snap.freq * 100.0) as u32,
            0x0B => (snap.energy.wh_a * 100.0) as u32,
            0x0E => (snap.energy.wh_total * 100.0) as u32,
            0xFF => match self.config.chip {
                ChipType::ATT7022E => 0x7022E,
                ChipType::RN8302B => 0x8302B,
            },
            _ => 0,
        };
        let mask = match self.config.chip {
            ChipType::ATT7022E => 0x7FFFF,
            ChipType::RN8302B => 0xFFFFFF,
        };
        format!("{:06X}", value & mask)
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
}

pub type MeterHandle = Arc<Mutex<VirtualMeter>>;
pub fn create_meter() -> MeterHandle {
    Arc::new(Mutex::new(VirtualMeter::new()))
}
