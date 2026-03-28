//! Physical unit enumeration
//!
//! Reference: Blue Book Part 2 §10, Green Book Ed.9 Annex A

/// COSEM physical unit codes (subset of standard units)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum Unit {
    /// 0 = No unit / dimensionless
    None = 0,
    /// 1 = Year
    Year = 1,
    /// 2 = Month
    Month = 2,
    /// 3 = Week
    Week = 3,
    /// 4 = Day
    Day = 4,
    /// 5 = Hour
    Hour = 5,
    /// 6 = Minute
    Minute = 6,
    /// 7 = Second
    Second = 7,
    /// 8 = Phase angle (degree)
    PhaseAngleDeg = 8,
    /// 9 = Temperature (°C)
    TemperatureC = 9,
    /// 21 = Energy (J)
    Joule = 21,
    /// 23 = Mass (kg)
    Kilogram = 23,
    /// 25 = Force (N)
    Newton = 25,
    /// 27 = Pressure (Pa)
    Pascal = 27,
    /// 29 = Power (W)
    Watt = 29,
    /// 30 = Active energy (Wh)
    WattHour = 30,
    /// 31 = Reactive energy (varh)
    VarHour = 31,
    /// 32 = Apparent energy (VAh)
    VaHour = 32,
    /// 33 = Voltage (V)
    Volt = 33,
    /// 34 = Current (A)
    Ampere = 34,
    /// 35 = Frequency (Hz)
    Hertz = 35,
    /// 36 = Power factor (dimensionless)
    PowerFactor = 36,
    /// 37 = Resistance (Ω)
    Ohm = 37,
    /// 38 = Conductance (S)
    Siemens = 38,
    /// 39 = Capacitance (F)
    Farad = 39,
    /// 40 = Inductance (H)
    Henry = 40,
    /// 255 = Not defined / count
    Count = 255,
}

impl Unit {
    /// Create from numeric code
    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            1 => Some(Self::Year),
            2 => Some(Self::Month),
            3 => Some(Self::Week),
            4 => Some(Self::Day),
            5 => Some(Self::Hour),
            6 => Some(Self::Minute),
            7 => Some(Self::Second),
            8 => Some(Self::PhaseAngleDeg),
            9 => Some(Self::TemperatureC),
            21 => Some(Self::Joule),
            23 => Some(Self::Kilogram),
            25 => Some(Self::Newton),
            27 => Some(Self::Pascal),
            29 => Some(Self::Watt),
            30 => Some(Self::WattHour),
            31 => Some(Self::VarHour),
            32 => Some(Self::VaHour),
            33 => Some(Self::Volt),
            34 => Some(Self::Ampere),
            35 => Some(Self::Hertz),
            36 => Some(Self::PowerFactor),
            37 => Some(Self::Ohm),
            38 => Some(Self::Siemens),
            39 => Some(Self::Farad),
            40 => Some(Self::Henry),
            255 => Some(Self::Count),
            _ => None,
        }
    }

    /// Get the numeric code
    pub fn code(&self) -> u16 {
        *self as u16
    }
}

impl Default for Unit {
    fn default() -> Self {
        Self::None
    }
}
