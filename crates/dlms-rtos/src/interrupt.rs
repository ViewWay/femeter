//! Interrupt management traits
//!
//! Provides interrupt control and context detection.

use core::fmt;

/// Interrupt state restoration token
///
/// When interrupts are disabled, this token can be used to restore
/// the previous state. This follows a RAII pattern.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct InterruptState {
    /// The previous interrupt state
    pub was_enabled: bool,
}

impl InterruptState {
    /// Create a new interrupt state token
    pub const fn new(was_enabled: bool) -> Self {
        Self { was_enabled }
    }

    /// Check if interrupts were previously enabled
    pub const fn was_enabled(&self) -> bool {
        self.was_enabled
    }
}

/// Interrupt-related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum InterruptError {
    /// Invalid interrupt state
    InvalidState,
    /// Operation not allowed in current context
    InvalidContext,
}

#[cfg(feature = "std")]
impl fmt::Display for InterruptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState => write!(f, "Invalid interrupt state"),
            Self::InvalidContext => write!(f, "Invalid context for operation"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InterruptError {}

/// RAII guard for restoring interrupt state
///
/// When dropped, restores the previous interrupt state.
/// The guard stores a function pointer to restore interrupts,
/// avoiding the need for a trait object.
pub struct InterruptGuard {
    state: InterruptState,
    restore_fn: fn(InterruptState),
}

impl InterruptGuard {
    /// Create a new interrupt guard with a restore function
    pub fn new(state: InterruptState, restore_fn: fn(InterruptState)) -> Self {
        Self { state, restore_fn }
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        (self.restore_fn)(self.state);
    }
}

/// RTOS interrupt management trait
///
/// Provides interrupt control and context detection for implementing
/// critical sections and ISR-aware code.
///
/// # Example
///
/// ```ignore
/// // Disable interrupts for a critical section
/// let state = rtos.disable_interrupts();
/// // ... critical section ...
/// rtos.restore_interrupt(state);
///
/// // Or use the RAII guard
/// let _guard = rtos.enter_critical();
/// // ... critical section ...
/// // Interrupts restored when guard is dropped
/// ```
pub trait RtosInterrupt: Sized {
    /// Disable all interrupts
    ///
    /// Returns the previous state which must be restored via `restore_interrupt`.
    fn disable_interrupts(&self) -> InterruptState;

    /// Enable all interrupts
    fn enable_interrupts(&self);

    /// Restore interrupt state to previous value
    ///
    /// Should be called with the state returned by `disable_interrupts`.
    fn restore_interrupt(&self, state: InterruptState);

    /// Enter a critical section (RAII guard)
    ///
    /// Returns a guard that restores interrupts when dropped.
    /// This must be implemented by concrete types.
    fn enter_critical(&self) -> InterruptGuard;

    /// Check if currently in interrupt service routine
    ///
    /// Returns true if called from an ISR context.
    fn is_in_isr(&self) -> bool;

    /// Check if interrupts are currently enabled
    fn are_interrupts_enabled(&self) -> bool {
        !self.is_in_isr() && !self.are_interrupts_disabled()
    }

    /// Check if interrupts are currently disabled
    fn are_interrupts_disabled(&self) -> bool;

    /// Get current nesting depth (ISR nesting)
    ///
    /// Returns 0 if not in ISR, 1 for single-level ISR, etc.
    fn isr_nesting_depth(&self) -> u32 {
        if self.is_in_isr() { 1 } else { 0 }
    }
}

/// Helper for implementing critical sections
///
/// This macro provides a cleaner syntax for critical sections.
///
/// # Example
///
/// ```ignore
/// critical_section!(rtos, {
///     // Critical section code here
///     // Interrupts are disabled
/// });
/// // Interrupts restored here
/// ```
#[macro_export]
macro_rules! critical_section {
    ($rtos:expr, $block:block) => {
        let _guard = $rtos.enter_critical();
        $block
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_state() {
        let state = InterruptState::new(true);
        assert!(state.was_enabled());
        assert_eq!(state.was_enabled, true);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_interrupt_error_display() {
        assert_eq!(
            alloc::format!("{}", InterruptError::InvalidState),
            "Invalid interrupt state"
        );
        assert_eq!(
            alloc::format!("{}", InterruptError::InvalidContext),
            "Invalid context for operation"
        );
    }
}
