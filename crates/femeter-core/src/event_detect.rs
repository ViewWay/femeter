/* ================================================================== */
/*                                                                    */
/*  events.rs — 电表事件检测引擎                                        */
/*                                                                    */
/*  监测断相、过压、欠压、过流、开盖、磁场等事件。                        */
/*  基于阈值比较，每次计量轮询后调用。                                    */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::PhaseData;

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

impl Default for VoltageThresholds {
    fn default() -> Self {
        Self {
            over_voltage: 26400,  // 264.0V
            under_voltage: 17600, // 176.0V
            lost_voltage: 3000,   // 30.0V
            duration_ms: 3000,    // 3s
        }
    }
}

impl Default for CurrentThresholds {
    fn default() -> Self {
        Self {
            over_current: 60000, // 60A
            duration_ms: 5000,   // 5s
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
}

/// 事件记录条目
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EventLogEntry {
    pub event: MeterEvent,
    pub timestamp: u32,
    pub value: u32,
    pub duration: u16,
    pub state: u8,
    pub _reserved: u8,
}

/* ── 每相异常持续跟踪 ── */

#[derive(Clone, Copy, Debug, Default)]
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

/* ── 辅助函数 ── */

fn event_from_bit(bit: u8) -> Option<MeterEvent> {
    Some(match bit {
        0x01 => MeterEvent::OverVoltageA,
        0x02 => MeterEvent::OverVoltageB,
        0x03 => MeterEvent::OverVoltageC,
        0x04 => MeterEvent::UnderVoltageA,
        0x05 => MeterEvent::UnderVoltageB,
        0x06 => MeterEvent::UnderVoltageC,
        0x07 => MeterEvent::PhaseLossA,
        0x08 => MeterEvent::PhaseLossB,
        0x09 => MeterEvent::PhaseLossC,
        0x0A => MeterEvent::OverCurrentA,
        0x0B => MeterEvent::OverCurrentB,
        0x0C => MeterEvent::OverCurrentC,
        0x0D => MeterEvent::CurrentUnbalance,
        0x0E => MeterEvent::VoltageUnbalance,
        _ => return None,
    })
}

/* ── 单相检查, 返回新事件位图 ── */

#[allow(clippy::too_many_arguments)]
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

/* ── 事件检测引擎 ── */

pub const MAX_EVENT_LOG: usize = 256;

pub struct EventDetector {
    v_thresholds: VoltageThresholds,
    c_thresholds: CurrentThresholds,
    phase_a: PhaseTracker,
    phase_b: PhaseTracker,
    phase_c: PhaseTracker,
    freq_deviation_ticks: u16,
    freq_deviation_active: bool,
    reverse_power_active: bool,
    event_log: [EventLogEntry; MAX_EVENT_LOG],
    event_log_len: usize,
    event_log_pos: usize,
    pending_events: u32,
    system_timestamp: u32,
}

impl Default for EventDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDetector {
    pub fn new() -> Self {
        Self {
            v_thresholds: VoltageThresholds::default(),
            c_thresholds: CurrentThresholds::default(),
            phase_a: PhaseTracker::default(),
            phase_b: PhaseTracker::default(),
            phase_c: PhaseTracker::default(),
            freq_deviation_ticks: 0,
            freq_deviation_active: false,
            reverse_power_active: false,
            event_log: [EventLogEntry {
                event: MeterEvent::BatteryLow,
                timestamp: 0,
                value: 0,
                duration: 0,
                state: 0,
                _reserved: 0,
            }; MAX_EVENT_LOG],
            event_log_len: 0,
            event_log_pos: 0,
            pending_events: 0,
            system_timestamp: 0,
        }
    }

    pub fn set_timestamp(&mut self, ts: u32) {
        self.system_timestamp = ts;
    }

    pub fn set_voltage_thresholds(&mut self, t: VoltageThresholds) {
        self.v_thresholds = t;
    }

    pub fn set_current_thresholds(&mut self, t: CurrentThresholds) {
        self.c_thresholds = t;
    }

    /// 外部事件触发 (开盖/磁场/电池等)
    pub fn trigger_external(&mut self, event: MeterEvent) {
        self.log_event(event, 0, 1);
        self.pending_events |= 1 << (event as u8);
    }

    /// 检查三相数据, 返回新发生的事件位图
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
        
        // Log voltage and current events
        for i in 0u8..24 {
            if new_events & (1 << i) != 0 {
                if let Some(event) = event_from_bit(i) {
                    let value = match event {
                        MeterEvent::OverVoltageA | MeterEvent::OverVoltageB | MeterEvent::OverVoltageC => {
                            match event {
                                MeterEvent::OverVoltageA => data.voltage_a,
                                MeterEvent::OverVoltageB => data.voltage_b,
                                MeterEvent::OverVoltageC => data.voltage_c,
                                _ => 0,
                            }
                        }
                        MeterEvent::UnderVoltageA | MeterEvent::UnderVoltageB | MeterEvent::UnderVoltageC => {
                            match event {
                                MeterEvent::UnderVoltageA => data.voltage_a,
                                MeterEvent::UnderVoltageB => data.voltage_b,
                                MeterEvent::UnderVoltageC => data.voltage_c,
                                _ => 0,
                            }
                        }
                        MeterEvent::PhaseLossA | MeterEvent::PhaseLossB | MeterEvent::PhaseLossC => {
                            match event {
                                MeterEvent::PhaseLossA => data.voltage_a,
                                MeterEvent::PhaseLossB => data.voltage_b,
                                MeterEvent::PhaseLossC => data.voltage_c,
                                _ => 0,
                            }
                        }
                        MeterEvent::OverCurrentA | MeterEvent::OverCurrentB | MeterEvent::OverCurrentC => {
                            match event {
                                MeterEvent::OverCurrentA => data.current_a,
                                MeterEvent::OverCurrentB => data.current_b,
                                MeterEvent::OverCurrentC => data.current_c,
                                _ => 0,
                            }
                        }
                        _ => 0,
                    };
                    self.log_event(event, value as u32, 1);
                }
            }
        }

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
                data.active_power_total.unsigned_abs(),
                1,
            );
            new_events |= 1 << (MeterEvent::ReversePower as u8);
        } else if data.active_power_total >= 0 && self.reverse_power_active {
            self.reverse_power_active = false;
            self.log_event(MeterEvent::ReversePower, 0, 0);
        }

        self.pending_events |= new_events;
        new_events
    }

    pub fn consume_events(&mut self) -> u32 {
        let events = self.pending_events;
        self.pending_events = 0;
        events
    }

    pub fn event_log(&self) -> &[EventLogEntry] {
        &self.event_log[..self.event_log_len]
    }

    pub fn clear_log(&mut self) {
        self.event_log_len = 0;
        self.event_log_pos = 0;
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
    }
}

/* ================================================================== */
/*  单元测试                                                           */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    fn default_data() -> crate::PhaseData {
        crate::PhaseData {
            voltage_a: 22000,
            voltage_b: 22000,
            voltage_c: 22000,
            current_a: 10000,
            current_b: 10000,
            current_c: 10000,
            active_power_total: 6600,
            reactive_power_total: 1000,
            apparent_power_total: 6676,
            frequency: 5000,
            power_factor_total: 989,
            active_power_a: 2200,
            active_power_b: 2200,
            active_power_c: 2200,
            reactive_power_a: 333,
            reactive_power_b: 333,
            reactive_power_c: 333,
            voltage_angle_a: 0,
            voltage_angle_b: 24000,
            voltage_angle_c: 48000,
        }
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
        // 每次调用 check, check_phase 给 phase_a ticks += 200
        // 3000 / 200 = 15 次, 第 16 次 check 后 ticks=3200 >= 3000
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
        // 过流需要 5000ms / 200ms = 25 次
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
        // trigger_external 会设置 pending_events
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
}
