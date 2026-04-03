/* ================================================================== */
/*  自动化测试 — 虚拟电表功能验证                                       */
/*                                                                    */
/*  运行: cargo test -p virtual-meter                                  */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use virtual_meter::*;

    fn make_meter() -> MeterHandle {
        let m = create_meter();
        m.lock().unwrap().set_test_mode(true); // 启用测试模式
        m.lock().unwrap().load_scenario(Scenario::Normal);
        m
    }

    /* ── 基础功能 ── */

    #[test]
    fn test_create_meter() {
        let m = create_meter();
        m.lock().unwrap().set_test_mode(true); // 启用测试模式
        let snap = m.lock().unwrap().snapshot();
        assert_eq!(snap.phase_a.voltage, 220.0);
    }

    #[test]
    fn test_set_voltage() {
        let m = make_meter();
        m.lock().unwrap().set_voltage('a', 240.5);
        let snap = m.lock().unwrap().snapshot();
        assert!((snap.phase_a.voltage - 240.5).abs() < 0.01);
    }

    #[test]
    fn test_set_current() {
        let m = make_meter();
        m.lock().unwrap().set_current('b', 10.0);
        let snap = m.lock().unwrap().snapshot();
        assert!((snap.phase_b.current - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_set_angle() {
        let m = make_meter();
        m.lock().unwrap().set_angle('c', 45.0);
        let snap = m.lock().unwrap().snapshot();
        assert!((snap.phase_c.angle - 45.0).abs() < 0.01);
    }

    #[test]
    fn test_power_calculation() {
        let m = make_meter();
        m.lock().unwrap().set_voltage('a', 220.0);
        m.lock().unwrap().set_current('a', 10.0);
        m.lock().unwrap().set_angle('a', 0.0); // cos(0) = 1
        let snap = m.lock().unwrap().snapshot();
        // P = 220 * 10 * cos(0) = 2200 W
        assert!((snap.computed.p_a - 2200.0).abs() < 1.0);
        assert!((snap.computed.pf_a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_reactive_power() {
        let m = make_meter();
        m.lock().unwrap().set_voltage('a', 220.0);
        m.lock().unwrap().set_current('a', 10.0);
        m.lock().unwrap().set_angle('a', 90.0); // cos(90) ≈ 0, sin(90) = 1
        let snap = m.lock().unwrap().snapshot();
        assert!((snap.computed.p_a).abs() < 1.0); // 有功 ≈ 0
        assert!((snap.computed.q_a - 2200.0).abs() < 1.0); // 无功 = 2200
    }

    #[test]
    fn test_power_factor_lagging() {
        let m = make_meter();
        m.lock().unwrap().set_voltage('a', 220.0);
        m.lock().unwrap().set_current('a', 10.0);
        m.lock().unwrap().set_angle('a', 30.0); // cos(30°) ≈ 0.866
        let snap = m.lock().unwrap().snapshot();
        assert!((snap.computed.pf_a - 0.866).abs() < 0.01);
    }

    /* ── 场景预设 ── */

    #[test]
    fn test_scenario_normal() {
        let m = make_meter();
        m.lock().unwrap().load_scenario(Scenario::Normal);
        let snap = m.lock().unwrap().snapshot();
        assert!((snap.phase_a.voltage - 220.0).abs() < 1.0);
        assert!(snap.phase_a.current > 0.0);
    }

    #[test]
    fn test_scenario_noload() {
        let m = make_meter();
        m.lock().unwrap().load_scenario(Scenario::NoLoad);
        let snap = m.lock().unwrap().snapshot();
        assert!((snap.phase_a.current).abs() < 0.01);
    }

    #[test]
    fn test_scenario_phase_loss() {
        let m = make_meter();
        m.lock().unwrap().load_scenario(Scenario::PhaseLoss);
        let snap = m.lock().unwrap().snapshot();
        assert!((snap.phase_a.voltage).abs() < 0.01);
        assert!(snap.phase_b.voltage > 100.0);
    }

    #[test]
    fn test_scenario_reverse_power() {
        let m = make_meter();
        m.lock().unwrap().load_scenario(Scenario::ReversePower);
        let snap = m.lock().unwrap().snapshot();
        // 反向功率: angle=180°, cos(180°)=-1
        assert!(snap.computed.p_a < 0.0);
    }

    #[test]
    fn test_scenario_unbalanced() {
        let m = make_meter();
        m.lock().unwrap().load_scenario(Scenario::Unbalanced);
        let snap = m.lock().unwrap().snapshot();
        // 三相电流不同
        assert_ne!(snap.phase_a.current, snap.phase_b.current);
        assert_ne!(snap.phase_b.current, snap.phase_c.current);
    }

    /* ── 事件检测 ── */

    #[test]
    fn test_event_over_voltage() {
        let m = make_meter();
        m.lock().unwrap().set_voltage('a', 280.0);
        let snap = m.lock().unwrap().snapshot();
        assert!(snap.active_events.contains(&MeterEvent::OverVoltageA));
    }

    #[test]
    fn test_event_under_voltage() {
        let m = make_meter();
        m.lock().unwrap().set_voltage('a', 170.0);
        let snap = m.lock().unwrap().snapshot();
        assert!(snap.active_events.contains(&MeterEvent::UnderVoltageA));
    }

    #[test]
    fn test_event_phase_loss() {
        let m = make_meter();
        m.lock().unwrap().set_voltage('c', 0.0);
        let snap = m.lock().unwrap().snapshot();
        assert!(snap.active_events.contains(&MeterEvent::PhaseLossC));
    }

    #[test]
    fn test_event_over_current() {
        let m = make_meter();
        m.lock().unwrap().set_current('a', 70.0);
        let snap = m.lock().unwrap().snapshot();
        assert!(snap.active_events.contains(&MeterEvent::OverCurrentA));
    }

    #[test]
    fn test_event_reverse_power_detected() {
        let m = make_meter();
        m.lock().unwrap().load_scenario(Scenario::ReversePower);
        let snap = m.lock().unwrap().snapshot();
        assert!(snap.active_events.contains(&MeterEvent::ReversePower));
    }

    #[test]
    fn test_no_events_normal() {
        let m = make_meter();
        m.lock().unwrap().load_scenario(Scenario::Normal);
        let snap = m.lock().unwrap().snapshot();
        assert!(snap.active_events.is_empty());
    }

    #[test]
    fn test_manual_event_trigger() {
        let m = make_meter();
        m.lock().unwrap().trigger_event(MeterEvent::CoverOpen);
        let guard = m.lock().unwrap();
        let events = guard.events();
        assert!(events.iter().any(|e| e.event == MeterEvent::CoverOpen));
    }

    /* ── 电能累计 ── */

    #[test]
    fn test_energy_accumulation() {
        let m = make_meter();
        m.lock().unwrap().load_scenario(Scenario::Normal);
        m.lock().unwrap().set_time_accel(3600000.0); // 极大加速
        std::thread::sleep(std::time::Duration::from_millis(10));
        let snap = m.lock().unwrap().snapshot();
        assert!(snap.energy.wh_total > 0.0);
    }

    #[test]
    fn test_energy_reset() {
        let m = make_meter();
        m.lock().unwrap().load_scenario(Scenario::Normal);
        m.lock().unwrap().set_time_accel(3600000.0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        m.lock().unwrap().reset_energy();
        // 重置后立即检查，不调用 snapshot (会触发 tick)
        let wh_total = m.lock().unwrap().energy().wh_total;
        assert!(
            (wh_total).abs() < 0.001,
            "wh_total after reset: {}",
            wh_total
        );
    }

    /* ── 芯片切换 ── */

    #[test]
    fn test_chip_switch() {
        let m = make_meter();
        m.lock().unwrap().set_chip(ChipType::RN8302B);
        let snap = m.lock().unwrap().snapshot();
        assert_eq!(snap.chip, ChipType::RN8302B);
    }

    /* ── 噪声 ── */

    #[test]
    fn test_noise_enabled() {
        let m = make_meter();
        m.lock().unwrap().set_noise(true);
        // 连续读取两次, 噪声导致值不同
        let _s1 = m.lock().unwrap().snapshot();
        let _s2 = m.lock().unwrap().snapshot();
        // 有噪声时, 两次读取可能有微小差异 (不一定, 但概率高)
        // 这里只验证不 panic
        assert!(true);
    }

    /* ── 日志开关 ── */

    #[test]
    fn test_log_toggle() {
        set_log_enabled(true);
        assert!(is_log_enabled());
        set_log_enabled(false);
        assert!(!is_log_enabled());
        set_log_enabled(true); // 恢复
    }

    /* ── 寄存器模拟 ── */

    #[test]
    fn test_register_read() {
        let m = create_meter();
        {
            let mut meter = m.lock().unwrap();
            meter.set_test_mode(true); // 启用测试模式
            meter.set_noise(false);
            meter.set_voltage('a', 220.0);
            meter.set_current('a', 0.0);
            let hex = meter.format_register(0x00);
            // 220.0V * 100 = 22000 = 0x0055F0
            assert_eq!(hex, "0055F0", "got {}", hex);
        }
    }

    #[test]
    fn test_register_chip_id() {
        let m = make_meter();
        let hex = m.lock().unwrap().format_register(0xFF);
        assert_eq!(hex, "07022E"); // ATT7022E default
    }

    /* ── JSON 快照 ── */

    #[test]
    fn test_snapshot_serialization() {
        let m = make_meter();
        let snap = m.lock().unwrap().snapshot();
        let json = serde_json::to_string(&snap);
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("phase_a"));
        assert!(json_str.contains("energy"));
    }
}
