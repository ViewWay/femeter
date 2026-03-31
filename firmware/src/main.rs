/* ================================================================== */
/*                                                                    */
/*  main.rs — FeMeter 三相智能电表固件入口                              */
/*                                                                    */
/*  硬件: FM33A068EV (512KB Flash, 80KB SRAM, Cortex-M0+ @ 64MHz)      */
/*  计量: ATT7022E / RN8302B / RN8615V2 (编译时选择)                   */
/*  通信: RS485 + 红外 + LoRaWAN + Cat.1/NB-IoT                       */
/*  显示: 4COM×44SEG 段码 LCD                                        */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

#![no_main]
#![no_std]
#![allow(invalid_reference_casting)]

use defmt_rtt as _;
use cortex_m_rt::entry;
use panic_halt as _;
use defmt::{info, warn, error, debug};
use crate::hal::{MeteringChip, LcdDriver, UartDriver};

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
mod task_scheduler;

use task_scheduler::{TaskScheduler, TaskId, TASK_NONE, task};

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
/*  应用状态                                                           */
/* ══════════════════════════════════════════════════════════════════ */

/// 全局应用状态
struct AppState<M: hal::MeteringChip> {
    /// 计量管理器 (封装计量芯片 + 电能累计 + 费率)
    metering_mgr: metering::MeteringManager<M>,
    /// LCD 驱动
    lcd: display::LcdPanel,
    /// LCD 显示内容
    lcd_content: hal::LcdContent,
    /// RS485 通信
    rs485: board::UartChannelDriver,
    /// 脉冲常数 (有功, imp/kWh)
    pulse_constant_active: u32,
    /// 脉冲常数 (无功, imp/kvarh)
    pulse_constant_reactive: u32,
    /// 有功电能小数累加器
    active_energy_accum: u32,
    /// 无功电能小数累加器
    reactive_energy_accum: u32,
    /// 心跳 LED 闪烁计数
    heartbeat_count: u32,
    /// 上次 RS485 收到的数据长度
    rs485_rx_len: usize,
}

/* ══════════════════════════════════════════════════════════════════ */
/*  主入口                                                             */
/* ══════════════════════════════════════════════════════════════════ */

#[entry]
fn main() -> ! {
    info!("FeMeter v{} starting...", VERSION);
    info!("MCU: FM33A068EV (512KB/80KB)");
    info!("Metering: {}", Metering::name());

    // 1. 硬件初始化
    use crate::att7022e::{BoardSpi0, BoardCs0};
    let chip = Metering::new(BoardSpi0, BoardCs0);
    let mut board = board::Board::new(chip);
    board.init();
    info!("Board initialized");

    // 2. 初始化计量管理器 (含校准)
    let calib = hal::CalibrationParams::default();
    let metering_mgr = metering::MeteringManager::new(board.metering, calib);
    info!("Metering manager initialized");

    // 3. 初始化 LCD
    let mut lcd = display::LcdPanel::new();
    lcd.init_hw();
    lcd.init();
    info!("LCD initialized");

    // 4. 初始化 RS485
    board.rs485.init(&hal::UartConfig {
        baudrate: 9600,
        data_bits: 8,
        stop_bits: 1,
        parity: hal::Parity::Even,
    }).ok();
    info!("RS485 initialized (9600 8E1)");

    // 5. 应用状态
    let mut state = AppState {
        metering_mgr,
        lcd,
        lcd_content: hal::LcdContent::default(),
        rs485: board.rs485,
        pulse_constant_active: 6400,
        pulse_constant_reactive: 6400,
        active_energy_accum: 0,
        reactive_energy_accum: 0,
        heartbeat_count: 0,
        rs485_rx_len: 0,
    };

    // 6. 注册任务
    let mut sched = TaskScheduler::new();
    sched.register(200);   // 0: 计量采样
    sched.register(1000);  // 1: 电能累计
    sched.register(500);   // 2: LCD 刷新
    sched.register(10);    // 3: RS485 通信
    sched.register(50);    // 4: 红外通信
    sched.register(50);    // 5: 按键扫描
    sched.register(100);   // 6: 脉冲输出
    sched.register(5000);  // 7: 防窃电
    sched.register(10000); // 8: 温度
    sched.register(1000);  // 9: 看门狗
    sched.register(30000); // 10: LoRaWAN
    sched.register(60000); // 11: 蜂窝

    info!("Task scheduler initialized (12 tasks)");

    // 7. 进入主循环
    info!("Entering main loop...");
    let mut uptime_ms: u64 = 0;
    let mut ready_buf = [TASK_NONE; 16];

    loop {
        uptime_ms += 5; // 假设每次循环 ~5ms

        // 查询就绪任务
        let n_ready = sched.poll(uptime_ms, &mut ready_buf);

        // 执行就绪任务
        for i in 0..n_ready {
            let task_id = ready_buf[i];
            match task_id {
                task::METERING => task_metering(&mut state),
                task::ENERGY => task_energy(&mut state),
                task::DISPLAY => task_display(&mut state),
                task::RS485 => task_rs485(&mut state),
                task::KEY => task_key(&mut state),
                task::PULSE => task_pulse(&mut state),
                task::TAMPER => task_tamper(&mut state),
                task::WATCHDOG => task_watchdog(&mut state),
                _ => {}
            }
        }

        // 低功耗延时 (等待下一个任务)
        let sleep_ms = sched.time_until_next(uptime_ms);
        if sleep_ms > 5 {
            delay_ms(sleep_ms.min(10)); // 最多睡 10ms, 避免错过中断
        } else {
            cortex_m::asm::nop();
        }
    }
}

/* ================================================================== */
/*  任务实现                                                           */
/* ================================================================== */

/// Task 0: 读取计量芯片实时数据 (200ms)
fn task_metering<M: hal::MeteringChip>(state: &mut AppState<M>) {
    let data = state.metering_mgr.poll_instant();
    // 更新 LCD 内容 (由 task_display 决定显示哪页)
    state.lcd_content.voltage = data.voltage_a; // 默认显示 A 相
    state.lcd_content.current = data.current_a;
    state.lcd_content.active_power = data.active_power_total;
    state.lcd_content.reactive_power = data.reactive_power_total;
    state.lcd_content.power_factor = data.power_factor_total;
    state.lcd_content.frequency = data.frequency;
}

/// Task 1: 电能累计 (1000ms)
fn task_energy<M: hal::MeteringChip>(state: &mut AppState<M>) {
    let energy = state.metering_mgr.poll_energy();
    state.lcd_content.active_import_energy = energy.active_import;
}

/// Task 2: LCD 刷新 (500ms)
fn task_display<M: hal::MeteringChip>(state: &mut AppState<M>) {
    state.lcd.update(&state.lcd_content);
}

/// Task 3: RS485 通信处理 (10ms)
fn task_rs485<M: hal::MeteringChip>(state: &mut AppState<M>) {
    let mut rx_buf = [0u8; 256];
    match state.rs485.read(&mut rx_buf, 5) {
        Ok(n) if n > 0 => {
            state.rs485_rx_len = n;
            debug!("RS485 rx {} bytes", n);
            // TODO: DLMS HDLC 帧解析 + COSEM 响应
        }
        _ => {}
    }
}

/// Task 5: 按键扫描 (50ms)
fn task_key<M: hal::MeteringChip>(state: &mut AppState<M>) {
    // TODO: 读取 GPIO 按键状态, 去抖, 生成事件
    // 翻页键 → lcd.next_page()
    // 编程键 → 进入/退出编程模式
    let _ = state;
}

/// Task 6: 脉冲输出 (100ms)
fn task_pulse<M: hal::MeteringChip>(state: &mut AppState<M>) {
    let active_power = state.metering_mgr.poll_instant().active_power_total as u32;
    let delta_wh = active_power / 36; // 100ms = 1/36 小时...简化计算
    state.active_energy_accum += delta_wh;
    if state.active_energy_accum >= 1000 {
        state.active_energy_accum -= 1000;
        // TODO: 脉冲输出 GPIO 翻转
    }
}

/// Task 7: 防窃电检测 (5000ms)
fn task_tamper<M: hal::MeteringChip>(state: &mut AppState<M>) {
    // TODO: 读取上盖/端子盖/磁场传感器
    // 检测到异常 → 告警 LED + 蜂鸣器 + 事件记录
    let _ = state;
}

/// Task 9: 看门狗喂狗 (1000ms)
fn task_watchdog<M: hal::MeteringChip>(_state: &mut AppState<M>) {
    // TODO: IWDT 喂狗 (写 0x12345678 到 IWDT_SERV)
    // 目前无看门狗, 仅心跳 LED
}

/* ================================================================== */
/*  延时函数                                                           */
/* ================================================================== */

fn delay_ms(ms: u32) {
    let loops = ms * 6400 / 10;
    let mut i: u32 = 0;
    while i < loops {
        i += 1;
        cortex_m::asm::nop();
    }
}
