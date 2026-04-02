/* ================================================================== */
/*                                                                    */
/*  event_detect.rs — 电表事件检测引擎                                  */
/*                                                                    */
/*  监测断相、过压、欠压、过流、开盖、磁场等事件。                        */
/*  基于阈值比较，每次计量轮询后调用。                                    */
/*  支持事件日志循环缓冲区、Flash 持久化、不平衡检测等。                  */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::hal::PhaseData;

/* ── 事件阈值 (单位与 PhaseData 一致) ── */

/// 电压阈值配置
#[derive(Clone, Copy, Debug)]
pub struct VoltageThresholds {
    /// 过压阈值 (0.01V), 默认 26400 = 264.0V (额定 220V × 1.2)
    pub over_voltage: u16,
    /// 欠压阈值 (0.01V), 默认 17600 = 176.0V (额定 220V × 0.8)
    pub under_voltage: u16,
    /// 断相阈值 (0.01V), 默认 3000 = 30.0V
    pub lost_voltage: u16,
    /// 持续时间 (ms), 超过此时间才触发事件
    pub duration_ms: u16,
}

/// 电流阈值配置
#[derive(Clone, Copy, Debug)]
pub struct CurrentThresholds {
    /// 过流阈值 (mA), 默认 60000 = 60A
    pub over_current: u16,
    /// 持续时间 (ms)
    pub duration_ms: u16,
}

/// 不平衡度阈值配置
#[derive(Clone, Copy, Debug)]
pub struct UnbalanceThresholds {
    /// 电压不平衡度告警阈值 (0.01%), 默认 2000 = 20%
    pub voltage_unbalance: u16,
    /// 电流不平衡度告警阈值 (0.01%), 默认 2000 = 20%
    pub current_unbalance: u16,
    /// 持续时间 (ms)
    pub duration_ms: u16,
}

/// 磁场干扰阈值配置
#[derive(Clone, Copy, Debug)]
pub struct MagneticThresholds {
    /// 磁场强度告警阈值 (0.1μT), 默认 500 = 50μT
    pub field_strength: u16,
    /// 持续时间 (ms)
    pub duration_ms: u16,
}

impl Default for VoltageThresholds {
    fn default() -> Self {
        Self {
            over_voltage: 26400,
            under_voltage: 17600,
            lost_voltage: 3000,
            duration_ms: 3000,
        }
    }
}

impl Default for CurrentThresholds {
    fn default() -> Self {
        Self {
            over_current: 60000,
            duration_ms: 5000,
        }
    }
}

impl Default for UnbalanceThresholds {
    fn default() -> Self {
        Self {
            voltage_unbalance: 2000,
            current_unbalance: 2000,
            duration_ms: 3000,
        }
    }
}

impl Default for MagneticThresholds {
    fn default() -> Self {
        Self {
            field_strength: 500,
            duration_ms: 1000,
        }
    }
}

/* ── 事件类型 ── */

/// 电表事件类型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    CurrentUnbalance = 0x0D,
    VoltageUnbalance = 0x0E,
    FrequencyDeviation = 0x0F,
    ZeroCrossingAnomaly = 0x10,
    CoverOpen = 0x11,
    TerminalCoverOpen = 0x12,
    MagneticTamper = 0x13,
    BatteryLow = 0x14,
    ClockBatteryLow = 0x15,
    ReversePower = 0x16,
    ClockSyncLost = 0x17,
    /// 功率因数过低
    LowPowerFactor = 0x18,
    /// 零线电流过大
    NeutralOverCurrent = 0x19,
    /// 谐波超标
    HarmonicOverLimit = 0x1A,
    /// 需量超限
    DemandOverLimit = 0x1B,
    /// 时钟错误
    ClockError = 0x1C,
    /// 掉电
    PowerFail = 0x1D,
    /// 上电
    PowerRestore = 0x1E,
    /// 复位
    WatchdogReset = 0x1F,
    /// 编程开始
    ProgrammingStart = 0x20,
    /// 编程结束
    ProgrammingEnd = 0x21,
}

/// 事件记录条目
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EventLogEntry {
    /// 事件类型
    pub event: MeterEvent,
    /// 发生时间戳（秒，从 2000-01-01 起）
    pub timestamp: u32,
    /// 事件关联值（如电压值、电流值等，单位同 PhaseData）
    pub value: u32,
    /// 事件持续时间（秒）
    pub duration: u16,
    /// 事件状态：0=结束/恢复，1=发生/开始
    pub state: u8,
    /// 保留
    pub _reserved: u8,
}

impl Default for EventLogEntry {
    fn default() -> Self {
        Self {
            event: MeterEvent::BatteryLow,
            timestamp: 0,
            value: 0,
            duration: 0,
            state: 0,
            _reserved: 0,
        }
    }
}

/// 事件日志头部（存储在 Flash 分区开头）
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct EventLogHeader {
    /// 魔数 "EVT1"
    pub magic: u32,
    /// 写入位置（循环偏移）
    pub write_pos: u32,
    /// 总写入计数
    pub total_count: u32,
    /// CRC32 校验
    pub crc: u32,
}

/* ── 每相异常持续跟踪 ── */

#[derive(Clone, Copy, Debug)]
struct PhaseTracker {
    over_voltage_ticks: u16,
    under_voltage_ticks: u16,
    lost_voltage_ticks: u16,
    over_current_ticks: u16,
    over_voltage_active: bool,
    under_voltage_active: bool,
    lost_voltage_active: bool,
    over_current_active: bool,
}

impl Default for PhaseTracker {
    fn default() -> Self {
        Self {
            over_voltage_ticks: 0,
            under_voltage_ticks: 0,
            lost_voltage_ticks: 0,
            over_current_ticks: 0,
            over_voltage_active: false,
            under_voltage_active: false,
            lost_voltage_active: false,
            over_current_active: false,
        }
    }
}

/* ── 不平衡度跟踪 ── */

#[derive(Clone, Copy, Debug, Default)]
struct UnbalanceTracker {
    voltage_ticks: u16,
    current_ticks: u16,
    voltage_active: bool,
    current_active: bool,
}

/* ── 磁场干扰跟踪 ── */

#[derive(Clone, Copy, Debug, Default)]
struct MagneticTracker {
    ticks: u16,
    active: bool,
}

/* ── 单相检查, 返回新事件位图 ── */

fn check_phase(
    tracker: &mut PhaseTracker,
    voltage: u16,
    current: u16,
    v_thresh: &VoltageThresholds,
    c_thresh: &CurrentThresholds,
    ev_over: MeterEvent,
    ev_under: MeterEvent,
    ev_lost: MeterEvent,
    ev_over_i: MeterEvent,
) -> u32 {
    let poll_ms: u16 = 200;
    let mut events = 0u32;

    // 过压
    if voltage > v_thresh.over_voltage {
        tracker.over_voltage_ticks = tracker.over_voltage_ticks.saturating_add(poll_ms);
        if !tracker.over_voltage_active && tracker.over_voltage_ticks >= v_thresh.duration_ms {
            tracker.over_voltage_active = true;
            events |= 1 << (ev_over as u8);
        }
    } else {
        tracker.over_voltage_active = false;
        tracker.over_voltage_ticks = 0;
    }

    // 欠压
    if voltage < v_thresh.under_voltage && voltage > v_thresh.lost_voltage {
        tracker.under_voltage_ticks = tracker.under_voltage_ticks.saturating_add(poll_ms);
        if !tracker.under_voltage_active && tracker.under_voltage_ticks >= v_thresh.duration_ms {
            tracker.under_voltage_active = true;
            events |= 1 << (ev_under as u8);
        }
    } else {
        tracker.under_voltage_active = false;
        tracker.under_voltage_ticks = 0;
    }

    // 断相
    if voltage <= v_thresh.lost_voltage {
        tracker.lost_voltage_ticks = tracker.lost_voltage_ticks.saturating_add(poll_ms);
        if !tracker.lost_voltage_active && tracker.lost_voltage_ticks >= v_thresh.duration_ms {
            tracker.lost_voltage_active = true;
            events |= 1 << (ev_lost as u8);
        }
    } else {
        tracker.lost_voltage_active = false;
        tracker.lost_voltage_ticks = 0;
    }

    // 过流
    if current > c_thresh.over_current {
        tracker.over_current_ticks = tracker.over_current_ticks.saturating_add(poll_ms);
        if !tracker.over_current_active && tracker.over_current_ticks >= c_thresh.duration_ms {
            tracker.over_current_active = true;
            events |= 1 << (ev_over_i as u8);
        }
    } else {
        tracker.over_current_active = false;
        tracker.over_current_ticks = 0;
    }

    events
}

/// 计算三相电压不平衡度 (0.01%)
///
/// 使用国标 GB/T 15543 公式：
/// 不平衡度 = (最大相电压偏差 / 平均电压) × 100%
fn calc_voltage_unbalance(va: u16, vb: u16, vc: u16) -> u16 {
    let sum = (va as u32).wrapping_add(vb as u32).wrapping_add(vc as u32);
    if sum == 0 {
        return 0;
    }
    let avg = sum / 3;

    let da = if va as u32 > avg {
        va as u32 - avg
    } else {
        avg - va as u32
    };
    let db = if vb as u32 > avg {
        vb as u32 - avg
    } else {
        avg - vb as u32
    };
    let dc = if vc as u32 > avg {
        vc as u32 - avg
    } else {
        avg - vc as u32
    };

    let max_dev = da.max(db).max(dc);
    // 不平衡度 = max_dev / avg * 10000 (0.01%)
    ((max_dev as u64 * 10000) / avg as u64) as u16
}

/// 计算三相电流不平衡度 (0.01%)
fn calc_current_unbalance(ia: u16, ib: u16, ic: u16) -> u16 {
    calc_voltage_unbalance(ia, ib, ic) // 算法相同
}

/* ── 事件检测引擎 ── */

/// 事件日志最大条目数（RAM 缓冲区）
pub const MAX_EVENT_LOG: usize = 256;

/// Flash 持久化事件日志最大条目数
pub const MAX_FLASH_EVENT_LOG: usize = 1024;

/// 事件日志头部魔数
pub const EVENT_LOG_MAGIC: u32 = 0x31545645; // "EVT1"

pub struct EventDetector {
    v_thresholds: VoltageThresholds,
    c_thresholds: CurrentThresholds,
    unbalance_thresholds: UnbalanceThresholds,
    magnetic_thresholds: MagneticThresholds,
    phase_a: PhaseTracker,
    phase_b: PhaseTracker,
    phase_c: PhaseTracker,
    unbalance: UnbalanceTracker,
    magnetic: MagneticTracker,
    freq_deviation_ticks: u16,
    freq_deviation_active: bool,
    reverse_power_active: bool,
    event_log: [EventLogEntry; MAX_EVENT_LOG],
    event_log_len: usize,
    event_log_pos: usize,
    pending_events: u32,
    system_timestamp: u32,
    /// Flash 写入位置（持久化用）
    flash_write_pos: u32,
    /// Flash 总写入计数
    flash_total_count: u32,
    /// 零线电流值（由外部设置）
    neutral_current: u16,
    /// 零线电流过流阈值 (mA)
    neutral_over_current: u16,
}

impl EventDetector {
    /// 创建事件检测器
    pub fn new() -> Self {
        Self {
            v_thresholds: VoltageThresholds::default(),
            c_thresholds: CurrentThresholds::default(),
            unbalance_thresholds: UnbalanceThresholds::default(),
            magnetic_thresholds: MagneticThresholds::default(),
            phase_a: PhaseTracker::default(),
            phase_b: PhaseTracker::default(),
            phase_c: PhaseTracker::default(),
            unbalance: UnbalanceTracker::default(),
            magnetic: MagneticTracker::default(),
            freq_deviation_ticks: 0,
            freq_deviation_active: false,
            reverse_power_active: false,
            event_log: [EventLogEntry::default(); MAX_EVENT_LOG],
            event_log_len: 0,
            event_log_pos: 0,
            pending_events: 0,
            system_timestamp: 0,
            flash_write_pos: 0,
            flash_total_count: 0,
            neutral_current: 0,
            neutral_over_current: 60000,
        }
    }

    /// 设置系统时间戳（秒）
    pub fn set_timestamp(&mut self, ts: u32) {
        self.system_timestamp = ts;
    }

    /// 设置电压阈值
    pub fn set_voltage_thresholds(&mut self, t: VoltageThresholds) {
        self.v_thresholds = t;
    }

    /// 设置电流阈值
    pub fn set_current_thresholds(&mut self, t: CurrentThresholds) {
        self.c_thresholds = t;
    }

    /// 设置不平衡度阈值
    pub fn set_unbalance_thresholds(&mut self, t: UnbalanceThresholds) {
        self.unbalance_thresholds = t;
    }

    /// 设置磁场干扰阈值
    pub fn set_magnetic_thresholds(&mut self, t: MagneticThresholds) {
        self.magnetic_thresholds = t;
    }

    /// 设置零线电流值（由计量任务更新）
    pub fn set_neutral_current(&mut self, current: u16) {
        self.neutral_current = current;
    }

    /// 外部事件触发 (开盖/磁场/电池等)
    ///
    /// 用于 GPIO 中断回调中触发事件。
    pub fn trigger_external(&mut self, event: MeterEvent) {
        self.log_event(event, 0, 1);
        self.pending_events |= 1 << (event as u8);
    }

    /// 外部事件触发（带关联值）
    pub fn trigger_external_with_value(&mut self, event: MeterEvent, value: u32) {
        self.log_event(event, value, 1);
        self.pending_events |= 1 << (event as u8);
    }

    /// 开盖检测中断回调
    ///
    /// 检测到上盖被打开时调用。
    pub fn on_cover_open(&mut self) {
        self.trigger_external(MeterEvent::CoverOpen);
        defmt::warn!("开盖事件检测");
    }

    /// 端子盖检测中断回调
    pub fn on_terminal_cover_open(&mut self) {
        self.trigger_external(MeterEvent::TerminalCoverOpen);
        defmt::warn!("端子盖打开事件检测");
    }

    /// 磁场干扰检测
    ///
    /// `field_strength`: 当前磁场强度 (0.1μT)
    pub fn check_magnetic(&mut self, field_strength: u16) -> bool {
        let poll_ms: u16 = 200;
        if field_strength > self.magnetic_thresholds.field_strength {
            self.magnetic.ticks = self.magnetic.ticks.saturating_add(poll_ms);
            if !self.magnetic.active && self.magnetic.ticks >= self.magnetic_thresholds.duration_ms
            {
                self.magnetic.active = true;
                self.trigger_external_with_value(MeterEvent::MagneticTamper, field_strength as u32);
                defmt::warn!("磁场干扰检测: {} mT", field_strength);
                return true;
            }
        } else {
            if self.magnetic.active {
                self.magnetic.active = false;
                self.log_event(MeterEvent::MagneticTamper, 0, 0);
            }
            self.magnetic.ticks = 0;
        }
        false
    }

    /// 掉电检测
    pub fn on_power_fail(&mut self) {
        self.trigger_external(MeterEvent::PowerFail);
        defmt::warn!("掉电事件检测");
    }

    /// 上电检测
    pub fn on_power_restore(&mut self) {
        self.trigger_external(MeterEvent::PowerRestore);
        defmt::info!("上电恢复");
    }

    /// 检查三相数据, 返回新发生的事件位图
    ///
    /// 每次计量轮询后调用，包含：
    /// - 断相检测
    /// - 过压/欠压检测
    /// - 过流检测
    /// - 频率越限检测
    /// - 反向功率检测
    /// - 电压/电流不平衡检测
    pub fn check(&mut self, data: &PhaseData) -> u32 {
        let mut new_events: u32 = 0;

        new_events |= check_phase(
            &mut self.phase_a,
            data.voltage_a,
            data.current_a,
            &self.v_thresholds,
            &self.c_thresholds,
            MeterEvent::OverVoltageA,
            MeterEvent::UnderVoltageA,
            MeterEvent::PhaseLossA,
            MeterEvent::OverCurrentA,
        );
        new_events |= check_phase(
            &mut self.phase_b,
            data.voltage_b,
            data.current_b,
            &self.v_thresholds,
            &self.c_thresholds,
            MeterEvent::OverVoltageB,
            MeterEvent::UnderVoltageB,
            MeterEvent::PhaseLossB,
            MeterEvent::OverCurrentB,
        );
        new_events |= check_phase(
            &mut self.phase_c,
            data.voltage_c,
            data.current_c,
            &self.v_thresholds,
            &self.c_thresholds,
            MeterEvent::OverVoltageC,
            MeterEvent::UnderVoltageC,
            MeterEvent::PhaseLossC,
            MeterEvent::OverCurrentC,
        );

        // 频率越限 (50Hz ± 2Hz → 4800~5200, 0.01Hz)
        if data.frequency < 4800 || data.frequency > 5200 {
            self.freq_deviation_ticks = self.freq_deviation_ticks.saturating_add(200);
            if !self.freq_deviation_active && self.freq_deviation_ticks >= 3000 {
                self.freq_deviation_active = true;
                self.log_event(MeterEvent::FrequencyDeviation, data.frequency as u32, 1);
                new_events |= 1 << (MeterEvent::FrequencyDeviation as u8);
            }
        } else {
            self.freq_deviation_active = false;
            self.freq_deviation_ticks = 0;
        }

        // 反向功率
        if data.active_power_total < 0 && !self.reverse_power_active {
            self.reverse_power_active = true;
            self.log_event(
                MeterEvent::ReversePower,
                data.active_power_total.unsigned_abs() as u32,
                1,
            );
            new_events |= 1 << (MeterEvent::ReversePower as u8);
        } else if data.active_power_total >= 0 && self.reverse_power_active {
            self.reverse_power_active = false;
            self.log_event(MeterEvent::ReversePower, 0, 0);
        }

        // 电压不平衡检测
        let v_unbal = calc_voltage_unbalance(data.voltage_a, data.voltage_b, data.voltage_c);
        if v_unbal > self.unbalance_thresholds.voltage_unbalance {
            self.unbalance.voltage_ticks = self.unbalance.voltage_ticks.saturating_add(200);
            if !self.unbalance.voltage_active
                && self.unbalance.voltage_ticks >= self.unbalance_thresholds.duration_ms
            {
                self.unbalance.voltage_active = true;
                self.log_event(MeterEvent::VoltageUnbalance, v_unbal as u32, 1);
                new_events |= 1 << (MeterEvent::VoltageUnbalance as u8);
            }
        } else {
            if self.unbalance.voltage_active {
                self.unbalance.voltage_active = false;
                self.log_event(MeterEvent::VoltageUnbalance, 0, 0);
            }
            self.unbalance.voltage_ticks = 0;
        }

        // 电流不平衡检测
        let i_unbal = calc_current_unbalance(data.current_a, data.current_b, data.current_c);
        if i_unbal > self.unbalance_thresholds.current_unbalance {
            self.unbalance.current_ticks = self.unbalance.current_ticks.saturating_add(200);
            if !self.unbalance.current_active
                && self.unbalance.current_ticks >= self.unbalance_thresholds.duration_ms
            {
                self.unbalance.current_active = true;
                self.log_event(MeterEvent::CurrentUnbalance, i_unbal as u32, 1);
                new_events |= 1 << (MeterEvent::CurrentUnbalance as u8);
            }
        } else {
            if self.unbalance.current_active {
                self.unbalance.current_active = false;
                self.log_event(MeterEvent::CurrentUnbalance, 0, 0);
            }
            self.unbalance.current_ticks = 0;
        }

        // 零线电流过流
        if self.neutral_current > self.neutral_over_current {
            if !self.is_event_active(MeterEvent::NeutralOverCurrent) {
                self.log_event(
                    MeterEvent::NeutralOverCurrent,
                    self.neutral_current as u32,
                    1,
                );
                new_events |= 1 << (MeterEvent::NeutralOverCurrent as u8);
            }
        }

        self.pending_events |= new_events;
        new_events
    }

    /// 检查指定事件是否当前处于活跃状态
    pub fn is_event_active(&self, event: MeterEvent) -> bool {
        match event {
            MeterEvent::OverVoltageA => self.phase_a.over_voltage_active,
            MeterEvent::OverVoltageB => self.phase_b.over_voltage_active,
            MeterEvent::OverVoltageC => self.phase_c.over_voltage_active,
            MeterEvent::UnderVoltageA => self.phase_a.under_voltage_active,
            MeterEvent::UnderVoltageB => self.phase_b.under_voltage_active,
            MeterEvent::UnderVoltageC => self.phase_c.under_voltage_active,
            MeterEvent::PhaseLossA => self.phase_a.lost_voltage_active,
            MeterEvent::PhaseLossB => self.phase_b.lost_voltage_active,
            MeterEvent::PhaseLossC => self.phase_c.lost_voltage_active,
            MeterEvent::OverCurrentA => self.phase_a.over_current_active,
            MeterEvent::OverCurrentB => self.phase_b.over_current_active,
            MeterEvent::OverCurrentC => self.phase_c.over_current_active,
            MeterEvent::VoltageUnbalance => self.unbalance.voltage_active,
            MeterEvent::CurrentUnbalance => self.unbalance.current_active,
            MeterEvent::FrequencyDeviation => self.freq_deviation_active,
            MeterEvent::ReversePower => self.reverse_power_active,
            MeterEvent::MagneticTamper => self.magnetic.active,
            _ => false,
        }
    }

    /// 消费所有待处理事件位图
    pub fn consume_events(&mut self) -> u32 {
        let events = self.pending_events;
        self.pending_events = 0;
        events
    }

    /// 获取事件日志（RAM 缓冲区）
    pub fn event_log(&self) -> &[EventLogEntry] {
        &self.event_log[..self.event_log_len]
    }

    /// 获取最新的 N 条事件日志
    pub fn recent_events(&self, n: usize) -> &[EventLogEntry] {
        let n = n.min(self.event_log_len);
        if n == 0 {
            return &[];
        }
        // 循环缓冲区：最新数据在 write_pos 之前
        let start = if self.event_log_pos >= n {
            self.event_log_pos - n
        } else {
            MAX_EVENT_LOG - (n - self.event_log_pos)
        };
        if start + n <= MAX_EVENT_LOG {
            &self.event_log[start..start + n]
        } else {
            // 跨越缓冲区边界，简化处理：返回连续部分
            &self.event_log[start..]
        }
    }

    /// 清除 RAM 事件日志
    pub fn clear_log(&mut self) {
        self.event_log_len = 0;
        self.event_log_pos = 0;
    }

    /// 获取 Flash 持久化写入位置
    pub fn flash_write_pos(&self) -> u32 {
        self.flash_write_pos
    }

    /// 设置 Flash 持久化写入位置（从 Flash 恢复时调用）
    pub fn set_flash_write_pos(&mut self, pos: u32, count: u32) {
        self.flash_write_pos = pos;
        self.flash_total_count = count;
    }

    /// 获取 Flash 总写入计数
    pub fn flash_total_count(&self) -> u32 {
        self.flash_total_count
    }

    /// 获取电压阈值引用
    pub fn voltage_thresholds(&self) -> &VoltageThresholds {
        &self.v_thresholds
    }

    /// 获取电流阈值引用
    pub fn current_thresholds(&self) -> &CurrentThresholds {
        &self.c_thresholds
    }

    fn log_event(&mut self, event: MeterEvent, value: u32, state: u8) {
        let entry = EventLogEntry {
            event,
            timestamp: self.system_timestamp,
            value,
            duration: 0,
            state,
            _reserved: 0,
        };
        self.event_log[self.event_log_pos] = entry;
        self.event_log_pos = (self.event_log_pos + 1) % MAX_EVENT_LOG;
        if self.event_log_len < MAX_EVENT_LOG {
            self.event_log_len += 1;
        }
        // 同步 Flash 写入位置
        self.flash_write_pos = self.flash_write_pos.wrapping_add(1);
        self.flash_total_count = self.flash_total_count.wrapping_add(1);
    }
}

/* ================================================================== */
/*  CRC32 辅助（用于 Flash 事件日志校验）                               */
/* ================================================================== */

/// CRC32 计算（多项式 0xEDB88320，与 zlib 兼容）
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/* ================================================================== */
/*  单元测试                                                           */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    fn default_data() -> crate::hal::PhaseData {
        crate::hal::PhaseData::default()
    }

    #[test]
    fn test_normal_no_events() {
        let mut det = EventDetector::new();
        assert_eq!(det.check(&default_data()), 0);
    }

    #[test]
    fn test_over_voltage_after_duration() {
        let mut det = EventDetector::new();
        let mut d = default_data();
        d.voltage_a = 27000;
        for _ in 0..20 {
            let ev = det.check(&d);
            if ev != 0 {
                assert_ne!(ev & (1 << (MeterEvent::OverVoltageA as u8)), 0);
                return;
            }
        }
        panic!("over voltage should trigger within 20 polls");
    }

    #[test]
    fn test_under_voltage() {
        let mut det = EventDetector::new();
        let mut d = default_data();
        d.voltage_b = 17000;
        for _ in 0..20 {
            if det.check(&d) & (1 << MeterEvent::UnderVoltageB as u8) != 0 {
                return;
            }
        }
        panic!("under voltage should trigger");
    }

    #[test]
    fn test_phase_loss() {
        let mut det = EventDetector::new();
        let mut d = default_data();
        d.voltage_c = 500;
        for _ in 0..20 {
            if det.check(&d) & (1 << MeterEvent::PhaseLossC as u8) != 0 {
                return;
            }
        }
        panic!("phase loss should trigger");
    }

    #[test]
    fn test_over_current_after_duration() {
        let mut det = EventDetector::new();
        let mut d = default_data();
        d.current_a = 65000;
        for _ in 0..30 {
            if det.check(&d) & (1 << MeterEvent::OverCurrentA as u8) != 0 {
                return;
            }
        }
        panic!("over current should trigger");
    }

    #[test]
    fn test_frequency_deviation() {
        let mut det = EventDetector::new();
        let mut d = default_data();
        d.frequency = 4700;
        for _ in 0..20 {
            if det.check(&d) & (1 << MeterEvent::FrequencyDeviation as u8) != 0 {
                return;
            }
        }
        panic!("frequency deviation should trigger");
    }

    #[test]
    fn test_reverse_power() {
        let mut det = EventDetector::new();
        let mut d = default_data();
        d.active_power_total = -500;
        assert_ne!(det.check(&d) & (1 << MeterEvent::ReversePower as u8), 0);
    }

    #[test]
    fn test_recovery_clears() {
        let mut det = EventDetector::new();
        let mut d = default_data();
        d.voltage_a = 27000;
        for _ in 0..16 {
            det.check(&d);
        }
        d.voltage_a = 22000;
        assert_eq!(det.check(&d) & (1 << MeterEvent::OverVoltageA as u8), 0);
    }

    #[test]
    fn test_event_log() {
        let mut det = EventDetector::new();
        det.set_timestamp(12345);
        det.trigger_external(MeterEvent::CoverOpen);
        let log = det.event_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].event, MeterEvent::CoverOpen);
        assert_eq!(log[0].timestamp, 12345);
    }

    #[test]
    fn test_event_log_overflow() {
        let mut det = EventDetector::new();
        for _ in 0..300 {
            det.trigger_external(MeterEvent::BatteryLow);
        }
        assert_eq!(det.event_log().len(), MAX_EVENT_LOG);
    }

    #[test]
    fn test_consume_events() {
        let mut det = EventDetector::new();
        det.trigger_external(MeterEvent::MagneticTamper);
        let c1 = det.consume_events();
        assert_ne!(c1, 0, "first consume should have events");
        let c2 = det.consume_events();
        assert_eq!(c2, 0, "second consume should be empty");
    }

    #[test]
    fn test_custom_thresholds_instant() {
        let mut det = EventDetector::new();
        det.set_voltage_thresholds(VoltageThresholds {
            over_voltage: 25000,
            under_voltage: 19000,
            lost_voltage: 5000,
            duration_ms: 0,
        });
        let mut d = default_data();
        d.voltage_a = 25500;
        assert_ne!(det.check(&d) & (1 << MeterEvent::OverVoltageA as u8), 0);
    }

    #[test]
    fn test_multi_phase() {
        let mut det = EventDetector::new();
        det.set_voltage_thresholds(VoltageThresholds {
            over_voltage: 25000,
            under_voltage: 19000,
            lost_voltage: 5000,
            duration_ms: 0,
        });
        let mut d = default_data();
        d.voltage_a = 27000;
        d.voltage_b = 18000;
        d.voltage_c = 1000;
        let ev = det.check(&d);
        assert_ne!(ev & (1 << MeterEvent::OverVoltageA as u8), 0);
        assert_ne!(ev & (1 << MeterEvent::UnderVoltageB as u8), 0);
        assert_ne!(ev & (1 << MeterEvent::PhaseLossC as u8), 0);
    }

    #[test]
    fn test_clear_log() {
        let mut det = EventDetector::new();
        det.trigger_external(MeterEvent::CoverOpen);
        det.trigger_external(MeterEvent::TerminalCoverOpen);
        assert_eq!(det.event_log().len(), 2);
        det.clear_log();
        assert_eq!(det.event_log().len(), 0);
    }

    #[test]
    fn test_voltage_unbalance() {
        // 220, 220, 180 → avg=207, max_dev=27, unbalance=27/207*10000=1304 (13.04%)
        let ub = calc_voltage_unbalance(22000, 22000, 18000);
        assert!(ub > 1000 && ub < 1500);
    }

    #[test]
    fn test_voltage_unbalance_balanced() {
        let ub = calc_voltage_unbalance(22000, 22000, 22000);
        assert_eq!(ub, 0);
    }

    #[test]
    fn test_crc32() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let crc = crc32(&data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_magnetic_detection() {
        let mut det = EventDetector::new();
        // 默认阈值 500, duration 1000ms, poll 200ms → 需要 5 次
        for _ in 0..10 {
            if det.check_magnetic(600) {
                return;
            }
        }
        panic!("magnetic tamper should trigger");
    }

    #[test]
    fn test_event_active_query() {
        let mut det = EventDetector::new();
        det.set_voltage_thresholds(VoltageThresholds {
            over_voltage: 25000,
            under_voltage: 19000,
            lost_voltage: 5000,
            duration_ms: 0,
        });
        let mut d = default_data();
        d.voltage_a = 26000;
        det.check(&d);
        assert!(det.is_event_active(MeterEvent::OverVoltageA));
        assert!(!det.is_event_active(MeterEvent::OverVoltageB));
    }
}
