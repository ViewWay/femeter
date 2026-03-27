//! OBIS (Object Identification System) code definition
//!
//! Reference: Blue Book Part 1 (DLMS UA 1000-1 Ed.16)
//! OBIS code = A-B-C-D-E-F six value groups

use core::fmt;

/// OBIS code: six value groups A-B-C-D-E-F
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct ObisCode {
    pub a: u8, // Medium type (0-15)
    pub b: u8, // Channel number (0-255)
    pub c: u8, // Abstract/physical data item
    pub d: u8, // Processing type / algorithm
    pub e: u8, // Further classification
    pub f: u8, // Historical value / billing period (255=not used)
}

impl ObisCode {
    pub const fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// Parse from "A.B.C.D.E.F" string
    #[cfg(feature = "std")]
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<u8> = s.split('.')
            .map(|p| p.parse::<u8>().ok())
            .collect::<Option<Vec<_>>>()?;
        if parts.len() != 6 { return None; }
        Some(Self::new(parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]))
    }

    /// To byte representation (6 bytes)
    pub const fn to_bytes(&self) -> [u8; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }

    /// From byte representation
    pub const fn from_bytes(b: &[u8; 6]) -> Self {
        Self::new(b[0], b[1], b[2], b[3], b[4], b[5])
    }

    /// Medium type description
    pub const fn medium(&self) -> &'static str {
        match self.a {
            0 => "Abstract",
            1 => "AC Electricity",
            2 => "DC Electricity",
            4 => "Heat Cost Allocator",
            5 | 6 => "Thermal Energy",
            7 => "Gas",
            8 => "Cold Water",
            9 => "Hot Water",
            _ => "Other",
        }
    }
}

impl fmt::Display for ObisCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}.{}.{}", self.a, self.b, self.c, self.d, self.e, self.f)
    }
}

// ============================================================
// Standard OBIS codes (AC Electricity, A=1)
// ============================================================

// --- Energy (C=1, D=8) ---
pub const TOTAL_ACTIVE_ENERGY_IMPORT: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 255);
pub const TOTAL_ACTIVE_ENERGY_EXPORT: ObisCode = ObisCode::new(1, 0, 2, 8, 0, 255);
pub const TOTAL_REACTIVE_ENERGY_IMPORT: ObisCode = ObisCode::new(1, 0, 3, 8, 0, 255);
pub const TOTAL_REACTIVE_ENERGY_EXPORT: ObisCode = ObisCode::new(1, 0, 4, 8, 0, 255);

// --- Active energy by tariff (E=0..4) ---
pub const ACTIVE_ENERGY_IMPORT_TARIFF1: ObisCode = ObisCode::new(1, 0, 1, 8, 1, 255);
pub const ACTIVE_ENERGY_IMPORT_TARIFF2: ObisCode = ObisCode::new(1, 0, 1, 8, 2, 255);
pub const ACTIVE_ENERGY_IMPORT_TARIFF3: ObisCode = ObisCode::new(1, 0, 1, 8, 3, 255);
pub const ACTIVE_ENERGY_IMPORT_TARIFF4: ObisCode = ObisCode::new(1, 0, 1, 8, 4, 255);

// --- Voltage (C=32) ---
pub const VOLTAGE_L1: ObisCode = ObisCode::new(1, 0, 32, 7, 0, 255);
pub const VOLTAGE_L2: ObisCode = ObisCode::new(1, 0, 52, 7, 0, 255);
pub const VOLTAGE_L3: ObisCode = ObisCode::new(1, 0, 72, 7, 0, 255);

// --- Current (C=31) ---
pub const CURRENT_L1: ObisCode = ObisCode::new(1, 0, 31, 7, 0, 255);
pub const CURRENT_L2: ObisCode = ObisCode::new(1, 0, 51, 7, 0, 255);
pub const CURRENT_L3: ObisCode = ObisCode::new(1, 0, 71, 7, 0, 255);

// --- Power (C=1..4, D=7) ---
pub const ACTIVE_POWER: ObisCode = ObisCode::new(1, 0, 1, 7, 0, 255);
pub const ACTIVE_POWER_L1: ObisCode = ObisCode::new(1, 0, 21, 7, 0, 255);
pub const ACTIVE_POWER_L2: ObisCode = ObisCode::new(1, 0, 41, 7, 0, 255);
pub const ACTIVE_POWER_L3: ObisCode = ObisCode::new(1, 0, 61, 7, 0, 255);
pub const REACTIVE_POWER: ObisCode = ObisCode::new(1, 0, 3, 7, 0, 255);
pub const APPARENT_POWER: ObisCode = ObisCode::new(1, 0, 9, 7, 0, 255);

// --- Power factor (C=13) ---
pub const POWER_FACTOR: ObisCode = ObisCode::new(1, 0, 13, 7, 0, 255);
pub const POWER_FACTOR_L1: ObisCode = ObisCode::new(1, 0, 33, 7, 0, 255);

// --- Frequency (C=14) ---
pub const FREQUENCY: ObisCode = ObisCode::new(1, 0, 14, 7, 0, 255);

// --- Demand (C=1, D=6) ---
pub const ACTIVE_DEMAND: ObisCode = ObisCode::new(1, 0, 1, 6, 0, 255);
pub const MAX_ACTIVE_DEMAND: ObisCode = ObisCode::new(1, 0, 1, 6, 5, 255);

// --- Profile / Load profile ---
pub const LOAD_PROFILE: ObisCode = ObisCode::new(1, 0, 99, 1, 0, 255);
pub const EVENT_LOG: ObisCode = ObisCode::new(0, 0, 99, 98, 0, 255);

// --- Clock ---
pub const CLOCK: ObisCode = ObisCode::new(0, 0, 1, 0, 0, 255);

// --- Association LN ---
pub const ASSOCIATION_LN: ObisCode = ObisCode::new(0, 0, 40, 0, 1, 255);
pub const ASSOCIATION_SN: ObisCode = ObisCode::new(0, 0, 40, 0, 2, 255);

// --- Security setup ---
pub const SECURITY_SETUP: ObisCode = ObisCode::new(0, 0, 43, 0, 1, 255);

// --- Disconnect control ---
pub const DISCONNECT_CONTROL: ObisCode = ObisCode::new(0, 0, 96, 3, 10, 255);

// --- Push setup ---
pub const PUSH_SETUP: ObisCode = ObisCode::new(0, 0, 96, 10, 0, 255);

// --- SAP Assignment ---
pub const SAP_ASSIGNMENT: ObisCode = ObisCode::new(0, 0, 41, 0, 0, 255);

// --- Image transfer ---
pub const IMAGE_TRANSFER: ObisCode = ObisCode::new(0, 0, 44, 0, 0, 255);

// --- Activity calendar ---
pub const ACTIVITY_CALENDAR: ObisCode = ObisCode::new(0, 0, 98, 0, 2, 255);

// --- Script table ---
pub const SCRIPT_TABLE: ObisCode = ObisCode::new(0, 0, 10, 0, 0, 255);

// --- Schedule ---
pub const SCHEDULE: ObisCode = ObisCode::new(0, 0, 11, 0, 0, 255);

// --- Special days table ---
pub const SPECIAL_DAYS_TABLE: ObisCode = ObisCode::new(0, 0, 11, 0, 1, 255);

// --- Register monitor ---
pub const REGISTER_MONITOR: ObisCode = ObisCode::new(0, 0, 15, 0, 0, 255);

// --- Limiter ---
pub const LIMITER: ObisCode = ObisCode::new(0, 0, 17, 0, 0, 255);

// --- HDLC setup ---
pub const HDLC_SETUP: ObisCode = ObisCode::new(0, 0, 22, 0, 0, 255);

// --- TCP/UDP setup ---
pub const TCP_UDP_SETUP: ObisCode = ObisCode::new(0, 0, 25, 0, 0, 255);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obis_display() {
        assert_eq!(TOTAL_ACTIVE_ENERGY_IMPORT.to_string(), "1.0.1.8.0.255");
        assert_eq!(CLOCK.to_string(), "0.0.1.0.0.255");
    }

    #[test]
    fn test_obis_bytes() {
        let obis = ObisCode::new(1, 0, 1, 8, 0, 255);
        assert_eq!(obis.to_bytes(), [1, 0, 1, 8, 0, 255]);
        assert_eq!(ObisCode::from_bytes(&[1, 0, 1, 8, 0, 255]), obis);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_obis_parse() {
        let obis = ObisCode::parse("1.0.1.8.0.255").unwrap();
        assert_eq!(obis, TOTAL_ACTIVE_ENERGY_IMPORT);
        assert!(ObisCode::parse("invalid").is_none());
    }

    #[test]
    fn test_medium() {
        assert_eq!(TOTAL_ACTIVE_ENERGY_IMPORT.medium(), "AC Electricity");
        assert_eq!(CLOCK.medium(), "Abstract");
    }
}
