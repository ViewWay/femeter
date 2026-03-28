//! Queue traits
//!
//! Provides thread-safe FIFO queues for inter-task communication.

use core::fmt;

#[cfg(feature = "std")]
extern crate std;

/// Queue handle for managing queue state
///
/// This handle provides operations on a queue instance.
pub trait QueueHandle: Send + Sync {
    /// Get the current number of items in the queue
    fn len(&self) -> usize;

    /// Check if the queue is empty
    fn is_empty(&self) -> bool;

    /// Get the queue capacity
    fn capacity(&self) -> usize;

    /// Get available space (capacity - len)
    fn available(&self) -> usize {
        self.capacity().saturating_sub(self.len())
    }
}

/// Queue-related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum QueueError {
    /// Queue is full
    Full,
    /// Queue is empty
    Empty,
    /// Invalid queue size
    InvalidSize,
    /// Queue was deleted
    Deleted,
    /// Operation not allowed from ISR
    NotFromIsr,
}

#[cfg(feature = "std")]
impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "Queue full"),
            Self::Empty => write!(f, "Queue empty"),
            Self::InvalidSize => write!(f, "Invalid queue size"),
            Self::Deleted => write!(f, "Queue deleted"),
            Self::NotFromIsr => write!(f, "Operation not allowed from ISR"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for QueueError {}

/// RTOS queue trait
///
/// Thread-safe FIFO queue for passing data between tasks. Supports both
/// blocking and non-blocking operations.
///
/// # Type Parameters
/// * `T` - The type of items stored in the queue
///
/// # Example
///
/// ```ignore
/// let queue = rtos.create_queue::<u32>(10);
///
/// // Send from one task
/// queue.send(&42).await;
///
/// // Receive from another task
/// let value = queue.receive().await;
/// ```
pub trait RtosQueue: Sized {
    /// Queue handle type
    type Handle<T: Send>: QueueHandle;

    /// Create a new queue
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of items in the queue
    ///
    /// # Panics
    /// Panics if capacity is 0
    fn create_queue<T: Send>(&self, capacity: usize) -> Result<Self::Handle<T>, QueueError>;

    /// Send an item (blocking until space available)
    fn send<T: Send>(&self, queue: &Self::Handle<T>, item: T) -> Result<(), QueueError>;

    /// Try to send without blocking
    ///
    /// Returns error if queue is full.
    fn try_send<T: Send>(&self, queue: &Self::Handle<T>, item: T) -> Result<(), QueueError>;

    /// Send with timeout
    ///
    /// Returns error if timeout expires before space is available.
    fn try_send_timeout<T: Send>(
        &self,
        queue: &Self::Handle<T>,
        item: T,
        millis: u32,
    ) -> Result<(), QueueError>;

    /// Receive an item (blocking until available)
    fn receive<T: Send>(&self, queue: &Self::Handle<T>) -> Result<T, QueueError>;

    /// Try to receive without blocking
    ///
    /// Returns error if queue is empty.
    fn try_receive<T: Send>(&self, queue: &Self::Handle<T>) -> Result<T, QueueError>;

    /// Receive with timeout
    ///
    /// Returns error if timeout expires before item is available.
    fn try_receive_timeout<T: Send>(
        &self,
        queue: &Self::Handle<T>,
        millis: u32,
    ) -> Result<T, QueueError>;

    /// Clear all items from the queue
    fn clear<T: Send>(&self, queue: &Self::Handle<T>);
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "std")]
    #[test]
    fn test_queue_error_display() {
        assert_eq!(alloc::format!("{}", super::QueueError::Full), "Queue full");
        assert_eq!(alloc::format!("{}", super::QueueError::Empty), "Queue empty");
    }
}
