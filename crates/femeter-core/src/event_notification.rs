/* ================================================================== */
/*                                                                    */
/*  event_notification.rs — DLMS 事件上报                              */
/*                                                                    */
/*  实现 event_detect → event_log → DLMS event notification 完整流程  */
/*  支持 DLMS IC10 Script Table 和事件通知                             */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::event_detect::{EventDetector, EventLogEntry, MeterEvent};

/// DLMS 事件代码映射 (IEC 62056-62)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DlmsEventCode {
    /// 电压骤降 (Sag)
    VoltageSag = 1,
    /// 电压骤升 (Swell)
    VoltageSwell = 2,
    /// 过压
    OverVoltage = 3,
    /// 欠压
    UnderVoltage = 4,
    /// 电流不平衡
    CurrentUnbalance = 5,
    /// 电压不平衡
    VoltageUnbalance = 6,
    /// 频率越限
    FrequencyDeviation = 7,
    /// 断相
    PhaseLoss = 8,
    /// 过流
    OverCurrent = 9,
    /// 反向功率
    ReversePower = 10,
    /// 开盖
    CoverOpen = 11,
    /// 端子盖打开
    TerminalCoverOpen = 12,
    /// 磁场干扰
    MagneticTamper = 13,
    /// 电池欠压
    BatteryLow = 14,
    /// 时钟电池欠压
    ClockBatteryLow = 15,
    /// 时钟失步
    ClockSyncLost = 16,
    /// 过零异常
    ZeroCrossingAnomaly = 17,
    /// 窃电检测
    TamperDetected = 18,
}

impl DlmsEventCode {
    /// 从 MeterEvent 转换
    pub fn from_meter_event(event: MeterEvent) -> Option<Self> {
        match event {
            MeterEvent::OverVoltageA
            | MeterEvent::OverVoltageB
            | MeterEvent::OverVoltageC => Some(Self::OverVoltage),
            
            MeterEvent::UnderVoltageA
            | MeterEvent::UnderVoltageB
            | MeterEvent::UnderVoltageC => Some(Self::UnderVoltage),
            
            MeterEvent::PhaseLossA
            | MeterEvent::PhaseLossB
            | MeterEvent::PhaseLossC => Some(Self::PhaseLoss),
            
            MeterEvent::OverCurrentA
            | MeterEvent::OverCurrentB
            | MeterEvent::OverCurrentC => Some(Self::OverCurrent),
            
            MeterEvent::CurrentUnbalance => Some(Self::CurrentUnbalance),
            MeterEvent::VoltageUnbalance => Some(Self::VoltageUnbalance),
            MeterEvent::FrequencyDeviation => Some(Self::FrequencyDeviation),
            MeterEvent::ZeroCrossingAnomaly => Some(Self::ZeroCrossingAnomaly),
            MeterEvent::CoverOpen => Some(Self::CoverOpen),
            MeterEvent::TerminalCoverOpen => Some(Self::TerminalCoverOpen),
            MeterEvent::MagneticTamper => Some(Self::MagneticTamper),
            MeterEvent::BatteryLow => Some(Self::BatteryLow),
            MeterEvent::ClockBatteryLow => Some(Self::ClockBatteryLow),
            MeterEvent::ReversePower => Some(Self::ReversePower),
            MeterEvent::ClockSyncLost => Some(Self::ClockSyncLost),
        }
    }

    /// 获取事件名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::VoltageSag => "Voltage Sag",
            Self::VoltageSwell => "Voltage Swell",
            Self::OverVoltage => "Over Voltage",
            Self::UnderVoltage => "Under Voltage",
            Self::CurrentUnbalance => "Current Unbalance",
            Self::VoltageUnbalance => "Voltage Unbalance",
            Self::FrequencyDeviation => "Frequency Deviation",
            Self::PhaseLoss => "Phase Loss",
            Self::OverCurrent => "Over Current",
            Self::ReversePower => "Reverse Power",
            Self::CoverOpen => "Cover Open",
            Self::TerminalCoverOpen => "Terminal Cover Open",
            Self::MagneticTamper => "Magnetic Tamper",
            Self::BatteryLow => "Battery Low",
            Self::ClockBatteryLow => "Clock Battery Low",
            Self::ClockSyncLost => "Clock Sync Lost",
            Self::ZeroCrossingAnomaly => "Zero Crossing Anomaly",
            Self::TamperDetected => "Tamper Detected",
        }
    }

    /// 获取事件优先级 (1=最高, 5=最低)
    pub fn priority(&self) -> u8 {
        match self {
            Self::PhaseLoss
            | Self::OverCurrent
            | Self::CoverOpen
            | Self::MagneticTamper
            | Self::TamperDetected => 1, // 紧急事件
            
            Self::OverVoltage
            | Self::UnderVoltage
            | Self::FrequencyDeviation
            | Self::ReversePower => 2, // 重要事件
            
            Self::CurrentUnbalance
            | Self::VoltageUnbalance
            | Self::VoltageSag
            | Self::VoltageSwell => 3, // 一般事件
            
            Self::ClockSyncLost
            | Self::ZeroCrossingAnomaly => 4, // 次要事件
            
            Self::BatteryLow
            | Self::ClockBatteryLow
            | Self::TerminalCoverOpen => 5, // 提示事件
        }
    }

    /// 是否需要立即上报
    pub fn requires_immediate_report(&self) -> bool {
        self.priority() <= 2
    }
}

/// DLMS 事件状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EventStatus {
    /// 事件发生
    Occurred = 1,
    /// 事件恢复
    Cleared = 0,
    /// 事件持续中
    Active = 2,
}

impl From<u8> for EventStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Cleared,
            2 => Self::Active,
            _ => Self::Occurred,
        }
    }
}

/// DLMS 事件通知
#[derive(Debug, Clone)]
pub struct DlmsEventNotification {
    /// 事件代码
    pub event_code: DlmsEventCode,
    /// 事件时间戳 (Unix timestamp)
    pub timestamp: u32,
    /// 事件状态
    pub status: EventStatus,
    /// 事件值 (如电压值、电流值)
    pub value: u32,
    /// 持续时间 (ms)
    pub duration_ms: u16,
    /// 相位标识 (0=总, 1=A相, 2=B相, 3=C相)
    pub phase: u8,
    /// 附加数据
    pub additional_data: [u8; 4],
}

impl DlmsEventNotification {
    /// 创建新的事件通知
    pub fn new(
        event_code: DlmsEventCode,
        timestamp: u32,
        status: EventStatus,
        value: u32,
        duration_ms: u16,
    ) -> Self {
        Self {
            event_code,
            timestamp,
            status,
            value,
            duration_ms,
            phase: 0,
            additional_data: [0; 4],
        }
    }

    /// 从 EventLogEntry 转换
    pub fn from_log_entry(entry: &EventLogEntry) -> Option<Self> {
        let event_code = DlmsEventCode::from_meter_event(entry.event)?;
        let status = EventStatus::from(entry.state);
        
        let mut notification = Self::new(
            event_code,
            entry.timestamp,
            status,
            entry.value,
            entry.duration,
        );
        
        // 解析相位
        notification.phase = match entry.event {
            MeterEvent::OverVoltageA
            | MeterEvent::UnderVoltageA
            | MeterEvent::PhaseLossA
            | MeterEvent::OverCurrentA => 1,
            
            MeterEvent::OverVoltageB
            | MeterEvent::UnderVoltageB
            | MeterEvent::PhaseLossB
            | MeterEvent::OverCurrentB => 2,
            
            MeterEvent::OverVoltageC
            | MeterEvent::UnderVoltageC
            | MeterEvent::PhaseLossC
            | MeterEvent::OverCurrentC => 3,
            
            _ => 0,
        };
        
        Some(notification)
    }

    /// 编码为 DLMS A-XDR 格式 (简化版)
    pub fn encode_axdr(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32);
        
        // Structure tag
        buf.push(0x02); // array
        buf.push(6);    // 6 elements
        
        // 1. Event code (double-long-unsigned)
        buf.push(0x06); // double-long-unsigned
        buf.extend_from_slice(&(self.event_code as u16 as u32).to_be_bytes()[..]);
        
        // 2. Timestamp (date-time)
        buf.push(0x09); // octet-string, length follows
        buf.push(12);   // 12 bytes for COSEM date-time
        // 简化：直接使用 Unix timestamp
        buf.extend_from_slice(&self.timestamp.to_be_bytes());
        buf.extend_from_slice(&[0u8; 8]); // 填充
        
        // 3. Status (unsigned)
        buf.push(0x0F); // unsigned
        buf.push(self.status as u8);
        
        // 4. Value (double-long)
        buf.push(0x05); // double-long
        buf.extend_from_slice(&self.value.to_be_bytes());
        
        // 5. Duration (long-unsigned)
        buf.push(0x12); // long-unsigned
        buf.extend_from_slice(&self.duration_ms.to_be_bytes());
        
        // 6. Phase (unsigned)
        buf.push(0x0F);
        buf.push(self.phase);
        
        buf
    }
}

/// 事件上报配置
#[derive(Debug, Clone)]
pub struct EventNotificationConfig {
    /// 是否启用事件上报
    pub enabled: bool,
    /// 上报目标地址
    pub destination: [u8; 6],
    /// 上报端口
    pub port: u16,
    /// 最大重试次数
    pub max_retries: u8,
    /// 重试间隔 (ms)
    pub retry_interval_ms: u16,
    /// 批量上报最大数量
    pub batch_size: u8,
    /// 是否过滤低优先级事件
    pub filter_low_priority: bool,
}

impl Default for EventNotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            destination: [0; 6],
            port: 4059, // DLMS 默认端口
            max_retries: 3,
            retry_interval_ms: 5000,
            batch_size: 10,
            filter_low_priority: false,
        }
    }
}

/// 事件上报管理器
#[derive(Debug)]
pub struct EventNotificationManager {
    /// 配置
    config: EventNotificationConfig,
    /// 待上报事件队列
    pending_notifications: Vec<DlmsEventNotification>,
    /// 已上报事件计数
    reported_count: u32,
    /// 上报失败计数
    failed_count: u32,
    /// 最后上报时间
    last_report_time: u32,
}

impl Default for EventNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EventNotificationManager {
    pub fn new() -> Self {
        Self {
            config: EventNotificationConfig::default(),
            pending_notifications: Vec::with_capacity(64),
            reported_count: 0,
            failed_count: 0,
            last_report_time: 0,
        }
    }

    /// 设置配置
    pub fn set_config(&mut self, config: EventNotificationConfig) {
        self.config = config;
    }

    /// 获取配置
    pub fn config(&self) -> &EventNotificationConfig {
        &self.config
    }

    /// 从事件检测器收集事件
    pub fn collect_from_detector(&mut self, detector: &EventDetector, current_time: u32) {
        if !self.config.enabled {
            return;
        }

        let log = detector.event_log();
        for entry in log {
            // 只处理新事件
            if entry.timestamp > self.last_report_time {
                if let Some(notification) = DlmsEventNotification::from_log_entry(entry) {
                    // 过滤低优先级事件
                    if self.config.filter_low_priority && notification.event_code.priority() > 3 {
                        continue;
                    }
                    
                    self.pending_notifications.push(notification);
                }
            }
        }

        self.last_report_time = current_time;
    }

    /// 手动添加事件通知
    pub fn add_notification(&mut self, notification: DlmsEventNotification) {
        if self.config.enabled {
            self.pending_notifications.push(notification);
        }
    }

    /// 获取待上报事件数量
    pub fn pending_count(&self) -> usize {
        self.pending_notifications.len()
    }

    /// 获取下一批待上报事件
    pub fn get_next_batch(&mut self) -> Vec<DlmsEventNotification> {
        let batch_size = self.config.batch_size as usize;
        let count = batch_size.min(self.pending_notifications.len());
        
        self.pending_notifications.drain(0..count).collect()
    }

    /// 标记上报成功
    pub fn mark_reported(&mut self, count: usize) {
        self.reported_count += count as u32;
    }

    /// 标记上报失败
    pub fn mark_failed(&mut self) {
        self.failed_count += 1;
    }

    /// 获取统计信息
    pub fn statistics(&self) -> EventNotificationStats {
        EventNotificationStats {
            pending_count: self.pending_count(),
            reported_count: self.reported_count,
            failed_count: self.failed_count,
            last_report_time: self.last_report_time,
        }
    }

    /// 清空待上报队列
    pub fn clear_pending(&mut self) {
        self.pending_notifications.clear();
    }

    /// 重置统计
    pub fn reset_statistics(&mut self) {
        self.reported_count = 0;
        self.failed_count = 0;
    }
}

/// 事件上报统计
#[derive(Debug, Clone, Copy)]
pub struct EventNotificationStats {
    pub pending_count: usize,
    pub reported_count: u32,
    pub failed_count: u32,
    pub last_report_time: u32,
}

/// DLMS IC10 Script Table 事件动作
#[derive(Debug, Clone)]
pub struct ScriptTableEntry {
    /// 脚本 ID
    pub script_id: u16,
    /// 触发事件
    pub trigger_event: DlmsEventCode,
    /// 动作类型
    pub action: ScriptAction,
}

/// 脚本动作类型
#[derive(Debug, Clone)]
pub enum ScriptAction {
    /// 发送事件通知
    SendNotification,
    /// 写入寄存器
    WriteRegister { register_id: u16, value: u32 },
    /// 执行方法
    ExecuteMethod { object_id: [u8; 6], method_id: u8 },
    /// 激活费率
    ActivateTariff { tariff_id: u8 },
}

/// 事件处理器 (IC10)
#[derive(Debug, Default)]
pub struct ScriptTable {
    entries: Vec<ScriptTableEntry>,
}

impl ScriptTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加脚本条目
    pub fn add_entry(&mut self, entry: ScriptTableEntry) {
        self.entries.push(entry);
    }

    /// 查找事件对应的脚本
    pub fn find_scripts_for_event(&self, event: DlmsEventCode) -> Vec<&ScriptTableEntry> {
        self.entries
            .iter()
            .filter(|e| e.trigger_event == event)
            .collect()
    }

    /// 执行脚本
    pub fn execute(&self, event: DlmsEventCode) -> Vec<ScriptAction> {
        self.entries
            .iter()
            .filter(|e| e.trigger_event == event)
            .map(|e| e.action.clone())
            .collect()
    }

    /// 获取所有条目
    pub fn entries(&self) -> &[ScriptTableEntry] {
        &self.entries
    }

    /// 清空
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/* ================================================================== */
/*  单元测试                                                           */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== DlmsEventCode 测试 ====================

    #[test]
    fn test_event_code_from_meter_event() {
        assert_eq!(
            DlmsEventCode::from_meter_event(MeterEvent::OverVoltageA),
            Some(DlmsEventCode::OverVoltage)
        );
        assert_eq!(
            DlmsEventCode::from_meter_event(MeterEvent::PhaseLossB),
            Some(DlmsEventCode::PhaseLoss)
        );
        assert_eq!(
            DlmsEventCode::from_meter_event(MeterEvent::CoverOpen),
            Some(DlmsEventCode::CoverOpen)
        );
    }

    #[test]
    fn test_event_code_name() {
        assert_eq!(DlmsEventCode::OverVoltage.name(), "Over Voltage");
        assert_eq!(DlmsEventCode::PhaseLoss.name(), "Phase Loss");
    }

    #[test]
    fn test_event_code_priority() {
        assert_eq!(DlmsEventCode::PhaseLoss.priority(), 1);
        assert_eq!(DlmsEventCode::OverVoltage.priority(), 2);
        assert_eq!(DlmsEventCode::BatteryLow.priority(), 5);
    }

    #[test]
    fn test_event_code_immediate_report() {
        assert!(DlmsEventCode::PhaseLoss.requires_immediate_report());
        assert!(DlmsEventCode::OverVoltage.requires_immediate_report());
        assert!(!DlmsEventCode::BatteryLow.requires_immediate_report());
    }

    // ==================== EventStatus 测试 ====================

    #[test]
    fn test_event_status_from_u8() {
        assert_eq!(EventStatus::from(0), EventStatus::Cleared);
        assert_eq!(EventStatus::from(1), EventStatus::Occurred);
        assert_eq!(EventStatus::from(2), EventStatus::Active);
    }

    // ==================== DlmsEventNotification 测试 ====================

    #[test]
    fn test_notification_new() {
        let notif = DlmsEventNotification::new(
            DlmsEventCode::OverVoltage,
            1234567890,
            EventStatus::Occurred,
            27000,
            3000,
        );
        
        assert_eq!(notif.event_code, DlmsEventCode::OverVoltage);
        assert_eq!(notif.timestamp, 1234567890);
        assert_eq!(notif.status, EventStatus::Occurred);
        assert_eq!(notif.value, 27000);
        assert_eq!(notif.duration_ms, 3000);
        assert_eq!(notif.phase, 0);
    }

    #[test]
    fn test_notification_from_log_entry() {
        let entry = EventLogEntry {
            event: MeterEvent::OverVoltageA,
            timestamp: 1234567890,
            value: 27000,
            duration: 3000,
            state: 1,
            _reserved: 0,
        };
        
        let notif = DlmsEventNotification::from_log_entry(&entry).unwrap();
        
        assert_eq!(notif.event_code, DlmsEventCode::OverVoltage);
        assert_eq!(notif.timestamp, 1234567890);
        assert_eq!(notif.status, EventStatus::Occurred);
        assert_eq!(notif.value, 27000);
        assert_eq!(notif.duration_ms, 3000);
        assert_eq!(notif.phase, 1); // A相
    }

    #[test]
    fn test_notification_encode_axdr() {
        let notif = DlmsEventNotification::new(
            DlmsEventCode::OverVoltage,
            1234567890,
            EventStatus::Occurred,
            27000,
            3000,
        );
        
        let encoded = notif.encode_axdr();
        
        // 检查基本结构
        assert!(!encoded.is_empty());
        assert_eq!(encoded[0], 0x02); // array tag
        assert_eq!(encoded[1], 6);    // 6 elements
    }

    #[test]
    fn test_notification_phase_b() {
        let entry = EventLogEntry {
            event: MeterEvent::UnderVoltageB,
            timestamp: 0,
            value: 0,
            duration: 0,
            state: 1,
            _reserved: 0,
        };
        
        let notif = DlmsEventNotification::from_log_entry(&entry).unwrap();
        assert_eq!(notif.phase, 2); // B相
    }

    #[test]
    fn test_notification_phase_c() {
        let entry = EventLogEntry {
            event: MeterEvent::OverCurrentC,
            timestamp: 0,
            value: 0,
            duration: 0,
            state: 1,
            _reserved: 0,
        };
        
        let notif = DlmsEventNotification::from_log_entry(&entry).unwrap();
        assert_eq!(notif.phase, 3); // C相
    }

    // ==================== EventNotificationConfig 测试 ====================

    #[test]
    fn test_config_default() {
        let config = EventNotificationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.port, 4059);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.batch_size, 10);
    }

    // ==================== EventNotificationManager 测试 ====================

    #[test]
    fn test_manager_new() {
        let manager = EventNotificationManager::new();
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_manager_add_notification() {
        let mut manager = EventNotificationManager::new();
        let notif = DlmsEventNotification::new(
            DlmsEventCode::OverVoltage,
            0,
            EventStatus::Occurred,
            0,
            0,
        );
        
        manager.add_notification(notif);
        assert_eq!(manager.pending_count(), 1);
    }

    #[test]
    fn test_manager_get_next_batch() {
        let mut manager = EventNotificationManager::new();
        
        for i in 0..15 {
            let notif = DlmsEventNotification::new(
                DlmsEventCode::OverVoltage,
                i,
                EventStatus::Occurred,
                0,
                0,
            );
            manager.add_notification(notif);
        }
        
        // 批量大小为 10
        let batch = manager.get_next_batch();
        assert_eq!(batch.len(), 10);
        assert_eq!(manager.pending_count(), 5);
    }

    #[test]
    fn test_manager_mark_reported() {
        let mut manager = EventNotificationManager::new();
        manager.mark_reported(5);
        
        let stats = manager.statistics();
        assert_eq!(stats.reported_count, 5);
    }

    #[test]
    fn test_manager_mark_failed() {
        let mut manager = EventNotificationManager::new();
        manager.mark_failed();
        manager.mark_failed();
        
        let stats = manager.statistics();
        assert_eq!(stats.failed_count, 2);
    }

    #[test]
    fn test_manager_clear_pending() {
        let mut manager = EventNotificationManager::new();
        manager.add_notification(DlmsEventNotification::new(
            DlmsEventCode::OverVoltage,
            0,
            EventStatus::Occurred,
            0,
            0,
        ));
        
        assert_eq!(manager.pending_count(), 1);
        manager.clear_pending();
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_manager_disabled() {
        let mut manager = EventNotificationManager::new();
        manager.set_config(EventNotificationConfig {
            enabled: false,
            ..Default::default()
        });
        
        manager.add_notification(DlmsEventNotification::new(
            DlmsEventCode::OverVoltage,
            0,
            EventStatus::Occurred,
            0,
            0,
        ));
        
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_manager_collect_from_detector() {
        let mut manager = EventNotificationManager::new();
        let mut detector = EventDetector::new();
        
        detector.set_timestamp(1000);
        detector.trigger_external(MeterEvent::CoverOpen);
        
        manager.collect_from_detector(&detector, 1000);
        
        assert_eq!(manager.pending_count(), 1);
    }

    #[test]
    fn test_manager_filter_low_priority() {
        let mut manager = EventNotificationManager::new();
        manager.set_config(EventNotificationConfig {
            filter_low_priority: true,
            ..Default::default()
        });
        
        let mut detector = EventDetector::new();
        detector.set_timestamp(1000);
        detector.trigger_external(MeterEvent::BatteryLow); // priority 5
        
        manager.collect_from_detector(&detector, 1000);
        
        assert_eq!(manager.pending_count(), 0); // 被过滤
    }

    #[test]
    fn test_manager_reset_statistics() {
        let mut manager = EventNotificationManager::new();
        manager.mark_reported(10);
        manager.mark_failed();
        
        manager.reset_statistics();
        
        let stats = manager.statistics();
        assert_eq!(stats.reported_count, 0);
        assert_eq!(stats.failed_count, 0);
    }

    // ==================== ScriptTable 测试 ====================

    #[test]
    fn test_script_table_new() {
        let table = ScriptTable::new();
        assert!(table.entries().is_empty());
    }

    #[test]
    fn test_script_table_add_entry() {
        let mut table = ScriptTable::new();
        let entry = ScriptTableEntry {
            script_id: 1,
            trigger_event: DlmsEventCode::OverVoltage,
            action: ScriptAction::SendNotification,
        };
        
        table.add_entry(entry);
        assert_eq!(table.entries().len(), 1);
    }

    #[test]
    fn test_script_table_find_scripts() {
        let mut table = ScriptTable::new();
        
        table.add_entry(ScriptTableEntry {
            script_id: 1,
            trigger_event: DlmsEventCode::OverVoltage,
            action: ScriptAction::SendNotification,
        });
        
        table.add_entry(ScriptTableEntry {
            script_id: 2,
            trigger_event: DlmsEventCode::OverVoltage,
            action: ScriptAction::ActivateTariff { tariff_id: 2 },
        });
        
        table.add_entry(ScriptTableEntry {
            script_id: 3,
            trigger_event: DlmsEventCode::PhaseLoss,
            action: ScriptAction::SendNotification,
        });
        
        let scripts = table.find_scripts_for_event(DlmsEventCode::OverVoltage);
        assert_eq!(scripts.len(), 2);
        
        let scripts = table.find_scripts_for_event(DlmsEventCode::PhaseLoss);
        assert_eq!(scripts.len(), 1);
    }

    #[test]
    fn test_script_table_execute() {
        let mut table = ScriptTable::new();
        
        table.add_entry(ScriptTableEntry {
            script_id: 1,
            trigger_event: DlmsEventCode::CoverOpen,
            action: ScriptAction::SendNotification,
        });
        
        table.add_entry(ScriptTableEntry {
            script_id: 2,
            trigger_event: DlmsEventCode::CoverOpen,
            action: ScriptAction::ExecuteMethod {
                object_id: [0, 0, 96, 1, 0, 255],
                method_id: 1,
            },
        });
        
        let actions = table.execute(DlmsEventCode::CoverOpen);
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn test_script_table_clear() {
        let mut table = ScriptTable::new();
        table.add_entry(ScriptTableEntry {
            script_id: 1,
            trigger_event: DlmsEventCode::OverVoltage,
            action: ScriptAction::SendNotification,
        });
        
        table.clear();
        assert!(table.entries().is_empty());
    }

    // ==================== ScriptAction 测试 ====================

    #[test]
    fn test_script_action_notification() {
        let action = ScriptAction::SendNotification;
        assert!(matches!(action, ScriptAction::SendNotification));
    }

    #[test]
    fn test_script_action_write_register() {
        let action = ScriptAction::WriteRegister {
            register_id: 0x1000,
            value: 12345,
        };
        
        if let ScriptAction::WriteRegister { register_id, value } = action {
            assert_eq!(register_id, 0x1000);
            assert_eq!(value, 12345);
        } else {
            panic!("Wrong action type");
        }
    }

    #[test]
    fn test_script_action_activate_tariff() {
        let action = ScriptAction::ActivateTariff { tariff_id: 3 };
        
        if let ScriptAction::ActivateTariff { tariff_id } = action {
            assert_eq!(tariff_id, 3);
        } else {
            panic!("Wrong action type");
        }
    }
}
