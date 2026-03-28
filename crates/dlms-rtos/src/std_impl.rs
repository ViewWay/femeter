//! Standard library implementation for testing
//!
//! This module provides a simple std-based implementation of all RTOS traits.
//! It is NOT suitable for production use - only for testing protocol logic
//! on host machines.

#![cfg(feature = "std")]

// Import std explicitly - this is allowed when the crate feature is enabled
extern crate std;

use core::cell::{Cell, RefCell};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex as StdMutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use super::*;

/// System tick counter (static for singleton behavior)
static SYSTEM_TICK: AtomicU64 = AtomicU64::new(0);

/// Increment system tick (called internally)
fn tick_ms(millis: u64) {
    SYSTEM_TICK.fetch_add(millis, Ordering::Relaxed);
}

/// Get current system tick in milliseconds
pub fn get_system_tick() -> u64 {
    SYSTEM_TICK.load(Ordering::Relaxed)
}

/// Standard library RTOS implementation
///
/// This provides simple std-backed implementations of all RTOS traits.
/// NOT suitable for production - only for host-based testing.
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

impl TaskHandle for StdTaskHandle {
    fn id(&self) -> TaskId {
        self.id
    }

    fn state(&self) -> TaskState {
        *self.state.read().unwrap()
    }

    fn suspend(&self) {
        *self.state.write().unwrap() = TaskState::Suspended;
    }

    fn resume(&self) {
        if self.state() == TaskState::Suspended {
            *self.state.write().unwrap() = TaskState::Ready;
        }
    }

    fn priority(&self) -> Priority {
        *self.priority.read().unwrap()
    }

    fn set_priority(&self, priority: Priority) {
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
    id: u32,
    running: Arc<AtomicBool>,
    period: Arc<AtomicU32>,
    mode: Arc<RwLock<TimerMode>>,
    start_time: Arc<RwLock<Option<Instant>>>,
}

impl StdTimerHandle {
    fn new(id: u32, period_ms: u32, mode: TimerMode) -> Self {
        Self {
            id,
            running: Arc::new(AtomicBool::new(false)),
            period: Arc::new(AtomicU32::new(period_ms)),
            mode: Arc::new(RwLock::new(mode)),
            start_time: Arc::new(RwLock::new(None)),
        }
    }
}

impl TimerHandle for StdTimerHandle {
    fn start(&self) {
        self.running.store(true, Ordering::Release);
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
        *self.mode.read().unwrap()
    }

    fn remaining(&self) -> Option<u32> {
        if !self.is_running() {
            return None;
        }
        let start = *self.start_time.read().unwrap()?;
        let elapsed = start.elapsed().as_millis() as u32;
        self.period().checked_sub(elapsed)
    }
}

impl RtosTimer for StdRtos {
    type Handle = StdTimerHandle;

    fn create_timer(
        &self,
        _callback: TimerCallback,
        config: TimerConfig,
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
// Mutex Implementation
// ============================================================================

#[derive(Debug)]
pub struct StdMutex<T> {
    inner: StdMutex<T>,
}

impl<T> StdMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: StdMutex::new(value),
        }
    }
}

impl<T> MutexPtr<T> for StdMutex<T> {
    fn lock(&self) -> impl Deref<Target = T> {
        StdMutexGuard {
            inner: self.inner.lock().unwrap(),
        }
    }

    fn try_lock(&self) -> Option<impl Deref<Target = T>> {
        self.inner.try_lock().ok().map(|g| StdMutexGuard { inner: g })
    }
}

pub struct StdMutexGuard<'a, T> {
    inner: std::sync::MutexGuard<'a, T>,
}

impl<'a, T> Deref for StdMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> DerefMut for StdMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, T> Send for StdMutexGuard<'a, T> {}

impl RtosMutex for StdRtos {
    type Guard<'a, T> = StdMutexGuard<'a, T> where T: 'a, Self: 'a;

    fn create_mutex(&self) -> impl MutexPtr<T>
    where
        T: Default,
    {
        StdMutex::new(T::default())
    }

    fn create_mutex_with(&self, value: T) -> impl MutexPtr<T> {
        StdMutex::new(value)
    }

    fn lock(&self, mutex: &impl MutexPtr<T>) -> impl Deref<Target = T> {
        mutex.lock()
    }

    fn try_lock(&self, mutex: &impl MutexPtr<T>) -> Option<impl Deref<Target = T>> {
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

// Helper trait for downcasting
pub trait AsAny {
    fn as_any(&self) -> &dyn core::any::Any;
}

impl AsAny for StdSemaphoreHandle {
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

impl RtosSemaphore for StdRtos {
    type Handle = StdSemaphoreHandle;

    fn create_semaphore(&self, initial_count: u32, max_count: u32) -> impl SemaphoreHandle {
        StdSemaphoreHandle::new(initial_count, max_count)
    }

    fn acquire(&self, sem: &impl SemaphoreHandle) {
        let sem = sem.as_any().downcast_ref::<StdSemaphoreHandle>().unwrap();
        while sem.count.load(Ordering::Acquire) == 0 {
            thread::yield_now();
        }
        sem.count.fetch_sub(1, Ordering::AcqRel);
    }

    fn try_acquire(&self, sem: &impl SemaphoreHandle) -> bool {
        let sem = sem.as_any().downcast_ref::<StdSemaphoreHandle>().unwrap();
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

    fn try_acquire_timeout(&self, sem: &impl SemaphoreHandle, millis: u32) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(millis as u64) {
            if self.try_acquire(sem) {
                return true;
            }
            thread::yield_now();
        }
        false
    }

    fn release(&self, sem: &impl SemaphoreHandle) {
        let sem = sem.as_any().downcast_ref::<StdSemaphoreHandle>().unwrap();
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

    fn flush(&self, sem: &impl SemaphoreHandle) {
        let sem = sem.as_any().downcast_ref::<StdSemaphoreHandle>().unwrap();
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

impl<T: Send> QueueHandle for StdQueueHandle<T> {
    fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}

impl RtosQueue for StdRtos {
    type Handle<T> = StdQueueHandle<T> where T: Send;

    fn create_queue<T>(&self, capacity: usize) -> Result<Self::Handle<T>, super::super::QueueError>
    where
        T: Send,
    {
        if capacity == 0 {
            return Err(super::super::QueueError::InvalidSize);
        }
        Ok(StdQueueHandle::new(capacity))
    }

    fn send<T>(&self, queue: &Self::Handle<T>, item: T) -> Result<(), super::super::QueueError>
    where
        T: Send,
    {
        let mut q = queue.inner.write().unwrap();
        if q.len() >= queue.capacity {
            return Err(super::super::QueueError::Full);
        }
        q.push_back(item);
        Ok(())
    }

    fn try_send<T>(&self, queue: &Self::Handle<T>, item: T) -> Result<(), super::super::QueueError>
    where
        T: Send,
    {
        self.send(queue, item)
    }

    fn try_send_timeout<T>(
        &self,
        queue: &Self::Handle<T>,
        item: T,
        millis: u32,
    ) -> Result<(), super::super::QueueError>
    where
        T: Send,
    {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(millis as u64) {
            if self.try_send(queue, item).is_ok() {
                return Ok(());
            }
            thread::yield_now();
        }
        Err(super::super::QueueError::Full)
    }

    fn receive<T>(&self, queue: &Self::Handle<T>) -> Result<T, super::super::QueueError>
    where
        T: Send,
    {
        let mut q = queue.inner.write().unwrap();
        q.pop_front().ok_or(super::super::QueueError::Empty)
    }

    fn try_receive<T>(&self, queue: &Self::Handle<T>) -> Result<T, super::super::QueueError>
    where
        T: Send,
    {
        self.receive(queue)
    }

    fn try_receive_timeout<T>(
        &self,
        queue: &Self::Handle<T>,
        millis: u32,
    ) -> Result<T, super::super::QueueError>
    where
        T: Send,
    {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(millis as u64) {
            if let Ok(item) = self.try_receive(queue) {
                return Ok(item);
            }
            thread::yield_now();
        }
        Err(super::super::QueueError::Empty)
    }

    fn peek<T>(&self, queue: &Self::Handle<T>) -> Result<&T, super::super::QueueError>
    where
        T: Send,
    {
        let q = queue.inner.read().unwrap();
        q.front().ok_or(super::super::QueueError::Empty)
    }

    fn clear<T>(&self, queue: &Self::Handle<T>)
    where
        T: Send,
    {
        queue.inner.write().unwrap().clear();
    }
}

// ============================================================================
// Memory Pool Implementation
// ============================================================================

#[derive(Debug)]
pub struct StdMemPoolHandle {
    blocks: RefCell<Vec<Option<Box<[u8]>>>>,
    block_size: usize,
    free_count: Cell<usize>,
}

impl StdMemPoolHandle {
    fn new(block_size: usize, block_count: usize) -> Self {
        let mut blocks = Vec::with_capacity(block_count);
        for _ in 0..block_count {
            blocks.push(Some(vec![0u8; block_size].into_boxed_slice()));
        }
        Self {
            blocks: RefCell::new(blocks),
            block_size,
            free_count: Cell::new(block_count),
        }
    }
}

impl MemPoolHandle for StdMemPoolHandle {
    fn block_size(&self) -> usize {
        self.block_size
    }

    fn block_count(&self) -> usize {
        self.blocks.borrow().len()
    }

    fn free_count(&self) -> usize {
        self.free_count.get()
    }
}

impl RtosMemPool for StdRtos {
    type Handle = StdMemPoolHandle;

    fn create_pool(&self, config: PoolConfig) -> Result<Self::Handle, super::super::MemPoolError> {
        if config.block_size == 0 || config.block_count == 0 {
            return Err(super::super::MemPoolError::InvalidConfig);
        }
        Ok(StdMemPoolHandle::new(config.block_size, config.block_count))
    }

    fn allocate(&self, pool: &Self::Handle) -> Result<*mut u8, super::super::MemPoolError> {
        let mut blocks = pool.blocks.borrow_mut();
        for (i, block) in blocks.iter_mut().enumerate() {
            if block.is_some() {
                let ptr = block.as_ref().unwrap().as_ptr() as *mut u8;
                *block = None;
                pool.free_count.set(pool.free_count.get() - 1);
                return Ok(ptr);
            }
        }
        Err(super::super::MemPoolError::OutOfMemory)
    }

    fn deallocate(&self, pool: &Self::Handle, _block: *mut u8) -> Result<(), super::super::MemPoolError> {
        let mut blocks = pool.blocks.borrow_mut();
        for b in blocks.iter_mut() {
            if b.is_none() {
                let new_block = vec![0u8; pool.block_size].into_boxed_slice();
                *b = Some(new_block);
                pool.free_count.set(pool.free_count.get() + 1);
                return Ok(());
            }
        }
        Err(super::super::MemPoolError::InvalidBlock)
    }
}

// ============================================================================
// Interrupt Implementation
// ============================================================================

thread_local! {
    static INTERRUPT_STATE: Cell<bool> = Cell::new(true);
}

impl RtosInterrupt for StdRtos {
    fn disable_interrupts(&self) -> InterruptState {
        let was_enabled = INTERRUPT_STATE.get();
        INTERRUPT_STATE.set(false);
        InterruptState::new(was_enabled)
    }

    fn enable_interrupts(&self) {
        INTERRUPT_STATE.set(true);
    }

    fn restore_interrupt(&self, state: InterruptState) {
        INTERRUPT_STATE.set(state.was_enabled);
    }

    fn enter_critical(&self) -> InterruptGuard {
        let state = self.disable_interrupts();
        InterruptGuard::new(state, |s| Self::restore_direct(s))
    }

    fn is_in_isr(&self) -> bool {
        false
    }

    fn are_interrupts_disabled(&self) -> bool {
        !INTERRUPT_STATE.get()
    }
}

impl StdRtos {
    fn restore_direct(state: InterruptState) {
        INTERRUPT_STATE.set(state.was_enabled);
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
        let config = TimerConfig::one_shot(1000);
        assert_eq!(config.mode, TimerMode::OneShot);
        assert_eq!(config.period_ms, 1000);
        assert!(!config.auto_start);

        let config = TimerConfig::periodic(500).auto_start();
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
        assert_eq!(rtos.try_send(&queue, 3), Err(super::super::QueueError::Full));
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
        assert_eq!(rtos.allocate(&pool), Err(super::super::MemPoolError::OutOfMemory));
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
