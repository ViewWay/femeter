/* ================================================================== */
/*                                                                    */
/*  task_scheduler.rs — 协作式任务调度器                               */
/*                                                                    */
/*  专为嵌入式电表设计:                                                */
/*    - 基于时间片的协作式调度 (无抢占)                                */
/*    - 每个任务有独立的执行周期                                       */
/*    - 支持一次性延迟任务 (用于告警、超时)                            */
/*    - 零堆分配, 全部栈/静态                                          */
/*                                                                    */
/*  典型任务分配:                                                      */
/*    Task 0: 计量采样   — 200ms                                       */
/*    Task 1: 电能累计   — 1000ms                                      */
/*    Task 2: LCD 刷新   — 500ms                                       */
/*    Task 3: RS485 通信 — 10ms (高优先)                               */
/*    Task 4: 红外通信   — 50ms                                        */
/*    Task 5: 按键扫描   — 50ms                                        */
/*    Task 6: 脉冲输出   — 100ms                                       */
/*    Task 7: 防窃电检测 — 5000ms                                      */
/*    Task 8: 温度采集   — 10000ms                                     */
/*    Task 9: 看门狗喂狗 — 1000ms                                      */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

/// 最大任务数
pub const MAX_TASKS: usize = 16;

/// 任务 ID 类型
pub type TaskId = u8;

/// 任务无效 ID
pub const TASK_NONE: TaskId = 0xFF;

/* ================================================================== */
/*  周期任务                                                           */
/* ================================================================== */

/// 周期性任务描述
#[derive(Clone, Copy)]
struct PeriodicTask {
    /// 执行周期 (ms)
    period_ms: u32,
    /// 上次执行时间 (ms)
    last_run_ms: u64,
    /// 是否使能
    enabled: bool,
}

/* ================================================================== */
/*  延迟任务 (一次性)                                                  */
/* ================================================================== */

/// 一次性延迟任务
#[derive(Clone, Copy)]
struct DelayedTask {
    /// 目标时间 (ms)
    target_ms: u64,
    /// 是否活跃
    active: bool,
}

/* ================================================================== */
/*  TaskScheduler                                                     */
/* ================================================================== */

/// 协作式任务调度器
pub struct TaskScheduler {
    /// 周期任务列表
    tasks: [PeriodicTask; MAX_TASKS],
    /// 已注册任务数
    task_count: u8,

    /// 延迟任务列表 (最多 4 个并行)
    delayed: [DelayedTask; 4],
}

impl TaskScheduler {
    /// 创建调度器
    pub const fn new() -> Self {
        Self {
            tasks: [PeriodicTask {
                period_ms: 0,
                last_run_ms: 0,
                enabled: false,
            }; MAX_TASKS],
            task_count: 0,
            delayed: [DelayedTask {
                target_ms: 0,
                active: false,
            }; 4],
        }
    }

    /// 注册周期任务, 返回任务 ID
    ///
    /// `period_ms`: 执行周期 (ms), 0 = 禁用
    pub fn register(&mut self, period_ms: u32) -> TaskId {
        if self.task_count as usize >= MAX_TASKS {
            return TASK_NONE;
        }
        let id = self.task_count;
        self.tasks[id as usize] = PeriodicTask {
            period_ms,
            last_run_ms: 0,
            enabled: period_ms > 0,
        };
        self.task_count += 1;
        id
    }

    /// 修改任务周期
    pub fn set_period(&mut self, id: TaskId, period_ms: u32) {
        if (id as usize) < self.task_count as usize {
            self.tasks[id as usize].period_ms = period_ms;
            self.tasks[id as usize].enabled = period_ms > 0;
        }
    }

    /// 使能/禁用任务
    pub fn set_enabled(&mut self, id: TaskId, enabled: bool) {
        if (id as usize) < self.task_count as usize {
            self.tasks[id as usize].enabled = enabled;
        }
    }

    /// 注册一次性延迟任务, 返回延迟任务 ID (0~3)
    ///
    /// `delay_ms`: 从当前时间起的延迟 (ms)
    pub fn schedule_once(&mut self, now_ms: u64, delay_ms: u32) -> Option<u8> {
        for (i, slot) in self.delayed.iter_mut().enumerate() {
            if !slot.active {
                slot.target_ms = now_ms + delay_ms as u64;
                slot.active = true;
                return Some(i as u8);
            }
        }
        None
    }

    /// 取消延迟任务
    pub fn cancel_delayed(&mut self, id: u8) {
        if (id as usize) < self.delayed.len() {
            self.delayed[id as usize].active = false;
        }
    }

    /// 轮询调度器, 返回所有就绪的任务 ID
    ///
    /// `now_ms`: 当前时间戳 (ms)
    /// `ready_buf`: 输出缓冲区, 写入就绪的任务 ID
    /// 返回就绪任务数
    pub fn poll(&mut self, now_ms: u64, ready_buf: &mut [TaskId; MAX_TASKS]) -> usize {
        let mut count = 0;

        // 周期任务
        for i in 0..self.task_count as usize {
            let task = &mut self.tasks[i];
            if !task.enabled || task.period_ms == 0 {
                continue;
            }
            let elapsed = now_ms.wrapping_sub(task.last_run_ms);
            if elapsed >= task.period_ms as u64 {
                task.last_run_ms = now_ms;
                if count < ready_buf.len() {
                    ready_buf[count] = i as TaskId;
                    count += 1;
                }
            }
        }

        // 延迟任务 (ID = 0x80 + index, 用于区分)
        for (i, slot) in self.delayed.iter_mut().enumerate() {
            if slot.active && now_ms >= slot.target_ms {
                slot.active = false;
                if count < ready_buf.len() {
                    ready_buf[count] = 0x80 | (i as TaskId);
                    count += 1;
                }
            }
        }

        count
    }

    /// 查询任务距下次执行还有多少 ms
    pub fn time_until_next(&self, now_ms: u64) -> u32 {
        let mut min_delay = u32::MAX;

        for i in 0..self.task_count as usize {
            let task = &self.tasks[i];
            if !task.enabled || task.period_ms == 0 {
                continue;
            }
            let elapsed = now_ms.wrapping_sub(task.last_run_ms);
            let remaining = task.period_ms.saturating_sub(elapsed as u32);
            if remaining < min_delay {
                min_delay = remaining;
            }
        }

        min_delay
    }
}

/* ================================================================== */
/*  任务 ID 定义 (与 main.rs 中注册顺序对应)                           */
/* ================================================================== */

pub mod task {
    /// 计量采样 (200ms)
    pub const METERING:   super::TaskId = 0;
    /// 电能累计 (1000ms)
    pub const ENERGY:     super::TaskId = 1;
    /// LCD 刷新 (500ms)
    pub const DISPLAY:    super::TaskId = 2;
    /// RS485 通信 (10ms)
    pub const RS485:      super::TaskId = 3;
    /// 红外通信 (50ms)
    pub const INFRARED:   super::TaskId = 4;
    /// 按键扫描 (50ms)
    pub const KEY:        super::TaskId = 5;
    /// 脉冲输出 (100ms)
    pub const PULSE:      super::TaskId = 6;
    /// 防窃电检测 (5000ms)
    pub const TAMPER:     super::TaskId = 7;
    /// 温度采集 (10000ms)
    pub const TEMPERATURE:super::TaskId = 8;
    /// 看门狗喂狗 (1000ms)
    pub const WATCHDOG:   super::TaskId = 9;
    /// LoRaWAN (30000ms)
    pub const LORAWAN:    super::TaskId = 10;
    /// 蜂窝通信 (60000ms)
    pub const CELLULAR:   super::TaskId = 11;
}
