//! 虚拟电表数据模型和计算
//!
//! 核心公式：
//! - 有功功率 P = U × I × cos(φ)
//! - 无功功率 Q = U × I × sin(φ)
//! - 视在功率 S = U × I
//! - 功率因数 PF = cos(φ)
//! - 电能累加 Wh += P × dt_hours

use chrono::{DateTime, Utc};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

/// 计量芯片类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChipType {
    /// ATT7022E: 19-bit, 精度 ±0.1%
    ATT7022E,
    /// RN8302B: 24-bit, 精度 ±0.01%
    RN8302B,
}

impl Default for ChipType {
    fn default() -> Self {
        ChipType::ATT7022E
    }
}

impl ChipType {
    /// 获取寄存器数据位宽
    pub fn bits(&self) -> u8 {
        match self {
            ChipType::ATT7022E => 19,
            ChipType::RN8302B => 24,
        }
    }

    /// 获取精度因子 (用于模拟噪声范围)
    pub fn precision_factor(&self) -> f64 {
        match self {
            ChipType::ATT7022E => 0.001, // ±0.1%
            ChipType::RN8302B => 0.0001, // ±0.01%
        }
    }
}

/// 单相数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseData {
    /// 电压 (V)
    pub voltage: f64,
    /// 电流 (A)
    pub current: f64,
    /// 相角 (度)
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
    /// 计算有功功率 P = U × I × cos(φ)
    pub fn active_power(&self) -> f64 {
        let angle_rad = self.angle * PI / 180.0;
        self.voltage * self.current * angle_rad.cos()
    }

    /// 计算无功功率 Q = U × I × sin(φ)
    pub fn reactive_power(&self) -> f64 {
        let angle_rad = self.angle * PI / 180.0;
        self.voltage * self.current * angle_rad.sin()
    }

    /// 计算视在功率 S = U × I
    pub fn apparent_power(&self) -> f64 {
        self.voltage * self.current
    }

    /// 计算功率因数 PF = cos(φ)
    pub fn power_factor(&self) -> f64 {
        let angle_rad = self.angle * PI / 180.0;
        angle_rad.cos()
    }
}

/// 电表配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterConfig {
    /// 计量芯片类型
    pub chip: ChipType,
    /// 频率 (Hz)
    pub freq: f64,
    /// 是否启用噪声模拟
    pub noise_enabled: bool,
}

impl Default for MeterConfig {
    fn default() -> Self {
        Self {
            chip: ChipType::ATT7022E,
            freq: 50.0,
            noise_enabled: false,
        }
    }
}

/// 电能累计数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnergyData {
    /// A相有功电能 (Wh)
    pub wh_a: f64,
    /// B相有功电能 (Wh)
    pub wh_b: f64,
    /// C相有功电能 (Wh)
    pub wh_c: f64,
    /// 总有功电能 (Wh)
    pub wh_total: f64,
    /// A相无功电能 (varh)
    pub varh_a: f64,
    /// B相无功电能 (varh)
    pub varh_b: f64,
    /// C相无功电能 (varh)
    pub varh_c: f64,
    /// 总无功电能 (varh)
    pub varh_total: f64,
}

/// 电表完整快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterSnapshot {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 芯片类型
    pub chip: ChipType,
    /// 频率 (Hz)
    pub freq: f64,
    /// A相数据
    pub phase_a: PhaseData,
    /// B相数据
    pub phase_b: PhaseData,
    /// C相数据
    pub phase_c: PhaseData,
    /// 计算值
    pub computed: ComputedValues,
    /// 电能累计
    pub energy: EnergyData,
}

/// 计算值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedValues {
    /// A相有功功率 (W)
    pub p_a: f64,
    /// B相有功功率 (W)
    pub p_b: f64,
    /// C相有功功率 (W)
    pub p_c: f64,
    /// 总有功功率 (W)
    pub p_total: f64,
    /// A相无功功率 (var)
    pub q_a: f64,
    /// B相无功功率 (var)
    pub q_b: f64,
    /// C相无功功率 (var)
    pub q_c: f64,
    /// 总无功功率 (var)
    pub q_total: f64,
    /// A相视在功率 (VA)
    pub s_a: f64,
    /// B相视在功率 (VA)
    pub s_b: f64,
    /// C相视在功率 (VA)
    pub s_c: f64,
    /// 总视在功率 (VA)
    pub s_total: f64,
    /// A相功率因数
    pub pf_a: f64,
    /// B相功率因数
    pub pf_b: f64,
    /// C相功率因数
    pub pf_c: f64,
    /// 总功率因数
    pub pf_total: f64,
}

/// 虚拟电表核心
#[derive(Debug)]
pub struct VirtualMeter {
    /// A相数据
    phase_a: PhaseData,
    /// B相数据
    phase_b: PhaseData,
    /// C相数据
    phase_c: PhaseData,
    /// 配置
    config: MeterConfig,
    /// 电能累计
    energy: EnergyData,
    /// 上次更新时间
    last_update: DateTime<Utc>,
    /// 随机数生成器
    rng: rand::rngs::StdRng,
}

impl Default for VirtualMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualMeter {
    /// 创建新电表
    pub fn new() -> Self {
        Self {
            phase_a: PhaseData::default(),
            phase_b: PhaseData::default(),
            phase_c: PhaseData::default(),
            config: MeterConfig::default(),
            energy: EnergyData::default(),
            last_update: Utc::now(),
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }

    /// 设置电压
    pub fn set_voltage(&mut self, phase: char, value: f64) {
        let phase = match phase.to_ascii_lowercase() {
            'a' => &mut self.phase_a,
            'b' => &mut self.phase_b,
            'c' => &mut self.phase_c,
            _ => return,
        };
        phase.voltage = value;
    }

    /// 设置电流
    pub fn set_current(&mut self, phase: char, value: f64) {
        let phase = match phase.to_ascii_lowercase() {
            'a' => &mut self.phase_a,
            'b' => &mut self.phase_b,
            'c' => &mut self.phase_c,
            _ => return,
        };
        phase.current = value;
    }

    /// 设置相角
    pub fn set_angle(&mut self, phase: char, value: f64) {
        let phase = match phase.to_ascii_lowercase() {
            'a' => &mut self.phase_a,
            'b' => &mut self.phase_b,
            'c' => &mut self.phase_c,
            _ => return,
        };
        phase.angle = value;
    }

    /// 设置频率
    pub fn set_freq(&mut self, freq: f64) {
        self.config.freq = freq;
    }

    /// 设置芯片类型
    pub fn set_chip(&mut self, chip: ChipType) {
        self.config.chip = chip;
    }

    /// 启用/禁用噪声
    pub fn set_noise(&mut self, enabled: bool) {
        self.config.noise_enabled = enabled;
    }

    /// 获取配置
    pub fn config(&self) -> &MeterConfig {
        &self.config
    }

    /// 获取电能数据
    pub fn energy(&self) -> &EnergyData {
        &self.energy
    }

    /// 重置电能
    pub fn reset_energy(&mut self) {
        self.energy = EnergyData::default();
        self.last_update = Utc::now();
    }

    /// 应用噪声
    fn apply_noise(&mut self, value: f64) -> f64 {
        if !self.config.noise_enabled {
            return value;
        }
        let factor = self.config.chip.precision_factor();
        let noise: f64 = self.rng.gen_range(-factor..factor);
        value * (1.0 + noise)
    }

    /// 更新电能累计
    pub fn update_energy(&mut self) {
        let now = Utc::now();
        let dt = (now - self.last_update).num_milliseconds() as f64 / 3_600_000.0; // 转为小时
        self.last_update = now;

        if dt <= 0.0 {
            return;
        }

        // 计算各相功率并累加电能
        let p_a = self.phase_a.active_power();
        let p_b = self.phase_b.active_power();
        let p_c = self.phase_c.active_power();
        let q_a = self.phase_a.reactive_power();
        let q_b = self.phase_b.reactive_power();
        let q_c = self.phase_c.reactive_power();

        self.energy.wh_a += p_a * dt;
        self.energy.wh_b += p_b * dt;
        self.energy.wh_c += p_c * dt;
        self.energy.wh_total += (p_a + p_b + p_c) * dt;

        self.energy.varh_a += q_a * dt;
        self.energy.varh_b += q_b * dt;
        self.energy.varh_c += q_c * dt;
        self.energy.varh_total += (q_a + q_b + q_c) * dt;
    }

    /// 获取完整快照
    pub fn snapshot(&mut self) -> MeterSnapshot {
        // 更新电能累计
        self.update_energy();

        // 计算各相值 (可能带噪声)
        let p_a = self.apply_noise(self.phase_a.active_power());
        let p_b = self.apply_noise(self.phase_b.active_power());
        let p_c = self.apply_noise(self.phase_c.active_power());
        let q_a = self.apply_noise(self.phase_a.reactive_power());
        let q_b = self.apply_noise(self.phase_b.reactive_power());
        let q_c = self.apply_noise(self.phase_c.reactive_power());
        let s_a = self.apply_noise(self.phase_a.apparent_power());
        let s_b = self.apply_noise(self.phase_b.apparent_power());
        let s_c = self.apply_noise(self.phase_c.apparent_power());
        let pf_a = self.phase_a.power_factor();
        let pf_b = self.phase_b.power_factor();
        let pf_c = self.phase_c.power_factor();

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
                pf_a,
                pf_b,
                pf_c,
                pf_total,
            },
            energy: self.energy.clone(),
        }
    }

    /// 格式化寄存器值为 hex (模拟芯片寄存器读取)
    pub fn format_register(&mut self, addr: u16) -> String {
        let snapshot = self.snapshot();

        // 根据地址返回对应寄存器值 (24-bit hex)
        let value: u32 = match addr {
            // 电压寄存器 (0x00-0x02)
            0x00 => (snapshot.phase_a.voltage * 1000.0) as u32,
            0x01 => (snapshot.phase_b.voltage * 1000.0) as u32,
            0x02 => (snapshot.phase_c.voltage * 1000.0) as u32,
            // 电流寄存器 (0x03-0x05)
            0x03 => (snapshot.phase_a.current * 1000.0) as u32,
            0x04 => (snapshot.phase_b.current * 1000.0) as u32,
            0x05 => (snapshot.phase_c.current * 1000.0) as u32,
            // 功率寄存器 (0x06-0x08)
            0x06 => (snapshot.computed.p_a * 100.0) as u32,
            0x07 => (snapshot.computed.p_b * 100.0) as u32,
            0x08 => (snapshot.computed.p_c * 100.0) as u32,
            // 总功率 (0x09)
            0x09 => (snapshot.computed.p_total * 100.0) as u32,
            // 频率 (0x0A)
            0x0A => (snapshot.freq * 100.0) as u32,
            // 电能 (0x0B-0x0D)
            0x0B => (snapshot.energy.wh_a * 100.0) as u32,
            0x0C => (snapshot.energy.wh_b * 100.0) as u32,
            0x0D => (snapshot.energy.wh_c * 100.0) as u32,
            // 总电能 (0x0E)
            0x0E => (snapshot.energy.wh_total * 100.0) as u32,
            // 芯片 ID
            0xFF => match self.config.chip {
                ChipType::ATT7022E => 0x7022E,
                ChipType::RN8302B => 0x8302B,
            },
            _ => 0,
        };

        // 根据芯片类型截断位数
        let mask = match self.config.chip {
            ChipType::ATT7022E => 0x7FFFF, // 19-bit
            ChipType::RN8302B => 0xFFFFFF, // 24-bit
        };

        format!("{:06X}", value & mask)
    }
}

/// 线程安全的电表句柄
pub type MeterHandle = Arc<Mutex<VirtualMeter>>;

/// 创建线程安全的电表
pub fn create_meter() -> MeterHandle {
    Arc::new(Mutex::new(VirtualMeter::new()))
}
