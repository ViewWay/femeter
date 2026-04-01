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
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

#![no_main]
#![no_std]
#![allow(invalid_reference_casting)]
#![allow(unused)]

use defmt_rtt as _;
use cortex_m_rt::entry;
use panic_halt as _;
use defmt::{info, warn, error, debug};
use crate::hal::{MeteringChip, LcdDriver, UartDriver};
use core::ffi::c_void;

// defmt 要求用户提供 _defmt_timestamp 实现 (空 = 无时间戳, 省空间)
#[no_mangle]
unsafe extern "C" fn _defmt_timestamp() {}

/* ── HAL 抽象层 ── */
mod hal;
mod fm33lg0;

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

/* ── 板级驱动 ── */
mod board;
mod display;
mod metering;
mod comm;

/* ── FreeRTOS (feature gate) ── */
#[cfg(feature = "freertos")]
mod freertos;
#[cfg(feature = "freertos")]
mod freertos_hooks;

/* ── 事件检测 & 存储 ── */
mod event_detect;
#[cfg(feature = "ext-flash")]
mod storage;

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
    task_create, delay_ms, tick_count, ms_to_ticks,
    Mutex, Queue, EventGroup, TaskParams, PORT_MAX_DELAY, events,
};

#[cfg(feature = "freertos")]
struct SharedState<M: hal::MeteringChip> {
    metering_mgr: metering::MeteringManager<M>,
    lcd: display::LcdPanel,
    lcd_content: hal::LcdContent,
    pulse_constant_active: u32,
    pulse_constant_reactive: u32,
    active_energy_accum: u32,
    reactive_energy_accum: u32,
    event_detector: event_detect::EventDetector,
}

#[cfg(feature = "freertos")]
static mut SHARED_STATE: *mut SharedState<Metering> = core::ptr::null_mut();

#[cfg(feature = "freertos")]
static mut STATE_MUTEX: *mut c_void = core::ptr::null_mut();

#[cfg(feature = "freertos")]
static mut SYS_EVENTS: *mut c_void = core::ptr::null_mut();

/* ══════════════════════════════════════════════════════════════════ */
/*  [FreeRTOS] 任务优先级 & 栈深度                                      */
/* ══════════════════════════════════════════════════════════════════ */

#[cfg(feature = "freertos")]
mod prio {
    pub const RS485:      u32 = 5;
    pub const INFRARED:   u32 = 4;
    pub const METERING:   u32 = 3;
    pub const PULSE:      u32 = 3;
    pub const KEY:        u32 = 2;
    pub const DISPLAY:    u32 = 2;
    pub const ENERGY:     u32 = 2;
    pub const WATCHDOG:   u32 = 2;
    pub const TAMPER:     u32 = 1;
    pub const TEMPERATURE:u32 = 1;
    pub const LORAWAN:    u32 = 1;
    pub const CELLULAR:   u32 = 1;
}

#[cfg(feature = "freertos")]
const STACK_SMALL:  u16 = 128;  // 512 bytes
#[cfg(feature = "freertos")]
const STACK_MEDIUM: u16 = 192;  // 768 bytes
#[cfg(feature = "freertos")]
const STACK_LARGE:  u16 = 256;  // 1024 bytes

/* ══════════════════════════════════════════════════════════════════ */
/*  [FreeRTOS] 辅助函数                                                 */
/* ══════════════════════════════════════════════════════════════════ */

/// 获取共享状态指针并加锁
///
/// 返回 (state_ptr, mutex_ref)
/// 调用者必须手动 unlock_state
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

/* ══════════════════════════════════════════════════════════════════ */
/*  [FreeRTOS] 任务函数                                                 */
/* ══════════════════════════════════════════════════════════════════ */

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_metering_entry(_arg: *mut c_void) {
    info!("Task: Metering started");
    loop {
        let (state, mtx) = lock_state();
        let data = (*state).metering_mgr.poll_instant();
        (*state).lcd_content.voltage = data.voltage_a;
        (*state).lcd_content.current = data.current_a;
        (*state).lcd_content.active_power = data.active_power_total;
        (*state).lcd_content.reactive_power = data.reactive_power_total;
        (*state).lcd_content.power_factor = data.power_factor_total;
        (*state).lcd_content.frequency = data.frequency;

        // 事件检测
        let new_events = (*state).event_detector.check(&data);
        if new_events != 0 {
            defmt::info!("Events: {:#010x}", new_events);
        }

        unlock_state(mtx);

        let ev = EventGroup::from_raw(SYS_EVENTS);
        ev.set(events::LCD_REFRESH);
        delay_ms(200);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_energy_entry(_arg: *mut c_void) {
    info!("Task: Energy started");
    loop {
        let (state, mtx) = lock_state();
        let energy = (*state).metering_mgr.poll_energy();
        (*state).lcd_content.active_import_energy = energy.active_import;
        unlock_state(mtx);
        delay_ms(1000);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_display_entry(_arg: *mut c_void) {
    info!("Task: Display started");
    let ev = EventGroup::from_raw(SYS_EVENTS);
    loop {
        ev.wait(events::LCD_REFRESH, false, true, 500);
        let (state, mtx) = lock_state();
        (*state).lcd.update(&(*state).lcd_content);
        unlock_state(mtx);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_rs485_entry(_arg: *mut c_void) {
    info!("Task: RS485 started");
    loop {
        // TODO: UART ISR + 队列驱动 (目前占位)
        delay_ms(10);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_infrared_entry(_arg: *mut c_void) {
    info!("Task: Infrared started");
    loop {
        // TODO: IEC 62056-21 红外收发
        delay_ms(50);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_key_entry(_arg: *mut c_void) {
    info!("Task: Key scan started");
    loop {
        // TODO: GPIO 按键读取, 去抖
        delay_ms(50);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_pulse_entry(_arg: *mut c_void) {
    info!("Task: Pulse started");
    loop {
        let (state, mtx) = lock_state();
        let active_power = (*state).metering_mgr.poll_instant().active_power_total as u32;
        (*state).active_energy_accum += active_power / 36;
        if (*state).active_energy_accum >= 1000 {
            (*state).active_energy_accum -= 1000;
            // TODO: 脉冲 GPIO 翻转
        }
        unlock_state(mtx);
        delay_ms(100);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_tamper_entry(_arg: *mut c_void) {
    info!("Task: Tamper started");
    loop {
        // TODO: 上盖/端子盖/磁场检测
        delay_ms(5000);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_temperature_entry(_arg: *mut c_void) {
    info!("Task: Temperature started");
    loop {
        // TODO: ADC 温度采集
        delay_ms(10000);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_watchdog_entry(_arg: *mut c_void) {
    info!("Task: Watchdog started");
    loop {
        // TODO: IWDT 喂狗
        delay_ms(1000);
    }
}

#[cfg(feature = "freertos")]
unsafe extern "C" fn task_lorawan_entry(_arg: *mut c_void) {
    info!("Task: LoRaWAN started");
    loop {
        // TODO: ASR6601 AT 指令上报
        delay_ms(30000);
    }
}

#[cfg(all(feature = "freertos", feature = "cellular"))]
unsafe extern "C" fn task_cellular_entry(_arg: *mut c_void) {
    info!("Task: Cellular started");
    loop {
        // TODO: EC800N/BC260Y MQTT/CoAP
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

    // 1. 硬件初始化
    let chip = Metering::new(
        unsafe { att7022e::BoardSpi0 },
        unsafe { att7022e::BoardCs0 },
    );
    let mut board = board::Board::new(chip);
    board.init();
    info!("Board initialized");

    // 2. 计量管理器
    let calib = hal::CalibrationParams::default();
    let metering_mgr = metering::MeteringManager::new(board.metering, calib);
    info!("Metering manager initialized");

    // 3. LCD
    let mut lcd = display::LcdPanel::new();
    lcd.init_hw();
    lcd.init();
    info!("LCD initialized");

    // 4. RS485
    board.rs485.init(&hal::UartConfig {
        baudrate: 9600,
        data_bits: 8,
        stop_bits: 1,
        parity: hal::Parity::Even,
    }).ok();
    info!("RS485 initialized (9600 8E1)");

    // 5. FreeRTOS 同步原语
    let state_mutex = Mutex::new().expect("state_mutex create failed");
    let sys_events = EventGroup::new().expect("event_group create failed");

    // 6. 共享状态
    let mut shared = SharedState {
        metering_mgr,
        lcd,
        lcd_content: hal::LcdContent::default(),
        pulse_constant_active: 6400,
        pulse_constant_reactive: 6400,
        active_energy_accum: 0,
        reactive_energy_accum: 0,
        event_detector: event_detect::EventDetector::new(),
    };

    unsafe {
        SHARED_STATE = &mut shared as *mut SharedState<Metering>;
        STATE_MUTEX = Mutex::into_raw(state_mutex);
        SYS_EVENTS = EventGroup::into_raw(sys_events);
    }

    // 7. 创建任务
    let task_defs: &[(&str, unsafe extern "C" fn(*mut c_void), u32, u16)] = &[
        ("metering",    task_metering_entry,     prio::METERING,    STACK_MEDIUM),
        ("energy",      task_energy_entry,        prio::ENERGY,      STACK_SMALL),
        ("display",     task_display_entry,       prio::DISPLAY,     STACK_MEDIUM),
        ("rs485",       task_rs485_entry,         prio::RS485,       STACK_LARGE),
        ("infrared",    task_infrared_entry,      prio::INFRARED,    STACK_MEDIUM),
        ("key",         task_key_entry,           prio::KEY,         STACK_SMALL),
        ("pulse",       task_pulse_entry,         prio::PULSE,       STACK_SMALL),
        ("tamper",      task_tamper_entry,        prio::TAMPER,      STACK_SMALL),
        ("temperature", task_temperature_entry,   prio::TEMPERATURE, STACK_SMALL),
        ("watchdog",    task_watchdog_entry,      prio::WATCHDOG,    STACK_SMALL),
        ("lorawan",     task_lorawan_entry,       prio::LORAWAN,     STACK_MEDIUM),
    ];

    let mut count: u8 = 0;
    for &(name, func, p, stack) in task_defs {
        match task_create(func, TaskParams {
            name,
            stack_depth: stack,
            priority: p,
            arg: core::ptr::null_mut(),
        }) {
            Ok(_) => { info!("  + {} (prio={}, stack={})", name, p, stack); count += 1; }
            Err(_) => { error!("  FAILED: {}", name); }
        }
    }

    // 蜂窝任务 (需要单独处理, 因为 feature gate)
    #[cfg(feature = "cellular")]
    {
        match task_create(task_cellular_entry, TaskParams {
            name: "cellular",
            stack_depth: STACK_MEDIUM,
            priority: prio::CELLULAR,
            arg: core::ptr::null_mut(),
        }) {
            Ok(_) => { info!("  + cellular (prio={}, stack={})", prio::CELLULAR, STACK_MEDIUM); count += 1; }
            Err(_) => { error!("  FAILED: cellular"); }
        }
    }

    info!("{} tasks created, starting scheduler...", count);

    // 8. 启动调度器 (永不返回)
    unsafe { freertos::vTaskStartScheduler() };
}

/* ══════════════════════════════════════════════════════════════════ */
/*  [Bare-metal] 裸机主入口 (freertos feature 关闭时)                   */
/* ══════════════════════════════════════════════════════════════════ */

#[cfg(not(feature = "freertos"))]
use task_scheduler::{TaskScheduler, TASK_NONE, task};

#[cfg(not(feature = "freertos"))]
struct BareState<M: hal::MeteringChip> {
    metering_mgr: metering::MeteringManager<M>,
    lcd: display::LcdPanel,
    lcd_content: hal::LcdContent,
    rs485: board::UartChannelDriver,
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

    let chip = Metering::new(
        unsafe { att7022e::BoardSpi0 },
        unsafe { att7022e::BoardCs0 },
    );
    let mut board = board::Board::new(chip);
    board.init();

    let calib = hal::CalibrationParams::default();
    let metering_mgr = metering::MeteringManager::new(board.metering, calib);

    let mut lcd = display::LcdPanel::new();
    lcd.init_hw();
    lcd.init();

    board.rs485.init(&hal::UartConfig {
        baudrate: 9600, data_bits: 8, stop_bits: 1, parity: hal::Parity::Even,
    }).ok();

    let mut state = BareState {
        metering_mgr, lcd,
        lcd_content: hal::LcdContent::default(),
        rs485: board.rs485,
        pulse_constant_active: 6400,
        pulse_constant_reactive: 6400,
        active_energy_accum: 0,
        reactive_energy_accum: 0,
        heartbeat_count: 0,
        rs485_rx_len: 0,
    };

    let mut sched = TaskScheduler::new();
    sched.register(200); sched.register(1000); sched.register(500);
    sched.register(10);  sched.register(50);   sched.register(50);
    sched.register(100); sched.register(5000);  sched.register(10000);
    sched.register(1000);sched.register(30000); sched.register(60000);

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
                    state.lcd_content.voltage = data.voltage_a;
                    state.lcd_content.current = data.current_a;
                    state.lcd_content.active_power = data.active_power_total;
                    state.lcd_content.reactive_power = data.reactive_power_total;
                    state.lcd_content.power_factor = data.power_factor_total;
                    state.lcd_content.frequency = data.frequency;
                }
                task::ENERGY => {
                    let energy = state.metering_mgr.poll_energy();
                    state.lcd_content.active_import_energy = energy.active_import;
                }
                task::DISPLAY => { state.lcd.update(&state.lcd_content); }
                task::RS485 => {
                    let mut rx_buf = [0u8; 256];
                    match state.rs485.read(&mut rx_buf, 5) {
                        Ok(n) if n > 0 => { state.rs485_rx_len = n; }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        let sleep_ms = sched.time_until_next(uptime_ms);
        if sleep_ms > 5 {
            let loops = sleep_ms.min(10) * 6400 / 10;
            let mut i: u32 = 0;
            while i < loops { i += 1; cortex_m::asm::nop(); }
        }
    }
}
