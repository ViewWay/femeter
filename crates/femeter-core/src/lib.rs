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
pub mod ota;
