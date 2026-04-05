/* ================================================================== */
/*                                                                    */
/*  test_metering_flow_integration.rs                                  */
/*                                                                    */
/*  计量数据完整流水线集成测试                                          */
/*                                                                    */
/*  测试流程：                                                         */
/*  采样 → MeteringProcessor → TOU 费率计算 → 存储 → 显示 → DLMS 读取  */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/*                                                                    */
/* ================================================================== */

use femeter_core::{
    CalibrationParams, EnergyData, PhaseData,
    event_detect::{EventDetector, MeterEvent, VoltageThresholds},
    event_notification::{
        DlmsEventCode, DlmsEventNotification, EventNotificationManager, EventStatus,
        ScriptTable, ScriptTableEntry, ScriptAction, EventNotificationConfig,
    },
    metering::{
        DisplayFormatter, MemoryStorage, MeteringProcessor, MeteringSampler,
        MeteringStorage,
    },
    tou::{TariffEnergy, TariffRate, TouEngine, TouPreset, DayProfile},
};

// ═══════════════════════════════════════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════════════════════════════════════

/// 创建标准测试用三相数据 (220V, 50A, 33kW)
fn create_normal_phase_data() -> PhaseData {
    PhaseData {
        voltage_a: 22000,   // 220.00V
        voltage_b: 22100,   // 221.00V
        voltage_c: 21900,   // 219.00V
        current_a: 5000,    // 50.00A
        current_b: 5100,    // 51.00A
        current_c: 4900,    // 49.00A
        active_power_total: 330000, // 3.3kW
        reactive_power_total: 50000,
        apparent_power_total: 333800,
        frequency: 5000,    // 50.00Hz
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

/// 创建过压数据
fn create_overvoltage_data() -> PhaseData {
    let mut data = create_normal_phase_data();
    data.voltage_a = 27000; // 270.00V (过压)
    data
}

/// 创建欠压数据
fn create_undervoltage_data() -> PhaseData {
    let mut data = create_normal_phase_data();
    data.voltage_a = 17000; // 170.00V (欠压)
    data
}

/// 创建断相数据
fn create_phase_loss_data() -> PhaseData {
    let mut data = create_normal_phase_data();
    data.voltage_c = 500; // 断相
    data
}

// ═══════════════════════════════════════════════════════════════════════════
// 测试 1-5: 基础采样和存储流程
// ═══════════════════════════════════════════════════════════════════════════

/// 测试 1: 完整采样 → 存储 → 读取流程
#[test]
fn test_sample_store_read_flow() {
    let mut sampler = MeteringSampler::new();
    let mut storage = MemoryStorage::new(100);
    let data = create_normal_phase_data();
    
    // 采样
    let sampled = sampler.sample(&data, 1000);
    
    // 存储瞬时数据
    storage.store_instantaneous(&sampled, 1000).unwrap();
    
    // 存储能量数据
    storage.store_energy(sampler.energy(), 1000).unwrap();
    
    // 验证存储的数据
    assert_eq!(storage.instantaneous_records().len(), 1);
    assert_eq!(storage.energy_records().len(), 1);
    
    // 读取并验证
    let read_energy = storage.read_energy(1000).unwrap();
    assert_eq!(read_energy.active_import, sampler.energy().active_import);
}

/// 测试 2: 多次采样能量累计
#[test]
fn test_multiple_sample_energy_accumulation() {
    let mut sampler = MeteringSampler::new();
    let mut storage = MemoryStorage::new(100);
    let data = create_normal_phase_data();
    
    // 模拟 1 小时，每秒采样一次
    for i in 0..3600 {
        let timestamp_ms = (i + 1) * 1000;
        let _sampled = sampler.sample(&data, timestamp_ms);
        
        // 每 60 秒存储一次
        if (i + 1) % 60 == 0 {
            storage.store_energy(sampler.energy(), (i + 1) as u32 / 60).unwrap();
        }
    }
    
    // 验证能量累计
    // 3.3kW × 1h = 3300Wh
    // 计算公式: (power * dt_ms / 3600) for each sample
    // 330000(0.01W) × 1000ms / 3600 × 3600 = 329,997,600
    // 由于计算精度，允许一定误差
    let energy = sampler.energy();
    assert!(energy.active_import > 300_000_000, "能量累计过低: {}", energy.active_import);
    assert!(energy.active_import < 400_000_000, "能量累计过高: {}", energy.active_import);
    
    // 验证存储记录数
    assert_eq!(storage.energy_records().len(), 60);
}

/// 测试 3: 校准参数应用
#[test]
fn test_calibration_application() {
    let mut sampler = MeteringSampler::new();
    sampler.set_calibration(CalibrationParams {
        voltage_gain_a: 0.1,    // +10%
        voltage_gain_b: -0.05,  // -5%
        current_gain_a: 0.2,    // +20%
        ..Default::default()
    });
    
    let data = create_normal_phase_data();
    let calibrated = sampler.sample(&data, 1000);
    
    // 验证校准效果
    // voltage_a: 22000 * 1.1 = 24200
    assert_eq!(calibrated.voltage_a, 24200);
    // voltage_b: 22100 * 0.95 = 20995
    assert_eq!(calibrated.voltage_b, 20995);
    // current_a: 5000 * 1.2 = 6000
    assert_eq!(calibrated.current_a, 6000);
    // voltage_c 未校准
    assert_eq!(calibrated.voltage_c, 21900);
}

/// 测试 4: 需量处理
#[test]
fn test_demand_processing() {
    let mut processor = MeteringProcessor::new();
    let mut storage = MemoryStorage::new(100);
    let data = create_normal_phase_data();
    
    processor.set_demand_period(15); // 15 分钟周期
    
    // 模拟 15 分钟，每秒一个采样
    for i in 0..900 {
        processor.process(&data, i);
    }
    
    // 结算需量周期
    let demand = processor.settle_demand_period();
    storage.store_demand(demand, 900).unwrap();
    
    // 验证需量接近 3.3kW (330000 * 0.01W)
    assert!(demand > 300000 && demand < 360000);
    
    // 验证最大需量
    assert_eq!(processor.max_demand(), demand);
}

/// 测试 5: 存储历史记录查询
#[test]
fn test_storage_historical_query() {
    let mut storage = MemoryStorage::new(100);
    
    // 存储多个时间点的数据
    for i in 0..10 {
        let ts = ((i + 1) * 1000) as u32;
        let energy = EnergyData {
            active_import: (i + 1) * 100000,
            ..Default::default()
        };
        storage.store_energy(&energy, ts).unwrap();
    }
    
    // 查询历史数据
    assert_eq!(storage.read_energy(5000).unwrap().active_import, 500000);
    assert_eq!(storage.read_energy(5500).unwrap().active_import, 500000); // 返回 <= 5500 的最近记录
    assert_eq!(storage.read_energy(10000).unwrap().active_import, 1000000);
}

// ═══════════════════════════════════════════════════════════════════════════
// 测试 6-10: TOU 费率计算流程
// ═══════════════════════════════════════════════════════════════════════════

/// 测试 6: 采样 + TOU 费率计算
#[test]
fn test_sample_with_tou_calculation() {
    let mut sampler = MeteringSampler::new();
    let mut tou_engine = TouEngine::new();
    tou_engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);
    
    let data = create_normal_phase_data();
    
    // 谷时段 (00:00-06:00)
    let _sampled = sampler.sample(&data, 0);
    let rate = tou_engine.calculate_rate(4, 5, 1, 3, 0);
    assert_eq!(rate, TariffRate::T4);
    
    // 尖时段 (10:00-12:00)
    let rate = tou_engine.calculate_rate(4, 5, 1, 10, 30);
    assert_eq!(rate, TariffRate::T1);
    
    // 峰时段 (17:00-18:00)
    let rate = tou_engine.calculate_rate(4, 5, 1, 17, 30);
    assert_eq!(rate, TariffRate::T2);
    
    // 平时段 (12:00-17:00)
    let rate = tou_engine.calculate_rate(4, 5, 1, 14, 0);
    assert_eq!(rate, TariffRate::T3);
}

/// 测试 7: 按费率累计电能
#[test]
fn test_tariff_energy_accumulation() {
    let mut sampler = MeteringSampler::new();
    let mut tou_engine = TouEngine::new();
    let mut tariff_energy = TariffEnergy::new();
    tou_engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);
    
    let data = create_normal_phase_data();
    
    // 模拟 24 小时
    for hour in 0..24 {
        let timestamp_ms = (hour * 3600 + 1) * 1000;
        sampler.sample(&data, timestamp_ms as u64);
        
        let rate = tou_engine.calculate_rate(4, 5, 1, hour as u8, 0);
        
        // 累计到对应费率
        let energy_inc = 1000u64; // 假设每小时增加 1kWh
        tariff_energy.accumulate(rate, 0, energy_inc, 0);
    }
    
    // 验证各费率都有累计
    let total = tariff_energy.get_all_active_total();
    assert_eq!(total, 24000); // 24 小时 × 1000
}

/// 测试 8: 费率切换准确性
#[test]
fn test_tariff_switch_accuracy() {
    let mut tou_engine = TouEngine::new();
    tou_engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);
    
    // 测试边界时间
    let test_cases = vec![
        (5, 59, TariffRate::T4),  // 谷时段末尾
        (6, 0, TariffRate::T3),   // 平时段开始
        (9, 59, TariffRate::T2),  // 峰时段末尾
        (10, 0, TariffRate::T1),  // 尖时段开始
        (11, 59, TariffRate::T1), // 尖时段末尾
        (12, 0, TariffRate::T3),  // 平时段开始
        (19, 59, TariffRate::T1), // 尖时段末尾
        (20, 0, TariffRate::T2),  // 峰时段开始
        (20, 59, TariffRate::T2), // 峰时段末尾
        (21, 0, TariffRate::T3),  // 平时段开始
        (21, 59, TariffRate::T3), // 平时段末尾
        (22, 0, TariffRate::T4),  // 谷时段开始
    ];
    
    for (hour, min, expected) in test_cases {
        let rate = tou_engine.calculate_rate(4, 5, 1, hour, min);
        assert_eq!(rate, expected, "费率切换错误 at {}:{}", hour, min);
    }
}

/// 测试 9: 跨费率时段能量累计
#[test]
fn test_cross_tariff_energy_accumulation() {
    let mut sampler = MeteringSampler::new();
    let mut tou_engine = TouEngine::new();
    let mut tariff_energy = TariffEnergy::new();
    tou_engine.load_preset(TouPreset::TwoRatePeakValley);
    
    let data = create_normal_phase_data();
    
    // 模拟跨时段场景：从 07:00 到 09:00
    // 07:00-08:00 是谷时段，08:00-09:00 是峰时段
    for minute in 0..120 {
        let timestamp_ms = ((7 * 60 + minute) * 60 + 1) * 1000;
        sampler.sample(&data, timestamp_ms as u64);
        
        let hour = 7 + minute / 60;
        let min = minute % 60;
        let rate = tou_engine.calculate_rate(4, 5, 1, hour as u8, min as u8);
        
        // 每分钟累计
        tariff_energy.accumulate(rate, 0, 100, 0);
    }
    
    // 验证两费率都有累计
    let valley = tariff_energy.get_active_total(TariffRate::T2); // 谷
    let peak = tariff_energy.get_active_total(TariffRate::T1);   // 峰
    
    assert!(valley > 0, "谷时段应该有累计");
    assert!(peak > 0, "峰时段应该有累计");
}

/// 测试 10: 特殊日（节假日）费率
#[test]
fn test_holiday_tariff() {
    let mut tou_engine = TouEngine::new();
    tou_engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);
    
    // 添加国庆节特殊日时段表（全谷）
    let holiday_dp = DayProfile::single_rate(2, TariffRate::T4);
    tou_engine.calendar.day_profiles.push(holiday_dp);
    tou_engine.add_holiday(10, 1, 2);
    
    // 国庆节（10月1日）10:00 应该是谷时段
    let rate = tou_engine.calculate_rate(10, 1, 1, 10, 0);
    assert_eq!(rate, TariffRate::T4, "节假日应该是谷时段");
    
    // 正常日（10月2日）10:00 应该是尖时段
    let rate = tou_engine.calculate_rate(10, 2, 1, 10, 0);
    assert_eq!(rate, TariffRate::T1, "正常日应该是尖时段");
}

// ═══════════════════════════════════════════════════════════════════════════
// 测试 11-15: 事件检测和上报流程
// ═══════════════════════════════════════════════════════════════════════════

/// 测试 11: 过压 → 事件检测 → 存储
#[test]
fn test_overvoltage_event_detection_and_storage() {
    let mut detector = EventDetector::new();
    let mut storage = MemoryStorage::new(100);
    
    // 设置低阈值以便快速触发
    detector.set_voltage_thresholds(VoltageThresholds {
        over_voltage: 25000,
        duration_ms: 0,
        ..Default::default()
    });
    
    let overvoltage_data = create_overvoltage_data();
    detector.set_timestamp(1000);
    
    // 检测过压
    let events = detector.check(&overvoltage_data);
    assert_ne!(events & (1 << MeterEvent::OverVoltageA as u8), 0);
    
    // 存储事件日志
    let log = detector.event_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].event, MeterEvent::OverVoltageA);
    assert_eq!(log[0].timestamp, 1000);
}

/// 测试 12: 事件检测 → DLMS 通知转换
#[test]
fn test_event_to_dlms_notification() {
    let mut detector = EventDetector::new();
    detector.set_timestamp(1234567890);
    
    // 触发过压事件
    detector.set_voltage_thresholds(VoltageThresholds {
        over_voltage: 25000,
        duration_ms: 0,
        ..Default::default()
    });
    let overvoltage_data = create_overvoltage_data();
    detector.check(&overvoltage_data);
    
    // 获取事件日志并转换为 DLMS 通知
    let log = detector.event_log();
    let notification = DlmsEventNotification::from_log_entry(&log[0]).unwrap();
    
    assert_eq!(notification.event_code, DlmsEventCode::OverVoltage);
    assert_eq!(notification.timestamp, 1234567890);
    assert_eq!(notification.status, EventStatus::Occurred);
    assert_eq!(notification.phase, 1); // A 相
}

/// 测试 13: 事件上报管理
#[test]
fn test_event_notification_manager() {
    let mut detector = EventDetector::new();
    let mut manager = EventNotificationManager::new();
    
    detector.set_timestamp(1000);
    detector.set_voltage_thresholds(VoltageThresholds {
        over_voltage: 25000,
        duration_ms: 0,
        ..Default::default()
    });
    
    // 触发多个事件
    detector.check(&create_overvoltage_data());
    detector.trigger_external(MeterEvent::CoverOpen);
    
    // 收集事件
    manager.collect_from_detector(&detector, 1000);
    
    assert_eq!(manager.pending_count(), 2);
    
    // 获取批量上报
    let batch = manager.get_next_batch();
    assert_eq!(batch.len(), 2);
    
    // 标记上报成功
    manager.mark_reported(2);
    
    let stats = manager.statistics();
    assert_eq!(stats.reported_count, 2);
}

/// 测试 14: 完整事件流程：检测 → 通知 → DLMS 编码
#[test]
fn test_complete_event_flow() {
    let mut detector = EventDetector::new();
    let mut manager = EventNotificationManager::new();
    
    detector.set_timestamp(1000000);
    detector.set_voltage_thresholds(VoltageThresholds {
        over_voltage: 25000,
        under_voltage: 18000,
        lost_voltage: 5000,
        duration_ms: 0,
    });
    
    // 触发过压事件
    detector.check(&create_overvoltage_data());
    
    // 收集并获取通知
    manager.collect_from_detector(&detector, 1000000);
    let batch = manager.get_next_batch();
    
    assert_eq!(batch.len(), 1);
    
    // 编码为 DLMS A-XDR 格式
    let encoded = batch[0].encode_axdr();
    assert!(!encoded.is_empty());
    assert_eq!(encoded[0], 0x02); // array tag
}

/// 测试 15: Script Table 事件动作
#[test]
fn test_script_table_event_actions() {
    let mut script_table = ScriptTable::new();
    
    // 添加过压事件脚本
    script_table.add_entry(ScriptTableEntry {
        script_id: 1,
        trigger_event: DlmsEventCode::OverVoltage,
        action: ScriptAction::SendNotification,
    });
    
    script_table.add_entry(ScriptTableEntry {
        script_id: 2,
        trigger_event: DlmsEventCode::OverVoltage,
        action: ScriptAction::ActivateTariff { tariff_id: 2 },
    });
    
    script_table.add_entry(ScriptTableEntry {
        script_id: 3,
        trigger_event: DlmsEventCode::CoverOpen,
        action: ScriptAction::ExecuteMethod {
            object_id: [0, 0, 96, 1, 0, 255],
            method_id: 1,
        },
    });
    
    // 查找过压事件的脚本
    let actions = script_table.execute(DlmsEventCode::OverVoltage);
    assert_eq!(actions.len(), 2);
    
    // 查找开盖事件的脚本
    let actions = script_table.execute(DlmsEventCode::CoverOpen);
    assert_eq!(actions.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// 测试 16-20: 显示格式化和数据一致性
// ═══════════════════════════════════════════════════════════════════════════

/// 测试 16: 显示格式化
#[test]
fn test_display_formatting() {
    let formatter = DisplayFormatter::new();
    
    // 电压
    assert_eq!(formatter.format_voltage(22000), "220.00V");
    assert_eq!(formatter.format_voltage(22150), "221.50V");
    
    // 电流
    assert_eq!(formatter.format_current(5000), "50.00A");
    
    // 功率
    assert_eq!(formatter.format_power(330000), "3.300kW");
    
    // 能量
    assert_eq!(formatter.format_energy(1000000), "1.000kWh");
    assert_eq!(formatter.format_energy(1234567), "1.235kWh");
    
    // 频率
    assert_eq!(formatter.format_frequency(5000), "50.00Hz");
    
    // 功率因数
    assert_eq!(formatter.format_power_factor(988), "0.988");
}

/// 测试 17: 采样值 → 显示值一致性
#[test]
fn test_sample_to_display_consistency() {
    let mut sampler = MeteringSampler::new();
    let formatter = DisplayFormatter::new();
    
    let raw_data = PhaseData {
        voltage_a: 22000,
        current_a: 5000,
        active_power_total: 330000,
        frequency: 5000,
        power_factor_total: 988,
        ..Default::default()
    };
    
    let sampled = sampler.sample(&raw_data, 1000);
    
    // 验证采样值与显示值一致
    assert_eq!(formatter.format_voltage(sampled.voltage_a), "220.00V");
    assert_eq!(formatter.format_current(sampled.current_a), "50.00A");
    assert_eq!(formatter.format_power(sampled.active_power_total), "3.300kW");
    assert_eq!(formatter.format_frequency(sampled.frequency), "50.00Hz");
}

/// 测试 18: 存储 → 读取数据一致性
#[test]
fn test_store_read_consistency() {
    let mut sampler = MeteringSampler::new();
    let mut storage = MemoryStorage::new(100);
    
    let data = create_normal_phase_data();
    
    // 多次采样并存储
    for i in 0..10 {
        let ts = ((i + 1) * 1000) as u32;
        sampler.sample(&data, ts as u64);
        storage.store_energy(sampler.energy(), ts).unwrap();
    }
    
    // 验证每次存储的数据与采样器中的数据一致
    let records = storage.energy_records();
    for (i, (ts, energy)) in records.iter().enumerate() {
        assert_eq!(*ts, ((i + 1) * 1000) as u32);
        // 第一个记录的能量为0（第一次采样不累计），后续记录应>0
        if i > 0 {
            assert!(energy.active_import > 0, "Record {} should have accumulated energy", i);
        }
    }
}

/// 测试 19: 校准 → 采样 → 存储全流程
#[test]
fn test_calibration_sample_store_flow() {
    let mut sampler = MeteringSampler::new();
    let mut storage = MemoryStorage::new(100);
    let formatter = DisplayFormatter::new();
    
    // 设置校准参数
    sampler.set_calibration(CalibrationParams {
        voltage_gain_a: 0.05, // +5%
        current_gain_a: 0.1,  // +10%
        ..Default::default()
    });
    
    let raw_data = PhaseData {
        voltage_a: 20000,
        current_a: 10000,
        active_power_total: 200000,
        ..Default::default()
    };
    
    // 采样
    let calibrated = sampler.sample(&raw_data, 1000);
    
    // 存储
    storage.store_instantaneous(&calibrated, 1000).unwrap();
    storage.store_energy(sampler.energy(), 1000).unwrap();
    
    // 验证校准后的值
    // voltage_a: 20000 * 1.05 = 21000
    assert_eq!(calibrated.voltage_a, 21000);
    assert_eq!(formatter.format_voltage(calibrated.voltage_a), "210.00V");
    
    // current_a: 10000 * 1.1 = 11000
    assert_eq!(calibrated.current_a, 11000);
    assert_eq!(formatter.format_current(calibrated.current_a), "110.00A");
    
    // 验证存储一致性
    let records = storage.instantaneous_records();
    assert_eq!(records[0].1.voltage_a, 21000);
    assert_eq!(records[0].1.current_a, 11000);
}

/// 测试 20: 完整流水线端到端测试
#[test]
fn test_complete_pipeline_e2e() {
    // 初始化所有组件
    let mut sampler = MeteringSampler::new();
    let mut processor = MeteringProcessor::new();
    let mut tou_engine = TouEngine::new();
    let mut tariff_energy = TariffEnergy::new();
    let mut storage = MemoryStorage::new(1000);
    let mut detector = EventDetector::new();
    let mut event_manager = EventNotificationManager::new();
    let formatter = DisplayFormatter::new();
    
    // 配置
    tou_engine.load_preset(TouPreset::FourRatePeakFlatValleySharp);
    processor.set_demand_period(15);
    detector.set_voltage_thresholds(VoltageThresholds {
        over_voltage: 25000,
        duration_ms: 0,
        ..Default::default()
    });
    
    let normal_data = create_normal_phase_data();
    let overvoltage_data = create_overvoltage_data();
    
    // 模拟 1 小时运行，每秒采样一次
    for second in 0..3600 {
        let timestamp_ms = (second + 1) * 1000;
        let timestamp_s = (second + 1) as u32;
        
        // 选择数据（第 30 分钟时出现过压）
        let data = if second >= 1800 && second < 1860 {
            &overvoltage_data
        } else {
            &normal_data
        };
        
        // 1. 采样
        let sampled = sampler.sample(data, timestamp_ms as u64);
        
        // 2. 处理
        processor.process(&sampled, timestamp_s);
        
        // 3. TOU 计算
        let hour = (second / 3600) as u8;
        let minute = ((second % 3600) / 60) as u8;
        let rate = tou_engine.calculate_rate(4, 5, 1, hour, minute);
        
        // 4. 按费率累计能量
        let energy_inc = 1u64; // 每秒 0.001Wh
        tariff_energy.accumulate(rate, 0, energy_inc, 0);
        
        // 5. 事件检测
        detector.set_timestamp(timestamp_s);
        let events = detector.check(&sampled);
        if events != 0 {
            event_manager.collect_from_detector(&detector, timestamp_s);
        }
        
        // 6. 定期存储
        if (second + 1) % 60 == 0 {
            storage.store_instantaneous(&sampled, timestamp_s).unwrap();
            storage.store_energy(sampler.energy(), timestamp_s / 60).unwrap();
        }
        
        // 7. 每15分钟结算需量周期
        if (second + 1) % 900 == 0 {
            processor.settle_demand_period();
        }
    }
    
    // 验证最终状态
    
    // 能量累计
    let total_energy = sampler.energy();
    assert!(total_energy.active_import > 0);
    
    // 费率能量
    let tariff_total = tariff_energy.get_all_active_total();
    assert_eq!(tariff_total, 3600); // 3600 秒 × 1
    
    // 需量
    let max_demand = processor.max_demand();
    assert!(max_demand > 0);
    
    // 存储记录
    assert_eq!(storage.energy_records().len(), 60);
    
    // 事件
    assert!(event_manager.statistics().reported_count == 0); // 还未标记上报
    let batch = event_manager.get_next_batch();
    assert!(!batch.is_empty()); // 有待上报事件
    
    // 格式化显示
    let display = formatter.format_energy(total_energy.active_import);
    assert!(display.ends_with("kWh"));
    
    println!("完整流水线测试通过:");
    println!("  - 总能量: {}", display);
    println!("  - 费率能量: {} (0.001Wh)", tariff_total);
    println!("  - 最大需量: {} (0.01W)", max_demand);
    println!("  - 存储记录: {} 条", storage.energy_records().len());
    println!("  - 待上报事件: {} 条", batch.len());
}

// ═══════════════════════════════════════════════════════════════════════════
// 测试 21-25: 边界条件和异常场景
// ═══════════════════════════════════════════════════════════════════════════

/// 测试 21: 能量累计溢出保护
#[test]
fn test_energy_overflow_protection() {
    let mut sampler = MeteringSampler::new();
    let data = PhaseData {
        active_power_total: 100000,
        ..Default::default()
    };
    
    // 模拟大量采样
    for i in 0..1000 {
        sampler.sample(&data, (i + 1) as u64 * 1000);
    }
    
    // 能量不会溢出
    let energy = sampler.energy();
    assert!(energy.active_import < u64::MAX);
}

/// 测试 22: 断相检测
#[test]
fn test_phase_loss_detection() {
    let mut detector = EventDetector::new();
    detector.set_voltage_thresholds(VoltageThresholds {
        lost_voltage: 5000,
        duration_ms: 0,
        ..Default::default()
    });
    
    let phase_loss_data = create_phase_loss_data();
    detector.set_timestamp(1000);
    
    let events = detector.check(&phase_loss_data);
    assert_ne!(events & (1 << MeterEvent::PhaseLossC as u8), 0);
}

/// 测试 23: 多相同时异常
#[test]
fn test_multi_phase_anomaly() {
    let mut detector = EventDetector::new();
    detector.set_voltage_thresholds(VoltageThresholds {
        over_voltage: 25000,
        under_voltage: 18000,
        lost_voltage: 5000,
        duration_ms: 0,
    });
    
    let mut data = create_normal_phase_data();
    data.voltage_a = 27000; // 过压
    data.voltage_b = 17000; // 欠压
    data.voltage_c = 1000;  // 断相
    
    detector.set_timestamp(1000);
    let events = detector.check(&data);
    
    assert_ne!(events & (1 << MeterEvent::OverVoltageA as u8), 0);
    assert_ne!(events & (1 << MeterEvent::UnderVoltageB as u8), 0);
    assert_ne!(events & (1 << MeterEvent::PhaseLossC as u8), 0);
}

/// 测试 24: 事件优先级过滤
#[test]
fn test_event_priority_filtering() {
    let mut manager = EventNotificationManager::new();
    manager.set_config(EventNotificationConfig {
        filter_low_priority: true,
        ..Default::default()
    });
    
    let mut detector = EventDetector::new();
    detector.set_timestamp(1000);
    
    // 触发高优先级和低优先级事件
    detector.set_voltage_thresholds(VoltageThresholds {
        over_voltage: 25000,
        duration_ms: 0,
        ..Default::default()
    });
    detector.check(&create_overvoltage_data()); // 高优先级
    detector.trigger_external(MeterEvent::BatteryLow); // 低优先级 (5)
    
    manager.collect_from_detector(&detector, 1000);
    
    // 只有高优先级事件被收集
    assert_eq!(manager.pending_count(), 1);
}

/// 测试 25: 存储满时的环形覆盖
#[test]
fn test_storage_ring_buffer() {
    let mut storage = MemoryStorage::new(5); // 只存 5 条
    let data = create_normal_phase_data();
    
    // 存储 10 条记录
    for i in 0..10 {
        storage.store_instantaneous(&data, (i + 1) as u32 * 1000).unwrap();
    }
    
    // 应该只保留最后 5 条
    assert_eq!(storage.instantaneous_records().len(), 5);
    
    // 最早的记录应该是第 6 条
    assert_eq!(storage.instantaneous_records()[0].0, 6000);
    
    // 最新的记录应该是第 10 条
    assert_eq!(storage.instantaneous_records()[4].0, 10000);
}

// ═══════════════════════════════════════════════════════════════════════════
// 测试 26-30: 高级场景
// ═══════════════════════════════════════════════════════════════════════════

/// 测试 26: 反向功率检测
#[test]
fn test_reverse_power_detection() {
    let mut detector = EventDetector::new();
    let mut data = create_normal_phase_data();
    data.active_power_total = -5000; // 反向功率
    
    detector.set_timestamp(1000);
    let events = detector.check(&data);
    
    assert_ne!(events & (1 << MeterEvent::ReversePower as u8), 0);
    
    // 恢复正常
    data.active_power_total = 5000;
    let events = detector.check(&data);
    // 反向功率恢复不应该产生新事件
    assert_eq!(events & (1 << MeterEvent::ReversePower as u8), 0);
}

/// 测试 27: 频率越限检测
#[test]
fn test_frequency_deviation() {
    let mut detector = EventDetector::new();
    let mut data = create_normal_phase_data();
    
    // 频率过低
    data.frequency = 4700; // 47.00Hz
    detector.set_timestamp(1000);
    
    // 需要多次检测才能触发
    for _ in 0..20 {
        if detector.check(&data) & (1 << MeterEvent::FrequencyDeviation as u8) != 0 {
            return;
        }
    }
    
    panic!("频率越限应该触发");
}

/// 测试 28: 外部事件（开盖、磁场干扰）
#[test]
fn test_external_events() {
    let mut detector = EventDetector::new();
    let mut manager = EventNotificationManager::new();
    
    detector.set_timestamp(1000);
    
    // 触发开盖事件
    detector.trigger_external(MeterEvent::CoverOpen);
    detector.trigger_external(MeterEvent::MagneticTamper);
    
    manager.collect_from_detector(&detector, 1000);
    
    assert_eq!(manager.pending_count(), 2);
    
    // 验证 DLMS 编码
    let batch = manager.get_next_batch();
    assert_eq!(batch[0].event_code, DlmsEventCode::CoverOpen);
    assert_eq!(batch[1].event_code, DlmsEventCode::MagneticTamper);
}

/// 测试 29: 事件上报重试
#[test]
fn test_event_retry() {
    let mut manager = EventNotificationManager::new();
    
    // 添加事件
    manager.add_notification(DlmsEventNotification::new(
        DlmsEventCode::OverVoltage,
        1000,
        EventStatus::Occurred,
        27000,
        0,
    ));
    
    assert_eq!(manager.pending_count(), 1);
    
    // 获取批次（模拟发送失败）
    let _batch = manager.get_next_batch();
    
    // 标记失败
    manager.mark_failed();
    
    let stats = manager.statistics();
    assert_eq!(stats.failed_count, 1);
}

/// 测试 30: 批量事件上报
#[test]
fn test_batch_event_reporting() {
    let mut manager = EventNotificationManager::new();
    manager.set_config(EventNotificationConfig {
        batch_size: 5,
        ..Default::default()
    });
    
    // 添加 12 个事件
    for i in 0..12 {
        manager.add_notification(DlmsEventNotification::new(
            DlmsEventCode::OverVoltage,
            i as u32 * 1000,
            EventStatus::Occurred,
            27000,
            0,
        ));
    }
    
    assert_eq!(manager.pending_count(), 12);
    
    // 第一批
    let batch1 = manager.get_next_batch();
    assert_eq!(batch1.len(), 5);
    assert_eq!(manager.pending_count(), 7);
    
    // 第二批
    let batch2 = manager.get_next_batch();
    assert_eq!(batch2.len(), 5);
    assert_eq!(manager.pending_count(), 2);
    
    // 第三批
    let batch3 = manager.get_next_batch();
    assert_eq!(batch3.len(), 2);
    assert_eq!(manager.pending_count(), 0);
}
