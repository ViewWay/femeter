//! LCD segment display driver
//!
//! Generic LCD segment display for smart meters.
//! Typically 7-segment + icons (tariff indicator, relay status, etc.)

/// Display data - what to show on the LCD
#[derive(Clone, Copy)]
pub enum DisplayPage {
    /// Show voltage and current (default)
    VoltageCurrent,
    /// Show active power
    Power,
    /// Show total energy (kWh)
    EnergyTotal,
    /// Show per-tariff energy
    EnergyTariff(u8),
    /// Show current tariff indicator
    TariffInfo,
    /// Error display
    Error(u8),
}

/// LCD driver state
pub struct LcdDriver {
    current_page: DisplayPage,
    auto_cycle: bool,
    cycle_counter: u32,
}

impl LcdDriver {
    pub fn new() -> Self {
        Self {
            current_page: DisplayPage::VoltageCurrent,
            auto_cycle: true,
            cycle_counter: 0,
        }
    }

    /// Update display with current values
    pub fn update(&mut self, voltage: u16, current: u16, power: i32, energy: u64, tariff: u8) {
        if self.auto_cycle {
            self.cycle_counter += 1;
            if self.cycle_counter >= 100 { // Switch page every ~50 cycles
                self.cycle_counter = 0;
                self.current_page = match self.current_page {
                    DisplayPage::VoltageCurrent => DisplayPage::Power,
                    DisplayPage::Power => DisplayPage::EnergyTotal,
                    DisplayPage::EnergyTotal => DisplayPage::VoltageCurrent,
                    _ => DisplayPage::VoltageCurrent,
                };
            }
        }

        match self.current_page {
            DisplayPage::VoltageCurrent => {
                self.write_voltage_current(voltage, current);
            }
            DisplayPage::Power => {
                self.write_power(power);
            }
            DisplayPage::EnergyTotal => {
                self.write_energy(energy);
            }
            _ => {}
        }

        // Always show tariff indicator
        self.write_tariff_icon(tariff);
    }

    fn write_voltage_current(&self, voltage: u16, current: u16) {
        // Format: "220.0V 05.23A"
        // Write to LCD segment controller
    }

    fn write_power(&self, power: i32) {
        // Format: " 1234 W"
    }

    fn write_energy(&self, energy: u64) {
        // Format: "12345.6 kWh"
    }

    fn write_tariff_icon(&self, tariff: u8) {
        // Light up tariff segment T1-T8
    }
}
