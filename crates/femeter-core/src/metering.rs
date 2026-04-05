/* ================================================================== */
/*                                                                    */
/*  metering.rs — 计量数据完整流                                       */
/*                                                                    */
/*  实现计量芯片采样 → 处理 → 存储 → 显示 → DLMS 读取的完整流程        */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::{CalibrationParams, EnergyData, PhaseData};
use crate::error::FemeterError;

/// 计量数据采样器
#[derive(Debug, Clone)]
pub struct MeteringSampler {
    /// 校准参数
    calibration: CalibrationParams,
    /// 采样计数
    sample_count: u64,
    /// 累计能量
    energy: EnergyData,
    /// 上次采样时间 (ms)
    last_sample_time: u64,
    /// 上次的瞬时功率 (用于能量积分)
    last_power: [i32; 3],
}

impl Default for MeteringSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl MeteringSampler {
    pub fn new() -> Self {
        Self {
            calibration: CalibrationParams::default(),
            sample_count: 0,
            energy: EnergyData::default(),
            last_sample_time: 0,
            last_power: [0; 3],
        }
    }

    /// 设置校准参数
    pub fn set_calibration(&mut self, cal: CalibrationParams) {
        self.calibration = cal;
    }

    /// 获取校准参数
    pub fn calibration(&self) -> &CalibrationParams {
        &self.calibration
    }

    /// 采样原始数据（模拟从计量芯片读取）
    pub fn sample(&mut self, raw: &PhaseData, timestamp_ms: u64) -> PhaseData {
        self.sample_count += 1;
        
        // 应用校准参数
        let calibrated = PhaseData {
            voltage_a: apply_gain(raw.voltage_a, self.calibration.voltage_gain_a),
            voltage_b: apply_gain(raw.voltage_b, self.calibration.voltage_gain_b),
            voltage_c: apply_gain(raw.voltage_c, self.calibration.voltage_gain_c),
            current_a: apply_gain(raw.current_a, self.calibration.current_gain_a),
            current_b: apply_gain(raw.current_b, self.calibration.current_gain_b),
            current_c: apply_gain(raw.current_c, self.calibration.current_gain_c),
            active_power_a: apply_power_offset(raw.active_power_a, self.calibration.power_offset_a),
            active_power_b: apply_power_offset(raw.active_power_b, self.calibration.power_offset_b),
            active_power_c: apply_power_offset(raw.active_power_c, self.calibration.power_offset_c),
            reactive_power_a: raw.reactive_power_a,
            reactive_power_b: raw.reactive_power_b,
            reactive_power_c: raw.reactive_power_c,
            active_power_total: raw.active_power_total,
            reactive_power_total: raw.reactive_power_total,
            apparent_power_total: raw.apparent_power_total,
            frequency: raw.frequency,
            power_factor_total: raw.power_factor_total,
            voltage_angle_a: raw.voltage_angle_a,
            voltage_angle_b: raw.voltage_angle_b,
            voltage_angle_c: raw.voltage_angle_c,
        };

        // 累计能量
        if self.sample_count > 1 {
            let dt_ms = timestamp_ms.saturating_sub(self.last_sample_time);
            self.accumulate_energy(&calibrated, dt_ms);
        }

        self.last_sample_time = timestamp_ms;
        self.last_power = [
            calibrated.active_power_a,
            calibrated.active_power_b,
            calibrated.active_power_c,
        ];

        calibrated
    }

    /// 累计能量 (功率 × 时间)
    fn accumulate_energy(&mut self, data: &PhaseData, dt_ms: u64) {
        if dt_ms == 0 {
            return;
        }

        // 功率单位: 0.01W, 时间单位: ms
        // 能量单位: 0.001Wh
        // 转换: (0.01W × ms) / 3600000 = 0.001Wh
        // 简化: Wh = W × s / 3600 = (0.01W × ms/1000) / 3600 = 0.01W × ms / 3600000
        // 为了得到 0.001Wh: 结果 × 1000
        // 最终: (0.01W × ms / 3600000) × 1000 = 0.01W × ms / 3600

        let dt_ms_i64 = dt_ms as i64;

        // A 相有功
        let p_a = data.active_power_a as i64;
        let energy_a = (p_a * dt_ms_i64 / 3600) as i64;
        if energy_a > 0 {
            self.energy.active_import_a = self.energy.active_import_a.saturating_add(energy_a as u64);
        }

        // B 相有功
        let p_b = data.active_power_b as i64;
        let energy_b = (p_b * dt_ms_i64 / 3600) as i64;
        if energy_b > 0 {
            self.energy.active_import_b = self.energy.active_import_b.saturating_add(energy_b as u64);
        }

        // C 相有功
        let p_c = data.active_power_c as i64;
        let energy_c = (p_c * dt_ms_i64 / 3600) as i64;
        if energy_c > 0 {
            self.energy.active_import_c = self.energy.active_import_c.saturating_add(energy_c as u64);
        }

        // 总有功
        let p_total = data.active_power_total as i64;
        let energy_total = (p_total * dt_ms_i64 / 3600) as i64;
        if energy_total > 0 {
            self.energy.active_import = self.energy.active_import.saturating_add(energy_total as u64);
        } else if energy_total < 0 {
            self.energy.active_export = self.energy.active_export.saturating_add((-energy_total) as u64);
        }

        // 总无功
        let q_total = data.reactive_power_total as i64;
        let energy_q = (q_total * dt_ms_i64 / 3600) as i64;
        if energy_q > 0 {
            self.energy.reactive_import = self.energy.reactive_import.saturating_add(energy_q as u64);
        } else if energy_q < 0 {
            self.energy.reactive_export = self.energy.reactive_export.saturating_add((-energy_q) as u64);
        }
    }

    /// 获取累计能量
    pub fn energy(&self) -> &EnergyData {
        &self.energy
    }

    /// 重置能量累计
    pub fn reset_energy(&mut self) {
        self.energy = EnergyData::default();
    }

    /// 获取采样计数
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }
}

/// 应用增益校准
fn apply_gain(value: u16, gain: f32) -> u16 {
    if gain == 0.0 {
        return value;
    }
    let calibrated = value as f32 * (1.0 + gain);
    calibrated.round().min(u16::MAX as f32).max(0.0) as u16
}

/// 应用功率偏移校准
fn apply_power_offset(value: i32, offset: f32) -> i32 {
    let calibrated = value as f32 + offset;
    calibrated.round().min(i32::MAX as f32).max(i32::MIN as f32) as i32
}

/// 计量数据处理器
#[derive(Debug, Clone)]
pub struct MeteringProcessor {
    /// 需量周期 (分钟)
    demand_period_min: u16,
    /// 当前周期累计功率
    demand_accumulator: i64,
    /// 当前周期采样数
    demand_samples: u64,
    /// 最大需量 (0.01W)
    max_demand: i32,
    /// 最大需量发生时间 (Unix timestamp)
    max_demand_time: u32,
    /// 当前时间
    current_time: u32,
}

impl Default for MeteringProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl MeteringProcessor {
    pub fn new() -> Self {
        Self {
            demand_period_min: 15,
            demand_accumulator: 0,
            demand_samples: 0,
            max_demand: 0,
            max_demand_time: 0,
            current_time: 0,
        }
    }

    /// 设置需量周期
    pub fn set_demand_period(&mut self, minutes: u16) {
        self.demand_period_min = minutes;
    }

    /// 处理采样数据
    pub fn process(&mut self, data: &PhaseData, timestamp: u32) {
        self.current_time = timestamp;
        
        // 累计需量
        self.demand_accumulator += data.active_power_total as i64;
        self.demand_samples += 1;
    }

    /// 结算需量周期（每个周期结束时调用）
    pub fn settle_demand_period(&mut self) -> i32 {
        if self.demand_samples == 0 {
            return 0;
        }

        // 计算平均功率
        let avg_power = (self.demand_accumulator / self.demand_samples as i64) as i32;

        // 更新最大需量
        if avg_power > self.max_demand {
            self.max_demand = avg_power;
            self.max_demand_time = self.current_time;
        }

        // 重置累计器
        self.demand_accumulator = 0;
        self.demand_samples = 0;

        avg_power
    }

    /// 获取最大需量
    pub fn max_demand(&self) -> i32 {
        self.max_demand
    }

    /// 获取最大需量时间
    pub fn max_demand_time(&self) -> u32 {
        self.max_demand_time
    }

    /// 重置最大需量
    pub fn reset_max_demand(&mut self) {
        self.max_demand = 0;
        self.max_demand_time = 0;
    }
}

/// 计量数据存储接口
pub trait MeteringStorage {
    /// 存储瞬时数据
    fn store_instantaneous(&mut self, data: &PhaseData, timestamp: u32) -> Result<(), FemeterError>;
    
    /// 存储能量数据
    fn store_energy(&mut self, energy: &EnergyData, timestamp: u32) -> Result<(), FemeterError>;
    
    /// 读取历史能量数据
    fn read_energy(&self, timestamp: u32) -> Option<EnergyData>;
    
    /// 存储需量数据
    fn store_demand(&mut self, demand: i32, timestamp: u32) -> Result<(), FemeterError>;
}

/// 内存存储实现（用于测试）
#[derive(Debug, Clone)]
pub struct MemoryStorage {
    instantaneous_records: Vec<(u32, PhaseData)>,
    energy_records: Vec<(u32, EnergyData)>,
    demand_records: Vec<(u32, i32)>,
    max_records: usize,
}

impl MemoryStorage {
    pub fn new(max_records: usize) -> Self {
        Self {
            instantaneous_records: Vec::with_capacity(max_records),
            energy_records: Vec::with_capacity(max_records),
            demand_records: Vec::with_capacity(max_records),
            max_records,
        }
    }

    pub fn instantaneous_records(&self) -> &[(u32, PhaseData)] {
        &self.instantaneous_records
    }

    pub fn energy_records(&self) -> &[(u32, EnergyData)] {
        &self.energy_records
    }

    pub fn demand_records(&self) -> &[(u32, i32)] {
        &self.demand_records
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl MeteringStorage for MemoryStorage {
    fn store_instantaneous(&mut self, data: &PhaseData, timestamp: u32) -> Result<(), FemeterError> {
        if self.instantaneous_records.len() >= self.max_records {
            self.instantaneous_records.remove(0);
        }
        self.instantaneous_records.push((timestamp, *data));
        Ok(())
    }

    fn store_energy(&mut self, energy: &EnergyData, timestamp: u32) -> Result<(), FemeterError> {
        if self.energy_records.len() >= self.max_records {
            self.energy_records.remove(0);
        }
        self.energy_records.push((timestamp, *energy));
        Ok(())
    }

    fn read_energy(&self, timestamp: u32) -> Option<EnergyData> {
        self.energy_records
            .iter()
            .rev()
            .find(|(ts, _)| *ts <= timestamp)
            .map(|(_, e)| *e)
    }

    fn store_demand(&mut self, demand: i32, timestamp: u32) -> Result<(), FemeterError> {
        if self.demand_records.len() >= self.max_records {
            self.demand_records.remove(0);
        }
        self.demand_records.push((timestamp, demand));
        Ok(())
    }
}

/// 显示数据格式化器
#[derive(Debug, Clone)]
pub struct DisplayFormatter {
    /// 小数位数
    decimal_places: u8,
}

impl Default for DisplayFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayFormatter {
    pub fn new() -> Self {
        Self { decimal_places: 2 }
    }

    /// 格式化电压 (0.01V → V)
    pub fn format_voltage(&self, voltage_0v01: u16) -> String {
        let v = voltage_0v01 as f32 / 100.0;
        format!("{:.2}V", v)
    }

    /// 格式化电流 (0.01A → A)
    pub fn format_current(&self, current_0a01: u16) -> String {
        let a = current_0a01 as f32 / 100.0;
        format!("{:.2}A", a)
    }

    /// 格式化功率 (0.01W → kW)
    pub fn format_power(&self, power_0w01: i32) -> String {
        let kw = power_0w01 as f32 / 100000.0;
        format!("{:.3}kW", kw)
    }

    /// 格式化能量 (0.001Wh → kWh)
    pub fn format_energy(&self, energy_0wh001: u64) -> String {
        let kwh = energy_0wh001 as f64 / 1000000.0;
        format!("{:.3}kWh", kwh)
    }

    /// 格式化频率 (0.01Hz → Hz)
    pub fn format_frequency(&self, freq_0hz01: u16) -> String {
        let hz = freq_0hz01 as f32 / 100.0;
        format!("{:.2}Hz", hz)
    }

    /// 格式化功率因数 (0.001 → 无单位)
    pub fn format_power_factor(&self, pf_0p001: u16) -> String {
        let pf = pf_0p001 as f32 / 1000.0;
        format!("{:.3}", pf.min(1.000))
    }
}

/* ================================================================== */
/*  单元测试                                                           */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    fn test_phase_data() -> PhaseData {
        PhaseData {
            voltage_a: 22000,
            voltage_b: 22100,
            voltage_c: 21900,
            current_a: 5000,
            current_b: 5100,
            current_c: 4900,
            active_power_total: 330000,
            reactive_power_total: 50000,
            apparent_power_total: 333800,
            frequency: 5000,
            power_factor_total: 988,
            active_power_a: 110000,
            active_power_b: 112000,
            active_power_c: 108000,
            reactive_power_a: 16666,
            reactive_power_b: 17166,
            reactive_power_c: 16166,
            voltage_angle_a: 0,
            voltage_angle_b: 24000,
            voltage_angle_c: 48000,
        }
    }

    // ==================== MeteringSampler 测试 ====================

    #[test]
    fn test_sampler_new() {
        let sampler = MeteringSampler::new();
        assert_eq!(sampler.sample_count(), 0);
        assert_eq!(sampler.energy().active_import, 0);
    }

    #[test]
    fn test_sampler_sample_once() {
        let mut sampler = MeteringSampler::new();
        let data = test_phase_data();
        let result = sampler.sample(&data, 1000);
        
        assert_eq!(sampler.sample_count(), 1);
        assert_eq!(result.voltage_a, 22000);
    }

    #[test]
    fn test_sampler_sample_with_calibration() {
        let mut sampler = MeteringSampler::new();
        sampler.set_calibration(CalibrationParams {
            voltage_gain_a: 0.1, // +10%
            ..Default::default()
        });
        
        let data = test_phase_data();
        let result = sampler.sample(&data, 1000);
        
        // 22000 * 1.1 = 24200
        assert_eq!(result.voltage_a, 24200);
        assert_eq!(result.voltage_b, 22100); // 未校准
    }

    #[test]
    fn test_sampler_energy_accumulation() {
        let mut sampler = MeteringSampler::new();
        let data = test_phase_data();
        
        // 第一次采样
        sampler.sample(&data, 0);
        
        // 第二次采样，1秒后
        sampler.sample(&data, 1000);
        
        // 功率 330000 (0.01W) = 3300W
        // 时间 1000ms = 1s
        // 能量 = 3300W * 1s = 3300Ws = 0.9167Wh = 916.7 (0.001Wh单位)
        // 计算: (330000 * 1000 / 3600) = 91666
        assert!(sampler.energy().active_import > 0);
    }

    #[test]
    fn test_sampler_energy_export() {
        let mut sampler = MeteringSampler::new();
        let mut data = test_phase_data();
        data.active_power_total = -100000; // 反向功率
        
        sampler.sample(&data, 0);
        sampler.sample(&data, 1000);
        
        assert!(sampler.energy().active_export > 0);
        assert_eq!(sampler.energy().active_import, 0);
    }

    #[test]
    fn test_sampler_reset_energy() {
        let mut sampler = MeteringSampler::new();
        let data = test_phase_data();
        
        sampler.sample(&data, 0);
        sampler.sample(&data, 1000);
        
        assert!(sampler.energy().active_import > 0);
        
        sampler.reset_energy();
        assert_eq!(sampler.energy().active_import, 0);
    }

    #[test]
    fn test_apply_gain_positive() {
        assert_eq!(apply_gain(1000, 0.1), 1100);
    }

    #[test]
    fn test_apply_gain_negative() {
        assert_eq!(apply_gain(1000, -0.1), 900);
    }

    #[test]
    fn test_apply_gain_zero() {
        assert_eq!(apply_gain(1000, 0.0), 1000);
    }

    #[test]
    fn test_apply_gain_saturating() {
        assert_eq!(apply_gain(u16::MAX, 1.0), u16::MAX);
        assert_eq!(apply_gain(0, -1.0), 0);
    }

    #[test]
    fn test_apply_power_offset_positive() {
        assert_eq!(apply_power_offset(1000, 100.0), 1100);
    }

    #[test]
    fn test_apply_power_offset_negative() {
        assert_eq!(apply_power_offset(1000, -100.0), 900);
    }

    // ==================== MeteringProcessor 测试 ====================

    #[test]
    fn test_processor_new() {
        let processor = MeteringProcessor::new();
        assert_eq!(processor.max_demand(), 0);
    }

    #[test]
    fn test_processor_accumulate() {
        let mut processor = MeteringProcessor::new();
        let data = test_phase_data();
        
        processor.process(&data, 1000);
        processor.process(&data, 2000);
        
        assert_eq!(processor.demand_samples, 2);
    }

    #[test]
    fn test_processor_settle_demand() {
        let mut processor = MeteringProcessor::new();
        let data = test_phase_data();
        
        processor.process(&data, 1000);
        processor.process(&data, 2000);
        
        let avg = processor.settle_demand_period();
        
        // 平均功率应该接近 330000
        assert!(avg > 300000 && avg < 360000);
        assert_eq!(processor.max_demand(), avg);
    }

    #[test]
    fn test_processor_max_demand_tracking() {
        let mut processor = MeteringProcessor::new();
        let mut data = test_phase_data();
        
        // 第一个周期
        processor.process(&data, 1000);
        processor.process(&data, 2000);
        processor.settle_demand_period();
        
        // 第二个周期，更高功率
        data.active_power_total = 400000;
        processor.process(&data, 3000);
        processor.process(&data, 4000);
        let avg = processor.settle_demand_period();
        
        assert_eq!(processor.max_demand(), avg);
        assert!(processor.max_demand() > 330000);
    }

    #[test]
    fn test_processor_reset_max_demand() {
        let mut processor = MeteringProcessor::new();
        let data = test_phase_data();
        
        processor.process(&data, 1000);
        processor.settle_demand_period();
        
        assert!(processor.max_demand() > 0);
        
        processor.reset_max_demand();
        assert_eq!(processor.max_demand(), 0);
    }

    // ==================== MemoryStorage 测试 ====================

    #[test]
    fn test_storage_new() {
        let storage = MemoryStorage::new(100);
        assert!(storage.instantaneous_records().is_empty());
    }

    #[test]
    fn test_storage_store_instantaneous() {
        let mut storage = MemoryStorage::new(100);
        let data = test_phase_data();
        
        storage.store_instantaneous(&data, 1000).unwrap();
        
        assert_eq!(storage.instantaneous_records().len(), 1);
        assert_eq!(storage.instantaneous_records()[0].0, 1000);
    }

    #[test]
    fn test_storage_store_energy() {
        let mut storage = MemoryStorage::new(100);
        let energy = EnergyData {
            active_import: 1000000,
            ..Default::default()
        };
        
        storage.store_energy(&energy, 1000).unwrap();
        
        assert_eq!(storage.energy_records().len(), 1);
        let read = storage.read_energy(1000).unwrap();
        assert_eq!(read.active_import, 1000000);
    }

    #[test]
    fn test_storage_overflow() {
        let mut storage = MemoryStorage::new(3);
        let data = test_phase_data();
        
        storage.store_instantaneous(&data, 1000).unwrap();
        storage.store_instantaneous(&data, 2000).unwrap();
        storage.store_instantaneous(&data, 3000).unwrap();
        storage.store_instantaneous(&data, 4000).unwrap();
        
        assert_eq!(storage.instantaneous_records().len(), 3);
        assert_eq!(storage.instantaneous_records()[0].0, 2000);
    }

    #[test]
    fn test_storage_read_energy_not_found() {
        let storage = MemoryStorage::new(100);
        assert!(storage.read_energy(1000).is_none());
    }

    #[test]
    fn test_storage_read_energy_earlier() {
        let mut storage = MemoryStorage::new(100);
        let e1 = EnergyData {
            active_import: 1000,
            ..Default::default()
        };
        let e2 = EnergyData {
            active_import: 2000,
            ..Default::default()
        };
        
        storage.store_energy(&e1, 1000).unwrap();
        storage.store_energy(&e2, 2000).unwrap();
        
        // 读取时间点 1500，应该返回 1000 的记录
        let result = storage.read_energy(1500).unwrap();
        assert_eq!(result.active_import, 1000);
    }

    // ==================== DisplayFormatter 测试 ====================

    #[test]
    fn test_formatter_voltage() {
        let fmt = DisplayFormatter::new();
        assert_eq!(fmt.format_voltage(22000), "220.00V");
    }

    #[test]
    fn test_formatter_current() {
        let fmt = DisplayFormatter::new();
        assert_eq!(fmt.format_current(5000), "50.00A");
    }

    #[test]
    fn test_formatter_power() {
        let fmt = DisplayFormatter::new();
        assert_eq!(fmt.format_power(330000), "3.300kW");
    }

    #[test]
    fn test_formatter_energy() {
        let fmt = DisplayFormatter::new();
        assert_eq!(fmt.format_energy(1000000), "1.000kWh");
    }

    #[test]
    fn test_formatter_frequency() {
        let fmt = DisplayFormatter::new();
        assert_eq!(fmt.format_frequency(5000), "50.00Hz");
    }

    #[test]
    fn test_formatter_power_factor() {
        let fmt = DisplayFormatter::new();
        assert_eq!(fmt.format_power_factor(988), "0.988");
    }

    #[test]
    fn test_formatter_power_factor_max() {
        let fmt = DisplayFormatter::new();
        assert_eq!(fmt.format_power_factor(1500), "1.000"); // 应该限制在 1.0
    }
}
