//! FM33LG0xx Board 初始化和主状态管理
//!
//! 复旦微电子 FM33LG0xx Cortex-M0+ @ 64MHz
//! 256KB Flash, 32KB RAM (24K main + 8K battery-backed)
//!
//! 硬件连接:
//!   SPI1  → 计量芯片 (ATT7022/BL6523)
//!   UART0 → RS-485 (HDLC/DLMS, 9600-115200 bps)
//!   UART1 → 红外 (IEC 62056-21, 300-9600 bps)
//!   UART2 → 模块通信 (38400 bps)
//!   LCD   → 内置 LCD 控制器 (4COM x 40SEG)
//!   GPIO  → 继电器控制、RS-485 DE/RE、红外收发使能
//!   RTC   → 内置 RTC (LSE 32.768kHz)
//!   IWDT  → 独立看门狗
//!   AES   → 内置 AES-128 硬件加速

use crate::comm::{CommDriver, UartChannel};
use crate::lcd::LcdDriver;
use crate::metering::{MeteringChip, MeteringDriver, MeteringData};
use crate::fm33lg0;

/// 即时电气量
#[derive(Clone, Copy)]
pub struct InstantaneousValues {
    pub voltage: u16,       // 0.1V (2200 = 220.0V)
    pub current: u16,       // mA
    pub active_power: i32,  // W (signed for export)
    pub reactive_power: i32,// var
    pub frequency: u16,     // 0.01Hz (5000 = 50.00Hz)
    pub power_factor: u16,  // 0-1000 (0.000-1.000)
}

impl Default for InstantaneousValues {
    fn default() -> Self {
        Self { voltage: 0, current: 0, active_power: 0,
               reactive_power: 0, frequency: 5000, power_factor: 1000 }
    }
}

/// 累计电能寄存器 (Wh)
#[derive(Clone, Copy)]
pub struct EnergyRegisters {
    pub active_import: u64,
    pub active_export: u64,
    pub reactive_import: u64,
    pub reactive_export: u64,
    /// 按费率的有功电能 (T1-T8)
    pub tariff_import: [u64; 8],
}

impl Default for EnergyRegisters {
    fn default() -> Self {
        Self { active_import: 0, active_export: 0, reactive_import: 0,
               reactive_export: 0, tariff_import: [0u64; 8] }
    }
}

/// 告警阈值配置
pub struct AlarmConfig {
    pub over_voltage: u16,      // 0.1V (e.g., 2640 = 264.0V → +20%)
    pub under_voltage: u16,     // 0.1V (e.g., 1760 = 176.0V → -20%)
    pub over_current: u16,      // mA
    pub over_power: i32,        // W
    pub over_frequency: u16,    // 0.01Hz
    pub under_frequency: u16,   // 0.01Hz
}

impl Default for AlarmConfig {
    fn default() -> Self {
        Self {
            over_voltage: 2640,   // +20%
            under_voltage: 1760,  // -20%
            over_current: 60000,  // 60A
            over_power: 13200,    // 13.2kW (60A * 220V)
            over_frequency: 5200, // 52.00Hz
            under_frequency: 4800,// 48.00Hz
        }
    }
}

/// 费率时段定义 (简化版, 最多8个费率)
#[derive(Clone, Copy)]
pub struct TariffPeriod {
    pub tariff_id: u8,      // 0-7 → T1-T8
    pub start_hour: u8,
    pub start_minute: u8,
}

/// Board 主状态
pub struct Board {
    systick: u64,
    pub instantaneous: InstantaneousValues,
    pub energy: EnergyRegisters,
    pub relay_closed: bool,
    pub current_tariff: u8,
    pub meter_time: u64,    // Unix timestamp (seconds)
    metering: MeteringDriver,
    comm: CommDriver,
    display: LcdDriver,
    alarm_config: AlarmConfig,
    tariff_table: [TariffPeriod; 8],
    tariff_count: usize,
    sample_count: u64,
    /// 上次功率计算时的电能值 (用于差分)
    last_energy_sample: u64,
}

impl Board {
    /// 硬件初始化
    pub fn init() -> Self {
        // 1. 系统时钟配置: HSE → PLL → 64MHz
        Self::init_clocks();
        // 2. GPIO 初始化
        Self::init_gpio();
        // 3. UART 初始化 (RS-485, 红外, 模块)
        // 4. SPI1 初始化 (计量芯片)
        // 5. LCD 控制器初始化
        // 6. RTC 初始化
        // 7. IWDT 启动
        // 8. AES 模块使能

        let mut board = Self {
            systick: 0,
            instantaneous: InstantaneousValues::default(),
            energy: EnergyRegisters::default(),
            relay_closed: true,
            current_tariff: 0,
            meter_time: 0,
            metering: MeteringDriver::new(MeteringChip::Bl6523),
            comm: CommDriver::new(),
            display: LcdDriver::new(),
            alarm_config: AlarmConfig::default(),
            tariff_table: [
                TariffPeriod { tariff_id: 0, start_hour: 0, start_minute: 0 };
                8
            ],
            tariff_count: 0,
            sample_count: 0,
            last_energy_sample: 0,
        };

        // 初始化 LCD
        board.display.init_hw();
        // 初始化通信
        board.comm.init_hw();

        board
    }

    /// 系统时钟配置
    fn init_clocks() {
        // FM33LG0xx RCC:
        // 1. 使能 HSE (外部高速晶振, 通常 8MHz)
        // 2. 配置 PLL: HSE * 8 = 64MHz
        // 3. 等待 PLL 锁定
        // 4. 切换系统时钟到 PLL
        // 5. 配置 AHB/APB 分频
        //    AHB = SYSCLK / 1 = 64MHz
        //    APB1 = SYSCLK / 1 = 64MHz
        //    APB2 = SYSCLK / 1 = 64MHz
    }

    /// GPIO 初始化
    fn init_gpio() {
        // RS-485 DE/RE 控制:
        //   DE  → PA8  (Output, Push-Pull)
        //   RE  → PA9  (Output, Push-Pull)
        //
        // 继电器控制:
        //   RELAY_CLOSE → PB0 (Output)
        //   RELAY_OPEN  → PB1 (Output)
        //   RELAY_STATE → PB2 (Input, 上拉) - 继电器状态反馈
        //
        // 红外:
        //   IR_TX_EN → PA10 (Output)
        //   IR_RX_EN → PA11 (Output)
        //
        // SPI1 CS (计量芯片):
        //   CS → PA4 (Output, Push-Pull, 默认高)
        //
        // 按键:
        //   BTN → PC13 (Input, 上拉)
    }

    /// 从 Flash 加载校准参数
    pub fn load_calibration(&mut self) {
        // 从 Flash 特定页读取校准数据:
        // - 电压增益
        // - 电流增益
        // - 有功功率增益
        // - 无功功率增益
        // - 相角校正
        // - 费率时段表
        // - 告警阈值
        // - 表号
    }

    /// SysTick (毫秒)
    pub fn systick_ms(&self) -> u64 { self.systick }

    /// SysTick 递增 (SysTick 中断调用)
    pub fn tick(&mut self) { self.systick = self.systick.wrapping_add(1); }

    // ── 周期任务实现 ──────────────────────────────────────────────

    /// 任务0: SPI 读取计量芯片 (1ms)
    pub fn sample_metering(&mut self) {
        self.sample_count += 1;

        // 读取计量数据 (根据芯片类型)
        let data = self.metering.read_bl6523();
        self.instantaneous = InstantaneousValues {
            voltage: data.voltage,
            current: data.current,
            active_power: data.active_power,
            reactive_power: data.reactive_power,
            frequency: data.frequency,
            power_factor: data.power_factor,
        };
    }

    /// 任务1: 功率计算 + 能量累加 (200ms)
    pub fn calculate_power_energy(&mut self) {
        // 能量增量 = 功率 × 时间间隔
        // 200ms 采样间隔, 能量单位 Wh
        let power_w = self.instantaneous.active_power;
        let energy_delta_wh = (power_w.abs() as u64) * 200 / 3600000;

        if power_w >= 0 {
            self.energy.active_import += energy_delta_wh;
            self.energy.tariff_import[self.current_tariff as usize] += energy_delta_wh;
        } else {
            self.energy.active_export += energy_delta_wh;
        }

        // 同时累加无功电能
        let reactive_var = self.instantaneous.reactive_power;
        let reactive_delta = (reactive_var.abs() as u64) * 200 / 3600000;
        if reactive_var >= 0 {
            self.energy.reactive_import += reactive_delta;
        } else {
            self.energy.reactive_export += reactive_delta;
        }
    }

    /// 任务2: LCD 显示刷新 (500ms)
    pub fn refresh_display(&mut self) {
        self.display.update(
            self.instantaneous.voltage,
            self.instantaneous.current,
            self.instantaneous.active_power,
            self.energy.active_import,
            self.current_tariff,
            self.instantaneous.frequency,
            self.instantaneous.power_factor,
        );
    }

    /// 任务3: RS-485 HDLC 通信处理 (10ms)
    pub fn process_rs485_hdlc(&mut self) {
        // 1. 检查 UART0 是否有数据
        // 2. 如果有: 逐字节 feed 到 HDLC 解码器
        // 3. 如果收到完整帧: 解码 APDU → 处理 → 编码响应 → 发送
        // 4. 如果有待发送数据: 继续发送
    }

    /// 任务4: 红外通信处理 (50ms)
    pub fn process_infrared(&mut self) {
        // 1. 检查 UART1 是否有数据
        // 2. IEC 62056-21 协议处理:
        //    - 收到 /?!<CR><LF> → 回应表号
        //    - 收到 ACK (波特率切换) → 切换波特率
        //    - 数据模式 → 发送 COSEM 数据
    }

    /// 任务5: 模块 UART 通信 (1000ms)
    pub fn process_module_uart(&mut self) {
        // 与外部模块通信 (如载波模块、GPRS 模块等)
        // UART2, 38400 bps, 8N1
        // 协议取决于模块类型
    }

    /// 任务6: 负荷曲线捕获 (15分钟)
    pub fn capture_load_profile(&mut self) {
        // 将当前所有计量数据打包存储到 Flash
        // Profile Generic IC7 格式:
        //   timestamp + voltage + current + power + energy + tariff + status
        // 存储到循环缓冲区 (Flash 页)
    }

    /// 任务7: 费率时段检查 (1分钟)
    pub fn check_tariff_schedule(&mut self) {
        // 获取当前时间
        // 与费率时段表比较
        // 如果费率发生变化: 切换 current_tariff
        let _hour = ((self.meter_time / 3600) % 24) as u8;
        let _minute = ((self.meter_time / 60) % 60) as u8;

        // 遍历费率表, 找到当前时段
        for i in 0..self.tariff_count {
            let _period = &self.tariff_table[i];
            // if current_time >= period.start_time: current_tariff = period.tariff_id
        }
    }

    /// 任务8: 越限告警检查 (200ms)
    pub fn check_alarm_thresholds(&mut self) {
        let v = self.instantaneous.voltage;
        let i = self.instantaneous.current;
        let p = self.instantaneous.active_power;
        let f = self.instantaneous.frequency;

        // 过压
        if v > self.alarm_config.over_voltage {
            self.trigger_alarm(AlarmCode::OverVoltage);
        }
        // 欠压
        if v < self.alarm_config.under_voltage && v > 0 {
            self.trigger_alarm(AlarmCode::UnderVoltage);
        }
        // 过流
        if i > self.alarm_config.over_current {
            self.trigger_alarm(AlarmCode::OverCurrent);
        }
        // 过功率
        if p > self.alarm_config.over_power {
            self.trigger_alarm(AlarmCode::OverPower);
        }
        // 频率异常
        if f > self.alarm_config.over_frequency || f < self.alarm_config.under_frequency {
            self.trigger_alarm(AlarmCode::FrequencyAbnormal);
        }
    }

    /// 任务9: 喂看门狗 (100ms)
    pub fn feed_watchdog(&mut self) {
        // FM33LG0xx IWDT 喂狗:
        // let iwdt = fm33lg0::iwdt();
        // iwdt.wdtrld = 0x5A5A5A5A; // reload key
    }

    // ── 辅助方法 ─────────────────────────────────────────────────

    /// 触发告警
    fn trigger_alarm(&mut self, code: AlarmCode) {
        // 1. 记录告警到 Flash 日志
        // 2. 如果配置了自动拉闸: 断开继电器
        // 3. 如果配置了告警上报: 通过 RS-485 发送通知
        defmt::warn!("Alarm: {:?}", code);
    }

    /// 继电器合闸
    pub fn relay_close(&mut self) {
        // PB0 = HIGH (pulse)
        self.relay_closed = true;
    }

    /// 继电器跳闸
    pub fn relay_open(&mut self) {
        // PB1 = HIGH (pulse)
        self.relay_closed = false;
    }
}

/// 告警代码
#[derive(Debug, Clone, Copy)]
pub enum AlarmCode {
    OverVoltage,
    UnderVoltage,
    OverCurrent,
    OverPower,
    FrequencyAbnormal,
    TamperDetect,       // 防窃电
    PowerReverse,       // 反向功率
    NeutralMissing,     // 缺零
    PhaseLoss,          // 缺相 (三相)
}
