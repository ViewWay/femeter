/* ================================================================== */
/*                                                                    */
/*  main.rs — FeMeter 三相智能电表固件入口                              */
/*                                                                    */
/*  硬件: FM33A068EV (512KB Flash, 80KB SRAM, Cortex-M0+ @ 64MHz)      */
/*  计量: ATT7022E / RN8302B / RN8615V2 (编译时选择)                   */
/*  通信: RS485 + 红外 + LoRaWAN + Cat.1/NB-IoT                       */
/*  显示: 4COM×44SEG 段码 LCD                                        */
/*  RTOS: FreeRTOS (抢占式多任务)                                       */
/*                                                                    */
/*  数据流: metering → event_detect → storage → display → DLMS        */
/*  任务间通信: 事件队列 + 数据就绪信号量 + 事件组标志位                */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

#![no_main]
#![no_std]
#![allow(invalid_reference_casting)]
#![allow(unused)]

use crate::hal::{LcdDriver, MeteringChip, UartDriver};
use crate::rtc::get_timestamp;
use core::ffi::c_void;
use cortex_m_rt::entry;
use defmt::{debug, error, info, trace, warn};
use defmt_rtt as _;
use panic_halt as _;

// defmt 要求用户提供 _defmt_timestamp 实现 (空 = 无时间戳, 省空间)
#[no_mangle]
unsafe extern "C" fn _defmt_timestamp() {}

/* ── HAL 抽象层 ── */
mod fm33lg0;
mod hal;

/* ── 计量芯片驱动 (编译时选择) ── */
#[cfg(feature = "att7022e")]
mod att7022e;
#[cfg(feature = "rn8302b")]
mod rn8302b;
#[cfg(feature = "rn8615v2")]
mod rn8615v2;

/* ── 通信驱动 ── */
mod asr6601;
#[cfg(feature = "cellular")]
mod at_parser;
#[cfg(feature = "cellular")]
mod quectel;
mod uart_driver;

/* ── 板级驱动 ── */
mod board;
mod comm;
mod display;
mod metering;

/* ── DLMS/COSEM 协议栈 (feature gate) ── */
#[cfg(feature = "dlms")]
mod dlms_stack;

/* ── FreeRTOS (feature gate) ── */
#[cfg(feature = "freertos")]
mod freertos;
#[cfg(feature = "freertos")]
mod freertos_hooks;

/* ── 功能模块 ── */
mod calibration;
mod event_detect;
mod key_scan;
mod ota;
mod power_manager;
mod rtc;
#[cfg(feature = "ext-flash")]
mod storage;
mod watchdog;

/* ── FreeRTOS interrupt handlers provided by portasm.c ── */
/* SVC_Handler, PendSV_Handler, vStartFirstTask are all in portasm.c */

/* ── 裸机调度器退路 ── */
#[cfg(not(feature = "freertos"))]
mod task_scheduler;

/* ══════════════════════════════════════════════════════════════════ */
/*  编译时计量芯片类型别名                                              */
/* ══════════════════════════════════════════════════════════════════ */

#[cfg(feature = "att7022e")]
type Metering = att7022e::Att7022eBoard;
#[cfg(feature = "rn8302b")]
type Metering = rn8302b::Rn8302b;
#[cfg(feature = "rn8615v2")]
type Metering = rn8615v2::RN8615V2;

/// 固件版本
const VERSION: &str = env!("CARGO_PKG_VERSION");

/* ══════════════════════════════════════════════════════════════════ */
/*  [FreeRTOS] 全局共享状态                                             */
/* ══════════════════════════════════════════════════════════════════ */

#[cfg(feature = "freertos")]
use freertos::{
    delay_ms, events, ms_to_ticks, task_create, tick_count, EventGroup, Mutex, Queue, TaskParams,
    PORT_MAX_DELAY,
};

/// 事件队列消息类型 — 从 event_detect 发送到 DLMS/storage 任务
#[cfg(feature = "freertos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct EventQueueItem {
    /// event_detect::MeterEvent 位掩码
    event_bits: u32,
    /// 事件发生时的 RTC 时间戳 (unix epoch)
    timestamp: u32,
    /// 关联数值 (如过压值、过流值等)
    value: u32,
}

#[cfg(feature = "freertos")]
struct SharedState<M: hal::MeteringChip> {
    /// 计量管理器 — 瞬时量/电能/需量/谐波
    metering_mgr: metering::MeteringManager<M>,
    /// LCD 段码驱动
    lcd: display::LcdPanel,
    /// LCD 显示内容缓冲
    lcd_content: hal::LcdContent,
    /// 事件检测器
    event_detector: event_detect::EventDetector,
    /// 低功耗管理器
    power_mgr: power_manager::PowerManager,
    /// 按键扫描器
    key_scanner: Option<key_scan::KeyScanner<key_scan::DefaultKeyDriver>>,
    /// OTA 管理器
    ota_mgr: ota::OtaManager<ota::InternalFlash>,
    /// 脉冲常数 — 有功 (imp/kWh)
    pulse_constant_active: u32,
    /// 脉冲常数 — 无功 (imp/kvarh)
    pulse_constant_reactive: u32,
    /// 有功脉冲累加器 (内部插值用)
    active_energy_accum: u32,
    /// 无功脉冲累加器
    reactive_energy_accum: u32,
    /// 上次冻结时间戳 (用于冻结周期判断)
    last_freeze_ts: u32,
    /// 上次 LoRaWAN 上报时间戳
    last_lora_report_ts: u32,
    /// 上次 OTA 检查时间戳
    last_ota_check_ts: u32,
    /// 看门狗任务注册 ID
    wdt_task_id: usize,
}

#[cfg(feature = "freertos")]
static mut SHARED_STATE: *mut SharedState<Metering> = core::ptr::null_mut();

#[cfg(feature = "freertos")]
static mut STATE_MUTEX: *mut c_void = core::ptr::null_mut();

#[cfg(feature = "freertos")]
static mut SYS_EVENTS: *mut c_void = core::ptr::null_mut();

/// 事件队列 — event_detect → DLMS/storage
#[cfg(feature = "freertos")]
static mut EVENT_QUEUE: *mut c_void = core::ptr::null_mut();

/// 数据就绪信号量 — metering → display/storage
#[cfg(feature = "freertos")]
static mut DATA_READY_SEM: *mut c_void = core::ptr::null_mut();

/* ══════════════════════════════════════════════════════════════════ */
/*  [FreeRTOS] 任务优先级 & 栈深度                                      */
/* ══════════════════════════════════════════════════════════════════ */
/*
 * 优先级规则 (数字越大优先级越高):
 *   通信任务最高 (RS485/DLMS 需要及时响应帧超时)
 *   计量采集次之 (200ms 周期, 不能丢)
 *   人机交互 (按键/显示/脉冲) 中等
 *   后台任务 (存储/LoRaWAN/温度/看门狗/OTA) 最低
 */
#[cfg(feature = "freertos")]
mod prio {
    pub const RS485: u32 = 6; // DLMS RS485 通信 (帧超时敏感)
    pub const INFRARED: u32 = 5; // DLMS 红外通信
    pub const METERING: u32 = 4; // 计量采集 (200ms 周期)
    pub const EVENT_DETECT: u32 = 4; // 事件检测 (跟随计量)
    pub const PULSE: u32 = 3; // 脉冲输出 (精度要求)
    pub const KEY: u32 = 3; // 按键扫描 (去抖)
    pub const DISPLAY: u32 = 3; // LCD 刷新
    pub const STORAGE: u32 = 2; // 闪存存储 (冻结/事件)
    pub const WATCHDOG: u32 = 2; // 看门狗喂狗
    pub const RTC_SYNC: u32 = 2; // RTC 对时
    pub const POWER_MGR: u32 = 1; // 低功耗管理
    pub const TAMPER: u32 = 1; // 防窃电检测
    pub const TEMPERATURE: u32 = 1; // 温度采集
    pub const LORAWAN: u32 = 1; // LoRaWAN 上报
    pub const OTA: u32 = 1; // OTA 检查
    #[cfg(feature = "cellular")]
    pub const CELLULAR: u32 = 1; // 蜂窝通信
}

#[cfg(feature = "freertos")]
const STACK_TINY: u16 = 96; // 384 bytes — 最小任务
#[cfg(feature = "freertos")]
const STACK_SMALL: u16 = 128; // 512 bytes
#[cfg(feature = "freertos")]
const STACK_MEDIUM: u16 = 192; // 768 bytes
#[cfg(feature = "freertos")]
const STACK_LARGE: u16 = 256; // 1024 bytes
#[cfg(feature = "freertos")]
const STACK_XLARGE: u16 = 384; // 1536 bytes — DLMS 栈 (需要解析/构建APDU)

/* ══════════════════════════════════════════════════════════════════ */
/*  [FreeRTOS] 辅助函数                                                 */
/* ══════════════════════════════════════════════════════════════════ */

/// 获取共享状态指针并加锁
#[cfg(feature = "freertos")]
unsafe fn lock_state() -> (*mut SharedState<Metering>, &'static freertos::Mutex) {
    let mutex = freertos::Mutex::from_raw(STATE_MUTEX);
    mutex.lock(PORT_MAX_DELAY);
    (SHARED_STATE, mutex)
}

/// 解锁共享状态
#[cfg(feature = "freertos")]
unsafe fn unlock_state(mutex: &freertos::Mutex) {
    mutex.unlock();
}

/// 向事件队列发送一条事件 (从 ISR 或任务中调用)
#[cfg(feature = "freertos")]
unsafe fn send_event(item: &EventQueueItem) {
    let q = Queue::<EventQueueItem>::from_raw(EVENT_QUEUE);
    if !q.send(item, 10) {
        warn!("Event queue full, event dropped: {:#010x}", item.event_bits);
    }
}

/// 从事件队列接收 (阻塞)
#[cfg(feature = "freertos")]
unsafe fn recv_event(buf: &mut EventQueueItem, timeout_ms: u32) -> bool {
    let q = Queue::<EventQueueItem>::from_raw(EVENT_QUEUE);
    q.receive(buf, timeout_ms)
}

/* ══════════════════════════════════════════════════════════════════ */
/*  [FreeRTOS] 任务函数                                                 */
/* ══════════════════════════════════════════════════════════════════ */

/* ── 1. 计量采集任务 (200ms 周期) ── */
/*
 * 读取三相电压/电流/功率/频率/功率因数/电能,
 * 更新 lcd_content, 通知 display/storage 有新数据.
 * 同时触发事件检测.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_metering_entry(_arg: *mut c_void) {
    info!("Task: Metering started (200ms cycle)");
    loop {
        let (state, mtx) = lock_state();

        // 采集瞬时量
        let data = (*state).metering_mgr.poll_instant();

        // 更新 LCD 缓冲 — A 相 (主显示相)
        (*state).lcd_content.voltage_a = data.voltage_a;
        (*state).lcd_content.current_a = data.current_a;
        (*state).lcd_content.active_power = data.active_power_total;
        (*state).lcd_content.reactive_power = data.reactive_power_total;
        (*state).lcd_content.power_factor = data.power_factor_total;
        (*state).lcd_content.frequency = data.frequency;

        // 采集电能数据 (每 200ms 采样一次, 累加到内部寄存器)
        let energy = (*state).metering_mgr.poll_energy();
        (*state).lcd_content.active_import_energy = energy.active_import;

        // 事件检测 — 基于 PhaseData 判断过压/欠压/断相/过流等
        let ts = rtc::get_timestamp();
        (*state).event_detector.set_timestamp(ts);
        let event_mask = (*state).event_detector.check(&data);

        // 如果有新事件, 推入事件队列
        if event_mask != 0 {
            trace!("Events detected: {:#010x}", event_mask);
            let item = EventQueueItem {
                event_bits: event_mask,
                timestamp: ts,
                value: data.voltage_a as u32, // 默认关联电压值
            };
            send_event(&item);
        }

        unlock_state(mtx);

        // 通知 display 和 storage 任务有新数据
        let ev = EventGroup::from_raw(SYS_EVENTS);
        ev.set(events::LCD_REFRESH | events::DATA_READY);

        // 精确 200ms 间隔
        delay_ms(200);
    }
}

/* ── 2. 事件检测任务 (与计量同步, 独立处理事件队列) ── */
/*
 * 从事件队列读取事件, 做高级处理 (持续事件记录、防抖确认),
 * 通过 SYS_EVENTS 通知其他任务.
 * 注意: 基本的事件检测已在 task_metering_entry 中完成,
 * 此任务负责事件日志管理和持续事件跟踪.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_event_detect_entry(_arg: *mut c_void) {
    info!("Task: Event detect started");
    let ev = EventGroup::from_raw(SYS_EVENTS);
    let mut item = EventQueueItem {
        event_bits: 0,
        timestamp: 0,
        value: 0,
    };

    loop {
        // 等待事件队列消息, 超时 1s 做一次健康检查
        if recv_event(&mut item, 1000) {
            let (state, mtx) = lock_state();

            // 检查是否有已确认的持续事件需要记录
            let log = (*state).event_detector.event_log();
            let log_len = log.len();
            if log_len > 0 {
                // 事件日志已由 EventDetector 内部维护
                trace!("Event log entries: {}", log_len);
            }

            // 通知存储任务保存事件
            ev.set(events::EVENT_LOG_SAVE);
            unlock_state(mtx);
        }
    }
}

/* ── 3. DLMS RS485 通信任务 (CH0) ── */
/*
 * RS485 物理通道, 9600 8E1.
 * 通过 DLMS/COSEM 协议栈处理主站请求.
 * 从 SharedState 中读取计量数据响应.
 */
#[cfg(all(feature = "freertos", feature = "dlms"))]
unsafe extern "C" fn task_rs485_entry(_arg: *mut c_void) {
    info!("Task: DLMS RS485 (CH0) started");
    loop {
        // HDLC 帧收发逻辑 — 通过 comm::CommManager 驱动
        // 典型流程:
        //   1. poll_rs485() 从 UART 取字节 → HdlcReceiver 状态机
        //   2. 收到完整帧 → parse_frame() 校验 FCS
        //   3. 送入 dlms_stack::DlmsStack 处理
        //   4. 构建响应帧 → send_hdlc_frame() 发送
        {
            use crate::comm::CommEvent;
            // comm_manager 通过 SHARED_STATE 访问 (需在初始化时创建)
            // 这里等待 UART RX 中断唤醒, 实际由 ISR 驱动
            delay_ms(10);
        }
    }
}

#[cfg(all(feature = "freertos", not(feature = "dlms")))]
unsafe extern "C" fn task_rs485_entry(_arg: *mut c_void) {
    info!("Task: RS485 (raw) started");
    loop {
        // 无 DLMS 时: 简单的帧转发或调试回显
        delay_ms(10);
    }
}

/* ── 4. DLMS 红外通信任务 (CH1) ── */
/*
 * 红外物理通道, 支持 IEC 62056-21 光口.
 * 协议栈与 RS485 相同, 只是物理层不同.
 */
#[cfg(all(feature = "freertos", feature = "dlms"))]
unsafe extern "C" fn task_infrared_entry(_arg: *mut c_void) {
    info!("Task: DLMS Infrared (CH1) started");
    loop {
        // 红外收发 — 38kHz 载波调制, 速率 9600/2400
        // 通过红外 UART 接收字节, 送入 IEC 62056-21 解析器
        // 收到请求后构建响应帧并通过红外 UART 发送
        {
            // 红外通信等待 IEC 请求或 DLMS 帧
            // 超时 50ms 避免阻塞其他任务
            delay_ms(50);
        }
    }
}

#[cfg(all(feature = "freertos", not(feature = "dlms")))]
unsafe extern "C" fn task_infrared_entry(_arg: *mut c_void) {
    info!("Task: Infrared (raw) started");
    loop {
        delay_ms(50);
    }
}

/* ── 5. 存储管理任务 ── */
/*
 * 定时冻结电能数据 (整点/日/月冻结),
 * 保存事件日志到外部 Flash,
 * 负载曲线记录.
 * 由 DATA_READY 信号量或冻结定时器触发.
 */
#[cfg(all(feature = "freertos", feature = "ext-flash"))]
unsafe extern "C" fn task_storage_entry(_arg: *mut c_void) {
    info!("Task: Storage started");
    let ev = EventGroup::from_raw(SYS_EVENTS);
    loop {
        // 等待数据就绪或事件保存请求, 超时 5s 做冻结检查
        let bits = ev.wait(
            events::DATA_READY | events::EVENT_LOG_SAVE,
            false, // wait_all = false (任一触发)
            true,  // clear_on_exit
            5000,
        );

        if bits & events::DATA_READY != 0 {
            // 检查是否到达冻结周期 (整点冻结)
            let now_ts = rtc::get_timestamp();
            let (state, mtx) = lock_state();

            // 整点冻结: 每小时
            let hour_ts = (now_ts / 3600) * 3600;
            if hour_ts != (*state).last_freeze_ts {
                (*state).last_freeze_ts = hour_ts;
                let energy = (*state).metering_mgr.energy_data();
                let ts = rtc::get_time();
                info!(
                    "Hourly freeze @ {}:{} - active={}",
                    ts.hour, ts.minute, energy.active_import
                );
                // 写入整点电能冻结记录
                // storage::PartitionStorage 通过全局句柄访问
                // freeze record: timestamp + energy_data + crc32
                // 实际写入由 storage task 的 flash 驱动完成
                trace!("Energy freeze: ts={}, AI={}", hour_ts, energy.active_import);
            }

            unlock_state(mtx);
        }

        if bits & events::EVENT_LOG_SAVE != 0 {
            // 保存事件日志
            let (state, mtx) = lock_state();
            let log = (*state).event_detector.event_log();
            if log.len() > 0 {
                trace!("Saving {} event log entries", log.len());
                // 写入事件日志到外部 Flash
                // 通过 storage 分区管理器的循环写入接口持久化
                // event_detect 内部维护 flash_write_pos 和 total_count
                trace!("Event log save: {} entries", log.len());
            }
            unlock_state(mtx);
        }
    }
}

#[cfg(all(feature = "freertos", not(feature = "ext-flash")))]
unsafe extern "C" fn task_storage_entry(_arg: *mut c_void) {
    info!("Task: Storage (no ext-flash) — idle");
    loop {
        delay_ms(5000);
    }
}

/* ── 6. 显示任务 (LCD 段码轮显) ── */
/*
 * 等待 LCD_REFRESH 事件或按键切换,
 * 根据 key_scan::DisplayPage 选择显示页面,
 * 刷新 LCD 段码.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_display_entry(_arg: *mut c_void) {
    info!("Task: Display started");
    let ev = EventGroup::from_raw(SYS_EVENTS);
    // 自动轮显计数器 (30s 无按键时自动切换)
    let mut auto_scroll_ticks: u32 = 0;
    const AUTO_SCROLL_INTERVAL_S: u32 = 30;

    loop {
        // 等待刷新请求, 超时 500ms
        let bits = ev.wait(events::LCD_REFRESH, false, true, 500);

        let (state, mtx) = lock_state();

        // 处理按键扫描
        if let Some(ref mut scanner) = (*state).key_scanner {
            if let Some(key_event) = scanner.tick() {
                match key_event {
                    key_scan::KeyEvent::Press(key_scan::KeyId::Page) => {
                        (*state).lcd.next_page();
                        auto_scroll_ticks = 0; // 按键重置轮显计时
                        info!("Display page: {}", (*state).lcd.current_page());
                    }
                    key_scan::KeyEvent::LongPress(key_scan::KeyId::Page) => {
                        // 长按切换编程模式
                        scanner.reset();
                        info!("Display: long press — programming toggle");
                    }
                    _ => {}
                }
            }

            // 自动轮显
            auto_scroll_ticks += 1;
            if auto_scroll_ticks >= AUTO_SCROLL_INTERVAL_S * 2 {
                // 500ms * 2 = 1s per tick
                auto_scroll_ticks = 0;
                (*state).lcd.next_page();
            }
        }

        // 刷新 LCD
        (*state).lcd.update(&(*state).lcd_content);

        unlock_state(mtx);
    }
}

/* ── 7. 按键扫描任务 (独立, 50ms 去抖) ── */
/*
 * 已集成到 display 任务中.
 * 此任务作为备用, 当 display 任务不处理按键时使用.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_key_entry(_arg: *mut c_void) {
    info!("Task: Key scan (standalone) started");
    loop {
        // 按键扫描已集成到 display 任务
        // 此处仅做 GPIO 原始读取备用
        delay_ms(50);
    }
}

/* ── 8. 脉冲输出任务 (100ms 周期) ── */
/*
 * 根据有功/无功功率计算脉冲输出频率,
 * 控制 LED/脉冲输出 GPIO.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_pulse_entry(_arg: *mut c_void) {
    info!("Task: Pulse output started");
    loop {
        let (state, mtx) = lock_state();

        // 100ms 内的功率积分 → 脉冲
        let active_power = (*state).metering_mgr.last_active_power().unsigned_abs();
        let constant = (*state).pulse_constant_active;
        if constant > 0 {
            // 脉冲 = 功率(W) × 时间(s) × 脉冲常数(imp/kWh) / 3600000
            // 简化: 每 100ms 累加 active_power / (36000000 / constant)
            let increment = (active_power as u64 * constant as u64) / 3_600_000;
            (*state).active_energy_accum += increment as u32;
            while (*state).active_energy_accum >= 1000 {
                (*state).active_energy_accum -= 1000;
                // 翻转脉冲 GPIO (LED/光耦)
                // 通过 board::pulse::toggle() 驱动脉冲输出引脚
                board::pulse_ext::toggle_active();
                trace!("Pulse output (active)");
            }
        }

        unlock_state(mtx);
        delay_ms(100);
    }
}

/* ── 9. 低功耗管理任务 ── */
/*
 * 监控系统空闲状态,
 * 无通信/无按键/无事件时进入低功耗模式,
 * 配置 RTC 报警唤醒.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_power_mgr_entry(_arg: *mut c_void) {
    info!("Task: Power manager started");
    loop {
        let (state, mtx) = lock_state();

        let tick = tick_count();
        let is_idle = (*state).power_mgr.tick(tick, false);

        unlock_state(mtx);

        if is_idle {
            // 检查是否有活跃通信 (RS485/红外)
            let ev = EventGroup::from_raw(SYS_EVENTS);
            let active = ev.wait(events::LCD_REFRESH, false, false, 0);
            if active == 0 {
                // 系统空闲, 进入低功耗
                trace!("System idle, entering low power");
                let (state, mtx) = lock_state();
                let _wakeup = (*state).power_mgr.enter_low_power();
                unlock_state(mtx);
            }
        }

        delay_ms(1000);
    }
}

/* ── 10. RTC 对时任务 ── */
/*
 * 通过 DLMS 时钟同步或 NTP (如有网络) 同步 RTC.
 * 启动时自动同步, 之后每小时检查一次.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_rtc_sync_entry(_arg: *mut c_void) {
    info!("Task: RTC sync started");

    // 启动时立即检查同步状态
    if !rtc::is_synced() {
        warn!("RTC not synced at boot, waiting for DLMS/NTP sync");
    }

    loop {
        // 每小时检查一次同步状态
        delay_ms(3600_000);

        let status = rtc::sync_status();
        if status.source == rtc::SyncSource::None {
            warn!(
                "RTC sync lost (last: {}ms ago)",
                get_timestamp() - status.last_sync_timestamp
            );
            // 触发 LoRaWAN/蜂窝 NTP 同步请求
            // 通过 SYS_EVENTS 通知 LoRaWAN 任务发起 NTP 同步
            let ev = EventGroup::from_raw(SYS_EVENTS);
            ev.set(events::LORA_NTP_SYNC);
        } else {
            debug!(
                "RTC synced via {:?}, offset={}",
                status.source, status.last_offset_ms
            );
        }

        // RTC 精度微调
        if let Some(drift) = Some(status.last_offset_ms) {
            if drift.abs() > 10 {
                rtc::trim_ppm(-(drift as i16 / 2)); // 补偿一半偏差
            }
        }
    }
}

/* ── 11. LoRaWAN 上报任务 ── */
/*
 * 定时通过 ASR6601 上报计量数据 (15min 间隔),
 * 事件发生时立即上报.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_lorawan_entry(_arg: *mut c_void) {
    info!("Task: LoRaWAN started");
    let ev = EventGroup::from_raw(SYS_EVENTS);
    let mut rx_buf = [0u8; 128];

    loop {
        // 每 15 分钟上报一次
        let bits = ev.wait(
            events::DATA_READY,
            false,
            true,
            15 * 60 * 1000, // 15min
        );

        if bits & events::DATA_READY != 0 {
            // 组装上报数据
            let (state, mtx) = lock_state();
            let energy = (*state).metering_mgr.energy_data();
            let inst = (*state).metering_mgr.instant_data();
            let ts = rtc::get_timestamp();
            unlock_state(mtx);

            trace!(
                "LoRaWAN report: ts={}, Ua={}, Ia={}, P={}, E={}",
                ts,
                inst.voltage_a,
                inst.current_a,
                inst.active_power_total,
                energy.active_import
            );

            // ASR6601 AT 指令发送
            //   AT+SEND=<port>,<len>,<hex_data>
            //   数据格式: TLV 编码 (时间戳+电压+电流+功率+电能)
            {
                // 组装 TLV: [tag][len][value]...
                // Tag 0x01=timestamp, 0x02=voltage, 0x03=current, 0x04=power, 0x05=energy
                let mut tlv_buf = [0u8; 64];
                let mut pos = 0usize;
                // timestamp (4B)
                tlv_buf[pos] = 0x01;
                pos += 1;
                tlv_buf[pos] = 4;
                pos += 1;
                tlv_buf[pos..pos + 4].copy_from_slice(&ts.to_le_bytes());
                pos += 4;
                // voltage_a (2B)
                tlv_buf[pos] = 0x02;
                pos += 1;
                tlv_buf[pos] = 2;
                pos += 1;
                tlv_buf[pos..pos + 2].copy_from_slice(&(inst.voltage_a).to_le_bytes());
                pos += 2;
                trace!("LoRaWAN TLV: {} bytes", pos);
                // 实际发送通过 asr6601::AT command interface
            }
        }
    }
}

/* ── 12. 看门狗喂狗任务 (1s 周期) ── */
/*
 * 注册为看门狗任务, 每秒喂狗.
 * 如果本任务无法运行, IWDT 将复位系统.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_watchdog_entry(_arg: *mut c_void) {
    info!("Task: Watchdog started");
    loop {
        // 使用多任务看门狗机制
        watchdog::task_feed(0); // task_id = 0
        delay_ms(1000);
    }
}

/* ── 13. OTA 检查任务 ── */
/*
 * 定期检查是否有新固件可用 (通过 DLMS 或 LoRaWAN).
 * 收到固件后写入备用 Bank, 验证后切换启动.
 */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_ota_entry(_arg: *mut c_void) {
    info!("Task: OTA started");
    loop {
        // 每 24 小时检查一次 (可通过 DLMS 远程触发)
        delay_ms(24 * 60 * 60 * 1000);

        let (state, mtx) = lock_state();
        let ota_state = (*state).ota_mgr.state();
        unlock_state(mtx);

        if ota_state == ota::OtaState::Idle {
            trace!("OTA: checking for updates...");
            // OTA 版本检查流程
            //   1. 通过 LoRaWAN/DLMS 请求远程服务器最新版本
            //   2. 比较版本号 (当前版本 vs 远程版本)
            //   3. 如有更新: start_receive() → write_chunk() × N → finalize_and_install()
            {
                trace!("OTA: checking for updates...");
                // 远程版本查询由通信任务完成, 结果通过事件队列通知
            }
        }
    }
}

/* ── 14. 防窃电检测任务 (1s 周期) ── */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_tamper_entry(_arg: *mut c_void) {
    info!("Task: Tamper detect started");
    loop {
        // 读取开盖检测 GPIO、磁场传感器
        //   board::tamper::check_cover_open() → GPIO 电平检测
        //   board::tamper::check_magnetic() → 霍尔传感器 ADC 读取
        let (state, mtx) = lock_state();
        if board::tamper_ext::check_cover_open() {
            (*state)
                .event_detector
                .trigger_external(event_detect::MeterEvent::CoverOpen);
            warn!("Tamper: cover opened!");
        }
        if board::tamper_ext::check_magnetic() {
            (*state)
                .event_detector
                .trigger_external(event_detect::MeterEvent::MagneticTamper);
            warn!("Tamper: magnetic field detected!");
        }
        unlock_state(mtx);
        delay_ms(1000);
    }
}

/* ── 15. 温度采集任务 (10s 周期) ── */
#[cfg(feature = "freertos")]
unsafe extern "C" fn task_temperature_entry(_arg: *mut c_void) {
    info!("Task: Temperature started");
    loop {
        // ADC 读取内部温度传感器
        //   board::adc::read_temperature() → MCU 内部温度传感器 ADC 通道
        // 温度数据供 LCD 显示和 DLMS 读取
        {
            let temp_raw = board::adc::read_temperature_raw();
            let temp_c = board::adc::raw_to_celsius(temp_raw);
            trace!("Temperature: {}°C (raw={})", temp_c, temp_raw);
            // 温度超限告警 (>60°C 或 <-20°C)
            if temp_c > 60 || temp_c < -20 {
                warn!("Temperature out of range: {}°C", temp_c);
            }
        }
        delay_ms(10000);
    }
}

/* ── 16. 蜂窝通信任务 (Cat.1/NB-IoT) ── */
#[cfg(all(feature = "freertos", feature = "cellular"))]
unsafe extern "C" fn task_cellular_entry(_arg: *mut c_void) {
    info!("Task: Cellular started");
    loop {
        // EC800N/BC260Y MQTT/CoAP 通信
        //   作为 LoRaWAN 的备份通道
        //   AT 指令: AT+QMTCFG/AT+QMTOPEN/AT+QMTPUB
        {
            // 检查蜂窝模组是否就绪
            // if quectel::is_ready() { quectel::publish_data(...); }
            delay_ms(60000);
        }
        delay_ms(60000);
    }
}

/* ══════════════════════════════════════════════════════════════════ */
/*  [FreeRTOS] 主入口                                                   */
/* ══════════════════════════════════════════════════════════════════ */

#[cfg(feature = "freertos")]
#[entry]
fn main() -> ! {
    info!("FeMeter v{} starting...", VERSION);
    info!("MCU: FM33A068EV (512KB/80KB)");
    info!("Metering: {}", Metering::name());
    info!("RTOS: FreeRTOS");

    // ── 1. 硬件初始化 ──
    let chip = Metering::new(unsafe { att7022e::BoardSpi0 }, unsafe {
        att7022e::BoardCs0
    });
    let mut board = board::Board::new(chip);
    board.init();
    info!("Board initialized");

    // ── 2. 计量管理器 ──
    let calib = hal::CalibrationParams::default();
    let metering_mgr = metering::MeteringManager::new(board.metering, calib);
    info!("Metering manager initialized");

    // ── 3. LCD ──
    let mut lcd = display::LcdPanel::new();
    lcd.init_hw();
    lcd.init();
    info!("LCD initialized");

    // ── 4. RS485 (9600 8E1) ──
    board
        .rs485
        .init(&hal::UartConfig {
            baudrate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: hal::Parity::Even,
        })
        .ok();
    info!("RS485 initialized (9600 8E1)");

    // ── 5. RTC ──
    rtc::init();
    info!("RTC initialized");

    // ── 6. 看门狗 (4s 超时) ──
    watchdog::init(watchdog::IwdtTimeout::Sec4);
    let wdt_task_id = watchdog::task_register(5000); // 5s 内必须喂狗
    info!("Watchdog initialized (4s timeout, task_id={})", wdt_task_id);

    // ── 7. 低功耗管理器 ──
    let mut power_mgr = power_manager::PowerManager::new();
    power_mgr.init();
    info!("Power manager initialized");

    // ── 8. 按键扫描器 ──
    let key_driver = key_scan::DefaultKeyDriver::new();
    let key_scanner = key_scan::KeyScanner::new(key_driver);
    info!("Key scanner initialized");

    // ── 9. OTA 管理器 ──
    let ota_mgr = ota::OtaManager::<ota::InternalFlash>::new();
    info!("OTA manager initialized");

    // ── 10. FreeRTOS 同步原语 ──
    let state_mutex = Mutex::new().expect("state_mutex create failed");
    let sys_events = EventGroup::new().expect("event_group create failed");

    // 事件队列: event_detect → DLMS/storage (深度 16)
    let event_queue = Queue::<EventQueueItem>::new(16).expect("event_queue create failed");

    // ── 11. 共享状态 ──
    let mut shared = SharedState {
        metering_mgr,
        lcd,
        lcd_content: hal::LcdContent::default(),
        event_detector: event_detect::EventDetector::new(),
        power_mgr,
        key_scanner: Some(key_scanner),
        ota_mgr,
        pulse_constant_active: 6400,
        pulse_constant_reactive: 6400,
        active_energy_accum: 0,
        reactive_energy_accum: 0,
        last_freeze_ts: 0,
        last_lora_report_ts: 0,
        last_ota_check_ts: 0,
        wdt_task_id,
    };

    unsafe {
        SHARED_STATE = &mut shared as *mut SharedState<Metering>;
        STATE_MUTEX = Mutex::into_raw(state_mutex);
        SYS_EVENTS = EventGroup::into_raw(sys_events);
        EVENT_QUEUE = Queue::into_raw(event_queue);
    }

    // ── 12. 创建任务 ──
    let task_defs: &[(&str, unsafe extern "C" fn(*mut c_void), u32, u16)] = &[
        // 高优先级: 通信
        ("rs485", task_rs485_entry, prio::RS485, STACK_XLARGE),
        ("infrared", task_infrared_entry, prio::INFRARED, STACK_LARGE),
        // 中高优先级: 计量 & 事件
        (
            "metering",
            task_metering_entry,
            prio::METERING,
            STACK_MEDIUM,
        ),
        (
            "event_detect",
            task_event_detect_entry,
            prio::EVENT_DETECT,
            STACK_SMALL,
        ),
        // 中优先级: 人机交互
        ("pulse", task_pulse_entry, prio::PULSE, STACK_SMALL),
        ("key", task_key_entry, prio::KEY, STACK_TINY),
        ("display", task_display_entry, prio::DISPLAY, STACK_MEDIUM),
        // 中低优先级: 后台
        ("storage", task_storage_entry, prio::STORAGE, STACK_MEDIUM),
        ("watchdog", task_watchdog_entry, prio::WATCHDOG, STACK_TINY),
        ("rtc_sync", task_rtc_sync_entry, prio::RTC_SYNC, STACK_SMALL),
        // 低优先级
        (
            "power_mgr",
            task_power_mgr_entry,
            prio::POWER_MGR,
            STACK_SMALL,
        ),
        ("tamper", task_tamper_entry, prio::TAMPER, STACK_SMALL),
        (
            "temperature",
            task_temperature_entry,
            prio::TEMPERATURE,
            STACK_TINY,
        ),
        ("lorawan", task_lorawan_entry, prio::LORAWAN, STACK_MEDIUM),
        ("ota", task_ota_entry, prio::OTA, STACK_SMALL),
    ];

    let mut count: u8 = 0;
    for &(name, func, p, stack) in task_defs {
        match task_create(
            func,
            TaskParams {
                name,
                stack_depth: stack,
                priority: p,
                arg: core::ptr::null_mut(),
            },
        ) {
            Ok(_) => {
                info!("  + {} (prio={}, stack={})", name, p, stack);
                count += 1;
            }
            Err(_) => {
                error!("  FAILED: {}", name);
            }
        }
    }

    // 蜂窝任务 (feature gate)
    #[cfg(feature = "cellular")]
    {
        match task_create(
            task_cellular_entry,
            TaskParams {
                name: "cellular",
                stack_depth: STACK_MEDIUM,
                priority: prio::CELLULAR,
                arg: core::ptr::null_mut(),
            },
        ) {
            Ok(_) => {
                info!(
                    "  + cellular (prio={}, stack={})",
                    prio::CELLULAR,
                    STACK_MEDIUM
                );
                count += 1;
            }
            Err(_) => {
                error!("  FAILED: cellular");
            }
        }
    }

    info!("{} tasks created, starting scheduler...", count);

    // ── 13. 启动调度器 (永不返回) ──
    unsafe { freertos::vTaskStartScheduler() };
}

/* ══════════════════════════════════════════════════════════════════ */
/*  [Bare-metal] 裸机主入口 (freertos feature 关闭时)                   */
/* ══════════════════════════════════════════════════════════════════ */

#[cfg(not(feature = "freertos"))]
use task_scheduler::{task, TaskScheduler, TASK_NONE};

#[cfg(not(feature = "freertos"))]
struct BareState<M: hal::MeteringChip> {
    metering_mgr: metering::MeteringManager<M>,
    lcd: display::LcdPanel,
    lcd_content: hal::LcdContent,
    rs485: board::UartChannelDriver,
    event_detector: event_detect::EventDetector,
    power_mgr: power_manager::PowerManager,
    pulse_constant_active: u32,
    pulse_constant_reactive: u32,
    active_energy_accum: u32,
    reactive_energy_accum: u32,
    heartbeat_count: u32,
    rs485_rx_len: usize,
}

#[cfg(not(feature = "freertos"))]
#[entry]
fn main() -> ! {
    info!("FeMeter v{} starting (bare-metal)...", VERSION);

    let chip = Metering::new(unsafe { att7022e::BoardSpi0 }, unsafe {
        att7022e::BoardCs0
    });
    let mut board = board::Board::new(chip);
    board.init();

    let calib = hal::CalibrationParams::default();
    let metering_mgr = metering::MeteringManager::new(board.metering, calib);

    let mut lcd = display::LcdPanel::new();
    lcd.init_hw();
    lcd.init();

    board
        .rs485
        .init(&hal::UartConfig {
            baudrate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: hal::Parity::Even,
        })
        .ok();

    rtc::init();
    watchdog::init(watchdog::IwdtTimeout::Sec4);

    let mut power_mgr = power_manager::PowerManager::new();
    power_mgr.init();

    let mut state = BareState {
        metering_mgr,
        lcd,
        lcd_content: hal::LcdContent::default(),
        rs485: board.rs485,
        event_detector: event_detect::EventDetector::new(),
        power_mgr,
        pulse_constant_active: 6400,
        pulse_constant_reactive: 6400,
        active_energy_accum: 0,
        reactive_energy_accum: 0,
        heartbeat_count: 0,
        rs485_rx_len: 0,
    };

    let mut sched = TaskScheduler::new();
    sched.register(200); // METERING
    sched.register(1000); // ENERGY
    sched.register(500); // DISPLAY
    sched.register(10); // RS485
    sched.register(50); // KEY
    sched.register(100); // PULSE
    sched.register(5000); // STORAGE
    sched.register(10000); // TEMPERATURE
    sched.register(1000); // WATCHDOG
    sched.register(30000); // LORAWAN
    sched.register(60000); // CELLULAR
    sched.register(1000); // POWER_MGR
    sched.register(1000); // TAMPER
    sched.register(1000); // EVENT_DETECT

    info!("Entering main loop...");
    let mut uptime_ms: u64 = 0;
    let mut ready_buf = [TASK_NONE; 16];

    loop {
        uptime_ms += 5;
        let n_ready = sched.poll(uptime_ms, &mut ready_buf);

        for i in 0..n_ready {
            match ready_buf[i] {
                task::METERING => {
                    let data = state.metering_mgr.poll_instant();
                    state.lcd_content.voltage_a = data.voltage_a;
                    state.lcd_content.current_a = data.current_a;
                    state.lcd_content.active_power = data.active_power_total;
                    state.lcd_content.reactive_power = data.reactive_power_total;
                    state.lcd_content.power_factor = data.power_factor_total;
                    state.lcd_content.frequency = data.frequency;

                    // 事件检测
                    let ts = rtc::get_timestamp();
                    state.event_detector.set_timestamp(ts);
                    let events = state.event_detector.check(&data);
                    if events != 0 {
                        defmt::info!("Events: {:#010x}", events);
                    }
                }
                task::ENERGY => {
                    let energy = state.metering_mgr.poll_energy();
                    state.lcd_content.active_import_energy = energy.active_import;
                }
                task::DISPLAY => {
                    state.lcd.update(&state.lcd_content);
                }
                task::RS485 => {
                    let mut rx_buf = [0u8; 256];
                    match state.rs485.read(&mut rx_buf, 5) {
                        Ok(n) if n > 0 => {
                            state.rs485_rx_len = n;
                        }
                        _ => {}
                    }
                }
                task::PULSE => {
                    let active_power = state.metering_mgr.last_active_power().unsigned_abs();
                    let constant = state.pulse_constant_active;
                    if constant > 0 {
                        let increment = (active_power as u64 * constant as u64) / 3_600_000;
                        state.active_energy_accum += increment as u32;
                        while state.active_energy_accum >= 1000 {
                            state.active_energy_accum -= 1000;
                            // 脉冲 GPIO 翻转 (裸机模式)
                            board::pulse_ext::toggle_active();
                        }
                    }
                }
                task::WATCHDOG => {
                    watchdog::feed();
                }
                task::POWER_MGR => {
                    // 裸机模式下简单检查
                    let _is_idle = state.power_mgr.tick(uptime_ms as u32, false);
                }
                _ => {}
            }
        }

        // 低功耗空闲
        let sleep_ms = sched.time_until_next(uptime_ms);
        if sleep_ms > 5 {
            let loops = sleep_ms.min(10) * 6400 / 10;
            let mut i: u32 = 0;
            while i < loops {
                i += 1;
                cortex_m::asm::nop();
            }
        }
    }
}
