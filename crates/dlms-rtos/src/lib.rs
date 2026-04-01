//! dlms-rtos: RTOS abstraction layer for DLMS/COSEM embedded targets
//!
//! This crate provides traits that abstract RTOS primitives. On real hardware,
//! users implement these traits using FreeRTOS, RTIC, Embassy, or their RTOS.
//! On std (host), we provide a simple implementation for testing.
//!
//! # Design Philosophy
//!
//! This is a hardware abstraction layer - not a full RTOS. Real implementations
//! will delegate to actual RTOS primitives. The std implementation exists solely
//! for testing protocol logic on host machines.
//!
//! # Example
//!
//! ```ignore
//! use dlms_rtos::{Rtos, Priority, TaskState};
//!
//! struct MyRtos;
//!
//! impl Rtos for MyRtos {
//!     // Delegate to your RTOS primitives
//! }
//! ```
//!
//! # Features
//!
//! - `std` - Enable std host implementation for testing
//! - `defmt-log` - Enable defmt formatting for all traits

#![no_std]
#![allow(dead_code, unused_imports)]

extern crate alloc;

pub use interrupt::{InterruptState, RtosInterrupt};
pub use mempool::{MemPoolHandle, PoolConfig, RtosMemPool};
pub use mutex::{MutexGuard, RtosMutex};
pub use queue::{QueueHandle, RtosQueue};
pub use semaphore::{RtosSemaphore, SemaphoreHandle};
pub use task::{Priority, RtosTask, TaskHandle, TaskId, TaskState};
pub use timer::{RtosTimer, TimerConfig, TimerHandle, TimerMode};

/// System tick type (milliseconds since boot)
pub type Tick = u64;

/// Combined RTOS trait - users implement this for their RTOS
///
/// This super-trait combines all RTOS primitives. On embedded targets,
/// implementations delegate to the actual RTOS (FreeRTOS, RTIC, Embassy).
///
/// # Example
///
/// ```ignore
/// use dlms_rtos::Rtos;
///
/// struct FreeRtosAdapter;
///
/// impl Rtos for FreeRtosAdapter {
///     type Task = FreeRtosTask;
///     type Timer = FreeRtosTimer;
///     // ... delegate to FreeRTOS APIs
/// }
/// ```
pub trait Rtos:
    RtosTask + RtosTimer + RtosMutex + RtosSemaphore + RtosQueue + RtosMemPool + RtosInterrupt
{
    /// Get system tick count in milliseconds
    fn system_tick(&self) -> Tick;
}

// Include sub-modules
mod interrupt;
mod mempool;
mod mutex;
mod queue;
mod semaphore;
mod task;
mod timer;

// std implementation for testing
#[cfg(feature = "std")]
pub mod std_impl;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Realtime > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
        assert!(Priority::Low > Priority::Idle);
    }

    #[test]
    fn test_task_state_debug() {
        assert_eq!(alloc::format!("{:?}", TaskState::Ready), "Ready");
        assert_eq!(alloc::format!("{:?}", TaskState::Running), "Running");
        assert_eq!(alloc::format!("{:?}", TaskState::Blocked), "Blocked");
        assert_eq!(alloc::format!("{:?}", TaskState::Suspended), "Suspended");
    }
}
