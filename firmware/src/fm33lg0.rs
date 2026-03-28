//! FM33LG0xx 寄存器定义 (最小集)
//!
//! 参考: FM33LG0xx User Manual (复旦微电子)
//! Cortex-M0+ @ 64MHz, 256KB Flash, 32KB RAM

/// FM33LG0xx 基地址
pub mod base {
    pub const GPIOA: usize = 0x4001_0000;
    pub const GPIOB: usize = 0x4001_0400;
    pub const GPIOC: usize = 0x4001_0800;
    pub const GPIOD: usize = 0x4001_0C00;
    pub const GPIOE: usize = 0x4001_1000;
    pub const GPIOF: usize = 0x4001_1400;

    pub const UART0: usize = 0x4001_8000;
    pub const UART1: usize = 0x4001_8400;
    pub const UART2: usize = 0x4001_8800;
    pub const UART3: usize = 0x4001_8C00;

    pub const SPI0:  usize = 0x4001_4000;
    pub const SPI1:  usize = 0x4001_4400;

    pub const I2C0:  usize = 0x4001_5000;
    pub const I2C1:  usize = 0x4001_5400;

    pub const ADC:   usize = 0x4002_0000;
    pub const LCD:   usize = 0x4002_4000;
    pub const RTC:   usize = 0x4002_8000;
    pub const AES:   usize = 0x4002_C000;
    pub const FLASH: usize = 0x4003_0000;
    pub const PMU:   usize = 0x4003_4000;  // Power Management Unit
    pub const RCC:   usize = 0x4003_8000;  // Reset & Clock Control
    pub const IWDT:  usize = 0x4003_C000;  // Independent Watchdog
    pub const WWDT:  usize = 0x4003_C400;  // Window Watchdog
    pub const TIM0:  usize = 0x4004_0000;
    pub const TIM1:  usize = 0x4004_0400;
    pub const TIM2:  usize = 0x4004_0800;
    pub const TIM3:  usize = 0x4004_0C00;
    pub const DMA:   usize = 0x4005_0000;
}

/// GPIO 寄存器结构 (每个 GPIO port)
#[repr(C)]
pub struct GpioRegs {
    pub doset: u32,      // 0x00: Data output set
    pub doclr: u32,      // 0x04: Data output clear
    pub din:   u32,      // 0x08: Data input
    pub dodt:  u32,      // 0x0C: Data output
    pub mode:  [u32; 4], // 0x10-0x1C: Pin mode (input/output/analog/alternate)
    pub otype: u32,      // 0x20: Output type (push-pull/open-drain)
    pub odsr:  u32,      // 0x24: Output drive strength
    pub pull:  u32,      // 0x28: Pull-up/pull-down
    pub irqen: u32,      // 0x2C: Interrupt enable
    pub irqtype: u32,    // 0x30: Interrupt type
    pub irqpend: u32,    // 0x34: Interrupt pending
}

/// UART 寄存器结构
#[repr(C)]
pub struct UartRegs {
    pub isr:   u32,    // 0x00: Interrupt Status
    pub isrclr: u32,   // 0x04: Interrupt Status Clear
    pub ier:   u32,    // 0x08: Interrupt Enable
    pub cr:    u32,    // 0x0C: Control Register
    pub sr:    u32,    // 0x10: Status Register
    pub dr:    u32,    // 0x14: Data Register (read=RX, write=TX)
    pub brr:   u32,    // 0x18: Baud Rate Register
    pub lcr:   u32,    // 0x1C: Line Control (8N1 config)
    pub crsr:  u32,    // 0x20: CRS/DE control (RS-485)
    pub rxdly: u32,    // 0x24: RX delay
    pub txdly: u32,    // 0x28: TX delay
}

/// SPI 寄存器结构
#[repr(C)]
pub struct SpiRegs {
    pub cr:    u32,    // 0x00: Control
    pub sr:    u32,    // 0x04: Status
    pub dr:    u32,    // 0x08: Data
    pub ccr:   u32,    // 0x0C: Clock Control
    pub ssen:  u32,    // 0x10: Slave Select
}

/// LCD 控制器寄存器
#[repr(C)]
pub struct LcdRegs {
    pub cr:     u32,   // 0x00: Control Register
    pub fcr:    u32,   // 0x04: Frame Control Register
    pub sr:     u32,   // 0x08: Status Register
    pub icr:    u32,   // 0x0C: Interrupt Control
    pub isr:    u32,   // 0x10: Interrupt Status
    pub contrast: u32, // 0x14: Contrast Control
    pub segen:  u32,   // 0x18: Segment Enable
    pub comen:  u32,   // 0x1C: Common Enable
    pub ram:    [u32; 16], // 0x20-0x5C: Display RAM (4COM x 40SEG)
}

/// RTC 寄存器结构
#[repr(C)]
pub struct RtcRegs {
    pub time: u32,      // 0x00: Time (BCD: HH:MM:SS)
    pub date: u32,      // 0x04: Date (BCD: YY:MM:DD)
    pub cr:   u32,      // 0x08: Control
    pub almh: u32,      // 0x0C: Alarm High
    pub alml: u32,      // 0x10: Alarm Low
    pub isr:  u32,      // 0x14: Interrupt Status
    pub ier:  u32,      // 0x18: Interrupt Enable
}

/// IWDT 独立看门狗寄存器
#[repr(C)]
pub struct IwdtRegs {
    pub wdgen:  u32,   // 0x00: Enable
    pub wdtcr:  u32,   // 0x04: Control
    pub wdtsr:  u32,   // 0x08: Status
    pub wdtvr:  u32,   // 0x0C: Value/Reload
    pub wdtrld: u32,   // 0x10: Reload key
}

/// RCC 时钟控制寄存器
#[repr(C)]
pub struct RccRegs {
    pub ahbenr:  u32,  // 0x00: AHB enable
    pub apbenr1: u32,  // 0x04: APB1 enable
    pub apbenr2: u32,  // 0x08: APB2 enable
    pub ahbrstr: u32,  // 0x0C: AHB reset
    pub apbrstr1: u32, // 0x10: APB1 reset
    pub apbrstr2: u32, // 0x14: APB2 reset
    pub clkcfg:  u32,  // 0x18: Clock config
    pub rdy:     u32,  // 0x1C: Ready flags
}

// ── 寄存器访问宏 ──────────────────────────────────────────────────

macro_rules! reg {
    ($addr:expr, $t:ty) => {
        unsafe { &*($addr as *const $t) }
    };
    ($addr:expr, $t:ty, mut) => {
        unsafe { &mut *($addr as *mut $t) }
    };
}

/// 获取 UART0 (RS-485) 寄存器
pub fn uart0() -> &'static UartRegs { reg!(base::UART0, UartRegs) }

/// 获取 UART1 (红外) 寄存器
pub fn uart1() -> &'static UartRegs { reg!(base::UART1, UartRegs) }

/// 获取 UART2 (模块通信) 寄存器
pub fn uart2() -> &'static UartRegs { reg!(base::UART2, UartRegs) }

/// 获取 SPI0 (计量芯片) 寄存器
pub fn spi0() -> &'static SpiRegs { reg!(base::SPI0, SpiRegs) }

/// 获取 LCD 控制器寄存器
pub fn lcd() -> &'static LcdRegs { reg!(base::LCD, LcdRegs) }

/// 获取 RTC 寄存器
pub fn rtc() -> &'static RtcRegs { reg!(base::RTC, RtcRegs) }

/// 获取 IWDT 寄存器
pub fn iwdt() -> &'static IwdtRegs { reg!(base::IWDT, IwdtRegs) }

/// 获取 RCC 寄存器
pub fn rcc() -> &'static RccRegs { reg!(base::RCC, RccRegs) }
