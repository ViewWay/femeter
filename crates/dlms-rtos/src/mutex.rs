//! Mutex (Mutual Exclusion) traits
//!
//! Provides RAII-style mutex for protecting shared data.

use core::fmt;
use core::ops::{Deref, DerefMut};

#[cfg(feature = "std")]
extern crate std;

/// Mutex guard providing RAII-style locking
///
/// The guard automatically releases the mutex when dropped.
pub trait MutexGuard<'a, T>: Deref<Target = T> + DerefMut {}

/// Mutex-related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum MutexError {
    /// Mutex is locked and operation would block
    WouldBlock,
    /// Lock was poisoned (holder panicked)
    Poisoned,
    /// Invalid mutex state
    InvalidState,
}

#[cfg(feature = "std")]
impl fmt::Display for MutexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WouldBlock => write!(f, "Would block"),
            Self::Poisoned => write!(f, "Mutex poisoned"),
            Self::InvalidState => write!(f, "Invalid mutex state"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MutexError {}

/// RTOS mutex trait
///
/// Provides mutual exclusion for protecting shared data. The mutex
/// supports both blocking and try-lock operations.
///
/// # Example
///
/// ```ignore
/// let mutex = rtos.create_mutex();
///
/// // Blocking lock - returns RAII guard
/// let guard = mutex.lock();
/// *guard = 42;  // Access protected data
/// // Guard dropped here, releases lock
/// ```
pub trait RtosMutex: Sized {
    /// Mutex pointer type for storage
    type MutexPtr<T: Send>: MutexPtr<T>;

    /// Guard type returned by lock operations
    type Guard<'a, T: Send>: MutexGuard<'a, T> where T: 'a, Self: 'a;

    /// Create a new mutex
    ///
    /// The mutex is initially unlocked.
    fn create_mutex<T: Default + Send>(&self) -> Self::MutexPtr<T>;

    /// Create a new mutex with an initial value
    fn create_mutex_with<T: Send>(&self, value: T) -> Self::MutexPtr<T>;

    /// Lock the mutex, blocking until acquired
    ///
    /// Returns a guard that automatically releases the mutex when dropped.
    fn lock<'a, T: Send>(&self, mutex: &'a Self::MutexPtr<T>) -> Self::Guard<'a, T>;

    /// Try to lock the mutex without blocking
    ///
    /// Returns None if the mutex is already locked.
    fn try_lock<'a, T: Send>(&self, mutex: &'a Self::MutexPtr<T>) -> Option<Self::Guard<'a, T>>;
}

/// Pointer type for mutex storage
///
/// This trait abstracts the actual mutex storage, allowing implementations
/// to use their own representation (e.g., wrapper around native mutex type).
pub trait MutexPtr<T>: Send + Sync {
    /// Lock the mutex (implementation detail)
    fn lock<'a>(&'a self) -> impl Deref<Target = T>;

    /// Try to lock without blocking (implementation detail)
    fn try_lock<'a>(&'a self) -> Option<impl Deref<Target = T>>;
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "std")]
    #[test]
    fn test_mutex_error_display() {
        assert_eq!(alloc::format!("{}", super::MutexError::WouldBlock), "Would block");
        assert_eq!(alloc::format!("{}", super::MutexError::Poisoned), "Mutex poisoned");
    }
}
