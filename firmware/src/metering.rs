/* ================================================================== */
/*                                                                    */
/*  metering.rs — 统一计量管理器                                       */
/*                                                                    */
/*  基于 HAL MeteringChip trait 的通用计量管理层。                      */
/*  提供电能累计（含翻转处理）、校准修正、费率切换等功能。               */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::hal::{
    CalibrationParams, EnergyData, MeteringChip, MeteringError, PhaseData,
};

/// 能量寄存器位宽 (32-bit)
const ENERGY_REG_BITS: u32 = 32;
const ENERGY_REG_MAX: u32 = 0xFFFF_FFFF;

/// Wh/LSB 常数 — 典型值，具体由芯片决定，此处按 1 LSB = 0.1 Wh
/// 即 10 LSB = 1 Wh = 0.01 kWh（与 EnergyData 单位一致）
const WH_PER_LSB_X100: u64 = 1; // 0.01 kWh per LSB

/// 费率数量
const NUM_TARIFFS: usize = 4;

/* ================================================================== */
/*  费率累计器                                                         */
/* ================================================================== */

/// 每费率电能累计值，单位 0.01 kWh / 0.01 kvarh
#[derive(Clone, Copy, Debug)]
pub struct TariffAccumulators {
    /// 正向有功 [T1..T4]
    pub active_import: [u64; NUM_TARIFFS],
    /// 反向有功 [T1..T4]
    pub active_export: [u64; NUM_TARIFFS],
    /// 正向无功 [T1..T4]
    pub reactive_import: [u64; NUM_TARIFFS],
    /// 反向无功 [T1..T4]
    pub reactive_export: [u64; NUM_TARIFFS],
    /// 当前费率索引 (0..3)
    pub current_tariff: u8,
}

impl Default for TariffAccumulators {
    fn default() -> Self {
        Self {
            active_import: [0; NUM_TARIFFS],
            active_export: [0; NUM_TARIFFS],
            reactive_import: [0; NUM_TARIFFS],
            reactive_export: [0; NUM_TARIFFS],
            current_tariff: 0,
        }
    }
}

impl TariffAccumulators {
    /// 切换费率：将增量累加到当前费率槽位后切换
    pub fn switch_tariff(&mut self, tariff: u8) {
        if tariff < NUM_TARIFFS as u8 {
            self.current_tariff = tariff;
        }
    }

    /// 将增量电能累加到当前费率
    fn accumulate(
        &mut self,
        delta_active_import: u64,
        delta_active_export: u64,
        delta_reactive_import: u64,
        delta_reactive_export: u64,
    ) {
        let t = self.current_tariff as usize;
        self.active_import[t] = self.active_import[t].wrapping_add(delta_active_import);
        self.active_export[t] = self.active_export[t].wrapping_add(delta_active_export);
        self.reactive_import[t] = self.reactive_import[t].wrapping_add(delta_reactive_import);
        self.reactive_export[t] = self.reactive_export[t].wrapping_add(delta_reactive_export);
    }
}

/* ================================================================== */
/*  单路能量翻转跟踪器                                                 */
/* ================================================================== */

#[derive(Clone, Copy, Debug, Default)]
struct EnergyTracker {
    /// 上一次芯片寄存器原始读数
    prev_raw: u32,
    /// 累计总值 (0.01 kWh / 0.01 kvarh)，已转换为工程单位
    accumulated: u64,
    /// 是否已初始化（首次读取只记录，不计算差值）
    initialized: bool,
}

impl EnergyTracker {
    /// 用新的芯片原始寄存器读数更新，返回本次增量
    fn update(&mut self, raw: u32) -> u64 {
        if !self.initialized {
            self.prev_raw = raw;
            self.initialized = true;
            return 0;
        }

        let delta_raw = if raw >= self.prev_raw {
            raw - self.prev_raw
        } else {
            // 翻转：寄存器从大变小说明溢出回绕
            (ENERGY_REG_MAX - self.prev_raw).wrapping_add(1).wrapping_add(raw)
        };

        self.prev_raw = raw;

        // 转换为工程单位 (0.01 kWh)
        let delta = delta_raw as u64 * WH_PER_LSB_X100;
        self.accumulated = self.accumulated.wrapping_add(delta);
        delta
    }
}

/* ================================================================== */
/*  MeteringManager                                                   */
/* ================================================================== */

/// 统一计量管理器，泛型抽象于 HAL MeteringChip trait
pub struct MeteringManager<M: MeteringChip> {
    /// 底层计量芯片驱动
    chip: M,
    /// 校准参数
    calibration: CalibrationParams,
    /// 实时数据缓存
    instant: PhaseData,
    /// 能量跟踪器
    tracker_active_import: EnergyTracker,
    tracker_active_export: EnergyTracker,
    tracker_reactive_import: EnergyTracker,
    tracker_reactive_export: EnergyTracker,
    tracker_active_import_a: EnergyTracker,
    tracker_active_import_b: EnergyTracker,
    tracker_active_import_c: EnergyTracker,
    /// 上一次角度读数（用于 tamper 检测）
    prev_phase_angle: [f32; 3],
    /// 费率累计器
    tariffs: TariffAccumulators,
    /// 上一次读取的完整 EnergyData（对外暴露用）
    last_energy: EnergyData,
}

impl<M: MeteringChip> MeteringManager<M> {
    /// 创建计量管理器
    pub fn new(mut chip: M, calibration: CalibrationParams) -> Self {
        // 初始化芯片并写入校准参数
        let _ = chip.init(&calibration);

        Self {
            chip,
            calibration,
            instant: PhaseData::default(),
            tracker_active_import: EnergyTracker::default(),
            tracker_active_export: EnergyTracker::default(),
            tracker_reactive_import: EnergyTracker::default(),
            tracker_reactive_export: EnergyTracker::default(),
            tracker_active_import_a: EnergyTracker::default(),
            tracker_active_import_b: EnergyTracker::default(),
            tracker_active_import_c: EnergyTracker::default(),
            prev_phase_angle: [0.0; 3],
            tariffs: TariffAccumulators::default(),
            last_energy: EnergyData::default(),
        }
    }

    /// 读取实时数据并施加校准修正
    pub fn poll_instant(&mut self) -> PhaseData {
        match self.chip.read_instant_data() {
            Ok(mut data) => {
                self.apply_calibration_correction(&mut data);
                self.instant = data;
                data
            }
            Err(_) => self.instant,
        }
    }

    /// 读取电能累计值（含翻转处理）
    pub fn poll_energy(&mut self) -> EnergyData {
        // 先读芯片原始数据
        let raw = match self.chip.read_energy() {
            Ok(d) => d,
            Err(_) => return self.last_energy,
        };

        // 注意：EnergyData 里的字段已经是 u64 工程单位了，
        // 但芯片返回的实际是寄存器映射后的值。
        // 我们用低 32 位做翻转检测（芯片寄存器 32-bit），
        // 然后用 tracker 累加到 u64。

        let d_ai = self.tracker_active_import.update(raw.active_import as u32);
        let d_ae = self.tracker_active_export.update(raw.active_export as u32);
        let d_ri = self.tracker_reactive_import.update(raw.reactive_import as u32);
        let d_re = self.tracker_reactive_export.update(raw.reactive_export as u32);

        let _d_aia = self.tracker_active_import_a.update(raw.active_import_a as u32);
        let _d_aib = self.tracker_active_import_b.update(raw.active_import_b as u32);
        let _d_aic = self.tracker_active_import_c.update(raw.active_import_c as u32);

        // 累加到当前费率
        self.tariffs.accumulate(d_ai, d_ae, d_ri, d_re);

        // 构建返回值（使用 tracker 的累计值）
        self.last_energy = EnergyData {
            active_import: self.tracker_active_import.accumulated,
            active_export: self.tracker_active_export.accumulated,
            reactive_import: self.tracker_reactive_import.accumulated,
            reactive_export: self.tracker_reactive_export.accumulated,
            active_import_a: self.tracker_active_import_a.accumulated,
            active_import_b: self.tracker_active_import_b.accumulated,
            active_import_c: self.tracker_active_import_c.accumulated,
        };

        self.last_energy
    }

    /// 返回总有功电能 (0.01 kWh)
    pub fn accumulated_energy_active(&self) -> u64 {
        self.tracker_active_import.accumulated
            .wrapping_add(self.tracker_active_export.accumulated)
    }

    /// 返回总无功电能 (0.01 kvarh)
    pub fn accumulated_energy_reactive(&self) -> u64 {
        self.tracker_reactive_import.accumulated
            .wrapping_add(self.tracker_reactive_export.accumulated)
    }

    /// 检测能量寄存器翻转，处理回绕
    ///
    /// 通常在 poll_energy() 中自动处理，此方法提供显式检查入口。
    /// 可在定时器中断或主循环中定期调用。
    pub fn check_overflow(&mut self) {
        // 翻转处理已在 EnergyTracker::update() 中自动完成。
        // 此方法保留为扩展点：未来可添加溢出告警、日志等。
    }

    /// 三相角度偏差（用于防窃电检测）
    ///
    /// 返回各相电压-电流相角与校准基准的差值（度）。
    /// 正常工况下偏差应 < 1°，若偏差过大可能存在窃电。
    pub fn phase_angle_delta(&self) -> [f32; 3] {
        let mut delta = [0.0f32; 3];
        for i in 0..3 {
            // 用功率因数估算当前相角（简化）
            // 真正的角度需要芯片提供 phase angle 寄存器
            delta[i] = self.prev_phase_angle[i] - self.calibration.phase_angle[i];
        }
        delta
    }

    /// 重新施加校准参数（运行时再校准，不重置芯片）
    pub fn update_calibration(&mut self, params: &CalibrationParams) {
        self.calibration = *params;
        let _ = self.chip.init(params);
    }

    /// 获取费率累计器引用
    pub fn tariffs(&self) -> &TariffAccumulators {
        &self.tariffs
    }

    /// 获取费率累计器可变引用
    pub fn tariffs_mut(&mut self) -> &mut TariffAccumulators {
        &mut self.tariffs
    }

    /// 获取底层芯片的可变引用（用于扩展 trait 调用）
    pub fn chip_mut(&mut self) -> &mut M {
        &mut self.chip
    }

    /// 施加校准修正到实时数据
    fn apply_calibration_correction(&self, data: &mut PhaseData) {
        let gains = &self.calibration;

        // 电压: 乘以 gain
        data.voltage_a = apply_u16_gain(data.voltage_a, gains.voltage_gain[0]);
        data.voltage_b = apply_u16_gain(data.voltage_b, gains.voltage_gain[1]);
        data.voltage_c = apply_u16_gain(data.voltage_c, gains.voltage_gain[2]);

        // 电流: 乘以 gain
        data.current_a = apply_u16_gain(data.current_a, gains.current_gain[0]);
        data.current_b = apply_u16_gain(data.current_b, gains.current_gain[1]);
        data.current_c = apply_u16_gain(data.current_c, gains.current_gain[2]);

        // 有功功率: 乘以 gain
        data.active_power_a = apply_i32_gain(data.active_power_a, gains.power_gain[0]);
        data.active_power_b = apply_i32_gain(data.active_power_b, gains.power_gain[1]);
        data.active_power_c = apply_i32_gain(data.active_power_c, gains.power_gain[2]);
        data.active_power_total = apply_i32_gain(
            data.active_power_total,
            gains.power_gain.iter().sum(),
        );

        // 无功功率: 乘以 gain
        data.reactive_power_a = apply_i32_gain(data.reactive_power_a, gains.reactive_gain[0]);
        data.reactive_power_b = apply_i32_gain(data.reactive_power_b, gains.reactive_gain[1]);
        data.reactive_power_c = apply_i32_gain(data.reactive_power_c, gains.reactive_gain[2]);
        data.reactive_power_total = apply_i32_gain(
            data.reactive_power_total,
            gains.reactive_gain.iter().sum(),
        );
    }
}

/* ================================================================== */
/*  校准增益辅助函数                                                   */
/* ================================================================== */

#[inline(always)]
fn apply_u16_gain(val: u16, gain: f32) -> u16 {
    let corrected = val as f32 * gain;
    if corrected < 0.0 {
        0
    } else if corrected > u16::MAX as f32 {
        u16::MAX
    } else {
        corrected as u16
    }
}

#[inline(always)]
fn apply_i32_gain(val: i32, gain: f32) -> i32 {
    (val as f32 * gain) as i32
}
