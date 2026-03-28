//! Board-level initialization and hardware abstraction

use crate::comm::CommDriver;
use crate::display::LcdDriver;
use crate::metering::Rn8209c;

/// Instantaneous electrical values
#[derive(Clone, Copy)]
pub struct InstantaneousValues {
    pub voltage_l1: u16,
    pub current_l1: u16,
    pub active_power: i32,
    pub reactive_power: i32,
    pub power_factor: u16,
    pub frequency: u16,
}

impl Default for InstantaneousValues {
    fn default() -> Self {
        Self { voltage_l1: 0, current_l1: 0, active_power: 0,
               reactive_power: 0, power_factor: 0, frequency: 5000 }
    }
}

/// Cumulative energy registers
#[derive(Clone, Copy)]
pub struct EnergyRegisters {
    pub active_import: u64,
    pub active_export: u64,
    pub reactive_import: u64,
    pub reactive_export: u64,
    pub tariff_import: [u64; 8],
}

impl Default for EnergyRegisters {
    fn default() -> Self {
        Self { active_import: 0, active_export: 0, reactive_import: 0,
               reactive_export: 0, tariff_import: [0u64; 8] }
    }
}

/// Complete meter state
pub struct MeterState {
    pub instantaneous: InstantaneousValues,
    pub energy: EnergyRegisters,
    pub relay_closed: bool,
    pub current_tariff: u8,
    pub meter_time: u64,
    pub sample_count: u64,
}

impl MeterState {
    pub fn new() -> Self {
        Self {
            instantaneous: InstantaneousValues::default(),
            energy: EnergyRegisters::default(),
            relay_closed: true,
            current_tariff: 0,
            meter_time: 0,
            sample_count: 0,
        }
    }
}

/// Board abstraction
pub struct Board {
    systick: u64,
    state: MeterState,
    _metering: Rn8209c,
    _comm: CommDriver,
    display: LcdDriver,
}

impl Board {
    pub fn init() -> Self {
        Self {
            systick: 0,
            state: MeterState::new(),
            _metering: Rn8209c::new(),
            _comm: CommDriver::new(0x0001),
            display: LcdDriver::new(),
        }
    }

    pub fn systick_ms(&self) -> u64 { self.systick }

    pub fn tick(&mut self) { self.systick = self.systick.wrapping_add(1); }

    pub fn sample_energy(&mut self) { self.state.sample_count += 1; }
    pub fn calculate_power(&mut self) {}
    pub fn update_display(&mut self) {
        self.display.update(
            self.state.instantaneous.voltage_l1,
            self.state.instantaneous.current_l1,
            self.state.instantaneous.active_power,
            self.state.energy.active_import,
            self.state.current_tariff,
        );
    }
    pub fn process_hdlc(&mut self) {}
    pub fn capture_profile(&mut self) {}
    pub fn check_tariff(&mut self) {}
    pub fn check_alarms(&mut self) {}
    pub fn feed_watchdog(&mut self) {}
}
