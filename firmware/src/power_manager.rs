/* ================================================================== */
/*                                                                    */
/*  power_manager.rs — 低功耗管理                                       */
/*                                                                    */
/*  状态机:                                                           */
/*    NormalRun → Sleep (无通信 5min)                                  */
/*    Sleep     → DeepSleep (无通信 30min)                              */
/*    电池模式  → 始终 DeepSleep                                        */
/*                                                                    */
/*  唤醒源: RS485 / 红外 / 按键 / RTC 闹钟 / LoRaWAN                  */
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

/* ================================================================== */
/*  超时阈值常量                                                       */
/* ================================================================== */

/// 无通信活动多久进入 Sleep（秒）
const SLEEP_TIMEOUT_SEC: u32 = 5 * 60;
/// 无通信活动多久进入 DeepSleep（秒）
const DEEP_SLEEP_TIMEOUT_SEC: u32 = 30 * 60;
/// 编程模式下不进入低功耗

/* ================================================================== */
/*  电池电压阈值 (mV)                                                  */
/* ================================================================== */

/// ER26500 锂亚电池: 标称 3.6V
/// 低电压警告阈值
const BAT_LOW_MV: u16 = 3300;
/// 极低电压阈值
const BAT_CRITICAL_MV: u16 = 3000;

/* ================================================================== */
/*  PowerManager                                                       */
/* ================================================================== */

/// 低功耗管理器
pub struct PowerManager {
    /// 当前状态
    state: PowerState,
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
}

impl PowerManager {
    /// 创建低功耗管理器
    pub const fn new() -> Self {
        Self {
            state: PowerState::NormalRun,
            battery_powered: false,
            last_activity_tick: 0,
            battery_mv: 3600,
            battery_alert: BatteryAlert::Normal,
            last_wakeup: WakeupSource::Unknown,
            current_tick: 0,
        }
    }

    /// 初始化
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

        // 判断供电模式: 读取掉电检测引脚
        // PB1 = POWER_FAIL, 低电平 = 电池供电
        self.battery_powered = !gpio_read_pin(pins::POWER_FAIL);

        // 电池模式直接进入 DeepSleep
        if self.battery_powered {
            self.state = PowerState::DeepSleepRamRetain;
        }
    }

    /// 周期性调用（在主循环或 FreeRTOS 任务中）
    ///
    /// 参数:
    ///   - tick: 当前 FreeRTOS tick (ms)
    ///   - programming_mode: 是否处于编程模式
    ///
    /// 返回: 是否应该进入低功耗（调用者应调用 enter_low_power()）
    pub fn tick(&mut self, tick: u32, programming_mode: bool) -> bool {
        self.current_tick = tick;

        // 编程模式下不进入低功耗
        if programming_mode {
            self.state = PowerState::NormalRun;
            return false;
        }

        let idle_ms = tick.wrapping_sub(self.last_activity_tick);
        let idle_sec = idle_ms / 1000;

        // 电池模式: 始终 DeepSleep
        if self.battery_powered {
            if self.state != PowerState::DeepSleepRamRetain {
                self.state = PowerState::DeepSleepRamRetain;
                return true;
            }
            return false;
        }

        // 市电模式: 根据空闲时间递进
        if idle_sec >= DEEP_SLEEP_TIMEOUT_SEC && self.state != PowerState::DeepSleep {
            self.state = PowerState::DeepSleep;
            return true;
        } else if idle_sec >= SLEEP_TIMEOUT_SEC && self.state != PowerState::Sleep {
            self.state = PowerState::Sleep;
            return true;
        }

        false
    }

    /// 记录通信活动（任何 UART 收发时调用）
    pub fn record_activity(&mut self, tick: u32) {
        self.last_activity_tick = tick;
        // 任何活动都回到正常运行
        if self.state != PowerState::NormalRun {
            self.state = PowerState::NormalRun;
        }
    }

    /// 进入低功耗模式
    ///
    /// 根据当前 state 执行对应的低功耗流程
    /// 返回唤醒源
    pub fn enter_low_power(&mut self) -> WakeupSource {
        match self.state {
            PowerState::NormalRun => {
                // 正常运行，不进入低功耗
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

        // 唤醒后
        self.on_wakeup()
    }

    /// 进入 Sleep 模式
    ///
    /// - CPU 执行 WFI 等待中断
    /// - UART 保持接收能力，任何通信唤醒
    /// - LCD 保持显示
    fn enter_sleep(&self) {
        let pmu = fm33lg0::pmu();

        // TODO: 配置 PMU 进入 Sleep 模式
        // FM33A068EV PMU CR 寄存器:
        //   bits[1:0] = LPMS: 00=Run, 01=Sleep, 10=DeepSleep, 11=DeepSleep+RAM
        unsafe {
            // 设置 Sleep 模式
            let cr = read_reg(&pmu.cr as *const u32);
            write_reg(&pmu.cr as *const u32 as *mut u32, (cr & !0x03) | 0x01);

            // 配置唤醒源: UART0 (RS485), UART1 (红外), GPIO (按键)
            // 唤醒使能寄存器: PMU WKFR 或 GPIO PINWKEN
            let gc = fm33lg0::gpio_common();
            // 使能 PB0, PB1 唤醒
            write_reg(&gc.pinwken as *const u32 as *mut u32,
                read_reg(&gc.pinwken as *const u32) | 0x03);
        }

        // 执行 WFI（等待中断唤醒）
        cortex_m::asm::wfi();

        // 唤醒后恢复
        unsafe {
            // 恢复 Run 模式
            let cr = read_reg(&pmu.cr as *const u32);
            write_reg(&pmu.cr as *const u32 as *mut u32, cr & !0x03);
        }
    }

    /// 进入 Deep Sleep 模式
    ///
    /// - 关闭主时钟（PLL_H, HCLK），保留 LSI/LSE
    /// - 关闭 LCD
    /// - 仅 LPUART 保持唤醒能力
    /// - ram_retain: 是否保持 SRAM 内容
    fn enter_deep_sleep(&self, ram_retain: bool) {
        let pmu = fm33lg0::pmu();

        // 1. 关闭 LCD 显示
        let lcd = fm33lg0::lcd();
        unsafe {
            let cr = read_reg(&lcd.cr as *const u32);
            write_reg(&lcd.cr as *const u32 as *mut u32, cr & !fm33lg0::lcd_cr::EN);
        }

        // 2. 配置唤醒源
        unsafe {
            // 使能 LPUART0 唤醒
            // TODO: LPUART0 唤醒配置，需要查阅 PMU WKTR 寄存器
            // 使能 GPIO 唤醒 (按键)
            let gc = fm33lg0::gpio_common();
            write_reg(&gc.pinwken as *const u32 as *mut u32,
                read_reg(&gc.pinwken as *const u32) | 0x03);
        }

        // 3. 配置 PMU 进入 DeepSleep
        unsafe {
            let lpms = if ram_retain { 0x03 } else { 0x02 }; // 11=DS+RAM, 10=DS
            let cr = read_reg(&pmu.cr as *const u32);
            write_reg(&pmu.cr as *const u32 as *mut u32, (cr & !0x03) | lpms);
        }

        // 4. 执行 WFI
        cortex_m::asm::wfi();

        // 5. 唤醒后恢复
        self.restore_peripherals();
    }

    /// 唤醒后恢复所有外设
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

        // 3. 重新开启 LCD
        let lcd = fm33lg0::lcd();
        unsafe {
            let cr = read_reg(&lcd.cr as *const u32);
            write_reg(&lcd.cr as *const u32 as *mut u32, cr | fm33lg0::lcd_cr::EN);
        }

        // 4. 恢复 UART 配置
        // TODO: 需要重新初始化 RS485/红外 UART 的波特率
        // 因为 DeepSleep 可能关闭了 UART 时钟

        // 5. 使能外设时钟
        unsafe {
            let cmu = fm33lg0::cmu();
            write_reg(&cmu.pclken1 as *const u32 as *mut u32,
                read_reg(&cmu.pclken1 as *const u32) | 0x7F);
            write_reg(&cmu.pclken2 as *const u32 as *mut u32,
                read_reg(&cmu.pclken2 as *const u32) | 0x4D);
            write_reg(&cmu.pclken3 as *const u32 as *mut u32,
                read_reg(&cmu.pclken3 as *const u32) | 0x43);
        }
    }

    /// 唤醒后判断唤醒源
    fn on_wakeup(&mut self) -> WakeupSource {
        let pmu = fm33lg0::pmu();

        // 读取 PMU 唤醒标志寄存器
        // TODO: PMU WKFR 寄存器位域定义需要完善
        // 暂时通过轮询各外设状态判断
        let source = WakeupSource::Unknown;

        // 检查 UART0 (RS485) 是否有数据
        unsafe {
            let uart0 = fm33lg0::uart0();
            if (read_reg(&uart0.isr as *const u32) & fm33lg0::uart_isr::RXBF) != 0 {
                self.last_wakeup = WakeupSource::Rs485;
                return WakeupSource::Rs485;
            }
        }

        // 检查 UART1 (红外) 是否有数据
        unsafe {
            let uart1 = fm33lg0::uart1();
            if (read_reg(&uart1.isr as *const u32) & fm33lg0::uart_isr::RXBF) != 0 {
                self.last_wakeup = WakeupSource::Infrared;
                return WakeupSource::Infrared;
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

        // TODO: 检查 RTC 闹钟标志
        // TODO: 检查 LPUART 唤醒标志

        self.last_wakeup = source;
        source
    }

    /// 读取电池电压（ADC, 单次转换）
    ///
    /// PF6 (ADC_IN5) 通过分压电阻接入:
    /// Vbat → R1(10K) → PF6 → R2(10K) → GND
    /// ADC 读数 = Vbat / 2
    /// ADC 12-bit: 0~4095 对应 0~VREF (3.3V)
    /// Vbat = ADC_val * 3300 / 4096 * 2
    fn read_battery_mv(&self) -> u16 {
        let adc = fm33lg0::adc();

        unsafe {
            // 选择通道 5 (PF6 = BAT_ADC)
            write_reg(&adc.cfgr as *const u32 as *mut u32, 0x05);

            // 启动转换 (CR bit1 = START)
            write_reg(&adc.cr as *const u32 as *mut u32, 0x03); // EN + START

            // 等待转换完成
            let mut timeout = 10000u32;
            while (read_reg(&adc.isr as *const u32) & 0x01) == 0 {
                timeout -= 1;
                if timeout == 0 {
                    return 0; // 超时
                }
            }

            // 读取结果 (12-bit)
            let val = read_reg(&adc.dr as *const u32) & 0x0FFF;

            // 转换为 mV: Vbat = val * 3300 / 4096 * 2
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
    /// 返回当前告警状态
    pub fn check_battery(&mut self) -> BatteryAlert {
        self.battery_mv = self.read_battery_mv();
        self.battery_alert = self.check_battery_alert();
        self.battery_alert
    }

    /// 获取当前低功耗状态
    pub fn state(&self) -> PowerState {
        self.state
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

    /// 强制设置状态（唤醒后恢复用）
    pub fn set_state(&mut self, state: PowerState) {
        self.state = state;
    }
}

/* ================================================================== */
/*  FreeRTOS Tickless Idle 集成                                         */
/* ================================================================== */

/// 预睡眠回调 — 在 FreeRTOS 进入 tickless idle 之前调用
///
/// 返回期望的睡眠时间（ms），0 表示不应睡眠
pub fn freertos_pre_sleep_processing(pm: &PowerManager) -> u32 {
    match pm.state() {
        PowerState::NormalRun => 0,
        PowerState::Sleep => {
            // 计算到下次 Sleep 超时的剩余时间
            // 由 FreeRTOS 自动管理 idle 时间
            1000 // 最大睡眠 1s, 由 FreeRTOS tickless 机制截断
        }
        PowerState::DeepSleep | PowerState::DeepSleepRamRetain => {
            // 深度睡眠: 让 FreeRTOS 进入最长 tickless idle
            // 实际睡眠时间由 PMU 硬件控制
            u32::MAX
        }
    }
}

/// 后睡眠回调 — FreeRTOS 从 tickless idle 唤醒后调用
///
/// 返回实际睡眠时间（ms）
pub fn freertos_post_sleep_processing(pm: &mut PowerManager, expected_ms: u32) -> u32 {
    // 唤醒源已在 enter_low_power 中判断
    // 这里更新 FreeRTOS tick 补偿
    // TODO: 集成 vTaskStepTick() 补偿 tickless 期间丢失的 tick
    expected_ms // placeholder
}
