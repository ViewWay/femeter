//! Power management
//!
//! Sleep modes, brown-out detection, power-on reset handling

use cortex_m::asm;

/// Power mode
#[derive(Clone, Copy)]
pub enum PowerMode {
    /// Normal operation (168MHz)
    Run,
    /// Reduced power (peripherals on, CPU sleeping)
    Sleep,
    /// Low power (main clocks off, LSI/LSE on)
    Stop,
    /// Minimal power (only RTC and backup domain)
    Standby,
}

pub struct PowerManager {
    mode: PowerMode,
}

impl PowerManager {
    pub const fn new() -> Self {
        Self { mode: PowerMode::Run }
    }

    /// Enter sleep mode until next interrupt
    pub fn enter_sleep(&self) {
        asm::wfi();
    }

    /// Check if brown-out reset occurred
    pub fn was_bor(&self) -> bool {
        // Check RCC CSR register BORRSTF flag
        false
    }

    /// Check if watchdog reset occurred
    pub fn was_iwdg(&self) -> bool {
        // Check RCC CSR register IWDGRSTF flag
        false
    }
}
