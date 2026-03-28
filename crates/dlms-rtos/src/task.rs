//! Task management traits
//!
//! Provides abstraction for task creation, lifecycle, and priority management.

use core::fmt;

/// Unique task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct TaskId(pub u32);

impl TaskId {
    /// Create a new task ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn get(&self) -> u32 {
        self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self(0)
    }
}

/// Task priority levels
///
/// Higher values indicate higher priority. Real RTOS may map these to
/// native priority values (e.g., FreeRTOS uses 0-24 where 24 is highest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum Priority {
    /// Idle priority - lowest, for background tasks
    Idle = 0,
    /// Low priority - below normal
    Low = 1,
    /// Normal priority - default for most tasks
    Normal = 2,
    /// High priority - for time-sensitive tasks
    High = 3,
    /// Realtime priority - highest, for critical operations
    Realtime = 4,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

impl Priority {
    /// Get the priority as a u8 value
    pub const fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Create from a u8 value (clamped to valid range)
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Idle,
            1 => Self::Low,
            2 => Self::Normal,
            3 => Self::High,
            _ => Self::Realtime,
        }
    }
}

/// Task state in the scheduler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum TaskState {
    /// Task is created but not yet started
    Suspended,
    /// Task is ready to run (waiting for CPU)
    Ready,
    /// Task is currently executing
    Running,
    /// Task is waiting (for resource, time, etc.)
    Blocked,
}

/// Task handle returned when spawning a task
///
/// This handle allows control over the spawned task.
pub trait TaskHandle: Send + Sync {
    /// Get the task ID
    fn id(&self) -> TaskId;

    /// Get the current task state
    fn state(&self) -> TaskState;

    /// Suspend the task
    fn suspend(&self);

    /// Resume a suspended task
    fn resume(&self);

    /// Get the task priority
    fn priority(&self) -> Priority;

    /// Set the task priority
    fn set_priority(&self, priority: Priority);
}

/// Task function type
pub type TaskFn = fn();

/// Task configuration for spawning
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub struct TaskConfig {
    /// Task priority
    pub priority: Priority,
    /// Stack size in bytes (0 = default)
    pub stack_size: usize,
    /// Initial state
    pub initial_state: TaskState,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            priority: Priority::default(),
            stack_size: 0,
            initial_state: TaskState::Ready,
        }
    }
}

impl TaskConfig {
    /// Create a new task config with default values
    pub const fn new() -> Self {
        Self {
            priority: Priority::Normal,
            stack_size: 0,
            initial_state: TaskState::Ready,
        }
    }

    /// Set the priority
    pub const fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the stack size
    pub const fn with_stack_size(mut self, size: usize) -> Self {
        self.stack_size = size;
        self
    }

    /// Set the initial state
    pub const fn with_state(mut self, state: TaskState) -> Self {
        self.initial_state = state;
        self
    }
}

/// RTOS task management trait
///
/// Abstracts task creation, lifecycle control, and priority management.
///
/// # Example
///
/// ```ignore
/// fn my_task() {
///     loop {
///         // Task work here
///     }
/// }
///
/// let handle = rtos.spawn_task(my_task, TaskConfig::new());
/// ```
pub trait RtosTask: Sized {
    /// Task handle type
    type Handle: TaskHandle;

    /// Spawn a new task
    ///
    /// # Errors
    /// Returns an error if:
    /// - Maximum task count reached
    /// - Insufficient memory
    /// - Invalid configuration
    fn spawn_task(&self, f: TaskFn, config: TaskConfig) -> Result<Self::Handle, TaskError>;

    /// Yield the current task, allowing other tasks to run
    fn yield_now(&self);

    /// Put the current task to sleep for a duration
    fn sleep(&self, millis: u32);

    /// Get the handle of the currently running task
    ///
    /// Returns None if called from ISR or when no task is running
    fn current_task(&self) -> Option<Self::Handle>;

    /// Get the number of active tasks
    fn task_count(&self) -> usize;

    /// Default implementation for system tick (can be overridden)
    #[doc(hidden)]
    fn system_tick_default() -> u64 {
        0
    }
}

/// Task-related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-log", derive(defmt::Format))]
pub enum TaskError {
    /// Maximum number of tasks reached
    TaskLimit,
    /// Insufficient memory to create task
    OutOfMemory,
    /// Invalid task configuration
    InvalidConfig,
    /// Task not found
    NotFound,
    /// Operation not allowed from ISR
    NotFromIsr,
}

#[cfg(feature = "std")]
impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskLimit => write!(f, "Task limit reached"),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::InvalidConfig => write!(f, "Invalid configuration"),
            Self::NotFound => write!(f, "Task not found"),
            Self::NotFromIsr => write!(f, "Operation not allowed from ISR"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TaskError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let id = TaskId::new(42);
        assert_eq!(id.get(), 42);
        assert_eq!(TaskId::default().get(), 0);
    }

    #[test]
    fn test_priority() {
        assert_eq!(Priority::Idle.as_u8(), 0);
        assert_eq!(Priority::Realtime.as_u8(), 4);
        assert_eq!(Priority::from_u8(1), Priority::Low);
        assert_eq!(Priority::from_u8(99), Priority::Realtime);
    }

    #[test]
    fn test_task_config() {
        let config = TaskConfig::new()
            .with_priority(Priority::High)
            .with_stack_size(4096);

        assert_eq!(config.priority, Priority::High);
        assert_eq!(config.stack_size, 4096);
        assert_eq!(config.initial_state, TaskState::Ready);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_task_error_display() {
        assert_eq!(alloc::format!("{}", TaskError::TaskLimit), "Task limit reached");
        assert_eq!(alloc::format!("{}", TaskError::NotFound), "Task not found");
    }
}
