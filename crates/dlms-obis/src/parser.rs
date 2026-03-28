//! OBIS code parser
//!
//! Parse "A.B.C.D.E.F" string format

use dlms_core::ObisCode;

/// Parse OBIS code from "A.B.C.D.E.F" format
pub fn parse_obis(s: &str) -> Option<ObisCode> {
    use alloc::vec::Vec;
    let parts: Vec<u8> = s.split('.')
        .map(|p| p.parse::<u8>().ok())
        .collect::<Option<Vec<_>>>()?;
    if parts.len() != 6 { return None; }
    Some(ObisCode::new(parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]))
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
}
