//! Display HAL trait
//!
//! Provides interface for display control (LCD, OLED, etc.).

use crate::HalResult;

/// Display HAL trait for display control
///
/// This trait is object-safe and can be used with `dyn DisplayHal`.
pub trait DisplayHal {
    /// Initialize the display
    fn init(&mut self) -> HalResult<()>;

    /// Clear the display
    fn clear(&mut self) -> HalResult<()>;

    /// Write a string to the display
    ///
    /// # Arguments
    /// * `text` - Text to display (ASCII or UTF-8)
    fn write_string(&mut self, text: &str) -> HalResult<()>;

    /// Set cursor position
    ///
    /// # Arguments
    /// * `row` - Row number (0-based)
    /// * `col` - Column number (0-based)
    fn set_cursor(&mut self, row: u8, col: u8) -> HalResult<()>;

    /// Set backlight brightness
    ///
    /// # Arguments
    /// * `brightness` - Brightness level (0-100)
    fn set_backlight(&mut self, brightness: u8) -> HalResult<()>;

    /// Turn display on
    fn on(&mut self) -> HalResult<()> {
        self.set_backlight(100)
    }

    /// Turn display off
    fn off(&mut self) -> HalResult<()> {
        self.set_backlight(0)
    }

    /// Get number of rows
    fn rows(&self) -> u8;

    /// Get number of columns
    fn columns(&self) -> u8;
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::HalError;
    use std::vec;

    const DISPLAY_ROWS: u8 = 4;
    const DISPLAY_COLS: u8 = 20;

    struct MockDisplay {
        initialized: bool,
        buffer: Vec<Vec<char>>,
        cursor_row: u8,
        cursor_col: u8,
        backlight: u8,
        power: bool,
    }

    impl MockDisplay {
        fn new() -> Self {
            Self {
                initialized: false,
                buffer: std::vec::from_elem(
                    std::vec::from_elem(' ', DISPLAY_COLS as usize),
                    DISPLAY_ROWS as usize,
                ),
                cursor_row: 0,
                cursor_col: 0,
                backlight: 100,
                power: true,
            }
        }

        fn get_buffer(&self) -> std::string::String {
            self.buffer
                .iter()
                .map(|row| row.iter().collect::<std::string::String>())
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    impl DisplayHal for MockDisplay {
        fn init(&mut self) -> HalResult<()> {
            self.initialized = true;
            Ok(())
        }

        fn clear(&mut self) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            for row in &mut self.buffer {
                for ch in row.iter_mut() {
                    *ch = ' ';
                }
            }
            self.cursor_row = 0;
            self.cursor_col = 0;
            Ok(())
        }

        fn write_string(&mut self, text: &str) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            for ch in text.chars() {
                if self.cursor_col >= DISPLAY_COLS {
                    self.cursor_col = 0;
                    self.cursor_row += 1;
                }
                if self.cursor_row >= DISPLAY_ROWS {
                    return Err(HalError::InvalidParam);
                }
                self.buffer[self.cursor_row as usize][self.cursor_col as usize] = ch;
                self.cursor_col += 1;
            }
            Ok(())
        }

        fn set_cursor(&mut self, row: u8, col: u8) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            if row >= DISPLAY_ROWS || col >= DISPLAY_COLS {
                return Err(HalError::InvalidParam);
            }
            self.cursor_row = row;
            self.cursor_col = col;
            Ok(())
        }

        fn set_backlight(&mut self, brightness: u8) -> HalResult<()> {
            if !self.initialized {
                return Err(HalError::NotInitialized);
            }
            if brightness > 100 {
                return Err(HalError::InvalidParam);
            }
            self.backlight = brightness;
            self.power = brightness > 0;
            Ok(())
        }

        fn rows(&self) -> u8 {
            DISPLAY_ROWS
        }

        fn columns(&self) -> u8 {
            DISPLAY_COLS
        }
    }

    #[test]
    fn test_display_init() {
        let mut display = MockDisplay::new();
        display.init().unwrap();
    }

    #[test]
    fn test_display_clear() {
        let mut display = MockDisplay::new();
        display.init().unwrap();
        display.write_string("Hello").unwrap();
        display.clear().unwrap();
        assert_eq!(display.get_buffer(), "                    \n                    \n                    \n                    ");
    }

    #[test]
    fn test_display_write_string() {
        let mut display = MockDisplay::new();
        display.init().unwrap();
        display.write_string("123.45 kWh").unwrap();
        assert_eq!(
            display.get_buffer().chars().take(11).collect::<std::string::String>(),
            "123.45 kWh"
        );
    }

    #[test]
    fn test_display_set_cursor() {
        let mut display = MockDisplay::new();
        display.init().unwrap();
        display.set_cursor(2, 5).unwrap();
        display.write_string("Test").unwrap();

        let buffer = display.get_buffer();
        let lines: Vec<&str> = buffer.split('\n').collect();
        assert!(lines[2].starts_with("     Test"));
    }

    #[test]
    fn test_display_invalid_cursor() {
        let mut display = MockDisplay::new();
        display.init().unwrap();
        assert_eq!(
            display.set_cursor(10, 0).unwrap_err(),
            HalError::InvalidParam
        );
        assert_eq!(
            display.set_cursor(0, 30).unwrap_err(),
            HalError::InvalidParam
        );
    }

    #[test]
    fn test_display_backlight() {
        let mut display = MockDisplay::new();
        display.init().unwrap();
        display.set_backlight(50).unwrap();
        display.set_backlight(0).unwrap();
        display.set_backlight(100).unwrap();
    }

    #[test]
    fn test_display_invalid_backlight() {
        let mut display = MockDisplay::new();
        display.init().unwrap();
        assert_eq!(
            display.set_backlight(150).unwrap_err(),
            HalError::InvalidParam
        );
    }

    #[test]
    fn test_display_not_initialized() {
        let mut display = MockDisplay::new();
        assert_eq!(display.clear().unwrap_err(), HalError::NotInitialized);
        assert_eq!(
            display.write_string("test").unwrap_err(),
            HalError::NotInitialized
        );
    }

    #[test]
    fn test_display_dimensions() {
        let display = MockDisplay::new();
        assert_eq!(display.rows(), 4);
        assert_eq!(display.columns(), 20);
    }

    #[test]
    fn test_display_on_off() {
        let mut display = MockDisplay::new();
        display.init().unwrap();
        display.on().unwrap();
        display.off().unwrap();
    }

    #[test]
    fn test_display_object_safe() {
        let mut display: std::boxed::Box<dyn DisplayHal> = std::boxed::Box::new(MockDisplay::new());
        display.init().unwrap();
        display.clear().unwrap();
        display.write_string("Test").unwrap();
    }
}
