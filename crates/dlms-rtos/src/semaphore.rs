//! Semaphore traits
//!
//! Provides counting semaphores for resource management and synchronization.

use core::fmt;

#[cfg(feature = "std")]
extern crate std;

/// Semaphore handle for managing semaphore state
///
/// This handle provides operations on a semaphore instance.
pub trait SemaphoreHandle: Send + Sync {
    /// Get the current semaphore count
    fn count(&self) -> u32;

    /// Get the maximum count (capacity)
    fn max_count(&self) -> u32;
}

/// Semaphore-related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
#[allow(dead_code)]
pub enum SemaphoreError {
    /// Semaphore was deleted
    Deleted,
    /// Timeout waiting for semaphore
    Timeout,
    /// Operation not allowed from ISR
    NotFromIsr,
    /// Invalid semaphore state
    InvalidState,
}

#[cfg(feature = "std")]
impl fmt::Display for SemaphoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deleted => write!(f, "Semaphore deleted"),
            Self::Timeout => write!(f, "Timeout waiting for semaphore"),
            Self::NotFromIsr => write!(f, "Operation not allowed from ISR"),
            Self::InvalidState => write!(f, "Invalid semaphore state"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SemaphoreError {}

/// RTOS semaphore trait
///
/// Counting semaphores are used for resource management and synchronization.
/// A semaphore has a count that is incremented by `release()` and decremented
/// by `acquire()`. The count is bounded by a maximum value.
///
/// # Example
///
/// ```ignore
/// // Create a binary semaphore (max count = 1)
/// let sem = rtos.create_semaphore(1);
///
/// // Acquire (blocks if count is 0)
/// sem.acquire();
///
/// // Critical section
///
/// // Release (increments count)
/// sem.release();
/// ```
pub trait RtosSemaphore: Sized {
    /// Semaphore handle type
    type Handle: SemaphoreHandle;

    /// Create a new counting semaphore
    ///
    /// # Arguments
    /// * `initial_count` - Initial count value
    /// * `max_count` - Maximum count (must be >= initial_count)
    ///
    /// # Panics
    /// Panics if max_count < initial_count
    fn create_semaphore(&self, initial_count: u32, max_count: u32) -> Self::Handle;

    /// Create a binary semaphore
    ///
    /// A binary semaphore has max_count = 1.
    fn create_binary_semaphore(&self) -> Self::Handle {
        self.create_semaphore(0, 1)
    }

    /// Acquire the semaphore (decrement count)
    ///
    /// Blocks until the semaphore is available (count > 0).
    fn acquire(&self, sem: &Self::Handle);

    /// Try to acquire without blocking
    ///
    /// Returns true if acquired, false if count is 0.
    fn try_acquire(&self, sem: &Self::Handle) -> bool;

    /// Try to acquire with timeout
    ///
    /// Returns true if acquired within timeout, false otherwise.
    fn try_acquire_timeout(&self, sem: &Self::Handle, millis: u32) -> bool;

    /// Release the semaphore (increment count)
    ///
    /// Increments the count, up to max_count. May wake a waiting task.
    fn release(&self, sem: &Self::Handle);

    /// Flush the semaphore
    ///
    /// Resets the count to initial value. The exact behavior depends on
    /// the RTOS implementation.
    fn flush(&self, sem: &Self::Handle);
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "std")]
    #[test]
    fn test_semaphore_error_display() {
        assert_eq!(alloc::format!("{}", super::SemaphoreError::Deleted), "Semaphore deleted");
        assert_eq!(alloc::format!("{}", super::SemaphoreError::Timeout), "Timeout waiting for semaphore");
    }
}
