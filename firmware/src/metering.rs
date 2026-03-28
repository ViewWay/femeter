//! RN8209C energy metering IC driver
//!
//! Communication via SPI (Mode 0, LSB first for RN8209C)
//! Provides: voltage RMS, current RMS, active power, reactive power, frequency, energy

/// RN8209C register addresses
pub mod registers {
    pub const DEVICE_ID: u8 = 0x00;
    pub const SYSCON: u8 = 0x40;
    pub const EMUCON: u8 = 0x41;
    pub const EMUCON2: u8 = 0x42;
    pub const HFCONST: u8 = 0x43;
    pub const PSTART: u8 = 0x44;
    pub const QSTART: u8 = 0x45;

    // Measurement results
    pub const U_RMS: u8 = 0x70;
    pub const I_A_RMS: u8 = 0x71;
    pub const I_B_RMS: u8 = 0x72;
    pub const P_A: u8 = 0x73;
    pub const P_B: u8 = 0x74;
    pub const Q_A: u8 = 0x75;
    pub const Q_B: u8 = 0x76;
    pub const FREQ: u8 = 0x77;

    // Energy registers
    pub const ENERGY_A_P: u8 = 0x80;
    pub const ENERGY_A_Q: u8 = 0x81;
    pub const ENERGY_B_P: u8 = 0x82;
    pub const ENERGY_B_Q: u8 = 0x83;

    // Calibration
    pub const U_GAIN: u8 = 0x90;
    pub const I_A_GAIN: u8 = 0x91;
    pub const I_B_GAIN: u8 = 0x92;
    pub const P_A_GAIN: u8 = 0x93;
    pub const P_B_GAIN: u8 = 0x94;
    pub const Q_A_GAIN: u8 = 0x95;
    pub const PHCAL_A: u8 = 0x96;
    pub const PHCAL_B: u8 = 0x97;
}

/// RN8209C metering driver
pub struct Rn8209c {
    /// Voltage calibration coefficient
    pub u_coeff: f32,
    /// Current channel A calibration coefficient
    pub ia_coeff: f32,
    /// Current channel B calibration coefficient
    pub ib_coeff: f32,
    /// Active power calibration coefficient
    pub p_coeff: f32,
    /// Reactive power calibration coefficient
    pub q_coeff: f32,
}

impl Rn8209c {
    pub fn new() -> Self {
        Self {
            u_coeff: 0.0001,   // Default, needs calibration per board
            ia_coeff: 0.001,
            ib_coeff: 0.001,
            p_coeff: 0.01,
            q_coeff: 0.01,
        }
    }

    /// Convert raw 24-bit RMS register to engineering value
    pub fn raw_to_voltage(&self, raw: u32) -> u16 {
        // RN8209C U_RMS is 24-bit unsigned, typically ~700000 at 220V
        let v = (raw as f32) * self.u_coeff;
        (v * 10.0) as u16 // Return in 0.1V units
    }

    pub fn raw_to_current(&self, raw: u32) -> u16 {
        let a = (raw as f32) * self.ia_coeff;
        (a * 1000.0) as u16 // Return in mA
    }

    pub fn raw_to_power(&self, raw: i32) -> i32 {
        (raw as f32 * self.p_coeff) as i32 // Return in W
    }

    pub fn raw_to_frequency(&self, raw: u32) -> u16 {
        // RN8209C frequency register: value = 3579545 / (2 * freq_reg)
        if raw > 0 {
            (3579545.0 / (2.0 * raw as f32) * 100.0) as u16 // 0.01Hz units
        } else {
            5000 // Default 50.00Hz
        }
    }
}
