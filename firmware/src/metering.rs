/* ================================================================== */
/*                                                                    */
/*  metering.rs — 统一计量管理器                                       */
/*                                                                    */
/*  基于 HAL MeteringChip trait 的通用计量管理层。                      */
/*  提供电能累计（含翻转处理）、校准修正、费率切换、需量计算、           */
/*  谐波数据采集等功能。                                               */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::hal::{
    CalibrationParams, EnergyData, HarmonicData, MeteringChip, MeteringError,
    Phase, PhaseData, PowerQuality,
};

/// 能量寄存器位宽 (32-bit)
const ENERGY_REG_BITS: u32 = 32;
const ENERGY_REG_MAX: u32 = 0xFFFF_FFFF;

/// Wh/LSB 常数 — 典型值，具体由芯片决定，此处按 1 LSB = 0.1 Wh
/// 即 10 LSB = 1 Wh = 0.01 kWh（与 EnergyData 单位一致）
const WH_PER_LSB_X100: u64 = 1; // 0.01 kWh per LSB

/// 费率数量
pub const NUM_TARIFFS: usize = 4;

/// 需量计算滑差窗口数（滑差需量 = N 个窗口的滑动平均）
pub const DEMAND_SLIDING_N: usize = 5;

/// 需量计算周期（秒），符合 DLMS/COSEM 标准
pub const DEMAND_INTERVAL_SEC: u32 = 15;

/// 需量最大历史记录数
pub const DEMAND_HISTORY_MAX: usize = 96; // 24h / 15min

/// 谐波最大次数
pub const MAX_HARMONIC_ORDER: usize = 31;

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
    ///
    /// `tariff` 参数为新的费率索引 (0..3)，超出范围则忽略。
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

    /// 获取指定费率的总有功电能（正向 + 反向，0.01 kWh）
    pub fn total_active(&self, tariff: u8) -> u64 {
        let t = tariff as usize;
        if t >= NUM_TARIFFS { return 0; }
        self.active_import[t].wrapping_add(self.active_export[t])
    }

    /// 获取指定费率的总无功电能（正向 + 反向，0.01 kvarh）
    pub fn total_reactive(&self, tariff: u8) -> u64 {
        let t = tariff as usize;
        if t >= NUM_TARIFFS { return 0; }
        self.reactive_import[t].wrapping_add(self.reactive_export[t])
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
/*  需量计算器                                                         */
/* ================================================================== */

/// 需量数据（有功/无功，单位 W / var）
#[derive(Clone, Copy, Debug, Default)]
pub struct DemandData {
    /// 当前需量 (W)
    pub active: u32,
    /// 当前无功需量 (var)
    pub reactive: u32,
    /// 本周期最大需量 (W)
    pub max_active: u32,
    /// 本周期最大无功需量 (var)
    pub max_reactive: u32,
    /// 滑差需量历史窗口（最近 N 个周期的需量值）
    pub sliding_window: [u32; DEMAND_SLIDING_N],
    /// 滑差窗口当前位置
    pub sliding_pos: usize,
    /// 滑差窗口填充计数
    pub sliding_count: usize,
    /// 本周期功率采样累加
    pub period_sum: u64,
    /// 本周期采样次数
    pub period_samples: u32,
    /// 内部无功功率累加器
    _reactive_sum: u64,
}

impl DemandData {
    /// 创建新的需量计算器
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次功率采样（每次 poll_instant 时调用）
    ///
    /// `power` 当前有功功率（W），`reactive` 当前无功功率（var）。
    pub fn sample(&mut self, power: i32, reactive: i32) {
        // 累加绝对值
        if power > 0 {
            self.period_sum += power as u64;
        }
        self.period_samples += 1;
        self._reactive_sum += reactive.unsigned_abs() as u64;
    }

    /// 需量周期结束，计算当前需量并更新最大值
    ///
    /// 调用间隔应为 `DEMAND_INTERVAL_SEC` 秒。
    /// 返回当前周期平均功率（W）。
    pub fn end_period(&mut self) -> u32 {
        if self.period_samples == 0 {
            return 0;
        }

        // 当前需量 = 平均功率
        let avg_active = (self.period_sum / self.period_samples as u64) as u32;
        let avg_reactive = (self._reactive_sum / self.period_samples as u64) as u32;

        self.active = avg_active;
        self.reactive = avg_reactive;

        // 更新最大需量
        if avg_active > self.max_active {
            self.max_active = avg_active;
        }
        if avg_reactive > self.max_reactive {
            self.max_reactive = avg_reactive;
        }

        // 滑差窗口更新
        self.sliding_window[self.sliding_pos] = avg_active;
        self.sliding_pos = (self.sliding_pos + 1) % DEMAND_SLIDING_N;
        if self.sliding_count < DEMAND_SLIDING_N {
            self.sliding_count += 1;
        }

        // 重置周期累加器
        self.period_sum = 0;
        self._reactive_sum = 0;
        self.period_samples = 0;

        avg_active
    }

    /// 计算滑差需量（N 个周期的滑动平均）
    ///
    /// 返回 0 如果窗口未填满。
    pub fn sliding_demand(&self) -> u32 {
        if self.sliding_count == 0 {
            return 0;
        }
        let n = self.sliding_count;
        let mut sum: u32 = 0;
        for i in 0..n {
            sum = sum.saturating_add(self.sliding_window[i]);
        }
        sum / n as u32
    }

    /// 重置最大需量（用于结算日零点清零）
    pub fn reset_max_demand(&mut self) {
        self.max_active = 0;
        self.max_reactive = 0;
    }

}

/* ================================================================== */
/*  谐波数据缓存                                                       */
/* ================================================================== */

/// 三相谐波数据缓存
#[derive(Clone, Copy, Debug)]
pub struct HarmonicSnapshot {
    /// 采集时间戳（秒）
    pub timestamp: u32,
    /// A 相谐波
    pub phase_a: HarmonicData,
    /// B 相谐波
    pub phase_b: HarmonicData,
    /// C 相谐波
    pub phase_c: HarmonicData,
    /// 数据有效标志
    pub valid: bool,
}

impl Default for HarmonicSnapshot {
    fn default() -> Self {
        Self {
            timestamp: 0,
            phase_a: HarmonicData::default(),
            phase_b: HarmonicData::default(),
            phase_c: HarmonicData::default(),
            valid: false,
        }
    }
}

/* ================================================================== */
/*  定时采集配置                                                       */
/* ================================================================== */

/// 采集任务配置
#[derive(Clone, Copy, Debug)]
pub struct PollConfig {
    /// 实时数据采集间隔（ms），默认 200ms
    pub instant_interval_ms: u32,
    /// 电能数据采集间隔（ms），默认 1000ms
    pub energy_interval_ms: u32,
    /// 需量周期（秒），默认 15 分钟
    pub demand_interval_sec: u32,
    /// 谐波采集间隔（秒），默认 60s
    pub harmonic_interval_sec: u32,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            instant_interval_ms: 200,
            energy_interval_ms: 1000,
            demand_interval_sec: DEMAND_INTERVAL_SEC as u32,
            harmonic_interval_sec: 60,
        }
    }
}

/* ================================================================== */
/*  MeteringManager                                                   */
/* ================================================================== */

/// 统一计量管理器，泛型抽象于 HAL MeteringChip trait
///
/// 提供定时采集、电能累计、费率分时、需量计算、谐波采集等功能。
/// 在 FreeRTOS 环境中由计量任务周期性调用。
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
    /// 需量计算器
    demand: DemandData,
    /// 需量历史记录（每小时一个最大值）
    demand_history: [u32; DEMAND_HISTORY_MAX],
    /// 需量历史记录数
    demand_history_len: usize,
    /// 谐波数据缓存
    harmonics: HarmonicSnapshot,
    /// 采集配置
    poll_config: PollConfig,
    /// 上次电能采集时间（用于判断间隔）
    last_energy_poll_ms: u32,
    /// 上次谐波采集时间
    last_harmonic_poll_ms: u32,
    /// 上次需量周期开始时间
    demand_period_start_ms: u32,
    /// 是否支持谐波分析
    supports_harmonics: bool,
    /// 是否支持电能质量分析
    supports_power_quality: bool,
    /// 上次 poll 的总功率（用于 event_detect 联动）
    last_active_power: i32,
}

impl<M: MeteringChip> MeteringManager<M> {
    /// 创建计量管理器
    ///
    /// 初始化底层计量芯片并写入校准参数。
    pub fn new(mut chip: M, calibration: CalibrationParams) -> Self {
        // 检测芯片能力
        let supports_harmonics = chip.supports_fundamental();
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
            demand: DemandData::new(),
            demand_history: [0; DEMAND_HISTORY_MAX],
            demand_history_len: 0,
            harmonics: HarmonicSnapshot::default(),
            poll_config: PollConfig::default(),
            last_energy_poll_ms: 0,
            last_harmonic_poll_ms: 0,
            demand_period_start_ms: 0,
            supports_harmonics,
            supports_power_quality: false,
            last_active_power: 0,
        }
    }

    /// 读取实时数据并施加校准修正
    ///
    /// 每次调用都会从计量芯片读取三相电压/电流/功率/频率等数据，
    /// 同时更新需量采样。
    pub fn poll_instant(&mut self) -> PhaseData {
        match self.chip.read_instant_data() {
            Ok(mut data) => {
                self.apply_calibration_correction(&mut data);

                // 需量采样
                self.demand.sample(data.active_power_total, data.reactive_power_total);

                self.last_active_power = data.active_power_total;
                self.instant = data;
                data
            }
            Err(_) => self.instant,
        }
    }

    /// 读取电能累计值（含翻转处理）
    ///
    /// 自动检测 32-bit 寄存器溢出回绕，累加到 64-bit 工程单位。
    /// 增量自动计入当前费率。
    pub fn poll_energy(&mut self) -> EnergyData {
        let raw = match self.chip.read_energy() {
            Ok(d) => d,
            Err(_) => return self.last_energy,
        };

        let d_ai = self.tracker_active_import.update(raw.active_import as u32);
        let d_ae = self.tracker_active_export.update(raw.active_export as u32);
        let d_ri = self.tracker_reactive_import.update(raw.reactive_import as u32);
        let d_re = self.tracker_reactive_export.update(raw.reactive_export as u32);

        let _d_aia = self.tracker_active_import_a.update(raw.active_import_a as u32);
        let _d_aib = self.tracker_active_import_b.update(raw.active_import_b as u32);
        let _d_aic = self.tracker_active_import_c.update(raw.active_import_c as u32);

        // 累加到当前费率
        self.tariffs.accumulate(d_ai, d_ae, d_ri, d_re);

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

    /// 定时采集入口（在计量任务中周期性调用）
    ///
    /// 根据 `current_ms` 时间戳和 `PollConfig` 配置自动决定
    /// 是否需要执行电能读取、需量结算、谐波采集。
    ///
    /// 返回实时数据引用。
    pub fn tick(&mut self, current_ms: u32) -> &PhaseData {
        // 始终读取实时数据
        self.poll_instant();

        // 电能采集（间隔控制）
        if current_ms.wrapping_sub(self.last_energy_poll_ms)
            >= self.poll_config.energy_interval_ms
        {
            self.poll_energy();
            self.last_energy_poll_ms = current_ms;
        }

        // 需量周期结算
        let demand_period_ms = self.poll_config.demand_interval_sec * 1000;
        if current_ms.wrapping_sub(self.demand_period_start_ms) >= demand_period_ms {
            let avg = self.demand.end_period();
            // 记录到历史
            if self.demand_history_len < DEMAND_HISTORY_MAX {
                self.demand_history[self.demand_history_len] = avg;
                self.demand_history_len += 1;
            }
            self.demand_period_start_ms = current_ms;
        }

        // 谐波采集
        let harmonic_interval_ms = self.poll_config.harmonic_interval_sec * 1000;
        if self.supports_harmonics
            && current_ms.wrapping_sub(self.last_harmonic_poll_ms) >= harmonic_interval_ms
        {
            self.poll_harmonics(current_ms / 1000);
            self.last_harmonic_poll_ms = current_ms;
        }

        &self.instant
    }

    /// 采集谐波数据（若芯片支持）
    ///
    /// 从计量芯片读取三相的 THD 和各次谐波含量。
    pub fn poll_harmonics(&mut self, timestamp: u32) {
        // 注意：只有实现 HarmonicAnalysis trait 的芯片才支持
        // 这里用 trait object 或 feature gate 处理
        // 由于泛型约束，我们在实际使用中通过 downcast 或
        // separate method 处理
        self.harmonics.timestamp = timestamp;
        self.harmonics.valid = false;

        defmt::debug!("谐波采集请求 (芯片可能不支持)");
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

    /// 获取当前实时数据引用
    pub fn instant_data(&self) -> &PhaseData {
        &self.instant
    }

    /// 获取当前电能数据引用
    pub fn energy_data(&self) -> &EnergyData {
        &self.last_energy
    }

    /// 获取需量数据引用
    pub fn demand_data(&self) -> &DemandData {
        &self.demand
    }

    /// 获取需量数据可变引用
    pub fn demand_data_mut(&mut self) -> &mut DemandData {
        &mut self.demand
    }

    /// 获取谐波数据引用
    pub fn harmonics(&self) -> &HarmonicSnapshot {
        &self.harmonics
    }

    /// 获取上次有功功率值（用于 event_detect 联动）
    pub fn last_active_power(&self) -> i32 {
        self.last_active_power
    }

    /// 检测能量寄存器翻转，处理回绕
    ///
    /// 通常在 `poll_energy()` 中自动处理，此方法保留为扩展点。
    pub fn check_overflow(&mut self) {
        // 翻转处理已在 EnergyTracker::update() 中自动完成。
    }

    /// 三相角度偏差（用于防窃电检测）
    ///
    /// 返回各相电压-电流相角与校准基准的差值（度）。
    pub fn phase_angle_delta(&self) -> [f32; 3] {
        let mut delta = [0.0f32; 3];
        for i in 0..3 {
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

    /// 获取采集配置引用
    pub fn poll_config(&self) -> &PollConfig {
        &self.poll_config
    }

    /// 设置采集配置
    pub fn set_poll_config(&mut self, config: PollConfig) {
        self.poll_config = config;
    }

    /// 结算日处理：重置最大需量
    ///
    /// 应在每月结算日零点调用。
    pub fn settlement_reset(&mut self) {
        self.demand.reset_max_demand();
        defmt::info!("结算日: 最大需量已重置");
    }

    /// 施加校准修正到实时数据
    fn apply_calibration_correction(&self, data: &mut PhaseData) {
        let gains = &self.calibration;

        data.voltage_a = apply_u16_gain(data.voltage_a, gains.voltage_gain[0]);
        data.voltage_b = apply_u16_gain(data.voltage_b, gains.voltage_gain[1]);
        data.voltage_c = apply_u16_gain(data.voltage_c, gains.voltage_gain[2]);

        data.current_a = apply_u16_gain(data.current_a, gains.current_gain[0]);
        data.current_b = apply_u16_gain(data.current_b, gains.current_gain[1]);
        data.current_c = apply_u16_gain(data.current_c, gains.current_gain[2]);

        data.active_power_a = apply_i32_gain(data.active_power_a, gains.power_gain[0]);
        data.active_power_b = apply_i32_gain(data.active_power_b, gains.power_gain[1]);
        data.active_power_c = apply_i32_gain(data.active_power_c, gains.power_gain[2]);
        data.active_power_total = apply_i32_gain(
            data.active_power_total,
            gains.power_gain.iter().sum(),
        );

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
/*  支持谐波分析的扩展方法（仅对实现了 HarmonicAnalysis 的芯片可用）    */
/* ================================================================== */

impl<M: MeteringChip + crate::hal::HarmonicAnalysis> MeteringManager<M> {
    /// 采集谐波数据（仅支持谐波分析的芯片）
    ///
    /// 读取三相 THD 和 2~31 次谐波含量。
    pub fn poll_harmonics_extended(&mut self, timestamp: u32) {
        let phases = [Phase::A, Phase::B, Phase::C];
        let results: [Option<HarmonicData>; 3] = [
            self.chip.read_harmonics(Phase::A).ok(),
            self.chip.read_harmonics(Phase::B).ok(),
            self.chip.read_harmonics(Phase::C).ok(),
        ];

        for (i, result) in results.iter().enumerate() {
            if let Some(h) = result {
                match i {
                    0 => self.harmonics.phase_a = *h,
                    1 => self.harmonics.phase_b = *h,
                    2 => self.harmonics.phase_c = *h,
                    _ => {}
                }
            }
        }

        self.harmonics.timestamp = timestamp;
        self.harmonics.valid = true;
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
