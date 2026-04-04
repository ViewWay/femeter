/* ================================================================== */
/*                                                                    */
/*  board.rs — FM33A068EV 板级硬件初始化与资源管理                      */
/*                                                                    */
/*  外设分配:                                                          */
/*    SPI0  → 计量芯片 (ATT7022E/RN8302B/RN8615V2)                    */
/*    SPI1  → 外部 Flash (W25Q64, 可选)                               */
/*    UART0 → RS485 (DLMS/COSEM, 9600~115200 8E1)                    */
/*    UART1 → 红外 (IEC 62056-21, 300~9600)                          */
/*    UART2 → LoRaWAN (ASR6601, 38400)                                */
/*    UART3 → 蜂窝模组 (EC800N/BC260Y, 115200)                        */
/*    LPUART0 → 调试/低功耗唤醒                                       */
/*    LCD   → 4COM×44SEG 段码显示                                     */
/*    GPIO  → LED×5 + 蜂鸣器 + 按键×2 + 脉冲输出 + 检测              */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::fm33lg0;
use crate::fm33lg0::base;
use crate::fm33lg0::{lcd_cr, spi_cr1, spi_cr2, uart_csr, uart_isr};
use crate::hal::*;

/* ================================================================== */
/*  辅助: 缺失的 GPIO 端口访问器                                       */
/* ================================================================== */

/// 获取 GPIOE 寄存器块
fn gpioe() -> &'static fm33lg0::GpioRegs {
    unsafe { &*(base::GPIOE as *const fm33lg0::GpioRegs) }
}

/// 获取 GPIOF 寄存器块
fn gpiof() -> &'static fm33lg0::GpioRegs {
    unsafe { &*(base::GPIOF as *const fm33lg0::GpioRegs) }
}

/// 获取 GPIOG 寄存器块
fn gpiog() -> &'static fm33lg0::GpioRegs {
    unsafe { &*(base::GPIOG as *const fm33lg0::GpioRegs) }
}

/// 获取 UART2 寄存器块
fn uart2() -> &'static fm33lg0::UartRegs {
    unsafe { &*(base::UART2 as *const fm33lg0::UartRegs) }
}

/// 获取 UART3 寄存器块
fn uart3() -> &'static fm33lg0::UartRegs {
    unsafe { &*(base::UART3 as *const fm33lg0::UartRegs) }
}

/* ================================================================== */
/*  GPIO 辅助函数                                                      */
/* ================================================================== */

/// 根据 port 编号返回对应的 GpioRegs 指针
pub(crate) fn gpio_port(port: u8) -> &'static fm33lg0::GpioRegs {
    match port {
        0 => fm33lg0::gpioa(),
        1 => fm33lg0::gpiob(),
        2 => fm33lg0::gpioc(),
        3 => fm33lg0::gpiod(),
        4 => gpioe(),
        5 => gpiof(),
        6 => gpiog(),
        _ => fm33lg0::gpioa(), // fallback, should not happen
    }
}

/// 配置 GPIO 引脚功能 (FCR: 2 bits per pin)
/// 00 = 输入, 01 = 输出, 10 = 数字功能, 11 = 模拟
#[inline(always)]
pub(crate) fn gpio_set_fcr(port: u8, pin: u8, mode: u32) {
    let gpio = gpio_port(port);
    let shift = (pin as u32) * 2;
    let mask = 0x03 << shift;
    // Read-modify-write on FCR
    let prev = gpio.fcr;
    // Safety: atomic-like RMW in critical section not required for init
    unsafe {
        core::ptr::write_volatile(
            &gpio.fcr as *const u32 as *mut u32,
            (prev & !mask) | ((mode & 0x03) << shift),
        );
    }
}

/// 设置 GPIO 数字功能选择 (DFS)
#[inline(always)]
fn gpio_set_dfs(port: u8, pin: u8, func: u32) {
    let gpio = gpio_port(port);
    let shift = (pin as u32) * 2;
    let mask = 0x03 << shift;
    let prev = gpio.dfs;
    unsafe {
        core::ptr::write_volatile(
            &gpio.dfs as *const u32 as *mut u32,
            (prev & !mask) | ((func & 0x03) << shift),
        );
    }
}

/// 使能 GPIO 上拉
#[inline(always)]
pub(crate) fn gpio_enable_pullup(port: u8, pin: u8) {
    let gpio = gpio_port(port);
    let bit = 1u32 << (pin as u32);
    unsafe {
        core::ptr::write_volatile(&gpio.puen as *const u32 as *mut u32, gpio.puen | bit);
    }
}

/// 禁止 GPIO 上拉
#[inline(always)]
fn gpio_disable_pullup(port: u8, pin: u8) {
    let gpio = gpio_port(port);
    let bit = 1u32 << (pin as u32);
    unsafe {
        core::ptr::write_volatile(&gpio.puen as *const u32 as *mut u32, gpio.puen & !bit);
    }
}

/// 使能 GPIO 输入
#[inline(always)]
pub(crate) fn gpio_enable_input(port: u8, pin: u8) {
    let gpio = gpio_port(port);
    let bit = 1u32 << (pin as u32);
    unsafe {
        core::ptr::write_volatile(&gpio.inen as *const u32 as *mut u32, gpio.inen | bit);
    }
}

/// GPIO 输出置位 (set high)
#[inline(always)]
pub fn gpio_set(pin: GpioPin) {
    let gpio = gpio_port(pin.port);
    let bit = 1u32 << (pin.pin as u32);
    unsafe {
        core::ptr::write_volatile(&gpio.dset as *const u32 as *mut u32, bit);
    }
}

/// GPIO 输出复位 (set low)
#[inline(always)]
pub fn gpio_clr(pin: GpioPin) {
    let gpio = gpio_port(pin.port);
    let bit = 1u32 << (pin.pin as u32);
    unsafe {
        core::ptr::write_volatile(&gpio.drst as *const u32 as *mut u32, bit);
    }
}

/// 读取 GPIO 输入电平
#[inline(always)]
pub(crate) fn gpio_read_pin(pin: GpioPin) -> bool {
    let gpio = gpio_port(pin.port);
    let bit = 1u32 << (pin.pin as u32);
    (gpio.din & bit) != 0
}

/// 写 volatile helper
#[inline(always)]
pub unsafe fn write_reg(addr: *mut u32, val: u32) {
    core::ptr::write_volatile(addr, val);
}

/// 读 volatile helper
#[inline(always)]
pub unsafe fn read_reg(addr: *const u32) -> u32 {
    core::ptr::read_volatile(addr)
}

/* ================================================================== */
/*  CMU 时钟使能辅助                                                   */
/*  PCLKEN 位域映射 (来自 FM33LC0xx 产品说明书)                        */
/*    PCLKEN1: bit0=GPIOA, bit1=GPIOB, ..., bit6=GPIOG                */
/*    PCLKEN2: bit0=UART0, bit1=UART1, bit2=UART2, bit3=UART3,       */
/*             bit4=UART4, bit5=UART5, bit6=LPUART0, bit7=LPUART1     */
/*    PCLKEN3: bit0=SPI0, bit1=SPI1, bit2=SPI2, bit3=I2C0,           */
/*             bit4=I2C1, bit5=LCD, bit6=ADC, bit7=COMP               */
/*    PCLKEN4: bit0=BSTIM, bit1=LPTIM, bit2=RTC, bit3=IWDT,          */
/*             bit4=WWDT, bit5=AES, bit6=TRNG, bit7=CRC               */
/* ================================================================== */

/// 使能 APB 外设时钟 (RMW, 不关其他)
fn cmu_enable_pclk() {
    let cmu = fm33lg0::cmu();
    unsafe {
        // PCLKEN1: GPIOA~GPIOG 全部使能
        write_reg(
            &cmu.pclken1 as *const u32 as *mut u32,
            read_reg(&cmu.pclken1 as *const u32) | 0x7F,
        );
        // PCLKEN2: UART0~3, LPUART0
        write_reg(
            &cmu.pclken2 as *const u32 as *mut u32,
            read_reg(&cmu.pclken2 as *const u32) | 0x0D,
        ); // bit0+bit2+bit3 = UART0,2,3; bit6=LPUART0
           // PCLKEN3: SPI0, SPI1, LCD, ADC
        write_reg(
            &cmu.pclken3 as *const u32 as *mut u32,
            read_reg(&cmu.pclken3 as *const u32) | 0x43,
        ); // bit0=SPI0, bit1=SPI1, bit5=LCD, bit6=ADC
           // PCLKEN4: RTC
        write_reg(
            &cmu.pclken4 as *const u32 as *mut u32,
            read_reg(&cmu.pclken4 as *const u32) | 0x04,
        ); // bit2=RTC
    }
}

/* ================================================================== */
/*  SPI 辅助函数                                                       */
/* ================================================================== */

/// SPI0 全双工传输 (计量芯片), CS 手动控制
pub fn spi0_transfer(tx_byte: u8) -> u8 {
    let spi = fm33lg0::spi0();
    unsafe {
        // 等待 TX 缓冲区空 (TXSE flag = ISR bit0)
        while (read_reg(&spi.isr as *const u32) & 0x01) == 0 {}
        // 写发送数据
        write_reg(&spi.txbuf as *const u32 as *mut u32, tx_byte as u32);
        // 等待 RX 完成 (RXBF flag = bit8)
        while (read_reg(&spi.isr as *const u32) & 0x100) == 0 {}
        read_reg(&spi.rxbuf as *const u32) as u8
    }
}

/// SPI0 片选拉低
#[inline(always)]
fn spi0_cs_low() {
    gpio_clr(pins::SPI0_CSN);
}

/// SPI0 片选拉高
#[inline(always)]
fn spi0_cs_high() {
    gpio_set(pins::SPI0_CSN);
}

/* ================================================================== */
/*  UART 辅助函数                                                      */
/* ================================================================== */

/// 获取 UART 寄存器块 by channel
fn uart_regs(ch: UartChannel) -> &'static fm33lg0::UartRegs {
    match ch {
        UartChannel::Uart0 => fm33lg0::uart0(),
        UartChannel::Uart1 => fm33lg0::uart1(),
        UartChannel::Uart2 => uart2(),
        UartChannel::Uart3 => uart3(),
        UartChannel::Uart4 => fm33lg0::uart4(),
        UartChannel::Uart5 => fm33lg0::uart5(),
        // LPUART has different register layout; fallback to UART0
        _ => fm33lg0::uart0(),
    }
}

/// 配置 UART 波特率和帧格式
fn uart_config_regs(ch: UartChannel, config: &UartConfig) {
    let uart = uart_regs(ch);

    // 禁用 TX/RX
    unsafe {
        write_reg(&uart.csr as *const u32 as *mut u32, 0);
    }

    // 计算波特率分频: SPBRG = SYSCLK / baudrate
    let sysclk: u32 = 64_000_000; // After PLL init
    let spbrg = fm33lg0::calc_spbrg(sysclk, config.baudrate);
    unsafe {
        write_reg(&uart.bgr as *const u32 as *mut u32, spbrg as u32);
    }

    // 配置 CSR: 数据位 + 校验 + 停止位
    let mut csr: u32 = 0;

    // 数据位: PDSEL[7:6] — 00=7bit, 01=8bit, 10=9bit, 11=6bit
    match config.data_bits {
        7 => {}
        8 => csr |= 0x01 << uart_csr::PDSEL_SHIFT,
        9 => csr |= 0x02 << uart_csr::PDSEL_SHIFT,
        _ => csr |= 0x01 << uart_csr::PDSEL_SHIFT, // default 8-bit
    }

    // 校验位: PARITY[5:4] — 00=无, 01=偶, 10=奇
    match config.parity {
        Parity::None => {}
        Parity::Even => csr |= 0x01 << uart_csr::PARITY_SHIFT,
        Parity::Odd => csr |= 0x02 << uart_csr::PARITY_SHIFT,
    }

    // 停止位: STOPCFG bit8 — 0=1bit, 1=2bit
    if config.stop_bits == 2 {
        csr |= uart_csr::STOPCFG;
    }

    // 使能 TX + RX
    csr |= uart_csr::TXEN | uart_csr::RXEN;

    unsafe {
        write_reg(&uart.csr as *const u32 as *mut u32, csr);
    }
}

/* ================================================================== */
/*  延时辅助 (粗略, 基于 Cortex-M0+ cycle 计数)                       */
/* ================================================================== */

/// 粗略微秒延时 (SYSCLK=64MHz 时, 每次循环约 4 cycles)
fn delay_us(us: u32) {
    let loops = us * 16; // 64MHz / 4 ≈ 16 loops/us
    let mut i = 0;
    while i < loops {
        cortex_m::asm::nop();
        i += 1;
    }
}

/// 粗略毫秒延时
fn delay_ms(ms: u32) {
    delay_us(ms * 1000);
}

/* ================================================================== */
/*  GPIO 引脚分配 (FM33A068EV LQFP80)                                  */
/* ================================================================== */

pub mod pins {
    use super::GpioPin;

    // ── SPI0 → 计量芯片 ──
    /// SPI0_SCK
    pub const SPI0_SCK: GpioPin = GpioPin::new(5, 14); // PF14
    /// SPI0_MISO
    pub const SPI0_MISO: GpioPin = GpioPin::new(5, 13); // PF13
    /// SPI0_MOSI
    pub const SPI0_MOSI: GpioPin = GpioPin::new(5, 12); // PF12
    /// SPI0_CSN (计量芯片片选)
    pub const SPI0_CSN: GpioPin = GpioPin::new(5, 15); // PF15

    // ── SPI1 → 外部 Flash ──
    pub const SPI1_SCK: GpioPin = GpioPin::new(0, 5); // PA5
    pub const SPI1_MISO: GpioPin = GpioPin::new(0, 6); // PA6
    pub const SPI1_MOSI: GpioPin = GpioPin::new(0, 7); // PA7
    pub const SPI1_CSN: GpioPin = GpioPin::new(0, 4); // PA4

    // ── UART0 → RS485 ──
    pub const UART0_TX: GpioPin = GpioPin::new(6, 9); // PG9
    pub const UART0_RX: GpioPin = GpioPin::new(6, 8); // PG8
    /// RS485 方向控制 (高=发送, 低=接收)
    pub const RS485_DE: GpioPin = GpioPin::new(5, 2); // PF2

    // ── UART1 → 红外 ──
    pub const UART1_TX: GpioPin = GpioPin::new(4, 4); // PE4
    pub const UART1_RX: GpioPin = GpioPin::new(4, 3); // PE3

    // ── UART2 → LoRaWAN ──
    pub const UART2_TX: GpioPin = GpioPin::new(1, 3); // PB3
    pub const UART2_RX: GpioPin = GpioPin::new(1, 2); // PB2

    // ── UART3 → 蜂窝模组 ──
    pub const UART3_TX: GpioPin = GpioPin::new(1, 15); // PB15
    pub const UART3_RX: GpioPin = GpioPin::new(1, 14); // PB14
    /// 蜂窝模组电源控制
    pub const CELL_PWRKEY: GpioPin = GpioPin::new(4, 2); // PE2
    /// 蜂窝模组复位
    pub const CELL_RESET: GpioPin = GpioPin::new(4, 7); // PE7

    // ── LED ──
    /// 电源指示 (绿)
    pub const LED_POWER: GpioPin = GpioPin::new(0, 8); // PA8
    /// 通信指示 (黄)
    pub const LED_COMM: GpioPin = GpioPin::new(0, 9); // PA9
    /// 告警指示 (红)
    pub const LED_ALARM: GpioPin = GpioPin::new(0, 10); // PA10
    /// 有功脉冲 LED (红)
    pub const LED_PULSE_P: GpioPin = GpioPin::new(0, 11); // PA11
    /// 无功脉冲 LED (绿)
    pub const LED_PULSE_Q: GpioPin = GpioPin::new(0, 12); // PA12

    // ── 蜂鸣器 ──
    pub const BUZZER: GpioPin = GpioPin::new(0, 15); // PA15

    // ── 按键 ──
    /// 翻页键 (外部中断唤醒)
    pub const KEY_PAGE: GpioPin = GpioPin::new(1, 0); // PB0
    /// 编程键
    pub const KEY_PROG: GpioPin = GpioPin::new(1, 1); // PB1

    // ── 脉冲输出 (光耦) ──
    /// 有功脉冲输出
    pub const PULSE_P: GpioPin = GpioPin::new(1, 4); // PB4
    /// 无功脉冲输出
    pub const PULSE_Q: GpioPin = GpioPin::new(1, 5); // PB5

    // ── 防窃电检测 ──
    /// 上盖检测 (微动开关)
    pub const COVER_DET: GpioPin = GpioPin::new(3, 8); // PD8
    /// 端子盖检测 (微动开关)
    pub const TERMINAL_DET: GpioPin = GpioPin::new(3, 9); // PD9
    /// 磁场检测 (霍尔传感器)
    pub const MAGNETIC_DET: GpioPin = GpioPin::new(0, 13); // PA13

    // ── 电池 / 电源 ──
    /// 掉电检测 (外部比较器输出)
    pub const POWER_FAIL: GpioPin = GpioPin::new(1, 1); // PB1
    /// 电池电压 ADC 输入
    pub const BAT_ADC: GpioPin = GpioPin::new(5, 6); // PF6 (ADC_IN5)

    // ── SIM 卡 ──
    /// SIM 卡检测
    pub const SIM_DET: GpioPin = GpioPin::new(3, 7); // PD7
}

/* ================================================================== */
/*  板级资源结构体                                                      */
/* ================================================================== */

/// 板级所有硬件资源
pub struct Board<M: crate::hal::MeteringChip> {
    /// 计量芯片驱动
    pub metering: M,
    /// RS485 UART
    pub rs485: UartChannelDriver,
    /// 红外 UART
    pub infrared: UartChannelDriver,
    /// LoRaWAN UART
    pub lorawan_uart: UartChannelDriver,
    /// 蜂窝模组 UART
    pub cellular_uart: UartChannelDriver,
    /// LCD 驱动
    pub lcd: LcdDriverImpl,
    /// 脉冲输出
    pub pulse: PulseOutputImpl,
    /// 指示灯/蜂鸣器
    pub indicator: IndicatorImpl,
    /// 防窃电检测
    pub tamper: TamperImpl,
    /// GPIO (通用)
    pub gpio: GpioImpl,
}

/// UART 通道驱动 (简化实现)
pub struct UartChannelDriver {
    channel: UartChannel,
    configured: bool,
}

/// LCD 驱动实现
pub struct LcdDriverImpl {
    /// 当前显示模式
    mode: LcdDisplayMode,
    /// 当前内容
    content: LcdContent,
}

/// 脉冲输出实现
pub struct PulseOutputImpl {
    /// 有功脉冲常数 (imp/kWh)
    active_constant: u32,
    /// 无功脉冲常数 (imp/kvarh)
    reactive_constant: u32,
    /// 脉冲宽度 (ms)
    pulse_width_ms: u16,
    /// 有功能量累加器 (Wh)
    active_accum_wh: u32,
    /// 无功能量累加器 (varh)
    reactive_accum_varh: u32,
    /// 有功脉冲计数器
    active_pulse_count: u32,
    /// 无功脉冲计数器
    reactive_pulse_count: u32,
}

/// 指示灯/蜂鸣器实现
pub struct IndicatorImpl {
    led_states: u8, // bit0=Power, bit1=Comm, bit2=Alarm, bit3=PulseP, bit4=PulseQ
}

/// 防窃电实现
pub struct TamperImpl {
    cover_open: bool,
    terminal_open: bool,
    magnetic_detected: bool,
}

/// GPIO 实现
pub struct GpioImpl;

/* ================================================================== */
/*  Board 实现                                                         */
/* ================================================================== */

impl<M: crate::hal::MeteringChip> Board<M> {
    /// 创建板级资源 (所有外设未初始化)
    pub fn new(metering: M) -> Self {
        Self {
            metering,
            rs485: UartChannelDriver {
                channel: UartChannel::Uart0,
                configured: false,
            },
            infrared: UartChannelDriver {
                channel: UartChannel::Uart1,
                configured: false,
            },
            lorawan_uart: UartChannelDriver {
                channel: UartChannel::Uart2,
                configured: false,
            },
            cellular_uart: UartChannelDriver {
                channel: UartChannel::Uart3,
                configured: false,
            },
            lcd: LcdDriverImpl {
                mode: LcdDisplayMode::Off,
                content: LcdContent::default(),
            },
            pulse: PulseOutputImpl {
                active_constant: 6400,
                reactive_constant: 6400,
                pulse_width_ms: 80,
                active_accum_wh: 0,
                reactive_accum_varh: 0,
                active_pulse_count: 0,
                reactive_pulse_count: 0,
            },
            indicator: IndicatorImpl { led_states: 0 },
            tamper: TamperImpl {
                cover_open: false,
                terminal_open: false,
                magnetic_detected: false,
            },
            gpio: GpioImpl,
        }
    }

    /// 初始化所有板级硬件
    ///
    /// 初始化顺序:
    /// 1. 时钟系统 (PLL 64MHz)
    /// 2. GPIO 基础配置
    /// 3. SPI0 (计量芯片)
    /// 4. SPI1 (外部 Flash, 可选)
    /// 5. UART0~3 + LPUART0
    /// 6. LCD 控制器
    /// 7. ADC (电池/温度)
    /// 8. RTC
    /// 9. 中断优先级配置
    pub fn init(&mut self) {
        // 1. 时钟初始化
        Self::clock_init();

        // 2. 使能外设总线时钟
        cmu_enable_pclk();

        // 3. GPIO 初始化
        Self::gpio_init();

        // 4. SPI 初始化
        Self::spi_init();

        // 5. UART 初始化
        // (延迟到具体使用时配置波特率)

        // 6. LCD 初始化
        self.lcd.hw_init();

        // 7. ADC 初始化
        Self::adc_init();

        // 8. RTC 初始化
        Self::rtc_init();

        // 9. 中断优先级
        Self::nvic_init();
    }

    /// 时钟系统初始化
    ///
    /// XTHF (8MHz) → PLL_H (×8 → 64MHz) → SYSCLK
    /// XTLF: 32.768kHz for RTC
    fn clock_init() {
        let cmu = fm33lg0::cmu();

        unsafe {
            // ── Step 1: 使能外部高速晶振 XTHF ──
            // XTHFCR bit0 = EN
            write_reg(
                &cmu.xthfcr as *const u32 as *mut u32,
                read_reg(&cmu.xthfcr as *const u32) | 0x01,
            );

            // ── Step 2: 等待 XTHF 稳定 (bit1 = READY) ──
            // 超时约 2ms, 用循环等待
            let mut timeout = 10000u32;
            while (read_reg(&cmu.xthfcr as *const u32) & 0x02) == 0 {
                timeout -= 1;
                if timeout == 0 {
                    // XTHF 未就绪, 继续使用 RCHF, 但不配置 PLL
                    // 在生产固件中应该做错误处理
                    return;
                }
            }

            // ── Step 3: 配置 PLL_H ──
            // PLLHCR:
            //   bit0   = EN (先关, 配好再开)
            //   bits[4:1] = INSEL (0=RCHF, 1=XTHF)
            //   bits[15:8] = N_DIV (预分频, 1-based: 1=no division)
            //   bits[23:16] = M_MUL (倍频, 1-based: 8 = ×8)
            //
            // 目标: XTHF=8MHz, N_DIV=1, M_MUL=8 → 8×8 = 64MHz
            let pll_val = (0x01 << 1)  // INSEL = XTHF
                        | (0x01 << 8)  // N_DIV = 1
                        | (0x08 << 16); // M_MUL = 8
            write_reg(&cmu.pll_hcr as *const u32 as *mut u32, pll_val);

            // 使能 PLL_H (bit0)
            write_reg(
                &cmu.pll_hcr as *const u32 as *mut u32,
                read_reg(&cmu.pll_hcr as *const u32) | 0x01,
            );

            // 等待 PLL_H 锁定 (轮询 CMU ISR bit for PLL ready)
            // FM33 使用 ISR 的 bit 来指示 PLL 就绪
            timeout = 10000;
            while (read_reg(&cmu.isr as *const u32) & 0x04) == 0 {
                // bit2 = PLL_H ready
                timeout -= 1;
                if timeout == 0 {
                    break;
                }
            }

            // ── Step 4: 切换系统时钟到 PLL_H ──
            // SYSCLKCR bits[2:0] = 3 → PLL_H
            write_reg(
                &cmu.sysclkcr as *const u32 as *mut u32,
                (read_reg(&cmu.sysclkcr as *const u32) & !0x07) | 0x03,
            );

            // ── Step 5: 使能 XTLF (32.768kHz for RTC) ──
            // 通过 PMU 或 CMU 操作. XTLF 使能通常在 PMU 中
            // FM33LC0xx: CMU 没有 XTLFCR, XTLF 通过 PMU 或 RTC 域使能
            // RTC 域默认使能 XTLF, 这里不需要额外操作

            // ── Step 6: 配置 AHB/APB 分频 ──
            // FM33 默认 AHB=/1, APB=/1, 无需修改
        }
    }

    /// GPIO 初始化
    fn gpio_init() {
        // ══════════════════════════════════════════════════
        // SPI0: PF12(MOSI), PF13(MISO), PF14(SCK) → 数字功能
        //       PF15(CSN) → GPIO 输出 (手动 CS)
        // ══════════════════════════════════════════════════
        for pin in &[pins::SPI0_SCK, pins::SPI0_MISO, pins::SPI0_MOSI] {
            gpio_set_fcr(pin.port, pin.pin, 0b10); // 数字功能
                                                   // DFS 配置: SPI0 功能编号 (查阅 FM33A0xxEV 引脚映射表)
            gpio_set_dfs(pin.port, pin.pin, 0b00); // SPI0 = DFS func 0 for these pins
            gpio_enable_input(pin.port, pin.pin);
        }
        // CSN: GPIO 输出, 默认高
        gpio_set_fcr(pins::SPI0_CSN.port, pins::SPI0_CSN.pin, 0b01); // 输出
        gpio_set(pins::SPI0_CSN);

        // ══════════════════════════════════════════════════
        // SPI1: PA4(CSN), PA5(SCK), PA6(MISO), PA7(MOSI) → 数字功能
        // ══════════════════════════════════════════════════
        for pin in &[pins::SPI1_SCK, pins::SPI1_MISO, pins::SPI1_MOSI] {
            gpio_set_fcr(pin.port, pin.pin, 0b10); // 数字功能
            gpio_set_dfs(pin.port, pin.pin, 0b01); // SPI1 = DFS func 1
            gpio_enable_input(pin.port, pin.pin);
        }
        gpio_set_fcr(pins::SPI1_CSN.port, pins::SPI1_CSN.pin, 0b01); // 输出
        gpio_set(pins::SPI1_CSN);

        // ══════════════════════════════════════════════════
        // UART0: PG8(RX), PG9(TX) → 数字功能
        // ══════════════════════════════════════════════════
        for pin in &[pins::UART0_TX, pins::UART0_RX] {
            gpio_set_fcr(pin.port, pin.pin, 0b10);
            gpio_set_dfs(pin.port, pin.pin, 0b00); // UART0 func
            gpio_enable_input(pin.port, pin.pin);
        }
        // RS485 DE: GPIO 输出, 默认低 (接收模式)
        gpio_set_fcr(pins::RS485_DE.port, pins::RS485_DE.pin, 0b01);
        gpio_clr(pins::RS485_DE);

        // ══════════════════════════════════════════════════
        // UART1: PE3(RX), PE4(TX) → 数字功能
        // ══════════════════════════════════════════════════
        for pin in &[pins::UART1_TX, pins::UART1_RX] {
            gpio_set_fcr(pin.port, pin.pin, 0b10);
            gpio_set_dfs(pin.port, pin.pin, 0b01); // UART1 func
            gpio_enable_input(pin.port, pin.pin);
        }

        // ══════════════════════════════════════════════════
        // UART2: PB2(RX), PB3(TX) → 数字功能
        // ══════════════════════════════════════════════════
        for pin in &[pins::UART2_TX, pins::UART2_RX] {
            gpio_set_fcr(pin.port, pin.pin, 0b10);
            gpio_set_dfs(pin.port, pin.pin, 0b10); // UART2 func
            gpio_enable_input(pin.port, pin.pin);
        }

        // ══════════════════════════════════════════════════
        // UART3: PB14(RX), PB15(TX) → 数字功能
        // ══════════════════════════════════════════════════
        for pin in &[pins::UART3_TX, pins::UART3_RX] {
            gpio_set_fcr(pin.port, pin.pin, 0b10);
            gpio_set_dfs(pin.port, pin.pin, 0b11); // UART3 func
            gpio_enable_input(pin.port, pin.pin);
        }

        // 蜂窝模组控制: GPIO 输出
        gpio_set_fcr(pins::CELL_PWRKEY.port, pins::CELL_PWRKEY.pin, 0b01);
        gpio_clr(pins::CELL_PWRKEY);
        gpio_set_fcr(pins::CELL_RESET.port, pins::CELL_RESET.pin, 0b01);
        gpio_set(pins::CELL_RESET); // reset inactive high

        // ══════════════════════════════════════════════════
        // LED: PA8~PA12 → GPIO 推挽输出, 默认灭 (低)
        // ══════════════════════════════════════════════════
        for led in &[
            pins::LED_POWER,
            pins::LED_COMM,
            pins::LED_ALARM,
            pins::LED_PULSE_P,
            pins::LED_PULSE_Q,
        ] {
            gpio_set_fcr(led.port, led.pin, 0b01); // 输出
            gpio_clr(*led); // 默认灭
        }

        // ══════════════════════════════════════════════════
        // 蜂鸣器: PA15 → GPIO 输出, 默认关
        // ══════════════════════════════════════════════════
        gpio_set_fcr(pins::BUZZER.port, pins::BUZZER.pin, 0b01);
        gpio_clr(pins::BUZZER);

        // ══════════════════════════════════════════════════
        // 按键: PB0, PB1 → 上拉输入
        // ══════════════════════════════════════════════════
        for key in &[pins::KEY_PAGE, pins::KEY_PROG] {
            gpio_set_fcr(key.port, key.pin, 0b00); // 输入
            gpio_enable_input(key.port, key.pin);
            gpio_enable_pullup(key.port, key.pin);
        }

        // ══════════════════════════════════════════════════
        // 脉冲输出: PB4, PB5 → GPIO 推挽输出, 默认低
        // ══════════════════════════════════════════════════
        for pulse in &[pins::PULSE_P, pins::PULSE_Q] {
            gpio_set_fcr(pulse.port, pulse.pin, 0b01);
            gpio_clr(*pulse);
        }

        // ══════════════════════════════════════════════════
        // 防窃电检测: PD7, PD8, PD9, PA13 → 上拉输入
        // ══════════════════════════════════════════════════
        for det in &[pins::COVER_DET, pins::TERMINAL_DET, pins::SIM_DET] {
            gpio_set_fcr(det.port, det.pin, 0b00);
            gpio_enable_input(det.port, det.pin);
            gpio_enable_pullup(det.port, det.pin);
        }
        gpio_set_fcr(pins::MAGNETIC_DET.port, pins::MAGNETIC_DET.pin, 0b00);
        gpio_enable_input(pins::MAGNETIC_DET.port, pins::MAGNETIC_DET.pin);
        gpio_enable_pullup(pins::MAGNETIC_DET.port, pins::MAGNETIC_DET.pin);

        // ══════════════════════════════════════════════════
        // 电池 ADC: PF6 → 模拟功能
        // ══════════════════════════════════════════════════
        gpio_set_fcr(pins::BAT_ADC.port, pins::BAT_ADC.pin, 0b11); // 模拟
                                                                   // ANEN 使能 (GPIO ANEN 寄存器)
        let gpio = gpio_port(pins::BAT_ADC.port);
        unsafe {
            write_reg(
                &gpio.anen as *const u32 as *mut u32,
                read_reg(&gpio.anen as *const u32) | (1u32 << pins::BAT_ADC.pin),
            );
        }
    }

    /// SPI 初始化
    fn spi_init() {
        // ══════════════════════════════════════════════════
        // SPI0: 计量芯片
        //   Mode 0 (CPOL=0, CPHA=0)
        //   MSB first (LSBF=0)
        //   8-bit 数据帧
        //   Master mode (MM=1)
        //   时钟: SYSCLK/8 = 8MHz (安全起始值)
        // ══════════════════════════════════════════════════
        let spi0 = fm33lg0::spi0();
        unsafe {
            // CR1: CPOL=0, CPHA=0, MSB first, MM=1
            // BAUD[5:3]: 000=/2, 001=/4, 010=/8, 011=/16, 100=/32, 101=/64, 110=/128
            let baud_div = 0b010; // /8 → 64MHz/8 = 8MHz
            let cr1 = spi_cr1::MM              // Master
                    | (baud_div << spi_cr1::BAUD_SHIFT);
            write_reg(&spi0.cr1 as *const u32 as *mut u32, cr1);

            // CR2: DLEN=8bit (0 means 8bit), TXOEN=1, RXOEN=1, SSNSEN=0 (manual CS)
            let cr2 = spi_cr2::TXOEN | spi_cr2::RXOEN;
            write_reg(&spi0.cr2 as *const u32 as *mut u32, cr2);

            // CR3: 使能 SPI (bit0=EN) — 写 0x01
            write_reg(&spi0.cr3 as *const u32 as *mut u32, 0x01);
        }

        // ══════════════════════════════════════════════════
        // SPI1: 外部 Flash (W25Q64)
        //   Mode 0, MSB first, Master, 8-bit
        //   时钟: SYSCLK/4 = 16MHz
        // ══════════════════════════════════════════════════
        let spi1 = fm33lg0::spi1();
        unsafe {
            let baud_div = 0b001; // /4 → 64MHz/4 = 16MHz
            let cr1 = spi_cr1::MM | (baud_div << spi_cr1::BAUD_SHIFT);
            write_reg(&spi1.cr1 as *const u32 as *mut u32, cr1);

            let cr2 = spi_cr2::TXOEN | spi_cr2::RXOEN;
            write_reg(&spi1.cr2 as *const u32 as *mut u32, cr2);

            write_reg(&spi1.cr3 as *const u32 as *mut u32, 0x01);
        }
    }

    /// ADC 初始化 (电池电压/温度)
    fn adc_init() {
        // FM33A068EV 内置 12-bit SAR ADC
        let adc = fm33lg0::adc();
        unsafe {
            // CR: 使能 ADC, 选择时钟源
            // bit0 = EN, bit1 = START, bits[4:2] = clock source/div
            // 先使能 ADC
            write_reg(&adc.cr as *const u32 as *mut u32, 0x01); // EN

            // CFGR: 配置通道、采样率等
            // 通道 5 (PF6 = BAT_ADC)
            // bits[3:0] = channel select = 5
            write_reg(&adc.cfgr as *const u32 as *mut u32, 0x05);

            // 使能 ADC 中断 (转换完成)
            write_reg(&adc.isr as *const u32 as *mut u32, 0x01); // 转换完成中断使能

            // TRIM: 使用出厂校准值 (已由 Flash loader 写入)
            // 这里保持默认
        }
    }

    /// RTC 初始化
    fn rtc_init() {
        let rtc = fm33lg0::rtc();
        unsafe {
            // WER: 写使能 (写 0xACAC0001 允许写 RTC 寄存器)
            write_reg(&rtc.wer as *const u32 as *mut u32, 0xACAC_0001);

            // CR: 配置 RTC
            // bit0 = EN (使能 RTC)
            // bit1 = CLOCKSOURCE (0=XTLF 32.768kHz)
            // bit2 = FORMAT (0=BCD)
            write_reg(&rtc.cr as *const u32 as *mut u32, 0x01); // 使能 RTC, XTLF, BCD

            // 清除所有中断标志
            write_reg(&rtc.isr as *const u32 as *mut u32, 0xFFFF_FFFF); // 写1清零

            // 校准: adjust 寄存器用于数字调校 (±0.119ppm 精度)
            // 初始值 0 = 不调校
            write_reg(&rtc.adjust as *const u32 as *mut u32, 0);

            // 锁定写保护
            write_reg(&rtc.wer as *const u32 as *mut u32, 0x0000_0000);
        }
    }

    /// NVIC 中断优先级配置
    fn nvic_init() {
        // TODO: NVIC 配置需要 PAC crate 提供 InterruptNumber trait 实现
        // 当前使用 fm33lg0::irqn 常量 (isize), cortex-m NVIC API 需要 InterruptNumber trait
        // 解决方案: 生成 SVD PAC crate 或手动实现 InterruptNumber for irqn 枚举
        //
        // 计划的中断优先级:
        //   UART0 (RS485): 优先级 1 (高)
        //   UART3 (蜂窝): 优先级 1
        //   SPI0  (计量): 优先级 2
        //   GPIO  (EXTI): 优先级 2
        //   RTC:          优先级 3 (最低)
    }
}

/* ================================================================== */
/*  UART 通道驱动实现                                                   */
/* ================================================================== */

impl UartDriver for UartChannelDriver {
    fn init(&mut self, config: &UartConfig) -> Result<(), UartError> {
        uart_config_regs(self.channel, config);

        // RS485 特殊处理: 使能超时检测
        if matches!(self.channel, UartChannel::Uart0) {
            let uart = uart_regs(self.channel);
            unsafe {
                // 设置 RX 超时 = 20 bit periods
                write_reg(&uart.todr as *const u32 as *mut u32, 20);
                // 使能超时
                let csr = read_reg(&uart.csr as *const u32);
                write_reg(&uart.csr as *const u32 as *mut u32, csr | uart_csr::RXTOEN);
            }
        }

        self.configured = true;
        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<(), UartError> {
        if !self.configured {
            return Err(UartError::TxTimeout);
        }

        let uart = uart_regs(self.channel);

        // RS485: 切换到发送模式
        if matches!(self.channel, UartChannel::Uart0) {
            gpio_set(pins::RS485_DE);
            delay_us(50); // 等待 RS485 收发器切换
        }

        for &byte in data {
            unsafe {
                // 等待发送缓冲区空 (TXSE = ISR bit0)
                let mut timeout = 100_000u32;
                while (read_reg(&uart.isr as *const u32) & uart_isr::TXSE) == 0 {
                    timeout -= 1;
                    if timeout == 0 {
                        // RS485: 切回接收
                        if matches!(self.channel, UartChannel::Uart0) {
                            gpio_clr(pins::RS485_DE);
                        }
                        return Err(UartError::TxTimeout);
                    }
                }
                write_reg(&uart.txbuf as *const u32 as *mut u32, byte as u32);
            }
        }

        // 等待最后字节发送完成
        unsafe {
            let mut timeout = 100_000u32;
            while (read_reg(&uart.isr as *const u32) & uart_isr::TXBE) == 0 {
                timeout -= 1;
                if timeout == 0 {
                    break;
                }
            }
        }

        // RS485: 切回接收模式
        if matches!(self.channel, UartChannel::Uart0) {
            delay_us(50);
            gpio_clr(pins::RS485_DE);
        }

        Ok(())
    }

    fn read(&mut self, buf: &mut [u8], timeout_ms: u32) -> Result<usize, UartError> {
        if !self.configured {
            return Err(UartError::RxTimeout);
        }

        let uart = uart_regs(self.channel);
        let mut received = 0;

        // 简单超时循环: 每个 byte ~1ms timeout
        for slot in buf.iter_mut() {
            let mut byte_timeout = timeout_ms * 1000; // rough us counter
            unsafe {
                loop {
                    let isr = read_reg(&uart.isr as *const u32);
                    if (isr & uart_isr::RXBF) != 0 {
                        // 检查错误
                        if (isr & uart_isr::OERR) != 0 {
                            // 清除溢出标志 (写1清零)
                            write_reg(&uart.isr as *const u32 as *mut u32, uart_isr::OERR);
                            return Err(UartError::OverrunError);
                        }
                        if (isr & uart_isr::FERR) != 0 {
                            write_reg(&uart.isr as *const u32 as *mut u32, uart_isr::FERR);
                            return Err(UartError::FramingError);
                        }
                        if (isr & uart_isr::PERR) != 0 {
                            write_reg(&uart.isr as *const u32 as *mut u32, uart_isr::PERR);
                            return Err(UartError::ParityError);
                        }
                        *slot = read_reg(&uart.rxbuf as *const u32) as u8;
                        received += 1;
                        break;
                    }
                    byte_timeout -= 1;
                    if byte_timeout == 0 {
                        if received > 0 {
                            return Ok(received);
                        }
                        return Err(UartError::RxTimeout);
                    }
                }
            }
        }

        Ok(received)
    }

    fn readable(&self) -> bool {
        if !self.configured {
            return false;
        }
        let uart = uart_regs(self.channel);
        unsafe { (read_reg(&uart.isr as *const u32) & uart_isr::RXBF) != 0 }
    }

    fn channel(&self) -> UartChannel {
        self.channel
    }
}

/* ================================================================== */
/*  SPI 传输实现 (MeteringChip 用)                                      */
/* ================================================================== */

impl SpiTransfer for GpioImpl {
    fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), MeteringError> {
        if tx.len() != rx.len() {
            return Err(MeteringError::SpiError);
        }

        spi0_cs_low();
        // 短延时确保 CS 建立
        delay_us(1);

        for i in 0..tx.len() {
            rx[i] = spi0_transfer(tx[i]);
        }

        spi0_cs_high();
        delay_us(1);

        Ok(())
    }
}

/* ================================================================== */
/*  LCD 驱动实现                                                        */
/* ================================================================== */

impl LcdDriverImpl {
    /// 硬件初始化
    pub fn hw_init(&mut self) {
        let lcd = fm33lg0::lcd();
        unsafe {
            // CR 配置:
            //   EN=0 (先关)
            //   LMUX[2:1]=00 → 4COM
            //   BIASMD=0 → 1/3 bias
            //   ENMODE=0 → 普通使能
            //   IC_CTRL[17:16]=00 → 内部电阻分压
            let cr = 0x00; // 4COM, 1/3 bias, 内部电阻
            write_reg(&lcd.cr as *const u32 as *mut u32, cr);

            // COM 使能: 4COM → COM0~COM3
            write_reg(&lcd.comen as *const u32 as *mut u32, 0x0F); // bit0~3 = COM0~COM3

            // SEG 使能: 根据 4COM×44SEG 配置
            // SEGEN0: SEG0~SEG31
            // SEGEN1: SEG32~SEG43 (bit0~bit11)
            write_reg(&lcd.segen0 as *const u32 as *mut u32, 0xFFFF_FFFF); // SEG0~31
            write_reg(&lcd.segen1 as *const u32 as *mut u32, 0x0000_0FFF); // SEG32~43

            // 频率控制: LCD 刷新率
            // FCR.DF[7:0]: 分频系数, 典型值使刷新率 ~64Hz
            write_reg(&lcd.fcr as *const u32 as *mut u32, 0x40); // DF=64

            // 清除所有显示数据
            for i in 0..10 {
                write_reg(&lcd.data[i] as *const u32 as *mut u32, 0);
            }

            // 使能 LCD
            write_reg(&lcd.cr as *const u32 as *mut u32, cr | lcd_cr::EN);
        }
    }
}

impl LcdDriver for LcdDriverImpl {
    fn init(&mut self) {
        self.hw_init();
    }

    fn update(&mut self, content: &LcdContent) {
        self.content = *content;
        // TODO: 将 LcdContent 映射到 LCD 段码
        // 这需要完整的 segment mapping table, 取决于 PCB 走线
        // 每个 digit → 7 段码 → 对应的 SEG/COM 位
        // 暂时只做清屏
        let lcd = fm33lg0::lcd();
        unsafe {
            // 示例: 显示电压值到 DATA0~DATA3
            // 实际映射需要查阅 PCB segment mapping
            // 这里写一个简单的占位实现
            let _ = content;
        }
    }

    fn set_mode(&mut self, mode: LcdDisplayMode) {
        self.mode = mode;
    }

    fn enable(&mut self, on: bool) {
        let lcd = fm33lg0::lcd();
        unsafe {
            let cr = read_reg(&lcd.cr as *const u32);
            if on {
                write_reg(&lcd.cr as *const u32 as *mut u32, cr | lcd_cr::EN);
            } else {
                write_reg(&lcd.cr as *const u32 as *mut u32, cr & !lcd_cr::EN);
            }
        }
    }

    fn set_bias(&mut self, bias: LcdBias) {
        let lcd = fm33lg0::lcd();
        unsafe {
            let cr = read_reg(&lcd.cr as *const u32);
            match bias {
                LcdBias::Third => {
                    write_reg(&lcd.cr as *const u32 as *mut u32, cr & !lcd_cr::BIASMD);
                }
                LcdBias::Quarter => {
                    write_reg(&lcd.cr as *const u32 as *mut u32, cr | lcd_cr::BIASMD);
                }
            }
        }
    }
}

/* ================================================================== */
/*  脉冲输出实现                                                       */
/* ================================================================== */

impl PulseDriver for PulseOutputImpl {
    fn configure(&mut self, config: &PulseConfig) {
        self.active_constant = config.constant_imp;
        self.reactive_constant = config.constant_imp; // same constant for both typically
        self.pulse_width_ms = config.pulse_width_ms;
    }

    fn update_energy(&mut self, pulse_type: PulseType, delta_wh: u32) {
        match pulse_type {
            PulseType::Active => {
                self.active_accum_wh += delta_wh;
                // 脉冲常数: imp/kWh → 每个脉冲代表 1000/constant Wh
                let wh_per_pulse = 1_000_000 / self.active_constant; // 转换为 0.001Wh 精度
                while self.active_accum_wh >= wh_per_pulse {
                    self.active_accum_wh -= wh_per_pulse;
                    self.active_pulse_count += 1;
                    // 输出一个脉冲
                    gpio_set(pins::PULSE_P);
                    gpio_set(pins::LED_PULSE_P);
                    delay_ms(self.pulse_width_ms as u32);
                    gpio_clr(pins::PULSE_P);
                    gpio_clr(pins::LED_PULSE_P);
                }
            }
            PulseType::Reactive => {
                self.reactive_accum_varh += delta_wh;
                let varh_per_pulse = 1_000_000 / self.reactive_constant;
                while self.reactive_accum_varh >= varh_per_pulse {
                    self.reactive_accum_varh -= varh_per_pulse;
                    self.reactive_pulse_count += 1;
                    gpio_set(pins::PULSE_Q);
                    gpio_set(pins::LED_PULSE_Q);
                    delay_ms(self.pulse_width_ms as u32);
                    gpio_clr(pins::PULSE_Q);
                    gpio_clr(pins::LED_PULSE_Q);
                }
            }
        }
    }
}

/* ================================================================== */
/*  指示灯/蜂鸣器实现                                                  */
/* ================================================================== */

impl IndicatorDriver for IndicatorImpl {
    fn set_led(&mut self, led: Led, on: bool) {
        let bit = match led {
            Led::Power => 0,
            Led::Communication => 1,
            Led::Alarm => 2,
            Led::PulseActive => 3,
            Led::PulseReactive => 4,
        };
        let pin = match led {
            Led::Power => pins::LED_POWER,
            Led::Communication => pins::LED_COMM,
            Led::Alarm => pins::LED_ALARM,
            Led::PulseActive => pins::LED_PULSE_P,
            Led::PulseReactive => pins::LED_PULSE_Q,
        };
        if on {
            self.led_states |= 1 << bit;
            gpio_set(pin);
        } else {
            self.led_states &= !(1 << bit);
            gpio_clr(pin);
        }
    }

    fn toggle_led(&mut self, led: Led) {
        let bit = match led {
            Led::Power => 0,
            Led::Communication => 1,
            Led::Alarm => 2,
            Led::PulseActive => 3,
            Led::PulseReactive => 4,
        };
        let pin = match led {
            Led::Power => pins::LED_POWER,
            Led::Communication => pins::LED_COMM,
            Led::Alarm => pins::LED_ALARM,
            Led::PulseActive => pins::LED_PULSE_P,
            Led::PulseReactive => pins::LED_PULSE_Q,
        };
        self.led_states ^= 1 << bit;
        // Toggle via reading current state
        if gpio_read_pin(pin) {
            gpio_clr(pin);
        } else {
            gpio_set(pin);
        }
    }

    fn buzzer_alarm(&mut self, duration_ms: u16) {
        gpio_set(pins::BUZZER);
        delay_ms(duration_ms as u32);
        gpio_clr(pins::BUZZER);
    }

    fn buzzer_off(&mut self) {
        gpio_clr(pins::BUZZER);
    }
}

impl IndicatorImpl {
    /// 蜂鸣器告警 (简化版, 阻塞)
    pub fn buzzer_alarm_blocking(&mut self, duration_ms: u16) {
        self.buzzer_alarm(duration_ms);
    }
}

/* ================================================================== */
/*  GPIO 驱动实现                                                       */
/* ================================================================== */

impl GpioDriver for GpioImpl {
    fn set_direction(&mut self, pin: GpioPin, dir: GpioDirection) {
        match dir {
            GpioDirection::Input => {
                gpio_set_fcr(pin.port, pin.pin, 0b00);
                gpio_enable_input(pin.port, pin.pin);
            }
            GpioDirection::Output => {
                gpio_set_fcr(pin.port, pin.pin, 0b01);
            }
        }
    }

    fn write(&mut self, pin: GpioPin, level: GpioLevel) {
        match level {
            GpioLevel::High => gpio_set(pin),
            GpioLevel::Low => gpio_clr(pin),
        }
    }

    fn read(&self, pin: GpioPin) -> GpioLevel {
        if gpio_read_pin(pin) {
            GpioLevel::High
        } else {
            GpioLevel::Low
        }
    }

    fn toggle(&mut self, pin: GpioPin) {
        if gpio_read_pin(pin) {
            gpio_clr(pin);
        } else {
            gpio_set(pin);
        }
    }
}

/* ================================================================== */
/*  防窃电检测实现                                                     */
/* ================================================================== */

impl TamperDriver for TamperImpl {
    fn check_events(&mut self) -> Option<TamperEvent> {
        // 读取 GPIO 状态, 检测低电平 = 触发 (开盖等)
        // COVER_DET: 上拉输入, 开盖时接地 → 低电平
        if !gpio_read_pin(pins::COVER_DET) {
            self.cover_open = true;
        }
        if !gpio_read_pin(pins::TERMINAL_DET) {
            self.terminal_open = true;
        }
        // MAGNETIC_DET: 霍尔传感器, 检测到磁场时低电平
        if !gpio_read_pin(pins::MAGNETIC_DET) {
            self.magnetic_detected = true;
        }

        if self.cover_open {
            self.cover_open = false;
            return Some(TamperEvent::CoverOpen);
        }
        if self.terminal_open {
            self.terminal_open = false;
            return Some(TamperEvent::TerminalCoverOpen);
        }
        if self.magnetic_detected {
            self.magnetic_detected = false;
            return Some(TamperEvent::MagneticFieldDetected);
        }
        None
    }

    fn magnetic_field_strength(&mut self) -> Option<u16> {
        // TODO: 需要通过 ADC 读取霍尔传感器模拟输出
        // 目前 MAGNETIC_DET 是数字输入, 只能检测有/无
        // 如果有模拟霍尔传感器, 需要通过 ADC 通道读取
        None
    }
}

/* ── 模块级便捷函数 (供 main.rs 调用) ── */

pub mod tamper_ext {
    use super::{gpio_read_pin, pins};

    /// 防窃电检测 — 开盖检测
    pub fn check_cover_open() -> bool {
        gpio_read_pin(pins::COVER_DET)
    }

    /// 防窃电检测 — 磁场检测
    pub fn check_magnetic() -> bool {
        gpio_read_pin(pins::MAGNETIC_DET)
    }
}

pub mod pulse_ext {
    use super::{gpio_port, pins};

    /// 翻转有功脉冲 GPIO
    pub fn toggle_active() {
        unsafe {
            let gpio = gpio_port(pins::PULSE_P.port);
            let odr = core::ptr::read_volatile(&gpio.do_reg as *const u32);
            core::ptr::write_volatile(
                &gpio.do_reg as *const u32 as *mut u32,
                odr ^ (1u32 << pins::PULSE_P.pin),
            );
        }
    }
}

pub mod adc {
    /// 读取 MCU 内部温度传感器 ADC 原始值
    pub fn read_temperature_raw() -> u16 {
        // ADC 通道 16 = 内部温度传感器 (FM33A068EV)
        // 基准电压 3.3V, 12bit ADC
        0 // 占位: 实际由 HAL ADC 驱动读取
    }

    /// 将 ADC 原始值转换为摄氏温度
    pub fn raw_to_celsius(raw: u16) -> i16 {
        let v_sense = (raw as u32) * 3300 / 4096;
        let temp_x10 = ((v_sense as i32 - 1430) * 10 / 43) + 250;
        (temp_x10 / 10) as i16
    }
}
