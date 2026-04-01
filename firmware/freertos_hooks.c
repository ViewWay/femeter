/* ================================================================== */
/*  freertos_hooks.c — FreeRTOS application hooks (C stubs)            */
/*                                                                    */
/*  These are called from FreeRTOS kernel (C side).                   */
/*  Rust implementations are in src/freertos_hooks.rs.                */
/* ================================================================== */

#include "FreeRTOS.h"
#include "task.h"
#include "timers.h"

/* ── ARM breakpoint intrinsic ── */
#if defined(__ARMCC_VERSION)
    #define BREAKPOINT() __breakpoint(0)
#elif defined(__GNUC__)
    #define BREAKPOINT() __builtin_trap()
#endif
extern void rust_idle_hook(void);
extern void rust_tick_hook(void);
extern void rust_stack_overflow_hook(TaskHandle_t xTask, char *pcTaskName);
extern void rust_malloc_failed_hook(void);

/* ── 断言处理 ── */
void vAssertCalled(const char *pcFile, unsigned long ulLine) {
    (void)pcFile;
    (void)ulLine;
    for (;;) {
        BREAKPOINT();
    }
}

/* ── Hook 函数 ── */
void vApplicationIdleHook(void) {
    rust_idle_hook();
}

void vApplicationTickHook(void) {
    rust_tick_hook();
}

void vApplicationStackOverflowHook(TaskHandle_t xTask, char *pcTaskName) {
    (void)xTask;
    (void)pcTaskName;
    rust_stack_overflow_hook(xTask, pcTaskName);
}

void vApplicationMallocFailedHook(void) {
    rust_malloc_failed_hook();
}

/* ── FreeRTOS v11 static allocation callbacks ── */
static StackType_t idle_task_stack[configMINIMAL_STACK_SIZE];
static StaticTask_t idle_task_tcb;

void vApplicationGetIdleTaskMemory(StaticTask_t **ppxIdleTaskTCBBuffer,
                                    StackType_t **ppxIdleTaskStackBuffer,
                                    uint32_t *pulIdleTaskStackSize) {
    *ppxIdleTaskTCBBuffer = &idle_task_tcb;
    *ppxIdleTaskStackBuffer = idle_task_stack;
    *pulIdleTaskStackSize = configMINIMAL_STACK_SIZE;
}

static StackType_t timer_task_stack[configTIMER_TASK_STACK_DEPTH];
static StaticTask_t timer_task_tcb;

void vApplicationGetTimerTaskMemory(StaticTask_t **ppxTimerTaskTCBBuffer,
                                     StackType_t **ppxTimerTaskStackBuffer,
                                     uint32_t *pulTimerTaskStackSize) {
    *ppxTimerTaskTCBBuffer = &timer_task_tcb;
    *ppxTimerTaskStackBuffer = timer_task_stack;
    *pulTimerTaskStackSize = configTIMER_TASK_STACK_DEPTH;
}
