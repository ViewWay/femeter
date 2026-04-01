//! Standard library implementation for testing
//!
//! This module provides a simple std-based implementation of all RTOS traits.
//! It is NOT suitable for production use - only for testing protocol logic
//! on host machines.

#![cfg(feature = "std")]

#[cfg(feature = "std")]
extern crate std;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::interrupt::{InterruptGuard, InterruptState, RtosInterrupt};
use crate::mempool::{MemPoolError, MemPoolHandle, PoolConfig, RtosMemPool};
use crate::queue::{QueueError, QueueHandle, RtosQueue};
use crate::semaphore::{RtosSemaphore, SemaphoreHandle};
use crate::task::{
    Priority, RtosTask, TaskConfig, TaskError, TaskFn, TaskHandle as CrateTaskHandle, TaskId,
    TaskState,
};
use crate::timer::{
    RtosTimer, TimerCallback, TimerConfig as TimerConfigType, TimerError,
    TimerHandle as CrateTimerHandle, TimerMode,
};
use crate::{Rtos, Tick};

/// System tick counter
static SYSTEM_TICK: AtomicU64 = AtomicU64::new(0);

fn tick_ms(millis: u64) {
    SYSTEM_TICK.fetch_add(millis, Ordering::Relaxed);
}

pub fn get_system_tick() -> u64 {
    SYSTEM_TICK.load(Ordering::Relaxed)
}

/// Standard library RTOS implementation
#[derive(Debug, Default)]
pub struct StdRtos;

impl StdRtos {
    pub const fn new() -> Self {
        Self
    }

    pub fn advance_time(&self, millis: u64) {
        tick_ms(millis);
    }
}

// ============================================================================
// Task Implementation
// ============================================================================

static NEXT_TASK_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone)]
pub struct StdTaskHandle {
    id: TaskId,
    state: Arc<RwLock<TaskState>>,
    priority: Arc<RwLock<Priority>>,
}

impl StdTaskHandle {
    fn new(id: TaskId, priority: Priority) -> Self {
        Self {
            id,
            state: Arc::new(RwLock::new(TaskState::Ready)),
            priority: Arc::new(RwLock::new(priority)),
        }
    }
}

impl CrateTaskHandle for StdTaskHandle {
    fn id(&self) -> TaskId {
        self.id
    }

    fn state(&self) -> TaskState {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        *self.state.read().unwrap()
    }

    fn suspend(&self) {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        *self.state.write().unwrap() = TaskState::Suspended;
    }

    fn resume(&self) {
        if self.state() == TaskState::Suspended {
            // Safety: In single-threaded std tests, RwLock won't be poisoned.
            // A poisoned lock indicates a test panic, so panicking here is appropriate.
            *self.state.write().unwrap() = TaskState::Ready;
        }
    }

    fn priority(&self) -> Priority {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        *self.priority.read().unwrap()
    }

    fn set_priority(&self, priority: Priority) {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        *self.priority.write().unwrap() = priority;
    }
}

impl RtosTask for StdRtos {
    type Handle = StdTaskHandle;

    fn spawn_task(&self, f: TaskFn, config: TaskConfig) -> Result<Self::Handle, TaskError> {
        let id = TaskId(NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst));
        let _handle = StdTaskHandle::new(id, config.priority);

        thread::spawn(move || {
            f();
        });

        Ok(StdTaskHandle::new(id, config.priority))
    }

    fn yield_now(&self) {
        thread::yield_now();
    }

    fn sleep(&self, millis: u32) {
        thread::sleep(Duration::from_millis(millis as u64));
    }

    fn current_task(&self) -> Option<Self::Handle> {
        None
    }

    fn task_count(&self) -> usize {
        0
    }
}

// ============================================================================
// Timer Implementation
// ============================================================================

static NEXT_TIMER_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone)]
pub struct StdTimerHandle {
    _id: u32,
    running: Arc<AtomicBool>,
    period: Arc<AtomicU32>,
    mode: Arc<RwLock<TimerMode>>,
    start_time: Arc<RwLock<Option<Instant>>>,
}

impl StdTimerHandle {
    fn new(_id: u32, period_ms: u32, mode: TimerMode) -> Self {
        Self {
            _id,
            running: Arc::new(AtomicBool::new(false)),
            period: Arc::new(AtomicU32::new(period_ms)),
            mode: Arc::new(RwLock::new(mode)),
            start_time: Arc::new(RwLock::new(None)),
        }
    }
}

impl CrateTimerHandle for StdTimerHandle {
    fn start(&self) {
        self.running.store(true, Ordering::Release);
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        *self.start_time.write().unwrap() = Some(Instant::now());
    }

    fn stop(&self) -> bool {
        let was_running = self.running.load(Ordering::Acquire);
        self.running.store(false, Ordering::Release);
        was_running
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    fn period(&self) -> u32 {
        self.period.load(Ordering::Acquire)
    }

    fn mode(&self) -> TimerMode {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        *self.mode.read().unwrap()
    }

    fn remaining(&self) -> Option<u32> {
        if !self.is_running() {
            return None;
        }
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        let start_guard = self.start_time.read().unwrap();
        let start = *start_guard.as_ref()?;
        drop(start_guard);
        let elapsed = start.elapsed().as_millis() as u32;
        self.period().checked_sub(elapsed)
    }
}

impl RtosTimer for StdRtos {
    type Handle = StdTimerHandle;

    fn create_timer(
        &self,
        _callback: TimerCallback,
        config: TimerConfigType,
    ) -> Result<Self::Handle, TimerError> {
        let id = NEXT_TIMER_ID.fetch_add(1, Ordering::SeqCst);
        let handle = StdTimerHandle::new(id, config.period_ms, config.mode);

        if config.auto_start {
            handle.start();
        }

        Ok(handle)
    }

    fn timer_count(&self) -> usize {
        0
    }

    fn tick_timers(&self) {}
}

// ============================================================================
// Mutex Implementation (minimal, just to satisfy trait bounds)
// ============================================================================

use crate::mutex::{MutexGuard, MutexPtr, RtosMutex};

// Minimal mutex wrapper - for testing only
pub struct StdMutex<T> {
    inner: RwLock<T>,
}

impl<T> StdMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: RwLock::new(value),
        }
    }
}

// SAFETY: RwLock is Send + Sync when T is Send
unsafe impl<T: Send> Send for StdMutex<T> {}
unsafe impl<T: Send> Sync for StdMutex<T> {}

#[allow(refining_impl_trait)]
impl<T: Send> MutexPtr<T> for StdMutex<T> {
    fn lock(&self) -> StdMutexGuardRef<'_, T> {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        StdMutexGuardRef {
            _borrow: self.inner.write().unwrap(),
        }
    }

    fn try_lock(&self) -> Option<StdMutexGuardRef<'_, T>> {
        self.inner
            .try_write()
            .ok()
            .map(|w| StdMutexGuardRef { _borrow: w })
    }
}

pub struct StdMutexGuardRef<'a, T> {
    _borrow: std::sync::RwLockWriteGuard<'a, T>,
}

impl<'a, T> core::ops::Deref for StdMutexGuardRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self._borrow
    }
}

impl<'a, T> core::ops::DerefMut for StdMutexGuardRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self._borrow
    }
}

unsafe impl<'a, T: Send> Send for StdMutexGuardRef<'a, T> {}

impl<'a, T> MutexGuard<'a, T> for StdMutexGuardRef<'a, T> {}

impl RtosMutex for StdRtos {
    type MutexPtr<T: Send> = StdMutex<T>;
    type Guard<'a, T: Send>
        = StdMutexGuardRef<'a, T>
    where
        T: 'a,
        Self: 'a;

    fn create_mutex<T: Default + Send>(&self) -> StdMutex<T> {
        StdMutex::new(T::default())
    }

    fn create_mutex_with<T: Send>(&self, value: T) -> StdMutex<T> {
        StdMutex::new(value)
    }

    fn lock<'a, T: Send>(&self, mutex: &'a StdMutex<T>) -> StdMutexGuardRef<'a, T> {
        mutex.lock()
    }

    fn try_lock<'a, T: Send>(&self, mutex: &'a StdMutex<T>) -> Option<StdMutexGuardRef<'a, T>> {
        mutex.try_lock()
    }
}

// ============================================================================
// Semaphore Implementation
// ============================================================================

#[derive(Debug, Clone)]
pub struct StdSemaphoreHandle {
    count: Arc<AtomicU32>,
    max_count: u32,
}

impl StdSemaphoreHandle {
    fn new(initial_count: u32, max_count: u32) -> Self {
        Self {
            count: Arc::new(AtomicU32::new(initial_count)),
            max_count,
        }
    }
}

impl SemaphoreHandle for StdSemaphoreHandle {
    fn count(&self) -> u32 {
        self.count.load(Ordering::Acquire)
    }

    fn max_count(&self) -> u32 {
        self.max_count
    }
}

impl RtosSemaphore for StdRtos {
    type Handle = StdSemaphoreHandle;

    fn create_semaphore(&self, initial_count: u32, max_count: u32) -> Self::Handle {
        StdSemaphoreHandle::new(initial_count, max_count)
    }

    fn acquire(&self, sem: &Self::Handle) {
        while sem.count.load(Ordering::Acquire) == 0 {
            thread::yield_now();
        }
        sem.count.fetch_sub(1, Ordering::AcqRel);
    }

    fn try_acquire(&self, sem: &Self::Handle) -> bool {
        let mut count = sem.count.load(Ordering::Acquire);
        while count > 0 {
            match sem.count.compare_exchange_weak(
                count,
                count - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(c) => count = c,
            }
        }
        false
    }

    fn try_acquire_timeout(&self, sem: &Self::Handle, millis: u32) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(millis as u64) {
            if self.try_acquire(sem) {
                return true;
            }
            thread::yield_now();
        }
        false
    }

    fn release(&self, sem: &Self::Handle) {
        let mut count = sem.count.load(Ordering::Acquire);
        while count < sem.max_count {
            match sem.count.compare_exchange_weak(
                count,
                count + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(c) => count = c,
            }
        }
    }

    fn flush(&self, sem: &Self::Handle) {
        sem.count.store(sem.max_count, Ordering::Release);
    }
}

// ============================================================================
// Queue Implementation
// ============================================================================

#[derive(Debug, Clone)]
pub struct StdQueueHandle<T: Send> {
    inner: Arc<RwLock<VecDeque<T>>>,
    capacity: usize,
}

impl<T: Send> StdQueueHandle<T> {
    fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }
}

// SAFETY: Only used in single-threaded tests
unsafe impl<T: Send> Send for StdQueueHandle<T> {}
unsafe impl<T: Send> Sync for StdQueueHandle<T> {}

impl<T: Send> QueueHandle for StdQueueHandle<T> {
    fn len(&self) -> usize {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        self.inner.read().unwrap().len()
    }

    fn is_empty(&self) -> bool {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        self.inner.read().unwrap().is_empty()
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}

impl RtosQueue for StdRtos {
    type Handle<T: Send> = StdQueueHandle<T>;

    fn create_queue<T: Send>(&self, capacity: usize) -> Result<Self::Handle<T>, QueueError> {
        if capacity == 0 {
            return Err(QueueError::InvalidSize);
        }
        Ok(StdQueueHandle::new(capacity))
    }

    fn send<T: Send>(&self, queue: &StdQueueHandle<T>, item: T) -> Result<(), QueueError> {
        let mut q = queue.inner.write().map_err(|_| QueueError::InvalidSize)?;
        if q.len() >= queue.capacity {
            return Err(QueueError::Full);
        }
        q.push_back(item);
        Ok(())
    }

    fn try_send<T: Send>(&self, queue: &StdQueueHandle<T>, item: T) -> Result<(), QueueError> {
        self.send(queue, item)
    }

    fn try_send_timeout<T: Send>(
        &self,
        queue: &StdQueueHandle<T>,
        item: T,
        millis: u32,
    ) -> Result<(), QueueError> {
        let start = Instant::now();
        let timeout = Duration::from_millis(millis as u64);

        while start.elapsed() < timeout {
            // Check if queue has space before attempting send
            let q = queue.inner.read().map_err(|_| QueueError::InvalidSize)?;
            if q.len() < queue.capacity {
                drop(q);
                return self.send(queue, item);
            }
            drop(q);
            thread::yield_now();
        }
        // One final attempt
        self.try_send(queue, item)
    }

    fn receive<T: Send>(&self, queue: &StdQueueHandle<T>) -> Result<T, QueueError> {
        let mut q = queue.inner.write().map_err(|_| QueueError::InvalidSize)?;
        q.pop_front().ok_or(QueueError::Empty)
    }

    fn try_receive<T: Send>(&self, queue: &StdQueueHandle<T>) -> Result<T, QueueError> {
        self.receive(queue)
    }

    fn try_receive_timeout<T: Send>(
        &self,
        queue: &StdQueueHandle<T>,
        millis: u32,
    ) -> Result<T, QueueError> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(millis as u64) {
            if let Ok(item) = self.try_receive(queue) {
                return Ok(item);
            }
            thread::yield_now();
        }
        Err(QueueError::Empty)
    }

    fn clear<T: Send>(&self, queue: &StdQueueHandle<T>) {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        // This method returns (), so we cannot return an error.
        let _ = queue.inner.write().map(|mut q| q.clear());
    }
}

// ============================================================================
// Memory Pool Implementation
// ============================================================================

/// Type alias for memory pool blocks to reduce type complexity
type MemPoolBlocks = Arc<RwLock<Vec<Option<Box<[u8]>>>>>;

#[derive(Debug)]
pub struct StdMemPoolHandle {
    blocks: MemPoolBlocks,
    block_size: usize,
    free_count: AtomicUsize,
}

impl StdMemPoolHandle {
    fn new(block_size: usize, block_count: usize) -> Self {
        let mut blocks = Vec::with_capacity(block_count);
        for _ in 0..block_count {
            blocks.push(Some(std::vec![0u8; block_size].into_boxed_slice()));
        }
        Self {
            blocks: Arc::new(RwLock::new(blocks)),
            block_size,
            free_count: AtomicUsize::new(block_count),
        }
    }
}

impl MemPoolHandle for StdMemPoolHandle {
    fn block_size(&self) -> usize {
        self.block_size
    }

    fn block_count(&self) -> usize {
        // Safety: In single-threaded std tests, RwLock won't be poisoned.
        // A poisoned lock indicates a test panic, so panicking here is appropriate.
        self.blocks.read().unwrap().len()
    }

    fn free_count(&self) -> usize {
        self.free_count.load(Ordering::Acquire)
    }
}

impl RtosMemPool for StdRtos {
    type Handle = StdMemPoolHandle;

    fn create_pool(&self, config: PoolConfig) -> Result<Self::Handle, MemPoolError> {
        if config.block_size == 0 || config.block_count == 0 {
            return Err(MemPoolError::InvalidConfig);
        }
        Ok(StdMemPoolHandle::new(config.block_size, config.block_count))
    }

    fn allocate(&self, pool: &StdMemPoolHandle) -> Result<*mut u8, MemPoolError> {
        let mut blocks = pool
            .blocks
            .write()
            .map_err(|_| MemPoolError::InvalidConfig)?;
        for block in blocks.iter_mut() {
            if block.is_some() {
                let ptr = block
                    .as_ref()
                    .expect("block.is_some() guarantees Some")
                    .as_ptr() as *mut u8;
                *block = None;
                pool.free_count.fetch_sub(1, Ordering::AcqRel);
                return Ok(ptr);
            }
        }
        Err(MemPoolError::OutOfMemory)
    }

    fn deallocate(&self, pool: &StdMemPoolHandle, _block: *mut u8) -> Result<(), MemPoolError> {
        let mut blocks = pool
            .blocks
            .write()
            .map_err(|_| MemPoolError::InvalidConfig)?;
        for _b in blocks.iter_mut() {
            if _b.is_none() {
                let new_block = std::vec![0u8; pool.block_size].into_boxed_slice();
                *_b = Some(new_block);
                pool.free_count.fetch_add(1, Ordering::AcqRel);
                return Ok(());
            }
        }
        Err(MemPoolError::InvalidBlock)
    }
}

// ============================================================================
// Interrupt Implementation
// ============================================================================

std::thread_local! {
    static INTERRUPT_STATE: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

impl RtosInterrupt for StdRtos {
    fn disable_interrupts(&self) -> InterruptState {
        let was_enabled = INTERRUPT_STATE.with(|s| s.get());
        INTERRUPT_STATE.with(|s| s.set(false));
        InterruptState::new(was_enabled)
    }

    fn enable_interrupts(&self) {
        INTERRUPT_STATE.with(|s| s.set(true));
    }

    fn restore_interrupt(&self, state: InterruptState) {
        INTERRUPT_STATE.with(|s| s.set(state.was_enabled));
    }

    fn enter_critical(&self) -> InterruptGuard {
        let state = self.disable_interrupts();
        let restore_fn: fn(InterruptState) = |s| {
            INTERRUPT_STATE.with(|state| state.set(s.was_enabled));
        };
        InterruptGuard::new(state, restore_fn)
    }

    fn is_in_isr(&self) -> bool {
        false
    }

    fn are_interrupts_disabled(&self) -> bool {
        !INTERRUPT_STATE.with(|s| s.get())
    }
}

// ============================================================================
// Rtos Implementation
// ============================================================================

impl Rtos for StdRtos {
    fn system_tick(&self) -> Tick {
        get_system_tick()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_std_rtos_create() {
        let _rtos = StdRtos::new();
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Realtime > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
        assert!(Priority::Low > Priority::Idle);
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

    #[test]
    fn test_timer_config() {
        let config = TimerConfigType::one_shot(1000);
        assert_eq!(config.mode, TimerMode::OneShot);
        assert_eq!(config.period_ms, 1000);
        assert!(!config.auto_start);

        let config = TimerConfigType::periodic(500).auto_start();
        assert_eq!(config.mode, TimerMode::Periodic);
        assert_eq!(config.period_ms, 500);
        assert!(config.auto_start);
    }

    #[test]
    fn test_pool_config() {
        let config = PoolConfig::new(256, 10);
        assert_eq!(config.block_size, 256);
        assert_eq!(config.block_count, 10);
        assert_eq!(config.total_size(), 2560);
    }

    #[test]
    fn test_semaphore_create() {
        let rtos = StdRtos::new();
        let sem = rtos.create_semaphore(5, 10);
        assert_eq!(sem.count(), 5);
        assert_eq!(sem.max_count(), 10);
    }

    #[test]
    fn test_semaphore_acquire_release() {
        let rtos = StdRtos::new();
        let sem = rtos.create_semaphore(1, 1);

        assert!(rtos.try_acquire(&sem));
        assert_eq!(sem.count(), 0);
        assert!(!rtos.try_acquire(&sem));

        rtos.release(&sem);
        assert_eq!(sem.count(), 1);
        assert!(rtos.try_acquire(&sem));
    }

    #[test]
    fn test_queue_create() {
        let rtos = StdRtos::new();
        let queue: StdQueueHandle<u32> = rtos.create_queue(10).unwrap();
        assert_eq!(queue.capacity(), 10);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_send_receive() {
        let rtos = StdRtos::new();
        let queue: StdQueueHandle<u32> = rtos.create_queue(10).unwrap();

        rtos.send(&queue, 42).unwrap();
        assert_eq!(queue.len(), 1);

        let value = rtos.receive(&queue).unwrap();
        assert_eq!(value, 42);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_full() {
        let rtos = StdRtos::new();
        let queue: StdQueueHandle<u32> = rtos.create_queue(2).unwrap();

        rtos.send(&queue, 1).unwrap();
        rtos.send(&queue, 2).unwrap();
        assert_eq!(rtos.try_send(&queue, 3), Err(QueueError::Full));
    }

    #[test]
    fn test_mempool_create() {
        let rtos = StdRtos::new();
        let config = PoolConfig::new(256, 10);
        let pool = rtos.create_pool(config).unwrap();
        assert_eq!(pool.block_size(), 256);
        assert_eq!(pool.block_count(), 10);
        assert_eq!(pool.free_count(), 10);
    }

    #[test]
    fn test_mempool_allocate() {
        let rtos = StdRtos::new();
        let config = PoolConfig::new(256, 10);
        let pool = rtos.create_pool(config).unwrap();

        let block1 = rtos.allocate(&pool).unwrap();
        assert!(!block1.is_null());
        assert_eq!(pool.free_count(), 9);

        let block2 = rtos.allocate(&pool).unwrap();
        assert!(!block2.is_null());
        assert_eq!(pool.free_count(), 8);
    }

    #[test]
    fn test_mempool_out_of_memory() {
        let rtos = StdRtos::new();
        let config = PoolConfig::new(256, 1);
        let pool = rtos.create_pool(config).unwrap();

        rtos.allocate(&pool).unwrap();
        assert_eq!(rtos.allocate(&pool), Err(MemPoolError::OutOfMemory));
    }

    #[test]
    fn test_interrupt_state() {
        let rtos = StdRtos::new();

        assert!(rtos.are_interrupts_enabled());
        assert!(!rtos.are_interrupts_disabled());
        assert!(!rtos.is_in_isr());

        let state = rtos.disable_interrupts();
        assert!(rtos.are_interrupts_disabled());

        rtos.restore_interrupt(state);
        assert!(rtos.are_interrupts_enabled());
    }

    #[test]
    fn test_critical_section() {
        let rtos = StdRtos::new();

        {
            let _guard = rtos.enter_critical();
            assert!(rtos.are_interrupts_disabled());
        }
        assert!(rtos.are_interrupts_enabled());
    }

    #[test]
    fn test_system_tick() {
        let rtos = StdRtos::new();
        let tick1 = rtos.system_tick();
        rtos.advance_time(100);
        let tick2 = rtos.system_tick();
        assert_eq!(tick2 - tick1, 100);
    }
}
