/* ================================================================== */
/*  freertos_hooks.rs — FreeRTOS hook 函数 (Rust 实现)                  */
/*                                                                    */
/*  由 freertos_hooks.c 中的 C stubs 调用                              */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use defmt::{error, trace};
use core::ffi::c_void;

/// 全局 tick 计数器 (供 Rust 侧使用)
static mut TICK_COUNT: u64 = 0;

/// 获取全局 tick 计数 (ms)
pub fn global_tick_ms() -> u64 {
    unsafe { TICK_COUNT }
}

/* ── C ABI Hook 实现 ── */

/// Idle Hook: 低功耗 — 进入 WFI 等待中断
#[no_mangle]
unsafe extern "C" fn rust_idle_hook() {
    cortex_m::asm::wfi();
}

/// Tick Hook: 递增全局计数器
#[no_mangle]
unsafe extern "C" fn rust_tick_hook() {
    TICK_COUNT += 1;
}

/// Stack Overflow Hook: 断言并停机
#[no_mangle]
unsafe extern "C" fn rust_stack_overflow_hook(_task: *mut c_void, name: *mut u8) {
    let _ = name; // 避免 unused warning
    error!("STACK OVERFLOW!");
    loop {
        cortex_m::asm::bkpt();
    }
}

/// Malloc Failed Hook: 断言并停机
#[no_mangle]
unsafe extern "C" fn rust_malloc_failed_hook() {
    error!("MALLOC FAILED!");
    loop {
        cortex_m::asm::bkpt();
    }
}
