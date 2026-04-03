//! 校准参数
//!
//! 比例系数, 相位补偿, 脉冲常数, ATT7022E 校准寄存器模拟
//!
//! v2.0 新增:
//! - PhaseCalibration: 单相校表系数 (增益/偏移/相角误差)
//! - CalibrationData: 完整校表数据 (三相 + 脉冲常数 + 启动电流 + 潜动阈值)

use serde::{Deserialize, Serialize};

/// 单相校表系数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhaseCalibration {
    /// 电压增益误差 (1.0 = 无误差)
    pub voltage_gain: f64,
    /// 电压偏移 (V)
    pub voltage_offset: f64,
    /// 电流增益误差
    pub current_gain: f64,
    /// 电流偏移 (A)
    pub current_offset: f64,
    /// 相角误差 (度)
    pub phase_angle_error: f64,
    /// 功率增益误差
    pub power_gain: f64,
}

impl Default for PhaseCalibration {
    fn default() -> Self {
        Self {
            voltage_gain: 1.0,
            voltage_offset: 0.0,
            current_gain: 1.0,
            current_offset: 0.0,
            phase_angle_error: 0.0,
            power_gain: 1.0,
        }
    }
}

impl PhaseCalibration {
    /// 应用校准到电压测量值
    pub fn calibrate_voltage(&self, raw: f64) -> f64 {
        raw * self.voltage_gain + self.voltage_offset
    }

    /// 应用校准到电流测量值
    pub fn calibrate_current(&self, raw: f64) -> f64 {
        raw * self.current_gain + self.current_offset
    }

    /// 应用校准到相角
    pub fn calibrate_angle(&self, raw: f64) -> f64 {
        raw + self.phase_angle_error
    }

    /// 应用校准到功率
    pub fn calibrate_power(&self, raw: f64) -> f64 {
        raw * self.power_gain
    }

    /// 重置为默认值
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// 完整校表数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationData {
    /// 三相校表系数 [A, B, C]
    pub phases: [PhaseCalibration; 3],
    /// 脉冲常数 (imp/kWh), 默认 3200
    pub pulse_constant: u32,
    /// 电表常数 (rounds/kWh)
    pub meter_constant: f64,
    /// 启动电流 (mA), 默认 0.4%Ib (Ib=10A -> 40mA)
    pub start_current_ma: f64,
    /// 潜动阈值 (W), 无电压时电流低于此值不计电能
    pub creep_threshold_w: f64,
}

impl Default for CalibrationData {
    fn default() -> Self {
        Self {
            phases: [PhaseCalibration::default(); 3],
            pulse_constant: 3200,
            meter_constant: 1.0,
            start_current_ma: 40.0, // 0.4% * 10A = 40mA
            creep_threshold_w: 1.0,
        }
    }
}

impl CalibrationData {
    /// 创建新的校表数据
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取指定相的校表系数
    pub fn phase(&self, index: usize) -> &PhaseCalibration {
        &self.phases[index.min(2)]
    }

    /// 获取可变指定相的校表系数
    pub fn phase_mut(&mut self, index: usize) -> &mut PhaseCalibration {
        &mut self.phases[index.min(2)]
    }

    /// 设置脉冲常数
    pub fn set_pulse_constant(&mut self, c: u32) {
        self.pulse_constant = c;
    }

    /// 设置启动电流 (mA)
    pub fn set_start_current(&mut self, ma: f64) {
        self.start_current_ma = ma;
    }

    /// 设置潜动阈值 (W)
    pub fn set_creep_threshold(&mut self, w: f64) {
        self.creep_threshold_w = w;
    }

    /// 获取启动电流 (A)
    pub fn start_current_a(&self) -> f64 {
        self.start_current_ma / 1000.0
    }

    /// 重置所有校表参数
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// 旧版校准参数 (兼容性保留)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationParams {
    /// 电压比例系数 (V/V)
    pub voltage_gain: [f64; 3],
    /// 电流比例系数 (A/A)
    pub current_gain: [f64; 3],
    /// 功率比例系数
    pub power_gain: f64,
    /// 频率比例系数
    pub freq_gain: f64,
    /// 相位补偿角 (度), [A, B, C]
    pub phase_comp: [f64; 3],
    /// 脉冲常数 (imp/kWh)
    pub pulse_constant: u32,
    /// 校准模式
    pub cal_mode: bool,
    /// 校准寄存器空间 (模拟 ATT7022E)
    pub registers: std::collections::HashMap<u16, u32>,
}

impl Default for CalibrationParams {
    fn default() -> Self {
        let mut registers = std::collections::HashMap::new();
        // ATT7022E 校准寄存器默认值
        registers.insert(0x20, 0x000000); // 电压增益A
        registers.insert(0x21, 0x000000); // 电压增益B
        registers.insert(0x22, 0x000000); // 电压增益C
        registers.insert(0x23, 0x000000); // 电流增益A
        registers.insert(0x24, 0x000000); // 电流增益B
        registers.insert(0x25, 0x000000); // 电流增益C
        registers.insert(0x2C, 0x000000); // 相位补偿A
        registers.insert(0x2D, 0x000000); // 相位补偿B
        registers.insert(0x2E, 0x000000); // 相位补偿C
        registers.insert(0x2F, 0x000000); // 功率增益
        Self {
            voltage_gain: [1.0, 1.0, 1.0],
            current_gain: [1.0, 1.0, 1.0],
            power_gain: 1.0,
            freq_gain: 1.0,
            phase_comp: [0.0, 0.0, 0.0],
            pulse_constant: 6400,
            cal_mode: false,
            registers,
        }
    }
}

impl CalibrationParams {
    pub fn set_voltage_gain(&mut self, phase: usize, gain: f64) {
        if phase < 3 {
            self.voltage_gain[phase] = gain;
        }
    }
    pub fn set_current_gain(&mut self, phase: usize, gain: f64) {
        if phase < 3 {
            self.current_gain[phase] = gain;
        }
    }
    pub fn set_phase_comp(&mut self, phase: usize, angle: f64) {
        if phase < 3 {
            self.phase_comp[phase] = angle;
        }
    }
    pub fn set_pulse_constant(&mut self, c: u32) {
        self.pulse_constant = c;
    }

    pub fn start_calibration(&mut self) {
        self.cal_mode = true;
    }
    pub fn end_calibration(&mut self) {
        self.cal_mode = false;
    }
    pub fn is_calibrating(&self) -> bool {
        self.cal_mode
    }

    /// 读写校准寄存器
    pub fn read_reg(&self, addr: u16) -> Option<u32> {
        self.registers.get(&addr).copied()
    }
    pub fn write_reg(&mut self, addr: u16, value: u32) {
        self.registers.insert(addr, value);
    }

    /// 应用校准到电压
    pub fn calibrate_voltage(&self, phase: usize, raw: f64) -> f64 {
        if phase < 3 {
            raw * self.voltage_gain[phase]
        } else {
            raw
        }
    }

    /// 应用校准到电流
    pub fn calibrate_current(&self, phase: usize, raw: f64) -> f64 {
        if phase < 3 {
            raw * self.current_gain[phase]
        } else {
            raw
        }
    }

    /// 应用校准到频率
    pub fn calibrate_freq(&self, raw: f64) -> f64 {
        raw * self.freq_gain
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_calibration() {
        let cal = CalibrationParams::default();
        assert_eq!(cal.pulse_constant, 6400);
        assert!(!cal.is_calibrating());
    }

    #[test]
    fn test_voltage_calibration() {
        let mut cal = CalibrationParams::default();
        cal.set_voltage_gain(0, 1.1);
        assert!((cal.calibrate_voltage(0, 220.0) - 242.0).abs() < 0.01);
    }

    #[test]
    fn test_registers() {
        let mut cal = CalibrationParams::default();
        cal.write_reg(0x20, 0x123456);
        assert_eq!(cal.read_reg(0x20), Some(0x123456));
    }

    #[test]
    fn test_calibration_mode() {
        let mut cal = CalibrationParams::default();
        cal.start_calibration();
        assert!(cal.is_calibrating());
        cal.end_calibration();
        assert!(!cal.is_calibrating());
    }
}
