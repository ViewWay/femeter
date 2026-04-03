/* ================================================================== */
/*                                                                    */
/*  lib.rs — FeMeter 核心逻辑 (host 可测试)                            */
/*                                                                    */
/*  从固件源码中提取的纯逻辑模块，无硬件依赖。                           */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

// ── 公共类型 (与固件 hal.rs 一致) ──

/// 三相电数据 (0.01V / 0.01A / 0.01W / 0.01Hz / 0.001)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PhaseData {
    pub voltage_a: u16,
    pub voltage_b: u16,
    pub voltage_c: u16,
    pub current_a: u16,
    pub current_b: u16,
    pub current_c: u16,
    pub active_power_total: i32,
    pub reactive_power_total: i32,
    pub apparent_power_total: i32,
    pub frequency: u16,
    pub power_factor_total: u16,
    pub active_power_a: i32,
    pub active_power_b: i32,
    pub active_power_c: i32,
    pub reactive_power_a: i32,
    pub reactive_power_b: i32,
    pub reactive_power_c: i32,
    pub voltage_angle_a: u16,
    pub voltage_angle_b: u16,
    pub voltage_angle_c: u16,
}

/// 能量累计数据 (0.001Wh / 0.001varh)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct EnergyData {
    pub active_import: u64,
    pub active_export: u64,
    pub reactive_import: u64,
    pub reactive_export: u64,
    pub active_import_a: u64,
    pub active_import_b: u64,
    pub active_import_c: u64,
}

/// 校准参数
#[derive(Clone, Copy, Debug, Default)]
pub struct CalibrationParams {
    pub voltage_gain_a: f32,
    pub voltage_gain_b: f32,
    pub voltage_gain_c: f32,
    pub current_gain_a: f32,
    pub current_gain_b: f32,
    pub current_gain_c: f32,
    pub phase_offset_a: f32,
    pub phase_offset_b: f32,
    pub phase_offset_c: f32,
    pub power_offset_a: f32,
    pub power_offset_b: f32,
    pub power_offset_c: f32,
}

pub mod event_detect;
pub mod load_forecast;
pub mod ota;
pub mod power_quality;
pub mod tamper_detection;

/* ================================================================== */
/*  单元测试                                                           */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    /* ── PhaseData ── */

    #[test]
    fn test_phase_data_default() {
        let d = PhaseData::default();
        assert_eq!(d.voltage_a, 0);
        assert_eq!(d.active_power_total, 0);
        assert_eq!(d.frequency, 0);
    }

    #[test]
    fn test_phase_data_clone_copy() {
        let d = PhaseData {
            voltage_a: 22000,
            current_a: 10000,
            active_power_total: -500,
            frequency: 5000,
            ..Default::default()
        };
        let d2 = d;
        assert_eq!(d2.voltage_a, 22000);
        assert_eq!(d2.active_power_total, -500);
    }

    #[test]
    fn test_phase_data_boundary_u16_max() {
        let d = PhaseData {
            voltage_a: u16::MAX,
            voltage_b: u16::MAX,
            voltage_c: u16::MAX,
            current_a: u16::MAX,
            current_b: u16::MAX,
            current_c: u16::MAX,
            frequency: u16::MAX,
            power_factor_total: u16::MAX,
            voltage_angle_a: u16::MAX,
            voltage_angle_b: u16::MAX,
            voltage_angle_c: u16::MAX,
            ..Default::default()
        };
        assert_eq!(d.voltage_a, 65535);
        assert_eq!(d.frequency, 65535);
    }

    #[test]
    fn test_phase_data_boundary_i32_min_max() {
        let d = PhaseData {
            active_power_total: i32::MIN,
            reactive_power_total: i32::MAX,
            ..Default::default()
        };
        assert_eq!(d.active_power_total, -2147483648);
        assert_eq!(d.reactive_power_total, 2147483647);
    }

    /* ── EnergyData ── */

    #[test]
    fn test_energy_data_default() {
        let e = EnergyData::default();
        assert_eq!(e.active_import, 0);
    }

    #[test]
    fn test_energy_data_boundary_u64_max() {
        let e = EnergyData {
            active_import: u64::MAX,
            active_export: u64::MAX,
            reactive_import: u64::MAX,
            reactive_export: u64::MAX,
            ..Default::default()
        };
        assert_eq!(e.active_import, 18446744073709551615);
    }

    #[test]
    fn test_energy_data_kwh_conversion() {
        // 0.001Wh 单位 → 1000000 = 1.000 kWh
        let e = EnergyData {
            active_import: 1_000_000,
            ..Default::default()
        };
        assert_eq!(e.active_import, 1_000_000);
    }

    /* ── CalibrationParams ── */

    #[test]
    fn test_calibration_default() {
        let c = CalibrationParams::default();
        assert_eq!(c.voltage_gain_a, 0.0);
    }

    #[test]
    fn test_calibration_custom_values() {
        let c = CalibrationParams {
            voltage_gain_a: 1.234,
            voltage_gain_b: 2.345,
            voltage_gain_c: 3.456,
            current_gain_a: 0.987,
            current_gain_b: 0.876,
            current_gain_c: 0.765,
            phase_offset_a: 0.1,
            phase_offset_b: 0.2,
            phase_offset_c: 0.3,
            power_offset_a: -1.0,
            power_offset_b: -2.0,
            power_offset_c: -3.0,
        };
        assert!((c.voltage_gain_a - 1.234).abs() < f32::EPSILON);
        assert!((c.power_offset_c - (-3.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calibration_f32_special_values() {
        let c = CalibrationParams {
            voltage_gain_a: f32::INFINITY,
            voltage_gain_b: f32::NEG_INFINITY,
            voltage_gain_c: f32::NAN,
            ..Default::default()
        };
        assert!(c.voltage_gain_a.is_infinite());
        assert!(c.voltage_gain_c.is_nan());
    }

    /* ── 跨模块集成 ── */

    #[test]
    fn test_event_detector_with_core_types() {
        let mut det = event_detect::EventDetector::new();
        let data = PhaseData {
            voltage_a: 22000,
            voltage_b: 22000,
            voltage_c: 22000,
            current_a: 5000,
            current_b: 5000,
            current_c: 5000,
            frequency: 5000,
            ..Default::default()
        };
        assert_eq!(det.check(&data), 0, "normal data should produce no events");
    }
}
