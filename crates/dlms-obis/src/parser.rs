//! OBIS code parser
//!
//! Parse "A.B.C.D.E.F" string format

use dlms_core::ObisCode;

/// Parse OBIS code from "A.B.C.D.E.F" format
pub fn parse_obis(s: &str) -> Option<ObisCode> {
    use alloc::vec::Vec;
    let parts: Vec<u8> = s
        .split('.')
        .map(|p| p.parse::<u8>().ok())
        .collect::<Option<Vec<_>>>()?;
    if parts.len() != 6 {
        return None;
    }
    Some(ObisCode::new(
        parts[0], parts[1], parts[2], parts[3], parts[4], parts[5],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid() {
        let code = parse_obis("1.0.1.8.0.255").unwrap();
        assert_eq!(code, ObisCode::new(1, 0, 1, 8, 0, 255));
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_obis("invalid").is_none());
        assert!(parse_obis("1.2.3").is_none());
        assert!(parse_obis("1.2.3.4.5.6.7").is_none());
    }

    #[test]
    fn test_parse_boundary() {
        let code = parse_obis("255.255.255.255.255.255").unwrap();
        assert_eq!(code, ObisCode::new(255, 255, 255, 255, 255, 255));
    }

    // ============================================================
    // Phase C — Boundary Tests
    // ============================================================

    #[test]
    fn test_all_zeros() {
        let code = parse_obis("0.0.0.0.0.0").unwrap();
        assert_eq!(code, ObisCode::new(0, 0, 0, 0, 0, 0));
    }

    #[test]
    fn test_all_255() {
        let code = parse_obis("255.255.255.255.255.255").unwrap();
        assert_eq!(code.to_bytes(), [255u8; 6]);
    }

    #[test]
    fn test_gb_obis_extensions() {
        // China GB/T 17215.301 standard OBIS codes
        let gb_codes = [
            "1.0.0.1.0.255",   // 标识
            "0.0.96.1.0.255",   // 逻辑设备名
            "1.0.1.8.0.255",   // 正向有功总电能
            "1.0.2.8.0.255",   // 反向有功总电能
            "1.0.3.8.0.255",   // 正向无功总电能
            "1.0.4.8.0.255",   // 反向无功总电能
            "1.0.31.7.0.255",  // A相电压
            "1.0.51.7.0.255",  // B相电压
            "1.0.71.7.0.255",  // C相电压
            "1.0.32.7.0.255",  // A相电流
            "1.0.52.7.0.255",  // B相电流
            "1.0.72.7.0.255",  // C相电流
            "1.0.14.7.0.255",  // 总有功功率
            "1.0.15.7.0.255",  // 总无功功率
            "1.0.21.7.0.255",  // 总功率因数
            "1.0.56.7.0.255",  // A相功率因数
            "1.0.76.7.0.255",  // B相功率因数
            "1.0.96.7.0.255",  // C相功率因数
            "0.0.1.0.0.255",   // 日期时间
            "0.0.96.10.1.255",  // 时区
        ];
        for code_str in &gb_codes {
            let code = parse_obis(code_str).unwrap();
            assert_eq!(code.to_bytes().len(), 6);
        }
    }

    #[test]
    fn test_single_digit_groups() {
        let code = parse_obis("1.0.1.8.0.0").unwrap();
        assert_eq!(code, ObisCode::new(1, 0, 1, 8, 0, 0));
    }

    #[test]
    fn test_extra_dots() {
        assert!(parse_obis("1.0.1.8.0.255.0").is_none());
    }

    #[test]
    fn test_non_numeric() {
        assert!(parse_obis("a.b.c.d.e.f").is_none());
        assert!(parse_obis("1.0.1.8.0.x").is_none());
    }

    #[test]
    fn test_empty_string() {
        assert!(parse_obis("").is_none());
    }

    #[test]
    fn test_just_dots() {
        assert!(parse_obis("......").is_none());
    }

    #[test]
    fn test_negative_values() {
        assert!(parse_obis("-1.0.1.8.0.255").is_none());
    }

    #[test]
    fn test_values_above_u8() {
        assert!(parse_obis("256.0.1.8.0.255").is_none());
    }

    #[test]
    fn test_whitespace() {
        assert!(parse_obis("1. 0.1.8.0.255").is_none());
    }
}
