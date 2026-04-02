/* ================================================================== */
/*  FreeRTOSConfig.h — FM33A068EV (Cortex-M0+ @ 64MHz)               */
/* ================================================================== */

#ifndef FREERTOS_CONFIG_H
#define FREERTOS_CONFIG_H

/* ── 基本配置 ── */
#define configUSE_PREEMPTION                    1
#define configUSE_PORT_OPTIMISED_TASK_SELECTION 0  /* CM0+ 无 BASEPRI, 用通用方式 */
#define configUSE_TICKLESS_IDLE                 1
#define configCPU_CLOCK_HZ                      64000000UL
#define configTICK_RATE_HZ                      ((TickType_t)1000)
#define configMAX_PRIORITIES                    7
#define configMINIMAL_STACK_SIZE                ((uint16_t)64)  /* words = 256 bytes */
#define configTOTAL_HEAP_SIZE                   ((size_t)(48 * 1024))
#define configMAX_TASK_NAME_LEN                 12
#ifndef TICK_TYPE_WIDTH_16_BITS
#define TICK_TYPE_WIDTH_16_BITS    0
#define TICK_TYPE_WIDTH_32_BITS    1
#define TICK_TYPE_WIDTH_64_BITS    2
#endif
#define configTICK_TYPE_WIDTH_IN_BITS         TICK_TYPE_WIDTH_32_BITS
#define configIDLE_SHOULD_YIELD                 1
#define configUSE_TASK_NOTIFICATIONS            1
#define configTASK_NOTIFICATION_ARRAY_ENTRIES   1
#define configQUEUE_REGISTRY_SIZE               8
#define configUSE_QUEUES                      1
#define configUSE_QUEUE_SETS                    1
#define configUSE_TIME_SLICING                  1

/* ── 内存分配 ── */
#define configSUPPORT_STATIC_ALLOCATION         1
#define configSUPPORT_DYNAMIC_ALLOCATION        1

/* ── 同步原语 ── */
#define configUSE_MUTEXES                       1
#define configUSE_RECURSIVE_MUTEXES             1
#define configUSE_COUNTING_SEMAPHORES           1

/* ── 软件定时器 ── */
#define configUSE_TIMERS                        1
#define configTIMER_TASK_PRIORITY               2
#define configTIMER_QUEUE_LENGTH                10
#define configTIMER_TASK_STACK_DEPTH            (configMINIMAL_STACK_SIZE * 2)

/* ── Hook 函数 ── */
#define configUSE_IDLE_HOOK                     1
#define configUSE_TICK_HOOK                     1
#define configCHECK_FOR_STACK_OVERFLOW          2   /* method 2: canary check */
#define configUSE_MALLOC_FAILED_HOOK            1
#define configENABLE_MPU                        0   /* CM0+ 无 MPU */

/* ── 运行时统计 ── */
#define configGENERATE_RUN_TIME_STATS           0
#define configUSE_TRACE_FACILITY                1
#define configUSE_STATS_FORMATTING_FUNCTIONS    0

/* ── 中断优先级 (CM0+ 只有 4 级: 0~3) ── */
#define configKERNEL_INTERRUPT_PRIORITY         (3 << 4)  /* 最低优先级 */
#define configCHECK_HANDLER_INSTALLATION         0  /* cortex-m-rt 管理向量表 */
/* API call 允许的最高优先级 (数值 >= 此值的中断可调 FreeRTOS API) */
#define configMAX_SYSCALL_INTERRUPT_PRIORITY    (2 << 4)

/* ── 断言 ── */
#ifdef __cplusplus
extern "C" {
#endif
void vAssertCalled(const char *pcFile, unsigned long ulLine);
#ifdef __cplusplus
}
#endif
#define configASSERT(x) if ((x) == 0) vAssertCalled(__FILE__, __LINE__)

/* ── API include 控制 (默认很多是 0，需要显式启用) ── */
#define INCLUDE_vTaskDelay                1
#define INCLUDE_vTaskDelayUntil           1
#define INCLUDE_vTaskDelete               1
#define INCLUDE_xTaskGetSchedulerState    1
#define INCLUDE_xSemaphoreGetMutexHolder  1

/* ── 函数映射 ── */
#define vPortSVCHandler    SVC_Handler
#define xPortPendSVHandler PendSV_Handler
#define xPortSysTickHandler SysTick_Handler

/* ── 可选: application-defined hook stubs ── */
/* vApplicationIdleHook    — defined in freertos_hooks.rs */
/* vApplicationTickHook    — defined in freertos_hooks.rs */
/* vApplicationStackOverflowHook — defined in freertos_hooks.rs */
/* vApplicationMallocFailedHook — defined in freertos_hooks.rs */

#include "portmacro.h"

#endif /* FREERTOS_CONFIG_H */
