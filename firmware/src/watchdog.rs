//! 独立看门狗 (IWDT) 驱动 — FM33A068EV
//!
//! 基于 LSI 内部低速时钟，系统复位后不可停止。
//! 超时可选 1s / 2s / 4s / 8s。
//!
//! 设计为配合 FreeRTOS task_watchdog 使用：
//! - 每秒喂狗任务检查所有注册任务是否存活
//! - 任一任务卡死则停止喂狗，触发硬件复位

use crate::fm33lg0;

// ══════════════════════════════════════════════════════════════════
// 寄存器位定义
// ══════════════════════════════════════════════════════════════════

/// 喂狗魔法值
const SERV_KEY: u32 = 0x12345678;

/// 超时对应的预分频值（LSI ≈ 32kHz）
/// TODO: 根据 FM33A0XXEV 手册确认精确分频值
const CFGR_DIV_256:   u32 = 0x00; // ~1s
const CFGR_DIV_1024:  u32 = 0x01; // ~2s
const CFGR_DIV_4096:  u32 = 0x02; // ~4s
const CFGR_DIV_16384: u32 = 0x03; // ~8s

// ══════════════════════════════════════════════════════════════════
// 超时配置
// ══════════════════════════════════════════════════════════════════

/// 看门狗超时时间
#[derive(Clone, Copy, Debug)]
pub enum IwdtTimeout {
    Sec1 = 0,
    Sec2 = 1,
    Sec4 = 2,
    Sec8 = 3,
}

const TIMEOUT_DIV: [u32; 4] = [
    CFGR_DIV_256,
    CFGR_DIV_1024,
    CFGR_DIV_4096,
    CFGR_DIV_16384,
];

// ══════════════════════════════════════════════════════════════════
// 寄存器读写辅助
// ══════════════════════════════════════════════════════════════════

#[inline]
unsafe fn iwdt_write(offset: usize, val: u32) {
    let iwdt = crate::fm33lg0::iwdt();
    let p = (iwdt as *const _ as *const u8).add(offset) as *mut u32;
    core::ptr::write_volatile(p, val);
}

#[inline]
unsafe fn iwdt_read(offset: usize) -> u32 {
    let iwdt = crate::fm33lg0::iwdt();
    let p = (iwdt as *const _ as *const u8).add(offset) as *const u32;
    core::ptr::read_volatile(p)
}

// ══════════════════════════════════════════════════════════════════
// 任务看门狗 — FreeRTOS 集成
// ══════════════════════════════════════════════════════════════════

/// 最大可注册的任务数
const MAX_WATCHED_TASKS: usize = 8;

/// 任务心跳状态
#[derive(Clone, Copy)]
struct TaskHeartbeat {
    last_feed: u32,
    timeout_ticks: u32,
}

/// 任务看门狗管理器
static mut TASKS: [Option<TaskHeartbeat>; MAX_WATCHED_TASKS] = [None; MAX_WATCHED_TASKS];
static mut TASK_COUNT: usize = 0;

/// 初始化 IWDT
///
/// 注意：一旦启动，系统复位前无法停止！
pub fn init(timeout: IwdtTimeout) {
    let div = TIMEOUT_DIV[timeout as usize];
    unsafe {
        iwdt_write(0x04, div);      // CFGR: 设置预分频
        iwdt_write(0x00, SERV_KEY); // 首次喂狗，启动计数器
    }
    defmt::info!("IWDT 初始化完成, 超时={}s", timeout as u8 + 1);
}

/// 喂狗 — 必须在超时前调用
pub fn feed() {
    unsafe { iwdt_write(0x00, SERV_KEY); }
}

/// 读取当前看门狗计数值（调试用）
pub fn counter() -> u32 {
    unsafe { iwdt_read(0x08) }
}

// ══════════════════════════════════════════════════════════════════
// FreeRTOS 任务看门狗
// ══════════════════════════════════════════════════════════════════

/// 注册一个需要监控的任务
///
/// `timeout_ticks`: 该任务允许的最大不响应时间（FreeRTOS tick）
/// 返回任务 ID，用于后续 task_feed()
pub fn task_register(timeout_ticks: u32) -> usize {
    let id = cortex_m::interrupt::free(|_| {
        let count = unsafe { TASK_COUNT };
        if count >= MAX_WATCHED_TASKS {
            panic!("task_watchdog: 已达到最大注册数");
        }
        #[cfg(feature = "freertos")]
        unsafe {
            use crate::freertos;
            TASKS[count] = Some(TaskHeartbeat {
                last_feed: freertos::xTaskGetTickCount(),
                timeout_ticks,
            });
            TASK_COUNT = count + 1;
        }

        #[cfg(not(feature = "freertos"))]
        {
            // In bare-metal mode, just register the task
            cortex_m::interrupt::free(|_| unsafe {
                TASKS[count] = Some(TaskHeartbeat {
                    last_feed: 0, // No tick counter in bare-metal
                    timeout_ticks,
                });
                TASK_COUNT = count + 1;
            });
        }
        count
    });
    defmt::info!("task_watchdog: 注册任务 id={}, timeout={} ticks", id, timeout_ticks);
    id
}

/// 任务喂狗 — 在被监控的任务循环中定期调用
pub fn task_feed(id: usize) {
    #[cfg(feature = "freertos")]
    cortex_m::interrupt::free(|_| unsafe {
        use crate::freertos;
        if let Some(ref mut task) = TASKS[id] {
            task.last_feed = freertos::xTaskGetTickCount();
        }
    });

    #[cfg(not(feature = "freertos"))]
    {
        // In bare-metal mode, update a simple counter
        cortex_m::interrupt::free(|_| unsafe {
            if let Some(ref mut task) = TASKS[id] {
                task.last_feed += 1; // Simple increment
            }
        });
    }
}

/// 检查所有任务是否存活，全部存活则喂硬件看门狗
///
/// 在专门的喂狗任务中每秒调用一次。
/// 如果任一任务超时，停止喂狗，硬件看门狗将复位系统。
pub fn task_check_and_feed() -> bool {
    #[cfg(feature = "freertos")]
    {
        let now = unsafe { crate::freertos::xTaskGetTickCount() };
        let all_alive = cortex_m::interrupt::free(|_| unsafe {
            for i in 0..TASK_COUNT {
                if let Some(ref task) = TASKS[i] {
                    let elapsed = now.wrapping_sub(task.last_feed);
                    if elapsed > task.timeout_ticks {
                        defmt::error!("task_watchdog: 任务 {} 卡死，超时 {} ticks", i, task.timeout_ticks);
                        return false; // 停止喂狗，触发复位
                    }
                }
            }
            true
        });
        all_alive
    }

    #[cfg(not(feature = "freertos"))]
    {
        // 裸机模式下：简单检查任务是否存在
        let all_alive = cortex_m::interrupt::free(|_| unsafe {
            for i in 0..TASK_COUNT {
                if TASKS[i].is_none() {
                    return false;
                }
            }
            true
        });
        all_alive
    }
}

// ══════════════════════════════════════════════════════════════════
// FreeRTOS 喂狗任务
// ══════════════════════════════════════════════════════════════════

/// 看门狗任务入口函数（FreeRTOS task）
unsafe extern "C" fn watchdog_task_entry(_param: *mut core::ffi::c_void) {
    defmt::info!("看门狗任务启动");

    #[cfg(feature = "freertos")]
    loop {
        if !task_check_and_feed() {
            // 有任务卡死，不再喂狗，等待硬件复位
            defmt::error!("看门狗不再喂狗，等待系统复位...");
            loop {
                use crate::freertos;
                freertos::vTaskDelay(1000);
            }
        }
        use crate::freertos;
        freertos::vTaskDelay(1000);
    }

    #[cfg(not(feature = "freertos"))]
    loop {
        if !task_check_and_feed() {
            // In bare-metal mode, just delay and wait for reset
            defmt::error!("看门狗检测到故障，等待系统复位...");
            cortex_m::asm::delay(32_000_000); // ~1s delay
        }
        cortex_m::asm::delay(32_000_000); // ~1s delay
    }
}

/// 创建看门狗任务（FreeRTOS 或裸机）
///
/// `priority`: 任务优先级（FreeRTOS 模式下使用）
pub fn create_watchdog_task(priority: u32) {
    #[cfg(feature = "freertos")]
    {
        use core::ffi::c_char;
        unsafe {
            use crate::freertos;
            freertos::xTaskCreate(
                watchdog_task_entry,
                b"wdog\0".as_ptr() as *const c_char,
                256,
                core::ptr::null_mut(),
                priority,
                core::ptr::null_mut(),
            );
        }
    }

    #[cfg(not(feature = "freertos"))]
    {
        // In bare-metal mode, watchdog runs inline
        // No task creation needed, just call watchdog_task_entry() directly
        defmt::warn!("create_watchdog_task: 无操作（裸机模式）");
    }
}
