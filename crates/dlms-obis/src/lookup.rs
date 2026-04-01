//! OBIS code lookup and description
//!
//! Provides description strings and group classification for OBIS codes.

use dlms_core::ObisCode;

/// OBIS medium group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObisGroup {
    Abstract,
    AcElectricity,
    DcElectricity,
    ThermalEnergy,
    Gas,
    Water,
    HeatMeter,
    Cooling,
    Other(u8),
}

/// Get the OBIS group from the A value
pub fn obis_group(code: &ObisCode) -> ObisGroup {
    match code.a {
        0 => ObisGroup::Abstract,
        1 => ObisGroup::AcElectricity,
        2 => ObisGroup::DcElectricity,
        5 | 6 => ObisGroup::ThermalEnergy,
        7 => ObisGroup::Gas,
        8 | 9 => ObisGroup::Water,
        10 => ObisGroup::HeatMeter,
        11 => ObisGroup::Cooling,
        a => ObisGroup::Other(a),
    }
}

/// Get a human-readable description for a known OBIS code
pub fn obis_description(code: &ObisCode) -> Option<&'static str> {
    // Check known codes
    match (code.a, code.b, code.c, code.d, code.e, code.f) {
        // AC Electricity
        (1, 0, 1, 8, 0, 255) => Some("Total active energy import (kWh)"),
        (1, 0, 2, 8, 0, 255) => Some("Total active energy export (kWh)"),
        (1, 0, 3, 8, 0, 255) => Some("Total reactive energy import (kvarh)"),
        (1, 0, 4, 8, 0, 255) => Some("Total reactive energy export (kvarh)"),
        (1, 0, 1, 8, 1..=4, 255) => Some("Active energy import by tariff"),
        (1, 0, 2, 8, 1..=4, 255) => Some("Active energy export by tariff"),
        (1, 0, 1, 7, 0, 255) => Some("Total active power import (W)"),
        (1, 0, 2, 7, 0, 255) => Some("Total active power export (W)"),
        (1, 0, 3, 7, 0, 255) => Some("Total reactive power import (var)"),
        (1, 0, 4, 7, 0, 255) => Some("Total reactive power export (var)"),
        (1, 0, 32, 7, 0, 255) => Some("Voltage L1 (V)"),
        (1, 0, 52, 7, 0, 255) => Some("Voltage L2 (V)"),
        (1, 0, 72, 7, 0, 255) => Some("Voltage L3 (V)"),
        (1, 0, 31, 7, 0, 255) => Some("Current L1 (A)"),
        (1, 0, 51, 7, 0, 255) => Some("Current L2 (A)"),
        (1, 0, 71, 7, 0, 255) => Some("Current L3 (A)"),
        (1, 0, 14, 7, 0, 255) => Some("Frequency (Hz)"),
        (1, 0, 13, 7, 0, 255) => Some("Total power factor"),
        (1, 0, 15, 7, 0, 255) => Some("Active power L1 (W)"),
        (1, 0, 1, 6, 0, 255) => Some("Current demand (A)"),
        (1, 0, 15, 6, 0, 255) => Some("Active power demand (W)"),

        // Abstract
        (0, 0, 1, 0, 0, 255) => Some("Clock"),
        (0, 0, 2, 0, 0, 255) => Some("Clock time"),
        (0, 0, 40, 0, 1, 255) => Some("Association LN (IC 15)"),
        (0, 0, 25, 0, 0, 255) => Some("TCP/UDP setup (IC 41)"),

        // Gas
        (7, 0, 1, 8, 0, 255) => Some("Total gas volume (m³)"),
        (7, 0, 1, 7, 0, 255) => Some("Gas flow rate (m³/h)"),

        // Water
        (8, 0, 1, 8, 0, 255) => Some("Total water volume (m³)"),
        (9, 0, 1, 8, 0, 255) => Some("Total hot water volume (m³)"),

        // Thermal
        (5, 0, 1, 8, 0, 255) => Some("Total thermal energy (J)"),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obis_group() {
        let code = ObisCode::new(1, 0, 1, 8, 0, 255);
        assert_eq!(obis_group(&code), ObisGroup::AcElectricity);

        let code = ObisCode::new(7, 0, 1, 8, 0, 255);
        assert_eq!(obis_group(&code), ObisGroup::Gas);
    }

    #[test]
    fn test_obis_description() {
        let code = ObisCode::new(1, 0, 1, 8, 0, 255);
        assert_eq!(
            obis_description(&code),
            Some("Total active energy import (kWh)")
        );

        let code = ObisCode::new(7, 0, 1, 8, 0, 255);
        assert_eq!(obis_description(&code), Some("Total gas volume (m³)"));

        let code = ObisCode::new(99, 0, 0, 0, 0, 255);
        assert!(obis_description(&code).is_none());
    }

    #[test]
    fn test_obis_group_all() {
        assert_eq!(
            obis_group(&ObisCode::new(0, 0, 0, 0, 0, 0)),
            ObisGroup::Abstract
        );
        assert_eq!(
            obis_group(&ObisCode::new(5, 0, 0, 0, 0, 0)),
            ObisGroup::ThermalEnergy
        );
        assert_eq!(
            obis_group(&ObisCode::new(10, 0, 0, 0, 0, 0)),
            ObisGroup::HeatMeter
        );
        assert_eq!(
            obis_group(&ObisCode::new(99, 0, 0, 0, 0, 0)),
            ObisGroup::Other(99)
        );
    }
}
