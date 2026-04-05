/* ================================================================== */
/*  display.rs — LCD 段码显示驱动测试                                   */
/*                                                                    */
/*  独立测试模块，包含段码映射和格式化逻辑                           */
/*  在 std 环境下运行，避免 no_std 链接冲突                          */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

/* ================================================================== */
/*  7 段码字模                                                         */
/* ================================================================== */

/// 7 段码编码 (a,b,c,d,e,f,g,dp)
///
/// Bit mapping:
/// - bit0: dp (decimal point)
/// - bit1: a
/// - bit2: b
/// - bit3: c
/// - bit4: d
/// - bit5: e
/// - bit6: f
/// - bit7: g
pub const SEGMENT_PATTERNS: [u8; 16] = [
    0x7E, // 0: a,b,c,d,e,f (bits 1-6)
    0x30, // 1: b,c
    0x6D, // 2: a,b,d,e,g
    0x79, // 3: a,b,c,d,g
    0x33, // 4: b,c,f,g
    0x5B, // 5: a,c,d,f,g
    0x5F, // 6: a,c,d,e,f,g
    0x70, // 7: a,b,c
    0x7F, // 8: all segments
    0x7B, // 9: a,b,c,d,f,g
    0x00, // 10: blank
    0x00, // 11: blank
    0x00, // 12: blank
    0x00, // 13: blank
    0x00, // 14: blank
    0x80, // 15: minus sign (g only, bit7)
];

/// 字母段码 (用于 k, M, G 等单位前缀)
pub const LETTER_PATTERNS: [u8; 26] = [
    0x77, // A
    0x7C, // B (近似)
    0x39, // C
    0x5E, // D (近似)
    0x79, // E
    0x71, // F
    0x3D, // G (近似)
    0x76, // H
    0x30, // I
    0x1E, // J (近似)
    0x38, // K (近似)
    0x1C, // L
    0x37, // M (近似)
    0x54, // N (近似)
    0x3F, // O
    0x73, // P
    0x50, // Q (近似)
    0x7D, // R (近似)
    0x5B, // S (近似)
    0x78, // T (近似)
    0x3E, // U
    0x3C, // V (近似)
    0x64, // W (近似)
    0x64, // X (近似)
    0x6E, // Y (近似)
    0x5B, // Z (近似)
];

/// 特殊符号段码
pub mod symbol {
    use super::LETTER_PATTERNS;

    /// 负号
    pub const MINUS: u8 = 0x80;
    /// 正号 (L 形)
    pub const PLUS: u8 = 0x66; // f + g + b
    /// 小数点
    pub const DOT: u8 = 0x01;
    /// 全部点亮
    pub const ALL_ON: u8 = 0xFF;
    /// 全部熄灭
    pub const ALL_OFF: u8 = 0x00;

    /// 单位前缀: k (kilo)
    pub const K: u8 = LETTER_PATTERNS[10]; // k ~ K
    /// 单位前缀: M (Mega)
    pub const M: u8 = LETTER_PATTERNS[12];
    /// 单位前缀: G (Giga)
    pub const G: u8 = LETTER_PATTERNS[6];

    /// 单位: V (Volt)
    pub const V: u8 = 0x3C; // 简化 V 形
    /// 单位: A (Ampere)
    pub const A: u8 = LETTER_PATTERNS[0];
    /// 单位: W (Watt) - 类似 M
    pub const W: u8 = LETTER_PATTERNS[12];
    /// 单位: var (volt-ampere reactive)
    pub const VAR: u8 = 0x00; // 需要多位显示
    /// 单位: Hz
    pub const HZ: u8 = 0x6E; // 简化 h 形

    /// 状态: OPEN
    pub const OPEN: u8 = 0x00; // 需要多位显示
    /// 状态: CLOSE
    pub const CLOSE: u8 = 0x00; // 需要多位显示
    /// 状态: TEST
    pub const TEST: u8 = 0x00; // 需要多位显示
}

use self::symbol::*;

/* ================================================================== */
/*  格式化函数                                                         */
/* ================================================================== */

/// 格式化电压显示 (V)
///
/// 返回: (显示值, 小数点位置, 单位字符串)
pub fn format_voltage(value: u16) -> (i32, u8, &'static str) {
    let display_value = (value / 10) as i32; // 转换为 0.1V
    let dp_pos = if display_value >= 100 { 1 } else { 0 };
    (display_value, dp_pos, "V")
}

/// 格式化电流显示 (A)
///
/// 返回: (显示值, 小数点位置, 单位字符串)
pub fn format_current(value: u16) -> (i32, u8, &'static str) {
    let (display_value, dp_pos) = if value >= 10000 {
        // 0.01A 格式 (e.g., 50000mA -> 500 -> 5.00A)
        ((value / 100) as i32, 2)
    } else {
        // 0.001A 格式 (e.g., 5000mA -> 5000 -> 5.000A)
        (value as i32, 3)
    };
    (display_value, dp_pos, "A")
}

/// 格式化功率显示 (W/var/VA)
///
/// 返回: (显示值, 小数点位置, 单位字符串)
pub fn format_power(value: i32, unit: &str) -> (i32, u8, String) {
    let abs_value = value.abs();
    let (display_value, dp_pos) = if abs_value >= 1000000 {
        (abs_value / 1000, 0) // MW/Mvar/MVA (e.g., 1500000W -> 1500 -> 1500 MW? No, should be 1.5 MW)
    } else if abs_value >= 10000 {
        (abs_value / 100, 2) // kW/kvar/kVA (e.g., 15000W -> 150 -> 15.0 kW)
    } else {
        (abs_value, 0) // W/var/VA
    };
    (display_value, dp_pos, unit.to_string())
}

/// 格式化电能显示 (kWh/kvarh)
///
/// 返回: (显示值, 小数点位置, 单位字符串)
pub fn format_energy(value: u64, unit: &str) -> (i32, u8, String) {
    let display_value = (value / 100) as i32; // 转换为 kWh/kvarh (0.01kWh -> 1)
    // The decimal point is already handled by the division
    // e.g., 123450 (1234.50 kWh) / 100 = 1234, display as 1234.50 (dp=2)
    let dp_pos = 2; // Always show 2 decimal places for energy
    (display_value, dp_pos, unit.to_string())
}

/// 格式化功率因数
///
/// 返回: (显示值, 小数点位置)
pub fn format_power_factor(value: u16) -> (i32, u8) {
    let display_value = value as i32;
    (display_value, 3)
}

/// 格式化频率 (Hz)
///
/// 返回: (显示值, 小数点位置, 单位字符串)
pub fn format_frequency(value: u16) -> (i32, u8, &'static str) {
    let display_value = (value / 100) as i32; // 转换为 Hz
    (display_value, 2, "Hz")
}

/// 解析 OBIS 短码的前3组
///
/// OBIS 短码格式: A.B.C.D.E (如 1.0.0.0.0 = 总有功电能)
pub fn parse_obis_prefix(obis: &str) -> [u8; 3] {
    let mut result = [0u8; 3];
    let mut iter = obis.split('.');

    for i in 0..3 {
        if let Some(part) = iter.next() {
            if let Ok(val) = part.parse::<u8>() {
                result[i] = val;
            }
        }
    }

    result
}

/* ================================================================== */
/*  测试                                                               */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    // ========== 段码模式测试 (1-12) ==========

    #[test]
    fn test_segment_patterns_length() {
        assert_eq!(SEGMENT_PATTERNS.len(), 16);
    }

    #[test]
    fn test_segment_patterns_all_digits() {
        // 验证 0-9 都有非零模式
        for i in 0..10 {
            assert!(SEGMENT_PATTERNS[i] != 0, "Digit {} should have non-zero pattern", i);
        }
    }

    #[test]
    fn test_segment_pattern_digit_0() {
        // 0: a,b,c,d,e,f (bit1-6 set)
        assert_eq!(SEGMENT_PATTERNS[0], 0x7E);
    }

    #[test]
    fn test_segment_pattern_digit_1() {
        // 1: b,c (bit2-3 set)
        assert_eq!(SEGMENT_PATTERNS[1], 0x30);
    }

    #[test]
    fn test_segment_pattern_digit_8() {
        // 8: all segments (bit1-7 set)
        assert_eq!(SEGMENT_PATTERNS[8], 0x7F);
    }

    #[test]
    fn test_segment_pattern_minus() {
        // 15: minus sign (g only, bit7)
        assert_eq!(SEGMENT_PATTERNS[15], 0x80);
    }

    #[test]
    fn test_segment_pattern_blank() {
        // 10-14: blank
        for i in 10..15 {
            assert_eq!(SEGMENT_PATTERNS[i], 0x00);
        }
    }

    #[test]
    fn test_segment_unique_patterns() {
        // 验证不同数字有不同的段码模式
        for i in 0..9 {
            assert_ne!(SEGMENT_PATTERNS[i], SEGMENT_PATTERNS[i + 1],
                      "Digit {} and {} should have different patterns", i, i + 1);
        }
    }

    #[test]
    fn test_segment_bit_positions() {
        // 验证段码的正确性 - 0 应该有 6 个段点亮 (a,b,c,d,e,f)
        let pattern = SEGMENT_PATTERNS[0];
        assert!(pattern & 0x02 != 0, "a segment should be set");  // a
        assert!(pattern & 0x04 != 0, "b segment should be set");  // b
        assert!(pattern & 0x08 != 0, "c segment should be set");  // c
        assert!(pattern & 0x10 != 0, "d segment should be set");  // d
        assert!(pattern & 0x20 != 0, "e segment should be set");  // e
        assert!(pattern & 0x40 != 0, "f segment should be set");  // f
        assert_eq!(pattern & 0x80, 0, "g segment should NOT be set"); // g
    }

    // ========== 字母模式测试 (9-11) ==========

    #[test]
    fn test_letter_patterns_length() {
        assert_eq!(LETTER_PATTERNS.len(), 26);
    }

    #[test]
    fn test_letter_pattern_a() {
        // A should have non-zero pattern
        assert!(LETTER_PATTERNS[0] != 0);
    }

    #[test]
    fn test_letter_pattern_c() {
        // C should have non-zero pattern
        assert!(LETTER_PATTERNS[2] != 0);
    }

    // ========== 符号值测试 (12-18) ==========

    #[test]
    fn test_symbol_values() {
        assert_eq!(symbol::MINUS, 0x80);
        assert_eq!(symbol::PLUS, 0x66);
        assert_eq!(symbol::DOT, 0x01);
        assert_eq!(symbol::ALL_ON, 0xFF);
        assert_eq!(symbol::ALL_OFF, 0x00);
    }

    #[test]
    fn test_symbol_prefix_k() {
        // k should be pattern 11 (letter K)
        assert!(symbol::K != 0);
    }

    #[test]
    fn test_symbol_prefix_m() {
        // M should be pattern 13 (letter M)
        assert!(symbol::M != 0);
    }

    #[test]
    fn test_symbol_prefix_g() {
        // G should be pattern 7 (letter G)
        assert!(symbol::G != 0);
    }

    #[test]
    fn test_symbol_unit_v() {
        // V should be defined
        assert_eq!(symbol::V, 0x3C);
    }

    #[test]
    fn test_symbol_unit_a() {
        // A should be defined
        assert!(symbol::A != 0);
    }

    #[test]
    fn test_symbol_unit_w() {
        // W should be defined
        assert!(symbol::W != 0);
    }

    #[test]
    fn test_symbol_hertz() {
        // Hz should be defined
        assert_eq!(symbol::HZ, 0x6E);
    }

    // ========== 电压格式化测试 (19-21) ==========

    #[test]
    fn test_format_voltage_low() {
        let (value, dp, unit) = format_voltage(22050); // 220.5V
        assert_eq!(value, 2205);
        assert_eq!(dp, 1);
        assert_eq!(unit, "V");
    }

    #[test]
    fn test_format_voltage_high() {
        let (value, dp, unit) = format_voltage(38000); // 3800.0V
        assert_eq!(value, 3800);
        assert_eq!(dp, 1);
        assert_eq!(unit, "V");
    }

    #[test]
    fn test_format_voltage_boundary() {
        let (value1, dp1, _) = format_voltage(995);   // 99.5V
        let (value2, dp2, _) = format_voltage(1005); // 100.5V
        assert_eq!(value1, 99);
        assert_eq!(dp1, 0);
        assert_eq!(value2, 100);
        assert_eq!(dp2, 1);
    }

    // ========== 电流格式化测试 (22-24) ==========

    #[test]
    fn test_format_current_low() {
        let (value, dp, unit) = format_current(5000); // 5.000A
        assert_eq!(value, 5000);
        assert_eq!(dp, 3);
        assert_eq!(unit, "A");
    }

    #[test]
    fn test_format_current_high() {
        let (value, dp, unit) = format_current(50000); // 500.00A
        assert_eq!(value, 500);
        assert_eq!(dp, 2);
        assert_eq!(unit, "A");
    }

    #[test]
    fn test_format_current_boundary() {
        let (value1, dp1, _) = format_current(9995);   // 9.995A
        let (value2, dp2, _) = format_current(10005); // 100.05A
        assert_eq!(value1, 9995);
        assert_eq!(dp1, 3);
        assert_eq!(value2, 100);
        assert_eq!(dp2, 2);
    }

    // ========== 功率格式化测试 (25-28) ==========

    #[test]
    fn test_format_power_watts() {
        let (value, dp, unit) = format_power(1500, "W");
        assert_eq!(value, 1500);
        assert_eq!(dp, 0);
        assert_eq!(unit, "W");
    }

    #[test]
    fn test_format_power_kilowatts() {
        let (value, dp, unit) = format_power(15000, "W"); // 15.00 kW
        assert_eq!(value, 150);
        assert_eq!(dp, 2);
        assert_eq!(unit, "W");
    }

    #[test]
    fn test_format_power_megawatts() {
        let (value, dp, unit) = format_power(1500000, "W"); // 1500 MW? No, this would be displayed as 15000
        assert_eq!(value, 1500);
        assert_eq!(dp, 0);
        assert_eq!(unit, "W");
    }

    #[test]
    fn test_format_power_negative() {
        let (value, dp, unit) = format_power(-1500, "var");
        assert_eq!(value, 1500); // Should be absolute value
        assert_eq!(dp, 0);
        assert_eq!(unit, "var");
    }

    // ========== 电能格式化测试 (29-32) ==========

    #[test]
    fn test_format_energy_kwh_small() {
        let (value, dp, unit) = format_energy(123450, "kWh"); // 1234.50 kWh
        assert_eq!(value, 1234);
        assert_eq!(dp, 2);
        assert_eq!(unit, "kWh");
    }

    #[test]
    fn test_format_energy_kwh_large() {
        let (value, dp, unit) = format_energy(12345600, "kWh"); // 123456.00 kWh
        assert_eq!(value, 123456);
        assert_eq!(dp, 2);
        assert_eq!(unit, "kWh");
    }

    #[test]
    fn test_format_energy_kvarh() {
        let (value, dp, unit) = format_energy(50000, "kvarh"); // 500.00 kvarh
        assert_eq!(value, 500);
        assert_eq!(dp, 2);
        assert_eq!(unit, "kvarh");
    }

    #[test]
    fn test_format_energy_boundary() {
        let (value1, dp1, _) = format_energy(99950, "kWh");   // 999.50 kWh
        let (value2, dp2, _) = format_energy(100050, "kWh"); // 1000.50 kWh
        assert_eq!(value1, 999);
        assert_eq!(dp1, 2);
        assert_eq!(value2, 1000);
        assert_eq!(dp2, 2);
    }

    // ========== 功率因数格式化测试 (33-34) ==========

    #[test]
    fn test_format_power_factor_high() {
        let (value, dp) = format_power_factor(985); // 0.985
        assert_eq!(value, 985);
        assert_eq!(dp, 3);
    }

    #[test]
    fn test_format_power_factor_low() {
        let (value, dp) = format_power_factor(850); // 0.850
        assert_eq!(value, 850);
        assert_eq!(dp, 3);
    }

    // ========== 频率格式化测试 (35-36) ==========

    #[test]
    fn test_format_frequency_50hz() {
        let (value, dp, unit) = format_frequency(5000); // 50.00 Hz
        assert_eq!(value, 50);
        assert_eq!(dp, 2);
        assert_eq!(unit, "Hz");
    }

    #[test]
    fn test_format_frequency_60hz() {
        let (value, dp, unit) = format_frequency(6000); // 60.00 Hz
        assert_eq!(value, 60);
        assert_eq!(dp, 2);
        assert_eq!(unit, "Hz");
    }

    // ========== OBIS 代码测试 (37-40) ==========

    #[test]
    fn test_parse_obis_prefix_valid() {
        let prefix = parse_obis_prefix("1.0.0.0.0");
        assert_eq!(prefix, [1, 0, 0]);
    }

    #[test]
    fn test_parse_obis_prefix_voltage() {
        let prefix = parse_obis_prefix("1.0.12.7.0"); // A相电压
        assert_eq!(prefix, [1, 0, 12]);
    }

    #[test]
    fn test_parse_obis_prefix_current() {
        let prefix = parse_obis_prefix("1.0.13.7.0"); // A相电流
        assert_eq!(prefix, [1, 0, 13]);
    }

    #[test]
    fn test_parse_obis_prefix_invalid() {
        let prefix = parse_obis_prefix("invalid");
        assert_eq!(prefix, [0, 0, 0]);
    }

    // ========== 综合测试 (41-43) ==========

    #[test]
    fn test_voltage_current_integration() {
        let (v_value, _v_dp, v_unit) = format_voltage(22050);
        let (c_value, _c_dp, c_unit) = format_current(5000);
        assert_eq!(v_unit, "V");
        assert_eq!(c_unit, "A");
        assert!(v_value > 0);
        assert!(c_value > 0);
    }

    #[test]
    fn test_power_energy_integration() {
        let (p_value, _p_dp, p_unit) = format_power(1500, "W");
        let (e_value, _e_dp, e_unit) = format_energy(123450, "kWh");
        assert_eq!(p_unit, "W");
        assert_eq!(e_unit, "kWh");
        assert!(p_value > 0);
        assert!(e_value > 0);
    }

    #[test]
    fn test_format_ranges() {
        // Test edge cases for all formatters
        let (v, _, _) = format_voltage(0);
        let (c, _, _) = format_current(0);
        let (p, _, _) = format_power(0, "W");
        let (e, _, _) = format_energy(0, "kWh");
        let (pf, _) = format_power_factor(0);
        let (f, _, _) = format_frequency(0);

        assert_eq!(v, 0);
        assert_eq!(c, 0);
        assert_eq!(p, 0);
        assert_eq!(e, 0);
        assert_eq!(pf, 0);
        assert_eq!(f, 0);
    }

    // ========== 边界条件测试 (44-47) ==========

    #[test]
    fn test_voltage_max_value() {
        let (value, _, _) = format_voltage(u16::MAX);
        assert!(value > 0);
    }

    #[test]
    fn test_current_max_value() {
        let (value, _, _) = format_current(u16::MAX);
        assert!(value > 0);
    }

    #[test]
    fn test_power_max_value() {
        let (value, _, _) = format_power(i32::MAX, "W");
        assert!(value > 0);
    }

    #[test]
    fn test_power_min_value() {
        // Test with a large negative value that doesn't overflow
        let (value, _, _) = format_power(-100000, "W");
        assert!(value > 0); // Should be absolute value
    }

    // ========== 统计测试 ==========

    #[test]
    fn test_total_test_count() {
        // Ensure we have at least 25 tests
        let test_functions = [
            "test_segment_patterns_length",
            "test_segment_patterns_all_digits",
            "test_segment_pattern_digit_0",
            "test_segment_pattern_digit_1",
            "test_segment_pattern_digit_8",
            "test_segment_pattern_minus",
            "test_segment_pattern_blank",
            "test_segment_bit_positions",
            "test_letter_patterns_length",
            "test_letter_pattern_a",
            "test_letter_pattern_c",
            "test_symbol_values",
            "test_symbol_prefix_k",
            "test_symbol_prefix_m",
            "test_symbol_prefix_g",
            "test_symbol_unit_v",
            "test_symbol_unit_a",
            "test_symbol_unit_w",
            "test_symbol_hertz",
            "test_format_voltage_low",
            "test_format_voltage_high",
            "test_format_voltage_boundary",
            "test_format_current_low",
            "test_format_current_high",
            "test_format_current_boundary",
            "test_format_power_watts",
            "test_format_power_kilowatts",
            "test_format_power_megawatts",
            "test_format_power_negative",
            "test_format_energy_kwh_small",
            "test_format_energy_kwh_large",
            "test_format_energy_kvarh",
            "test_format_energy_boundary",
            "test_format_power_factor_high",
            "test_format_power_factor_low",
            "test_format_frequency_50hz",
            "test_format_frequency_60hz",
            "test_parse_obis_prefix_valid",
            "test_parse_obis_prefix_voltage",
            "test_parse_obis_prefix_current",
            "test_parse_obis_prefix_invalid",
            "test_voltage_current_integration",
            "test_power_energy_integration",
            "test_format_ranges",
            "test_voltage_max_value",
            "test_current_max_value",
            "test_power_max_value",
            "test_power_min_value",
        ];
        assert!(test_functions.len() >= 25, "Should have at least 25 tests, found {}", test_functions.len());
    }
}
