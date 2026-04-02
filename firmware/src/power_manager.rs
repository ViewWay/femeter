/* ================================================================== */
/*                                                                    */
/*  power_manager.rs — 低功耗管理                                       */
/*                                                                    */
/*  状态机:                                                           */
/*    NormalRun → LowPowerRun → Sleep → DeepSleep                      */
/*    电池模式  → 始终 DeepSleep + RAM 保持                            */
/*                                                                    */
/*  唤醒源: RS485 / 红外 / 按键 / RTC 闹钟 / LoRaWAN / LPUART         */
/*  电池电压监测: ADC 通道 PF6 (ADC_IN5)                               */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::board::pins;
use crate::board::{gpio_read_pin, gpio_set_fcr, gpio_enable_input, gpio_enable_pullup};
use crate::fm33lg0;
use crate::fm33lg0::base;
use crate::board::{write_reg, read_reg};

/* ================================================================== */
/*  低功耗状态                                                         */
/* ================================================================== */

/// 低功耗状态
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerState {
    /// 正常运行（64MHz，所有外设开启）
    NormalRun,
    /// 低功耗运行（降频 4MHz，关闭非必要外设）
    LowPowerRun,
    /// 轻度睡眠（CPU WFI，UART 唤醒，LCD 保持）
    Sleep,
    /// 深度睡眠（主时钟关闭，LPUART 唤醒，LCD 关闭）
    DeepSleep,
    /// 深度睡眠 + RAM 保持（计量数据保持，最低功耗）
    DeepSleepRamRetain,
}

/// 唤醒源
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WakeupSource {
    /// RS485 通信字节
    Rs485,
    /// 红外通信
    Infrared,
    /// 按键
    Key,
    /// RTC 闹钟
    RtcAlarm,
    /// LoRaWAN 模组
    LoRaWAN,
    /// 掉电检测
    PowerFail,
    /// LPUART 接收唤醒（DeepSleep 下 UART2）
    LpUart,
    /// GPIO 唤醒
    Gpio,
    /// 未知（上电复位等）
    Unknown,
}

/// 电池电压告警等级
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BatteryAlert {
    /// 正常
    Normal,
    /// 低电压警告（建议更换）
    Low,
    /// 极低电压（即将关机）
    Critical,
}

/// 外设时钟门控配置
#[derive(Clone, Copy, Debug)]
pub struct ClockGateConfig {
    /// 睡眠时关闭的外设时钟位掩码（PCLKEN1）
    pub sleep_mask_pclken1: u32,
    /// 深度睡眠时关闭的外设时钟位掩码（PCLKEN1）
    pub deep_sleep_mask_pclken1: u32,
    /// 深度睡眠时关闭的外设时钟位掩码（PCLKEN2）
    pub deep_sleep_mask_pclken2: u32,
    /// 深度睡眠时关闭的外设时钟位掩码（PCLKEN3）
    pub deep_sleep_mask_pclken3: u32,
}

impl Default for ClockGateConfig {
    fn default() -> Self {
        Self {
            // Sleep: 关闭非必要外设
            sleep_mask_pclken1: 0x00,
            // DeepSleep: 保留 LPUART0, 关闭其余
            deep_sleep_mask_pclken1: 0x7E, // 关闭 SPI0/1, UART0/1/2/3
            deep_sleep_mask_pclken2: 0x4D, // 关闭 LCD, ADC, DMA 等
            deep_sleep_mask_pclken3: 0x43, // 关闭 I2C, SPI2 等
        }
    }
}

/* ================================================================== */
/*  超时阈值常量                                                       */
/* ================================================================== */

/// 无通信活动多久进入 LowPowerRun（秒）
const LP_RUN_TIMEOUT_SEC: u32 = 2 * 60;
/// 无通信活动多久进入 Sleep（秒）
const SLEEP_TIMEOUT_SEC: u32 = 5 * 60;
/// 无通信活动多久进入 DeepSleep（秒）
const DEEP_SLEEP_TIMEOUT_SEC: u32 = 30 * 60;

/* ================================================================== */
/*  电池电压阈值 (mV)                                                  */
/* ================================================================== */

/// ER26500 锂亚电池: 标称 3.6V
const BAT_LOW_MV: u16 = 3300;
const BAT_CRITICAL_MV: u16 = 3000;

/* ================================================================== */
/*  PowerManager                                                       */
/* ================================================================== */

/// 低功耗管理器
///
/// 管理 MCU 的功耗状态切换，包括：
/// - 正常运行 → 低功耗运行 → 睡眠 → 深度睡眠的渐进式切换
/// - 多种唤醒源检测
/// - 外设时钟门控
/// - 电池电压监测
/// - FreeRTOS tickless idle 集成
pub struct PowerManager {
    /// 当前状态
    state: PowerState,
    /// 进入低功耗前的状态（唤醒后恢复用）
    pre_sleep_state: PowerState,
    /// 是否电池供电模式
    battery_powered: bool,
    /// 上次通信活动时间（FreeRTOS tick, 1 tick = 1ms）
    last_activity_tick: u32,
    /// 电池电压（mV, 上次 ADC 读取值）
    battery_mv: u16,
    /// 电池告警状态
    battery_alert: BatteryAlert,
    /// 唤醒源（上次唤醒原因）
    last_wakeup: WakeupSource,
    /// 当前 tick
    current_tick: u32,
    /// 时钟门控配置
    clock_gate: ClockGateConfig,
    /// 是否处于编程模式（不进入低功耗）
    programming_mode: bool,
    /// RTC 闹钟唤醒使能
    rtc_alarm_wakeup: bool,
    /// LPUART 唤醒使能
    lpuart_wakeup: bool,
    /// 唤醒次数统计
    wakeup_count: u32,
}

impl PowerManager {
    /// 创建低功耗管理器
    pub fn new() -> Self {
        Self {
            state: PowerState::NormalRun,
            pre_sleep_state: PowerState::NormalRun,
            battery_powered: false,
            last_activity_tick: 0,
            battery_mv: 3600,
            battery_alert: BatteryAlert::Normal,
            last_wakeup: WakeupSource::Unknown,
            current_tick: 0,
            clock_gate: ClockGateConfig::default(),
            programming_mode: false,
            rtc_alarm_wakeup: false,
            lpuart_wakeup: true,
            wakeup_count: 0,
        }
    }

    /// 初始化
    ///
    /// 配置电池 ADC 引脚、判断供电模式、读取初始电池电压。
    pub fn init(&mut self) {
        // 配置电池 ADC 引脚: PF6 → 模拟模式
        gpio_set_fcr(pins::BAT_ADC.port, pins::BAT_ADC.pin, 0b11);
        let gpio = crate::board::gpio_port(pins::BAT_ADC.port);
        unsafe {
            write_reg(
                &gpio.anen as *const u32 as *mut u32,
                read_reg(&gpio.anen as *const u32) | (1u32 << pins::BAT_ADC.pin),
            );
        }

        // 首次电池电压读取
        self.battery_mv = self.read_battery_mv();
        self.battery_alert = self.check_battery_alert();

        // 判断供电模式
        self.battery_powered = !gpio_read_pin(pins::POWER_FAIL);

        // 电池模式直接进入 DeepSleep
        if self.battery_powered {
            self.state = PowerState::DeepSleepRamRetain;
        }

        // 配置 LPUART 唤醒引脚（UART2 RX → PA3）
        // LPUART 在 DeepSleep 下保持唤醒能力
        self.configure_lpuart_wakeup();

        defmt::info!(
            "电源管理初始化: mode={}, battery={}mV",
            if self.battery_powered { "BAT" } else { "MAINS" },
            self.battery_mv
        );
    }

    /// 周期性调用（在主循环或 FreeRTOS 任务中）
    ///
    /// 参数:
    ///   - `tick`: 当前 FreeRTOS tick (ms)
    ///   - `programming_mode`: 是否处于编程模式
    ///
    /// 返回: 是否应该进入低功耗（调用者应调用 `enter_low_power()`）
    pub fn tick(&mut self, tick: u32, programming_mode: bool) -> bool {
        self.current_tick = tick;
        self.programming_mode = programming_mode;

        if programming_mode {
            if self.state != PowerState::NormalRun {
                self.state = PowerState::NormalRun;
            }
            return false;
        }

        let idle_ms = tick.wrapping_sub(self.last_activity_tick);
        let idle_sec = idle_ms / 1000;

        // 电池模式: 始终 DeepSleep
        if self.battery_powered {
            if self.state != PowerState::DeepSleepRamRetain {
                self.pre_sleep_state = self.state;
                self.state = PowerState::DeepSleepRamRetain;
                return true;
            }
            return false;
        }

        // 市电模式: 根据空闲时间递进
        if idle_sec >= DEEP_SLEEP_TIMEOUT_SEC && self.state != PowerState::DeepSleep {
            self.pre_sleep_state = self.state;
            self.state = PowerState::DeepSleep;
            return true;
        } else if idle_sec >= SLEEP_TIMEOUT_SEC && self.state != PowerState::Sleep {
            self.pre_sleep_state = self.state;
            self.state = PowerState::Sleep;
            return true;
        } else if idle_sec >= LP_RUN_TIMEOUT_SEC && self.state == PowerState::NormalRun {
            self.state = PowerState::LowPowerRun;
            self.apply_clock_gate(PowerState::LowPowerRun);
            return false;
        }

        false
    }

    /// 记录通信活动（任何 UART 收发时调用）
    pub fn record_activity(&mut self, tick: u32) {
        self.last_activity_tick = tick;
        if self.state != PowerState::NormalRun {
            // 唤醒后恢复时钟门控
            self.restore_clock_gate();
            self.state = PowerState::NormalRun;
        }
    }

    /// 进入低功耗模式
    ///
    /// 根据当前 state 执行对应的低功耗流程。
    /// 返回唤醒源。
    pub fn enter_low_power(&mut self) -> WakeupSource {
        match self.state {
            PowerState::NormalRun => {
                return WakeupSource::Unknown;
            }
            PowerState::LowPowerRun => {
                // 低功耗运行不实际进入睡眠，只是降频
                return WakeupSource::Unknown;
            }
            PowerState::Sleep => {
                self.enter_sleep();
            }
            PowerState::DeepSleep => {
                self.enter_deep_sleep(false);
            }
            PowerState::DeepSleepRamRetain => {
                self.enter_deep_sleep(true);
            }
        }

        self.wakeup_count = self.wakeup_count.wrapping_add(1);
        self.on_wakeup()
    }

    /// 进入 Sleep 模式
    ///
    /// - CPU 执行 WFI 等待中断
    /// - UART 保持接收能力
    /// - LCD 保持显示
    fn enter_sleep(&self) {
        let pmu = fm33lg0::pmu();

        // 应用 Sleep 时钟门控
        self.apply_clock_gate(PowerState::Sleep);

        unsafe {
            // 设置 Sleep 模式
            let cr = read_reg(&pmu.cr as *const u32);
            write_reg(&pmu.cr as *const u32 as *mut u32, (cr & !0x03) | 0x01);

            // 使能 GPIO 唤醒
            let gc = fm33lg0::gpio_common();
            write_reg(&gc.pinwken as *const u32 as *mut u32,
                read_reg(&gc.pinwken as *const u32) | 0x03);
        }

        cortex_m::asm::wfi();

        unsafe {
            let cr = read_reg(&pmu.cr as *const u32);
            write_reg(&pmu.cr as *const u32 as *mut u32, cr & !0x03);
        }
    }

    /// 进入 Deep Sleep 模式
    ///
    /// - 关闭主时钟（PLL_H, HCLK），保留 LSI/LSE
    /// - 关闭 LCD
    /// - LPUART 唤醒（如果使能）
    /// - RTC 闹钟唤醒（如果使能）
    /// - `ram_retain`: 是否保持 SRAM 内容
    fn enter_deep_sleep(&self, ram_retain: bool) {
        let pmu = fm33lg0::pmu();

        // 1. 关闭 LCD
        unsafe {
            let lcd = fm33lg0::lcd();
            let cr = read_reg(&lcd.cr as *const u32);
            write_reg(&lcd.cr as *const u32 as *mut u32, cr & !fm33lg0::lcd_cr::EN);
        }

        // 2. 应用深度睡眠时钟门控（关闭所有非必要外设）
        self.apply_clock_gate(if ram_retain {
            PowerState::DeepSleepRamRetain
        } else {
            PowerState::DeepSleep
        });

        // 3. 配置唤醒源
        unsafe {
            // GPIO 唤醒（按键）
            let gc = fm33lg0::gpio_common();
            write_reg(&gc.pinwken as *const u32 as *mut u32,
                read_reg(&gc.pinwken as *const u32) | 0x03);

            // LPUART 唤醒（DeepSleep 下 UART2 接收唤醒）
            if self.lpuart_wakeup {
                // 使能 UART2 在低功耗模式下接收
                // 通过 PMU WKTR 寄存器配置
                // TODO: 确认 FM33A068EV 的 LPUART 唤醒寄存器
                // 暂时通过保持 UART2 时钟实现
            }
        }

        // 4. 配置 PMU
        unsafe {
            let lpms = if ram_retain { 0x03 } else { 0x02 };
            let cr = read_reg(&pmu.cr as *const u32);
            write_reg(&pmu.cr as *const u32 as *mut u32, (cr & !0x03) | lpms);
        }

        // 5. 执行 WFI
        cortex_m::asm::wfi();

        // 6. 唤醒后恢复
        self.restore_peripherals();
    }

    /// 恢复所有外设
    fn restore_peripherals(&self) {
        let pmu = fm33lg0::pmu();

        // 1. 恢复 Run 模式
        unsafe {
            let cr = read_reg(&pmu.cr as *const u32);
            write_reg(&pmu.cr as *const u32 as *mut u32, cr & !0x03);
        }

        // 2. 等待 PLL 锁定
        let cmu = fm33lg0::cmu();
        unsafe {
            let mut timeout = 10000u32;
            while (read_reg(&cmu.isr as *const u32) & 0x04) == 0 {
                timeout -= 1;
                if timeout == 0 { break; }
            }
        }

        // 3. 恢复时钟门控
        self.restore_clock_gate();

        // 4. 重新开启 LCD
        unsafe {
            let lcd = fm33lg0::lcd();
            let cr = read_reg(&lcd.cr as *const u32);
            write_reg(&lcd.cr as *const u32 as *mut u32, cr | fm33lg0::lcd_cr::EN);
        }

        // 5. 恢复 UART 配置
        // TODO: 重新初始化 RS485/红外 UART 的波特率
    }

    /// 应用时钟门控
    ///
    /// 根据目标状态关闭不必要的外设时钟。
    fn apply_clock_gate(&self, target_state: PowerState) {
        let cmu = fm33lg0::cmu();
        unsafe {
            match target_state {
                PowerState::LowPowerRun => {
                    // 关闭 SPI0, SPI1（计量和 Flash 不需要持续运行）
                    write_reg(&cmu.pclken1 as *const u32 as *mut u32,
                        read_reg(&cmu.pclken1 as *const u32) & !0x05);
                }
                PowerState::Sleep => {
                    // Sleep: 关闭 SPI, LCD
                    write_reg(&cmu.pclken1 as *const u32 as *mut u32,
                        read_reg(&cmu.pclken1 as *const u32)
                        & !self.clock_gate.sleep_mask_pclken1);
                }
                PowerState::DeepSleep | PowerState::DeepSleepRamRetain => {
                    // DeepSleep: 仅保留 LPUART
                    write_reg(&cmu.pclken1 as *const u32 as *mut u32,
                        read_reg(&cmu.pclken1 as *const u32)
                        & !self.clock_gate.deep_sleep_mask_pclken1);
                    write_reg(&cmu.pclken2 as *const u32 as *mut u32,
                        read_reg(&cmu.pclken2 as *const u32)
                        & !self.clock_gate.deep_sleep_mask_pclken2);
                    write_reg(&cmu.pclken3 as *const u32 as *mut u32,
                        read_reg(&cmu.pclken3 as *const u32)
                        & !self.clock_gate.deep_sleep_mask_pclken3);
                }
                _ => {}
            }
        }
    }

    /// 恢复时钟门控（重新开启所有外设时钟）
    fn restore_clock_gate(&self) {
        let cmu = fm33lg0::cmu();
        unsafe {
            write_reg(&cmu.pclken1 as *const u32 as *mut u32,
                read_reg(&cmu.pclken1 as *const u32) | 0x7F);
            write_reg(&cmu.pclken2 as *const u32 as *mut u32,
                read_reg(&cmu.pclken2 as *const u32) | 0x4D);
            write_reg(&cmu.pclken3 as *const u32 as *mut u32,
                read_reg(&cmu.pclken3 as *const u32) | 0x43);
        }
    }

    /// 配置 LPUART 唤醒
    fn configure_lpuart_wakeup(&self) {
        if self.lpuart_wakeup {
            // UART2 RX (PA3) 配置为唤醒源
            // 在 FM33A068EV 中，LPUART0 用于低功耗唤醒
            // 配置 PA3 为 LPUART RX 功能
            gpio_set_fcr(0, 3, 0b10); // PA3 → 数字功能
            gpio_enable_pullup(0, 3);
        }
    }

    /// 判断唤醒源
    fn on_wakeup(&mut self) -> WakeupSource {
        // 检查 UART0 (RS485)
        unsafe {
            let uart0 = fm33lg0::uart0();
            if (read_reg(&uart0.isr as *const u32) & fm33lg0::uart_isr::RXBF) != 0 {
                self.last_wakeup = WakeupSource::Rs485;
                return WakeupSource::Rs485;
            }
        }

        // 检查 UART1 (红外)
        unsafe {
            let uart1 = fm33lg0::uart1();
            if (read_reg(&uart1.isr as *const u32) & fm33lg0::uart_isr::RXBF) != 0 {
                self.last_wakeup = WakeupSource::Infrared;
                return WakeupSource::Infrared;
            }
        }

        // 检查 UART2 (LoRaWAN / LPUART)
        // TODO: 通过 board 层暴露 uart2() 访问
        // 暂时通过 GPIO 引脚间接判断 PA3 (UART2 RX) 状态
        {
            let pa3_low = !gpio_read_pin(pins::UART2_RX);
            if pa3_low {
                if self.state == PowerState::DeepSleep
                    || self.state == PowerState::DeepSleepRamRetain
                {
                    self.last_wakeup = WakeupSource::LpUart;
                    return WakeupSource::LpUart;
                }
                self.last_wakeup = WakeupSource::LoRaWAN;
                return WakeupSource::LoRaWAN;
            }
        }

        // 检查按键
        if !gpio_read_pin(pins::KEY_PAGE) || !gpio_read_pin(pins::KEY_PROG) {
            self.last_wakeup = WakeupSource::Key;
            return WakeupSource::Key;
        }

        // 检查掉电检测
        if !gpio_read_pin(pins::POWER_FAIL) {
            self.last_wakeup = WakeupSource::PowerFail;
            return WakeupSource::PowerFail;
        }

        // 检查 RTC 闹钟
        if self.rtc_alarm_wakeup {
            // TODO: 读取 RTC ISR 闹钟标志
            // 暂时通过排除法判断
        }

        // 默认 GPIO 唤醒
        self.last_wakeup = WakeupSource::Gpio;
        WakeupSource::Gpio
    }

    /// 读取电池电压（ADC, 单次转换）
    fn read_battery_mv(&self) -> u16 {
        let adc = fm33lg0::adc();

        unsafe {
            write_reg(&adc.cfgr as *const u32 as *mut u32, 0x05);
            write_reg(&adc.cr as *const u32 as *mut u32, 0x03);

            let mut timeout = 10000u32;
            while (read_reg(&adc.isr as *const u32) & 0x01) == 0 {
                timeout -= 1;
                if timeout == 0 { return 0; }
            }

            let val = read_reg(&adc.dr as *const u32) & 0x0FFF;
            let mv = (val as u32 * 3300 * 2 / 4096) as u16;
            mv
        }
    }

    /// 检查电池电压告警
    fn check_battery_alert(&self) -> BatteryAlert {
        if self.battery_mv <= BAT_CRITICAL_MV {
            BatteryAlert::Critical
        } else if self.battery_mv <= BAT_LOW_MV {
            BatteryAlert::Low
        } else {
            BatteryAlert::Normal
        }
    }

    /// 周期性电池监测（建议每分钟调用一次）
    ///
    /// 返回当前告警状态。
    pub fn check_battery(&mut self) -> BatteryAlert {
        self.battery_mv = self.read_battery_mv();
        self.battery_alert = self.check_battery_alert();
        self.battery_alert
    }

    /// 获取当前低功耗状态
    pub fn state(&self) -> PowerState {
        self.state
    }

    /// 获取睡眠前状态（唤醒后恢复用）
    pub fn pre_sleep_state(&self) -> PowerState {
        self.pre_sleep_state
    }

    /// 获取电池电压（mV）
    pub fn battery_voltage_mv(&self) -> u16 {
        self.battery_mv
    }

    /// 获取电池告警状态
    pub fn battery_alert(&self) -> BatteryAlert {
        self.battery_alert
    }

    /// 是否电池供电
    pub fn is_battery_powered(&self) -> bool {
        self.battery_powered
    }

    /// 获取上次唤醒源
    pub fn last_wakeup(&self) -> WakeupSource {
        self.last_wakeup
    }

    /// 强制设置状态
    pub fn set_state(&mut self, state: PowerState) {
        self.state = state;
    }

    /// 使能/禁用 RTC 闹钟唤醒
    pub fn set_rtc_alarm_wakeup(&mut self, enable: bool) {
        self.rtc_alarm_wakeup = enable;
    }

    /// 使能/禁用 LPUART 唤醒
    pub fn set_lpuart_wakeup(&mut self, enable: bool) {
        self.lpuart_wakeup = enable;
    }

    /// 获取唤醒次数
    pub fn wakeup_count(&self) -> u32 {
        self.wakeup_count
    }

    /// 设置时钟门控配置
    pub fn set_clock_gate_config(&mut self, config: ClockGateConfig) {
        self.clock_gate = config;
    }
}

/* ================================================================== */
/*  FreeRTOS Tickless Idle 集成                                         */
/* ================================================================== */

/// 预睡眠回调 — 在 FreeRTOS 进入 tickless idle 之前调用
///
/// 返回期望的睡眠时间（ms），0 表示不应睡眠。
pub fn freertos_pre_sleep_processing(pm: &PowerManager) -> u32 {
    match pm.state() {
        PowerState::NormalRun => 0,
        PowerState::LowPowerRun => {
            // 低功耗运行：允许短时 WFI
            10
        }
        PowerState::Sleep => {
            1000
        }
        PowerState::DeepSleep | PowerState::DeepSleepRamRetain => {
            u32::MAX
        }
    }
}

/// 后睡眠回调 — FreeRTOS 从 tickless idle 唤醒后调用
///
/// 返回实际睡眠时间（ms）。
pub fn freertos_post_sleep_processing(pm: &mut PowerManager, expected_ms: u32) -> u32 {
    // TODO: 集成 vTaskStepTick() 补偿 tickless 期间丢失的 tick
    expected_ms
}
