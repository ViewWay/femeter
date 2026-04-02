//! 校准参数
//!
//! 比例系数, 相位补偿, 脉冲常数, ATT7022E 校准寄存器模拟

use serde::{Deserialize, Serialize};

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
