/* ================================================================== */
/*                                                                    */
/*  freertos.rs — FreeRTOS Rust FFI 绑定 + 安全封装                    */
/*                                                                    */
/*  MCU: FM33A068EV (Cortex-M0+ @ 64MHz)                              */
/*  提供任务、队列、信号量、事件组的 Rust 接口                           */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use core::ffi::{c_void, c_char};
use core::ptr;
#[allow(unused_imports)]
use core::time::Duration;

/* ================================================================== */
/*  基础类型                                                           */
/* ================================================================== */

/// FreeRTOS TickType (32-bit, configUSE_16_BIT_TICKS = 0)
pub type TickType = u32;

/// FreeRTOS 任务句柄
pub type TaskHandle = *mut c_void;

/// FreeRTOS 队列句柄
pub type QueueHandle = *mut c_void;

/// FreeRTOS 信号量/互斥量句柄
pub type SemaphoreHandle = *mut c_void;

/// FreeRTOS 事件组句柄
pub type EventGroupHandle = *mut c_void;

/// FreeRTOS 定时器句柄
pub type TimerHandle = *mut c_void;

/// 任务函数指针类型 (C ABI)
pub type TaskFunction = unsafe extern "C" fn(*mut c_void);

/// 定时器回调类型 (C ABI)
pub type TimerCallback = unsafe extern "C" fn(TimerHandle);

/// 无限等待
pub const PORT_MAX_DELAY: TickType = 0xFFFFFFFF;

/// ms 转 ticks
pub const fn ms_to_ticks(ms: u32) -> TickType {
    // configTICK_RATE_HZ = 1000, 所以 1 tick = 1 ms
    ms
}

/* ================================================================== */
/*  FFI 声明 (extern "C")                                             */
/* ================================================================== */

extern "C" {
    /* ── 任务管理 ── */
    pub fn xTaskCreate(
        pvTaskCode: TaskFunction,
        pcName: *const c_char,
        usStackDepth: u16,
        pvParameters: *mut c_void,
        uxPriority: u32,
        pxCreatedTask: *mut TaskHandle,
    ) -> i32;  // pdPASS = 1
    pub fn vTaskDelete(pxTask: TaskHandle);
    pub fn vTaskDelay(xTicksToDelay: TickType);
    pub fn xTaskGetTickCount() -> TickType;
    pub fn vTaskSuspend(xTask: TaskHandle);
    pub fn vTaskResume(xTask: TaskHandle);
    pub fn xTaskGetCurrentTaskHandle() -> TaskHandle;

    /* ── 队列 ── */
    /* FreeRTOS 11.x: xQueueCreate 是宏 → xQueueGenericCreate */
    pub fn xQueueGenericCreate(uxQueueLength: u32, uxItemSize: u32, ucQueueType: u8) -> QueueHandle;
    /* FreeRTOS 11.x: xQueueSend 是宏 → xQueueGenericSend */
    pub fn xQueueGenericSend(
        xQueue: QueueHandle,
        pvItemToQueue: *const c_void,
        xTicksToWait: TickType,
        xCopyPosition: u32,
    ) -> i32;
    pub fn xQueueReceive(
        xQueue: QueueHandle,
        pvBuffer: *mut c_void,
        xTicksToWait: TickType,
    ) -> i32;
    /* FreeRTOS 11.x: xQueueSendFromISR 是宏 → xQueueGenericSendFromISR */
    pub fn xQueueGenericSendFromISR(
        xQueue: QueueHandle,
        pvItemToQueue: *const c_void,
        pxHigherPriorityTaskWoken: *mut i32,
        xCopyPosition: u32,
    ) -> i32;
    pub fn xQueueReceiveFromISR(
        xQueue: QueueHandle,
        pvBuffer: *mut c_void,
        pxHigherPriorityTaskWoken: *mut i32,
    ) -> i32;
    pub fn uxQueueMessagesWaiting(xQueue: QueueHandle) -> u32;
    /* FreeRTOS 11.x: xQueueReset 是宏 → xQueueGenericReset */
    pub fn xQueueGenericReset(xQueue: QueueHandle, xNewQueue: i32) -> i32;

    /* ── 信号量 ── */
    /* FreeRTOS 11.x: semaphore API 是宏，映射到 xQueue* 内部函数 */
    pub fn xQueueCreateMutex(ucQueueType: u8) -> SemaphoreHandle;
    pub fn xQueueSemaphoreTake(xSemaphore: SemaphoreHandle, xTicksToWait: TickType) -> i32;

    /* ── 事件组 ── */
    pub fn xEventGroupCreate() -> EventGroupHandle;
    pub fn xEventGroupSetBits(xEventGroup: EventGroupHandle, uxBitsToSet: u32) -> u32;
    pub fn xEventGroupClearBits(xEventGroup: EventGroupHandle, uxBitsToClear: u32) -> u32;
    pub fn xEventGroupWaitBits(
        xEventGroup: EventGroupHandle,
        uxBitsToWaitFor: u32,
        xClearOnExit: i32,
        xWaitForAllBits: i32,
        xTicksToWait: TickType,
    ) -> u32;
    pub fn xEventGroupSetBitsFromISR(
        xEventGroup: EventGroupHandle,
        uxBitsToSet: u32,
        pxHigherPriorityTaskWoken: *mut i32,
    ) -> i32;

    /* ── 软件定时器 ── */
    pub fn xTimerCreate(
        pcTimerName: *const c_char,
        xTimerPeriodInTicks: TickType,
        uxAutoReload: u32,
        pvTimerID: *mut c_void,
        pxCallbackFunction: TimerCallback,
    ) -> TimerHandle;
    pub fn xTimerStart(xTimer: TimerHandle, xTicksToWait: TickType) -> i32;
    pub fn xTimerStop(xTimer: TimerHandle, xTicksToWait: TickType) -> i32;
    pub fn xTimerReset(xTimer: TimerHandle, xTicksToWait: TickType) -> i32;
    pub fn xTimerChangePeriod(xTimer: TimerHandle, xNewPeriod: TickType, xTicksToWait: TickType) -> i32;
    pub fn pvTimerGetTimerID(xTimer: TimerHandle) -> *mut c_void;
    pub fn vTimerSetTimerID(xTimer: TimerHandle, pvNewID: *mut c_void);

    /* ── 调度器 ── */
    pub fn vTaskStartScheduler() -> !;
}

/* ================================================================== */
/*  安全封装: Task                                                     */
/* ================================================================== */

/// 任务创建参数
pub struct TaskParams {
    /// 任务名称 (ASCII, 最多 11 字符 + null)
    pub name: &'static str,
    /// 栈深度 (words, 最小 64)
    pub stack_depth: u16,
    /// 优先级 (1 = 最低, configMAX_PRIORITIES-1 = 最高)
    pub priority: u32,
    /// 传给任务函数的参数
    pub arg: *mut c_void,
}

/// 创建并启动 FreeRTOS 任务
///
/// 返回 Ok(句柄) 或 Err(()) (创建失败, 通常是内存不足)
pub fn task_create(
    func: TaskFunction,
    params: TaskParams,
) -> Result<TaskHandle, ()> {
    let mut handle: TaskHandle = ptr::null_mut();
    // 确保名称以 null 结尾
    let name_bytes: [u8; 16] = {
        let mut buf = [0u8; 16];
        let src = params.name.as_bytes();
        let len = src.len().min(15);
        buf[..len].copy_from_slice(&src[..len]);
        buf
    };

    let result = unsafe {
        xTaskCreate(
            func,
            name_bytes.as_ptr() as *const c_char,
            params.stack_depth,
            params.arg,
            params.priority,
            &mut handle,
        )
    };

    if result == 1 {  // pdPASS
        Ok(handle)
    } else {
        Err(())
    }
}

/// 延时当前任务
pub fn delay_ms(ms: u32) {
    unsafe { vTaskDelay(ms_to_ticks(ms)) }
}

/// 获取系统 tick 计数 (ms)
pub fn tick_count() -> TickType {
    unsafe { xTaskGetTickCount() }
}

/* ================================================================== */
/*  安全封装: Queue<T>                                                 */
/* ================================================================== */

/// 类型安全的 FreeRTOS 队列
///
/// 注意: T 必须是 #[repr(C)] 且不含引用类型
pub struct Queue<T> {
    handle: QueueHandle,
    _marker: core::marker::PhantomData<T>,
}

impl<T> Queue<T> {
    /// 从原始指针创建 Queue 引用
    ///
    /// Safety: 指针必须来自 into_raw() 且未被销毁
    pub unsafe fn from_raw(ptr: *mut c_void) -> &'static Self {
        &*(ptr as *const Self)
    }

    /// 消耗 Queue, 返回原始指针
    pub fn into_raw(q: Self) -> *mut c_void {
        let ptr = q.handle;
        core::mem::forget(q);
        ptr as *mut c_void
    }

    /// FreeRTOS 队列类型: queueQUEUE_TYPE_BASE = 0
    const QUEUE_TYPE_BASE: u8 = 0;
    /// FreeRTOS 发送位置: queueSEND_TO_BACK = 1
    const SEND_TO_BACK: u32 = 1;

    /// 创建队列
    /// capacity: 队列深度
    pub fn new(capacity: u32) -> Option<Self> {
        let handle = unsafe { xQueueGenericCreate(capacity, core::mem::size_of::<T>() as u32, Self::QUEUE_TYPE_BASE) };
        if handle.is_null() {
            None
        } else {
            Some(Self {
                handle,
                _marker: core::marker::PhantomData,
            })
        }
    }

    /// 发送数据到队列 (ISR 中使用 send_from_isr)
    pub fn send(&self, item: &T, timeout_ms: u32) -> bool {
        unsafe {
            xQueueGenericSend(self.handle, item as *const T as *const c_void, ms_to_ticks(timeout_ms), Self::SEND_TO_BACK) == 1
        }
    }

    /// 从队列接收数据
    pub fn receive(&self, buf: &mut T, timeout_ms: u32) -> bool {
        unsafe {
            xQueueReceive(self.handle, buf as *mut T as *mut c_void, ms_to_ticks(timeout_ms)) == 1
        }
    }

    /// 查询队列中消息数量
    pub fn len(&self) -> u32 {
        unsafe { uxQueueMessagesWaiting(self.handle) }
    }

    /// 队列是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// Safety: FreeRTOS Queue 是内核管理的, 可以跨线程使用
unsafe impl<T: Send> Send for Queue<T> {}
unsafe impl<T: Send> Sync for Queue<T> {}

/* ================================================================== */
/*  安全封装: Mutex                                                    */
/* ================================================================== */

/// FreeRTOS 互斥量
pub struct Mutex {
    handle: SemaphoreHandle,
}

impl Mutex {
    /// 从原始指针创建 Mutex 引用 (用于全局静态指针)
    ///
    /// Safety: 指针必须来自 into_raw() 且未被销毁
    pub unsafe fn from_raw(ptr: *mut c_void) -> &'static Self {
        &*(ptr as *const Self)
    }

    /// 消耗 Mutex, 返回原始指针
    pub fn into_raw(m: Self) -> *mut c_void {
        let ptr = m.handle;
        core::mem::forget(m);
        ptr as *mut c_void
    }

    /// 创建互斥量
    pub fn new() -> Option<Self> {
        /* queueQUEUE_TYPE_MUTEX = 1 */
        let handle = unsafe { xQueueCreateMutex(1u8) };
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// 获取互斥量 (阻塞直到获取成功)
    pub fn lock(&self, timeout_ms: u32) -> bool {
        unsafe { xQueueSemaphoreTake(self.handle, ms_to_ticks(timeout_ms)) == 1 }
    }

    /// 释放互斥量
    pub fn unlock(&self) -> bool {
        unsafe {
            // xSemaphoreGive -> xQueueGenericSend(queue, NULL, 0, queueSEND_TO_BACK)
            xQueueGenericSend(self.handle, ptr::null(), 0, 1) == 1
        }
    }
}

// Safety: FreeRTOS Mutex 本身是线程安全的同步原语
unsafe impl Send for Mutex {}
unsafe impl Sync for Mutex {}

/// RAII Mutex Guard
pub struct MutexGuard<'a> {
    mutex: &'a Mutex,
}

impl<'a> MutexGuard<'a> {
    fn new(mutex: &'a Mutex) -> Option<Self> {
        if mutex.lock(PORT_MAX_DELAY) {
            Some(Self { mutex })
        } else {
            None
        }
    }
}

impl Drop for MutexGuard<'_> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

/* ================================================================== */
/*  安全封装: EventGroup                                               */
/* ================================================================== */

/// FreeRTOS 事件组
pub struct EventGroup {
    handle: EventGroupHandle,
}

impl EventGroup {
    /// 从原始指针创建 EventGroup 引用
    ///
    /// Safety: 指针必须来自 into_raw() 且未被销毁
    pub unsafe fn from_raw(ptr: *mut c_void) -> &'static Self {
        &*(ptr as *const Self)
    }

    /// 消耗 EventGroup, 返回原始指针
    pub fn into_raw(eg: Self) -> *mut c_void {
        let ptr = eg.handle;
        core::mem::forget(eg);
        ptr as *mut c_void
    }

    pub fn new() -> Option<Self> {
        let handle = unsafe { xEventGroupCreate() };
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// 设置事件位
    pub fn set(&self, bits: u32) -> u32 {
        unsafe { xEventGroupSetBits(self.handle, bits) }
    }

    /// 清除事件位
    pub fn clear(&self, bits: u32) -> u32 {
        unsafe { xEventGroupClearBits(self.handle, bits) }
    }

    /// 等待事件位
    ///
    /// wait_all: true = 等待所有指定位就绪, false = 任意一位就绪
    /// clear_on_exit: true = 返回前清除等待的位
    pub fn wait(&self, bits: u32, wait_all: bool, clear_on_exit: bool, timeout_ms: u32) -> u32 {
        unsafe {
            xEventGroupWaitBits(
                self.handle,
                bits,
                if clear_on_exit { 1 } else { 0 },
                if wait_all { 1 } else { 0 },
                ms_to_ticks(timeout_ms),
            )
        }
    }
}

unsafe impl Send for EventGroup {}
unsafe impl Sync for EventGroup {}

/* ================================================================== */
/*  事件位定义 (电表系统事件)                                           */
/* ================================================================== */

pub mod events {
    /// 计量数据更新完成
    pub const METERING_READY: u32      = (1 << 0);
    /// RS485 收到帧
    pub const RS485_FRAME_RECEIVED: u32 = (1 << 1);
    /// 红外收到帧
    pub const IR_FRAME_RECEIVED: u32   = (1 << 2);
    /// 按键按下
    pub const KEY_PRESSED: u32         = (1 << 3);
    /// 编程键按下
    pub const PROG_KEY_PRESSED: u32    = (1 << 4);
    /// 防窃电事件
    pub const TAMPER_EVENT: u32        = (1 << 5);
    /// 上盖打开
    pub const COVER_OPENED: u32        = (1 << 6);
    /// 端子盖打开
    pub const TERMINAL_OPENED: u32     = (1 << 7);
    /// 磁场检测
    pub const MAGNETIC_DETECTED: u32   = (1 << 8);
    /// LCD 需要刷新
    pub const LCD_REFRESH: u32         = (1 << 9);
    /// 数据就绪 (metering → display/storage)
    pub const DATA_READY: u32          = (1 << 10);
    /// 事件日志需要保存
    pub const EVENT_LOG_SAVE: u32      = (1 << 11);
    /// 事件检测完成
    pub const EVENTS_DETECTED: u32     = (1 << 12);
}
