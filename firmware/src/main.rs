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

use cortex_m_rt::entry;
use panic_halt as _;
use defmt::{info, warn, error, debug};

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
mod at_parser;
mod quectel;

/* ── 板级驱动 ── */
mod board;
mod display;
mod metering;
mod comm;

/* ══════════════════════════════════════════════════════════════════ */
/*  编译时计量芯片类型别名                                              */
/* ══════════════════════════════════════════════════════════════════ */

#[cfg(feature = "att7022e")]
type Metering = att7022e::Att7022e;
#[cfg(feature = "rn8302b")]
type Metering = rn8302b::Rn8302b;
#[cfg(feature = "rn8615v2")]
type Metering = rn8615v2::RN8615V2;

/// 固件版本
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 编译时间戳
const BUILD_TIMESTAMP: &str = concat!(env!("CARGO_PKG_VERSION"), " ", file!());

/* ══════════════════════════════════════════════════════════════════ */
/*  应用状态                                                           */
/* ══════════════════════════════════════════════════════════════════ */

/// 全局应用状态
struct AppState {
    /// 计量数据
    metering_data: hal::PhaseData,
    /// 电能数据
    energy_data: hal::EnergyData,
    /// LCD 内容
    lcd_content: hal::LcdContent,
    /// 电源状态
    power_state: hal::PowerState,
    /// 心跳计数 (ms)
    uptime_ms: u32,
    /// 轮显索引
    display_page: u8,
    /// 脉冲常数
    pulse_constant_active: u32,
    pulse_constant_reactive: u32,
    /// 有功电能小数累加器
    active_energy_accum: u32,
    /// 无功电能小数累加器
    reactive_energy_accum: u32,
}

impl AppState {
    const fn new() -> Self {
        Self {
            metering_data: hal::PhaseData::default(),
            energy_data: hal::EnergyData::default(),
            lcd_content: hal::LcdContent::default(),
            power_state: hal::PowerState::MainsNormal,
            uptime_ms: 0,
            display_page: 0,
            pulse_constant_active: 6400,
            pulse_constant_reactive: 6400,
            active_energy_accum: 0,
            reactive_energy_accum: 0,
        }
    }
}

/* ══════════════════════════════════════════════════════════════════ */
/*  主循环                                                             */
/* ══════════════════════════════════════════════════════════════════ */

#[entry]
fn main() -> ! {
    info!("FeMeter v{} starting...", VERSION);
    info!("MCU: FM33A068EV (512KB/80KB)");
    info!("Metering: {}", Metering::name());

    // 1. 硬件初始化
    let metering_chip = Metering::new();
    let mut board = board::Board::new(metering_chip);
    board.init();
    info!("Board initialized");

    // 2. 初始化计量芯片
    let calib = hal::CalibrationParams::default();
    board.metering.init(&calib).unwrap_or_else(|e| {
        error!("Metering chip init failed: {:?}", e);
    });
    info!("Metering chip initialized");

    // 3. 初始化 LCD
    board.lcd.init();
    board.lcd.set_mode(hal::LcdDisplayMode::AutoRotate { interval_sec: 5 });
    info!("LCD initialized");

    // 4. 初始化通信通道
    board.rs485.init(&hal::UartConfig {
        baudrate: 9600,
        data_bits: 8,
        stop_bits: 1,
        parity: hal::Parity::Even,
    }).ok();
    info!("RS485 initialized (9600 8E1)");

    // 5. 应用状态
    let mut state = AppState::new();

    // 6. 进入主循环
    info!("Entering main loop...");

    loop {
        state.uptime_ms += 10; // 假设每次循环 ~10ms

        // ── 读取计量数据 ──
        if let Ok(data) = board.metering.read_instant_data() {
            state.metering_data = data;
        }

        // ── 读取电能数据 (每 1 秒) ──
        if state.uptime_ms % 1000 == 0 {
            if let Ok(energy) = board.metering.read_energy() {
                state.energy_data = energy;
            }

            // ── 更新 LCD ──
            update_lcd_content(&mut state);
            board.lcd.update(&state.lcd_content);

            // ── 脉冲输出 ──
            // 根据功率积分电能, 脉冲输出
            let active_power_w = state.metering_data.active_power_total as u32;
            let delta_wh = active_power_w / 3600; // 简化: 每秒增量
            state.active_energy_accum += delta_wh;
            board.pulse.update_energy(hal::PulseType::Active, state.active_energy_accum / 1000);
            if state.active_energy_accum >= 1000 {
                state.active_energy_accum -= 1000;
            }

            let reactive_power_var = state.metering_data.reactive_power_total.abs() as u32;
            let delta_varh = reactive_power_var / 3600;
            state.reactive_energy_accum += delta_varh;
            board.pulse.update_energy(hal::PulseType::Reactive, state.reactive_energy_accum / 1000);
            if state.reactive_energy_accum >= 1000 {
                state.reactive_energy_accum -= 1000;
            }
        }

        // ── 通信处理 (每 100ms) ──
        if state.uptime_ms % 100 == 0 {
            // RS485 DLMS 数据处理
            comm_process(&mut board, &state);

            // 红外数据处理
            // TODO: IEC 62056-21 处理
        }

        // ── 按键处理 ──
        // TODO: 按键扫描和事件分发

        // ── 防窃电检测 (每 5 秒) ──
        if state.uptime_ms % 5000 == 0 {
            if let Some(event) = board.tamper.check_events() {
                warn!("Tamper detected: {:?}", event);
                board.indicator.buzzer_alarm(200);
                board.indicator.set_led(hal::Led::Alarm, true);
            }
        }

        // ── 电网质量事件 (RN8615V2) ──
        #[cfg(feature = "pq-analysis")]
        {
            use hal::PowerQuality;
            if let Some(pq_event) = board.metering.check_pq_event() {
                warn!("Power quality event: {:?}", pq_event);
            }
        }

        // ── 简单延时 ──
        delay_ms(10);
    }
}

/* ================================================================== */
/*  LCD 内容更新                                                       */
/* ================================================================== */

fn update_lcd_content(state: &mut AppState) {
    let d = &state.metering_data;
    let e = &state.energy_data;

    // 自动轮显: 每 5 秒切换一页
    state.display_page = ((state.uptime_ms / 5000) % 8) as u8;

    match state.display_page {
        0 => {
            // 第 1 页: A 相电压 + A 相电流
            state.lcd_content.voltage = d.voltage_a;
            state.lcd_content.current = d.current_a;
        }
        1 => {
            // 第 2 页: B 相电压 + B 相电流
            state.lcd_content.voltage = d.voltage_b;
            state.lcd_content.current = d.current_b;
        }
        2 => {
            // 第 3 页: C 相电压 + C 相电流
            state.lcd_content.voltage = d.voltage_c;
            state.lcd_content.current = d.current_c;
        }
        3 => {
            // 第 4 页: 总有功功率
            state.lcd_content.active_power = d.active_power_total;
        }
        4 => {
            // 第 5 页: 总无功功率
            state.lcd_content.reactive_power = d.reactive_power_total;
        }
        5 => {
            // 第 6 页: 总功率因数 + 频率
            state.lcd_content.power_factor = d.power_factor_total;
            state.lcd_content.frequency = d.frequency;
        }
        6 => {
            // 第 7 页: 正向有功总电能
            state.lcd_content.active_import_energy = e.active_import;
        }
        7 => {
            // 第 8 页: 费率 + 通信状态
            state.lcd_content.tariff = 0;
            state.lcd_content.comm_status = 0;
        }
        _ => {}
    }
}

/* ================================================================== */
/*  通信处理                                                           */
/* ================================================================== */

fn comm_process(board: &mut board::Board<Metering>, state: &AppState) {
    // RS485 DLMS 接收缓冲区
    let mut rx_buf = [0u8; 256];

    match board.rs485.read(&mut rx_buf, 10) {
        Ok(n) if n > 0 => {
            debug!("RS485 received {} bytes", n);
            // TODO: DLMS HDLC 帧解析
            // TODO: COSEM 对象处理
            // TODO: 构造响应帧
        }
        _ => {}
    }
}

/* ================================================================== */
/*  简单延时                                                           */
/* ================================================================== */

fn delay_ms(ms: u32) {
    let loops = ms * 6400 / 10;
    let mut i: u32 = 0;
    while i < loops {
        i += 1;
        cortex_m::asm::nop();
    }
}
