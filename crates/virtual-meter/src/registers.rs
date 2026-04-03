//! ATT7022E 寄存器模型
//!
//! 模拟 ATT7022E 计量芯片的寄存器读写行为：
//! - 电压/电流/功率/电能寄存器
//! - 状态字
//! - 校表参数
//! - 24-bit 数据格式

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::meter::{InternalSnapshot, MeterEvent};
use crate::pulse::PulseAccumulator;

/// 寄存器格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterFormat {
    /// 无符号 24-bit (电压/电流)
    Unsigned24,
    /// 有符号 24-bit 补码 (功率)
    Signed24,
    /// BCD 编码 (电能)
    Bcd24,
    /// 状态字
    Status,
}

/// ATT7022E 寄存器映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Att7022eRegisters {
    /// 数据寄存器 (地址 -> 值)
    data: HashMap<u16, u32>,
    /// 状态字
    status: u32,
    /// 校表参数
    calibration: HashMap<u16, u32>,
    /// 配置寄存器
    config: HashMap<u16, u32>,
}

impl Default for Att7022eRegisters {
    fn default() -> Self {
        let mut reg = Self {
            data: HashMap::new(),
            status: 0,
            calibration: HashMap::new(),
            config: HashMap::new(),
        };
        reg.init_calibration();
        reg
    }
}

impl Att7022eRegisters {
    /// 创建新的寄存器模型
    pub fn new() -> Self {
        Self::default()
    }

    /// 初始化校表参数默认值
    fn init_calibration(&mut self) {
        // 电压增益 (A/B/C)
        self.calibration.insert(0x20, 0x000000);
        self.calibration.insert(0x21, 0x000000);
        self.calibration.insert(0x22, 0x000000);
        // 电流增益 (A/B/C)
        self.calibration.insert(0x23, 0x000000);
        self.calibration.insert(0x24, 0x000000);
        self.calibration.insert(0x25, 0x000000);
        // 相位补偿 (A/B/C)
        self.calibration.insert(0x2C, 0x000000);
        self.calibration.insert(0x2D, 0x000000);
        self.calibration.insert(0x2E, 0x000000);
        // 功率增益
        self.calibration.insert(0x2F, 0x000000);
        // 启动电流
        self.calibration.insert(0x30, 0x000010);
        // 潜动阈值
        self.calibration.insert(0x31, 0x000010);
    }

    /// 从数据更新寄存器值
    pub fn update_from_data(
        &mut self,
        snap: &InternalSnapshot,
        pulse: &PulseAccumulator,
        active_events: &[MeterEvent],
    ) {
        // 电压 RMS (0x00-0x02)
        self.data.insert(
            0x00,
            from_float_to_register(snap.measured_voltage[0], RegisterFormat::Unsigned24, 100.0),
        );
        self.data.insert(
            0x01,
            from_float_to_register(snap.measured_voltage[1], RegisterFormat::Unsigned24, 100.0),
        );
        self.data.insert(
            0x02,
            from_float_to_register(snap.measured_voltage[2], RegisterFormat::Unsigned24, 100.0),
        );

        // 电流 RMS (0x03-0x05)
        self.data.insert(
            0x03,
            from_float_to_register(snap.measured_current[0], RegisterFormat::Unsigned24, 1000.0),
        );
        self.data.insert(
            0x04,
            from_float_to_register(snap.measured_current[1], RegisterFormat::Unsigned24, 1000.0),
        );
        self.data.insert(
            0x05,
            from_float_to_register(snap.measured_current[2], RegisterFormat::Unsigned24, 1000.0),
        );

        // 有功功率 (0x06-0x08)
        self.data.insert(
            0x06,
            from_float_to_register(
                snap.measured_power[0].active,
                RegisterFormat::Signed24,
                100.0,
            ),
        );
        self.data.insert(
            0x07,
            from_float_to_register(
                snap.measured_power[1].active,
                RegisterFormat::Signed24,
                100.0,
            ),
        );
        self.data.insert(
            0x08,
            from_float_to_register(
                snap.measured_power[2].active,
                RegisterFormat::Signed24,
                100.0,
            ),
        );

        // 无功功率 (0x09-0x0B)
        self.data.insert(
            0x09,
            from_float_to_register(
                snap.measured_power[0].reactive,
                RegisterFormat::Signed24,
                100.0,
            ),
        );
        self.data.insert(
            0x0A,
            from_float_to_register(
                snap.measured_power[1].reactive,
                RegisterFormat::Signed24,
                100.0,
            ),
        );
        self.data.insert(
            0x0B,
            from_float_to_register(
                snap.measured_power[2].reactive,
                RegisterFormat::Signed24,
                100.0,
            ),
        );

        // 视在功率 (0x0C-0x0E)
        self.data.insert(
            0x0C,
            from_float_to_register(
                snap.measured_power[0].apparent,
                RegisterFormat::Unsigned24,
                100.0,
            ),
        );
        self.data.insert(
            0x0D,
            from_float_to_register(
                snap.measured_power[1].apparent,
                RegisterFormat::Unsigned24,
                100.0,
            ),
        );
        self.data.insert(
            0x0E,
            from_float_to_register(
                snap.measured_power[2].apparent,
                RegisterFormat::Unsigned24,
                100.0,
            ),
        );

        // 合相功率 (0x0F-0x11)
        let p_total = snap.measured_power[0].active
            + snap.measured_power[1].active
            + snap.measured_power[2].active;
        let q_total = snap.measured_power[0].reactive
            + snap.measured_power[1].reactive
            + snap.measured_power[2].reactive;
        let s_total = snap.measured_power[0].apparent
            + snap.measured_power[1].apparent
            + snap.measured_power[2].apparent;

        self.data.insert(
            0x0F,
            from_float_to_register(p_total, RegisterFormat::Signed24, 100.0),
        );
        self.data.insert(
            0x10,
            from_float_to_register(q_total, RegisterFormat::Signed24, 100.0),
        );
        self.data.insert(
            0x11,
            from_float_to_register(s_total, RegisterFormat::Unsigned24, 100.0),
        );

        // 频率 (0x12)
        self.data.insert(
            0x12,
            from_float_to_register(snap.freq, RegisterFormat::Unsigned24, 100.0),
        );

        // 有功电能 (0x13-0x15)
        self.data
            .insert(0x13, (pulse.active_energy_wh[0] * 100.0) as u32 & 0xFFFFFF);
        self.data
            .insert(0x14, (pulse.active_energy_wh[1] * 100.0) as u32 & 0xFFFFFF);
        self.data
            .insert(0x15, (pulse.active_energy_wh[2] * 100.0) as u32 & 0xFFFFFF);

        // 无功电能 (0x16-0x18)
        self.data.insert(
            0x16,
            (pulse.reactive_energy_varh[0] * 100.0) as u32 & 0xFFFFFF,
        );
        self.data.insert(
            0x17,
            (pulse.reactive_energy_varh[1] * 100.0) as u32 & 0xFFFFFF,
        );
        self.data.insert(
            0x18,
            (pulse.reactive_energy_varh[2] * 100.0) as u32 & 0xFFFFFF,
        );

        // 合相电能 (0x1C-0x1D)
        self.data
            .insert(0x1C, (pulse.active_total_wh * 100.0) as u32 & 0xFFFFFF);
        self.data
            .insert(0x1D, (pulse.reactive_total_varh * 100.0) as u32 & 0xFFFFFF);

        // 功率因数 (0x1E)
        let pf_total = if s_total > 0.0 {
            (p_total / s_total).abs()
        } else {
            1.0
        };
        self.data.insert(
            0x1E,
            from_float_to_register(pf_total, RegisterFormat::Unsigned24, 10000.0),
        );

        // 状态字 (0x1F)
        self.update_status_from_events(active_events);
    }

    /// 更新状态字
    fn update_status_from_events(&mut self, active_events: &[MeterEvent]) {
        let mut status: u32 = 0;

        // 从 active_events 更新状态位
        for event in active_events {
            status |= event_to_status_bit(*event);
        }

        // 数据更新标志
        status |= 0x80000000; // 数据有效

        self.status = status;
    }

    /// 读取寄存器 (返回 24-bit 值)
    pub fn read(&self, addr: u16) -> u32 {
        match addr {
            // 数据寄存器
            0x00..=0x1E => self.data.get(&addr).copied().unwrap_or(0),
            // 状态字
            0x1F => self.status,
            // 校表参数
            0x20..=0x3F => self.calibration.get(&addr).copied().unwrap_or(0),
            // 配置寄存器
            0x40..=0x7F => self.config.get(&addr).copied().unwrap_or(0),
            // 芯片 ID (特殊地址)
            0xFF => 0x7022E, // ATT7022E 芯片 ID
            _ => 0,
        }
    }

    /// 写入寄存器 (校表参数)
    pub fn write(&mut self, addr: u16, data: u32) -> Result<(), &'static str> {
        match addr {
            0x20..=0x3F => {
                // 校表参数，只保留 24-bit
                self.calibration.insert(addr, data & 0xFFFFFF);
                Ok(())
            }
            0x40..=0x7F => {
                // 配置寄存器
                self.config.insert(addr, data & 0xFFFFFF);
                Ok(())
            }
            _ => Err("Read-only register"),
        }
    }

    /// 获取校准增益
    pub fn get_calibration_gain(&self, addr: u16) -> f64 {
        let raw = self.calibration.get(&addr).copied().unwrap_or(0);
        // 校准寄存器是有符号 24-bit，增益 = 1 + reg / 2^23
        let signed = to_signed_24(raw);
        1.0 + signed as f64 / 8_388_608.0
    }

    /// 设置校准增益
    pub fn set_calibration_gain(&mut self, addr: u16, gain: f64) {
        // gain = 1 + reg / 2^23
        let signed = ((gain - 1.0) * 8_388_608.0) as i32;
        let raw = from_signed_24(signed);
        self.calibration.insert(addr, raw);
    }

    /// 读取十六进制格式
    pub fn read_hex(&self, addr: u16) -> String {
        format!("{:06X}", self.read(addr))
    }
}

/// 24-bit 补码转换为有符号整数
pub fn to_signed_24(val: u32) -> i32 {
    let val = val & 0xFFFFFF;
    if val & 0x800000 != 0 {
        // 负数：补码转换
        (val as i32) - 0x1000000
    } else {
        val as i32
    }
}

/// 有符号整数转换为 24-bit 补码
pub fn from_signed_24(val: i32) -> u32 {
    if val < 0 {
        ((val + 0x1000000) as u32) & 0xFFFFFF
    } else {
        (val as u32) & 0xFFFFFF
    }
}

/// 浮点数转换为寄存器值
///
/// # 参数
/// - `val`: 物理值
/// - `format`: 寄存器格式
/// - `scale`: 缩放因子 (如电压 *100, 电流 *1000)
pub fn from_float_to_register(val: f64, format: RegisterFormat, scale: f64) -> u32 {
    let scaled = (val * scale) as i64;

    match format {
        RegisterFormat::Unsigned24 => {
            if scaled < 0 {
                0
            } else {
                (scaled as u32) & 0xFFFFFF
            }
        }
        RegisterFormat::Signed24 => from_signed_24(scaled as i32),
        RegisterFormat::Bcd24 => {
            // BCD 编码：每 4-bit 表示一个十进制位
            let abs_val = scaled.abs().try_into().unwrap_or(0);
            let mut bcd = 0u32;
            let mut shift = 0u32;
            let mut remaining = abs_val;

            for _ in 0..6 {
                let digit = remaining % 10;
                bcd |= digit << shift;
                remaining /= 10;
                shift += 4;
            }

            bcd & 0xFFFFFF
        }
        RegisterFormat::Status => scaled as u32 & 0xFFFFFF,
    }
}

/// 寄存器值转换为浮点数
pub fn to_float_from_register(val: u32, format: RegisterFormat, scale: f64) -> f64 {
    match format {
        RegisterFormat::Unsigned24 => (val & 0xFFFFFF) as f64 / scale,
        RegisterFormat::Signed24 => to_signed_24(val) as f64 / scale,
        RegisterFormat::Bcd24 => {
            // BCD 解码
            let mut result = 0u32;
            let mut multiplier = 1u32;
            let mut remaining = val & 0xFFFFFF;

            for _ in 0..6 {
                let digit = remaining & 0xF;
                result += digit * multiplier;
                remaining >>= 4;
                multiplier *= 10;
            }

            result as f64 / scale
        }
        RegisterFormat::Status => val as f64,
    }
}

/// 事件到状态位映射
fn event_to_status_bit(event: MeterEvent) -> u32 {
    use MeterEvent::*;

    match event {
        OverVoltageA => 0x000001,
        OverVoltageB => 0x000002,
        OverVoltageC => 0x000004,
        UnderVoltageA => 0x000008,
        UnderVoltageB => 0x000010,
        UnderVoltageC => 0x000020,
        PhaseLossA => 0x000040,
        PhaseLossB => 0x000080,
        PhaseLossC => 0x000100,
        OverCurrentA => 0x000200,
        OverCurrentB => 0x000400,
        OverCurrentC => 0x000800,
        ReversePower => 0x001000,
        CoverOpen => 0x002000,
        TerminalCoverOpen => 0x004000,
        MagneticTamper => 0x008000,
        BatteryLow => 0x010000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signed_24_conversion() {
        assert_eq!(to_signed_24(0x000001), 1);
        assert_eq!(to_signed_24(0x7FFFFF), 8_388_607);
        assert_eq!(to_signed_24(0x800000), -8_388_608);
        assert_eq!(to_signed_24(0xFFFFFF), -1);

        assert_eq!(from_signed_24(1), 0x000001);
        assert_eq!(from_signed_24(-1), 0xFFFFFF);
        assert_eq!(from_signed_24(8_388_607), 0x7FFFFF);
        assert_eq!(from_signed_24(-8_388_608), 0x800000);
    }

    #[test]
    fn test_float_to_register() {
        // 电压 220.0V, scale=100 -> 22000
        let reg = from_float_to_register(220.0, RegisterFormat::Unsigned24, 100.0);
        assert_eq!(reg, 22000);

        // 功率 -1000W, scale=100 -> -100000
        let reg = from_float_to_register(-1000.0, RegisterFormat::Signed24, 100.0);
        assert_eq!(to_signed_24(reg), -100000);
    }

    #[test]
    fn test_register_to_float() {
        let val = to_float_from_register(22000, RegisterFormat::Unsigned24, 100.0);
        assert!((val - 220.0).abs() < 0.01);

        let val = to_float_from_register(0xFFFFFF, RegisterFormat::Signed24, 100.0);
        assert!((val - (-0.01)).abs() < 0.001);
    }

    #[test]
    fn test_calibration_gain() {
        let mut reg = Att7022eRegisters::new();

        // 增益 1.0 -> 寄存器 0
        reg.set_calibration_gain(0x20, 1.0);
        assert_eq!(reg.calibration.get(&0x20), Some(&0));

        // 增益 1.1
        reg.set_calibration_gain(0x20, 1.1);
        let gain = reg.get_calibration_gain(0x20);
        assert!((gain - 1.1).abs() < 0.001);
    }

    #[test]
    fn test_read_write() {
        let mut reg = Att7022eRegisters::new();

        // 写入校表参数
        assert!(reg.write(0x20, 0x123456).is_ok());
        assert_eq!(reg.read(0x20), 0x123456);

        // 只读寄存器
        assert!(reg.write(0x00, 0x123456).is_err());
    }
}
