//! GPIO (General Purpose I/O) HAL trait
//!
//! Provides interface for controlling GPIO pins.

use crate::HalResult;

/// GPIO pin direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum GpioDirection {
    Input,
    Output,
}

/// GPIO HAL trait for controlling digital I/O pins
///
/// This trait is object-safe and can be used with `dyn GpioHal`.
pub trait GpioHal {
    /// Set the direction of a pin
    ///
    /// # Arguments
    /// * `pin` - Pin number
    /// * `direction` - Input or Output
    fn set_direction(&mut self, pin: u8, direction: GpioDirection) -> HalResult<()>;

    /// Set a pin output high
    fn set_high(&mut self, pin: u8) -> HalResult<()>;

    /// Set a pin output low
    fn set_low(&mut self, pin: u8) -> HalResult<()>;

    /// Check if a pin is high
    fn is_high(&mut self, pin: u8) -> HalResult<bool>;

    /// Check if a pin is low
    fn is_low(&mut self, pin: u8) -> HalResult<bool> {
        self.is_high(pin).map(|v| !v)
    }

    /// Toggle a pin output
    fn toggle(&mut self, pin: u8) -> HalResult<()> {
        let high = self.is_high(pin)?;
        if high {
            self.set_low(pin)
        } else {
            self.set_high(pin)
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::HalError;

    struct MockGpio {
        pin_states: [bool; 32],
        pin_directions: [Option<GpioDirection>; 32],
    }

    impl MockGpio {
        fn new() -> Self {
            Self {
                pin_states: [false; 32],
                pin_directions: [None; 32],
            }
        }
    }

    impl GpioHal for MockGpio {
        fn set_direction(&mut self, pin: u8, direction: GpioDirection) -> HalResult<()> {
            if pin >= 32 {
                return Err(HalError::InvalidParam);
            }
            self.pin_directions[pin as usize] = Some(direction);
            Ok(())
        }

        fn set_high(&mut self, pin: u8) -> HalResult<()> {
            if pin >= 32 {
                return Err(HalError::InvalidParam);
            }
            if self.pin_directions[pin as usize] != Some(GpioDirection::Output) {
                return Err(HalError::InvalidParam);
            }
            self.pin_states[pin as usize] = true;
            Ok(())
        }

        fn set_low(&mut self, pin: u8) -> HalResult<()> {
            if pin >= 32 {
                return Err(HalError::InvalidParam);
            }
            if self.pin_directions[pin as usize] != Some(GpioDirection::Output) {
                return Err(HalError::InvalidParam);
            }
            self.pin_states[pin as usize] = false;
            Ok(())
        }

        fn is_high(&mut self, pin: u8) -> HalResult<bool> {
            if pin >= 32 {
                return Err(HalError::InvalidParam);
            }
            Ok(self.pin_states[pin as usize])
        }
    }

    #[test]
    fn test_gpio_set_direction() {
        let mut gpio = MockGpio::new();
        gpio.set_direction(5, GpioDirection::Output).unwrap();
        gpio.set_direction(10, GpioDirection::Input).unwrap();
    }

    #[test]
    fn test_gpio_invalid_pin() {
        let mut gpio = MockGpio::new();
        assert_eq!(
            gpio.set_direction(32, GpioDirection::Output).unwrap_err(),
            HalError::InvalidParam
        );
        assert_eq!(gpio.set_high(100).unwrap_err(), HalError::InvalidParam);
    }

    #[test]
    fn test_gpio_set_output() {
        let mut gpio = MockGpio::new();
        gpio.set_direction(0, GpioDirection::Output).unwrap();
        gpio.set_high(0).unwrap();
        assert!(gpio.is_high(0).unwrap());
        assert!(!gpio.is_low(0).unwrap());

        gpio.set_low(0).unwrap();
        assert!(!gpio.is_high(0).unwrap());
        assert!(gpio.is_low(0).unwrap());
    }

    #[test]
    fn test_gpio_output_without_direction() {
        let mut gpio = MockGpio::new();
        assert_eq!(gpio.set_high(5).unwrap_err(), HalError::InvalidParam);
    }

    #[test]
    fn test_gpio_toggle() {
        let mut gpio = MockGpio::new();
        gpio.set_direction(3, GpioDirection::Output).unwrap();

        gpio.set_high(3).unwrap();
        assert!(gpio.is_high(3).unwrap());

        gpio.toggle(3).unwrap();
        assert!(gpio.is_low(3).unwrap());

        gpio.toggle(3).unwrap();
        assert!(gpio.is_high(3).unwrap());
    }

    #[test]
    fn test_gpio_object_safe() {
        let mut gpio: std::boxed::Box<dyn GpioHal> = std::boxed::Box::new(MockGpio::new());
        gpio.set_direction(0, GpioDirection::Output).unwrap();
        gpio.set_high(0).unwrap();
        assert!(gpio.is_high(0).unwrap());
    }
}
