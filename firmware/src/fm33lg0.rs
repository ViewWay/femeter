//! FM33LG0xx / FM33A0xxEV / FM33LC0xx 寄存器定义
//!
//! 基于:
//!   - FM33A0XXEV.h (CMSIS SVD 生成, 官方头文件) — 主要参考
//!   - FM33LC0xx 产品说明书 V2.9.1 — 寄存器位域细节
//!
//! ARM Cortex-M0+, 最高 64MHz, 256KB Flash, 24KB RAM
//! FM33A0xxEV 是 FM33LC0xx/LG0xx 的超集，寄存器结构兼容

/// 外设基地址 — 来自 FM33A0XXEV.h
pub mod base {
    pub const PERIPH: usize = 0x4000_0000;

    // ── 系统控制 ──
    pub const DBG:    usize = PERIPH + 0x0000;   // Debug
    pub const QSPI:   usize = PERIPH + 0x0800;   // Quad SPI
    pub const DMA:    usize = PERIPH + 0x0400;   // DMA (12通道)
    pub const PAE:    usize = PERIPH + 0x1400;   // 加密加速引擎
    pub const HASH:   usize = PERIPH + 0x1800;   // Hash
    pub const FLS:    usize = PERIPH + 0x1000;   // Flash 控制器
    pub const PMU:    usize = PERIPH + 0x2000;   // 电源管理
    pub const CMU:    usize = PERIPH + 0x2400;   // 时钟管理
    pub const RMU:    usize = PERIPH + 0x2800;   // 复位管理

    // ── GPIO (每组 0x40) ──
    pub const GPIOA:  usize = PERIPH + 0x0C00;
    pub const GPIOB:  usize = PERIPH + 0x0C40;
    pub const GPIOC:  usize = PERIPH + 0x0C80;
    pub const GPIOD:  usize = PERIPH + 0x0CC0;
    pub const GPIOE:  usize = PERIPH + 0x0D00;
    pub const GPIOF:  usize = PERIPH + 0x0D40;
    pub const GPIOG:  usize = PERIPH + 0x0D80;
    pub const GPIOH:  usize = PERIPH + 0xFC00;   // 低功耗GPIO (简化结构)
    pub const GPIO_COMMON: usize = PERIPH + 0x0DC0; // EXTI, Wakeup

    // ── APB1 外设 ──
    pub const SPI0:   usize = PERIPH + 0x0400;   // 注意: FM33A0xx有SPI0, FM33LC0xx没有
    pub const SPI1:   usize = PERIPH + 0x0800;
    pub const CRC:    usize = PERIPH + 0x0000;   // CRC
    pub const LCD:    usize = PERIPH + 0x0C00;
    pub const RTC:    usize = PERIPH + 0x1000;
    pub const IWDT:   usize = PERIPH + 0x1400;
    pub const WWDT:   usize = PERIPH + 0x1800;
    pub const U7816:  usize = PERIPH + 0x1C00;   // ISO7816智能卡
    pub const UART0:  usize = PERIPH + 0x2000;
    pub const UART1:  usize = PERIPH + 0x6800;
    pub const UART2:  usize = PERIPH + 0x6C00;
    pub const UART3:  usize = PERIPH + 0x7000;
    pub const UART4:  usize = PERIPH + 0x7400;
    pub const UART5:  usize = PERIPH + 0x7800;
    pub const UARTIR: usize = PERIPH + 0x7C00;   // 红外调制 (独立模块!)
    pub const LPUART0: usize = PERIPH + 0x4000;
    pub const LPUART1: usize = PERIPH + 0x4400;
    pub const SPI2:   usize = PERIPH + 0x4800;
    pub const SPI3:   usize = PERIPH + 0x4C00;
    pub const SPI4:   usize = PERIPH + 0x6400;
    pub const I2C0:   usize = PERIPH + 0x2400;
    pub const I2C1:   usize = PERIPH + 0x5000;
    pub const BSTIM:  usize = PERIPH + 0x6000;   // 基本定时器
    pub const LPTIM:  usize = PERIPH + 0x3400;   // 低功耗定时器
    pub const COMP:   usize = PERIPH + 0x5400;   // 比较器
    pub const CIC:    usize = PERIPH + 0x5C00;   // CIC滤波器
    pub const SVD:    usize = PERIPH + 0x2800;   // 电源监测
    pub const ADC:    usize = PERIPH + 0xFA00;
    pub const AES:    usize = PERIPH + 0x3800;
    pub const TRNG:   usize = PERIPH + 0x3C00;

    // ── 存储器 ──
    pub const FLASH_MAIN: usize = 0x0000_0000;
    pub const SRAM:       usize = 0x2000_0000;
}

// ══════════════════════════════════════════════════════════════════
// PMU — 电源管理 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct PmuRegs {
    pub cr:   u32,    // 0x00: 电源管理控制
    pub wktr: u32,    // 0x04: 唤醒时间
    pub wkfr: u32,    // 0x08: 唤醒源标志
    pub ier:  u32,    // 0x0C: 中断使能
    pub isr:  u32,    // 0x10: 中断标志
}

// ══════════════════════════════════════════════════════════════════
// CMU — 时钟管理 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct CmuRegs {
    pub sysclkcr: u32, // 0x00: 系统时钟控制
    pub rchfcr:   u32, // 0x04: RCHF控制
    pub rchftr:   u32, // 0x08: RCHF调校
    pub plllcr:   u32, // 0x0C: PLL_L控制
    pub pllHcr:   u32, // 0x10: PLL_H控制
    pub xthfcr:   u32, // 0x14: XTHF控制
    pub ier:      u32, // 0x18: 中断使能
    pub isr:      u32, // 0x1C: 中断标志
    pub pclken1:  u32, // 0x20: 外设总线时钟使能1
    pub pclken2:  u32, // 0x24: 外设总线时钟使能2
    pub pclken3:  u32, // 0x28: 外设总线时钟使能3
    pub pclken4:  u32, // 0x2C: 外设总线时钟使能4
    pub opccr1:   u32, // 0x30: 外设操作时钟控制1
    pub opccr2:   u32, // 0x34: 外设操作时钟控制2
}

// ══════════════════════════════════════════════════════════════════
// RMU — 复位管理 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct RmuRegs {
    pub pdrcr:   u32,  // 0x00: PDR控制
    pub borcr:   u32,  // 0x04: BOR控制
    pub rstcfgr: u32,  // 0x08: 复位配置
    pub softrst: u32,  // 0x0C: 软件复位 (WO)
    pub rsr:     u32,  // 0x10: 复位状态
    pub prsten:  u32,  // 0x14: 外设复位使能 (WO)
    pub ahbrst:  u32,  // 0x18: AHB外设复位
    pub apbrst1: u32,  // 0x1C: APB外设复位1
    pub apbrst2: u32,  // 0x20: APB外设复位2
}

// ══════════════════════════════════════════════════════════════════
// GPIO — 通用IO (FM33A0XXEV.h)
// 每组 (GPIOA~GPIOG) 占 0x40 字节
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct GpioRegs {
    pub inen:  u32,    // 0x00: 输入使能
    pub puen:  u32,    // 0x04: 上拉使能
    pub oden:  u32,    // 0x08: 开漏使能
    pub fcr:   u32,    // 0x0C: 功能选择 (2bit/引脚: 00=输入,01=输出,10=数字,11=模拟)
    pub do_reg: u32,   // 0x10: 输出数据
    pub dset:  u32,    // 0x14: 输出置位 (WO)
    pub drst:  u32,    // 0x18: 输出复位 (WO)
    pub din:   u32,    // 0x1C: 输入数据 (RO)
    pub dfs:   u32,    // 0x20: 数字功能选择 ★ (FM33A0xx 新增)
    _reserved: u32,    // 0x24
    pub anen:  u32,    // 0x28: 模拟通道使能 ★ (FM33A0xx 新增)
}

/// GPIOH (低功耗域) — 简化结构
#[repr(C)]
pub struct GpiohRegs {
    pub inen: u32,     // 0x00
    pub puen: u32,     // 0x04
    pub fcr:  u32,     // 0x08
    pub do_reg: u32,   // 0x0C
    pub din:  u32,     // 0x10
}

/// GPIO 共享寄存器 (EXTI, Wakeup)
#[repr(C)]
pub struct GpioCommonRegs {
    pub extisel0: u32,  // 0x00: 外部中断输入选择0
    pub extisel1: u32,  // 0x04: 外部中断输入选择1
    pub extieds0: u32,  // 0x08: 外部中断边沿选择使能0
    pub extieds1: u32,  // 0x0C: 外部中断边沿选择使能1
    pub extidf:  u32,   // 0x10: 外部中断数字滤波
    pub extiisr: u32,   // 0x14: 外部中断标志
    pub extidi:  u32,   // 0x18: 外部中断数据输入 (RO)
    _res1: [u32; 9],    // 0x1C~0x3C
    pub foutsel: u32,   // 0x40: 频率输出选择
    pub iomcr:   u32,   // 0x44: IO MUX控制
    _res2: [u32; 62],   // 0x48~0x13C
    pub pinwken: u32,   // 0x140: 唤醒使能
}

// ══════════════════════════════════════════════════════════════════
// UART — 通用异步收发器 (FM33A0XXEV.h)
// 红外调制是独立模块 UARTIR, 不在 UART 结构内!
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct UartRegs {
    pub csr:   u32,    // 0x00: 控制状态 (TXEN/RXEN/PDSEL/PARITY/STOPCFG/BITORD...)
    pub ier:   u32,    // 0x04: 中断使能
    pub isr:   u32,    // 0x08: 中断标志 (写1清零)
    pub todr:  u32,    // 0x0C: 超时和延迟 (TXDLY_LEN[15:8] + RXTO_LEN[7:0])
    pub rxbuf: u32,    // 0x10: 接收缓冲 (RO)
    pub txbuf: u32,    // 0x14: 发送缓冲
    pub bgr:   u32,    // 0x18: 波特率 (SPBRG[15:0])
}

/// UART CSR 位域 (来自 FM33LC0xx 产品说明书 20.11.2)
pub mod uart_csr {
    pub const TXEN:     u32 = 1 << 0;
    pub const RXEN:     u32 = 1 << 1;
    pub const TXPOL:    u32 = 1 << 2;
    pub const RXPOL:    u32 = 1 << 3;
    pub const PARITY_SHIFT: u32 = 4;   // 00=无, 01=偶, 10=奇
    pub const PARITY_MASK:  u32 = 0x03;
    pub const PDSEL_SHIFT:  u32 = 6;   // 00=7bit, 01=8bit, 10=9bit, 11=6bit
    pub const PDSEL_MASK:   u32 = 0x03;
    pub const STOPCFG:  u32 = 1 << 8;
    pub const BITORD:   u32 = 1 << 9;
    pub const DMATXIFCFG: u32 = 1 << 10;
    pub const IOSWAP:   u32 = 1 << 12;
    pub const RXTOEN:   u32 = 1 << 16;
    pub const TXIREN:   u32 = 1 << 17;
    pub const BUSY:     u32 = 1 << 24;
}

pub mod uart_isr {
    pub const TXSE:  u32 = 1 << 0;
    pub const TXBE:  u32 = 1 << 1;
    pub const RXBF:  u32 = 1 << 8;
    pub const RXTO:  u32 = 1 << 11;
    pub const OERR:  u32 = 1 << 16;
    pub const FERR:  u32 = 1 << 17;
    pub const PERR:  u32 = 1 << 18;
}

pub mod uart_ier {
    pub const TXSE_IE:  u32 = 1 << 0;
    pub const TXBE_IE:  u32 = 1 << 1;
    pub const RXBF_IE:  u32 = 1 << 8;
    pub const RXERR_IE: u32 = 1 << 9;
    pub const RXTO_IE:  u32 = 1 << 11;
}

/// 红外调制控制 (独立模块, 不在 UART 结构内!)
#[repr(C)]
pub struct UartirRegs {
    pub cr: u32,  // 0x00: 红外调制控制
}

// ══════════════════════════════════════════════════════════════════
// LPUART — 低功耗异步收发器 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct LpuartRegs {
    pub csr:   u32,    // 0x00: 控制状态
    pub ier:   u32,    // 0x04: 中断使能
    pub isr:   u32,    // 0x08: 中断标志
    pub bmr:   u32,    // 0x0C: 波特率调制
    pub rxbuf: u32,    // 0x10: 接收缓冲
    pub txbuf: u32,    // 0x14: 发送缓冲
    pub dmr:   u32,    // 0x18: 数据匹配
}

// ══════════════════════════════════════════════════════════════════
// SPI — 串行外设接口 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct SpiRegs {
    pub cr1:   u32,    // 0x00: 控制1 (CPHA/CPOL/MM/BAUD...)
    pub cr2:   u32,    // 0x04: 控制2 (SSN/DLEN...)
    pub cr3:   u32,    // 0x08: 控制3 (WO)
    pub ier:   u32,    // 0x0C: 中断使能
    pub isr:   u32,    // 0x10: 中断/状态
    pub txbuf: u32,    // 0x14: 发送缓冲 (WO)
    pub rxbuf: u32,    // 0x18: 接收缓冲 (RO)
}

pub mod spi_cr1 {
    pub const CPHA:       u32 = 1 << 0;
    pub const CPOL:       u32 = 1 << 1;
    pub const LSBF:       u32 = 1 << 2;
    pub const BAUD_SHIFT: u32 = 3;
    pub const BAUD_MASK:  u32 = 0x07;
    pub const WAIT_SHIFT: u32 = 6;
    pub const MM:         u32 = 1 << 8;   // 默认1=主机
    pub const SSPA:       u32 = 1 << 9;
    pub const MSPA:       u32 = 1 << 10;
    pub const IOSWAP:     u32 = 1 << 11;
}

pub mod spi_cr2 {
    pub const SSN:      u32 = 1 << 0;
    pub const SSNSEN:   u32 = 1 << 1;
    pub const TXOEN:    u32 = 1 << 2;
    pub const RXOEN:    u32 = 1 << 3;
    pub const DLEN_SHIFT: u32 = 4;
    pub const DLEN_MASK:  u32 = 0x0F;
}

// ══════════════════════════════════════════════════════════════════
// LCD — 段码LCD控制器 (FM33A0XXEV.h)
// 支持 4COM~8COM, DATA0~DATA9 (10个数据寄存器)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct LcdRegs {
    pub cr:      u32,  // 0x00: 显示控制 (EN/LMUX/WFT/BIAS/FLICK/ENMODE/IC_CTRL)
    pub test:    u32,  // 0x04: 测试控制
    pub fcr:     u32,  // 0x08: 频率控制 (DF[7:0])
    pub flkt:    u32,  // 0x0C: 闪烁时间 (TOFF[15:8] + TON[7:0])
    _reserved1: u32,  // 0x10
    pub ier:     u32,  // 0x14: 中断使能 (DONIE/DOFFIE)
    pub isr:     u32,  // 0x18: 中断标志 (DONIF/DOFFIF)
    _reserved2: [u32; 2], // 0x1C~0x20
    pub data:    [u32; 10], // 0x24~0x48: DATA0~DATA9 显示数据缓存 ★
    _reserved3: u32,  // 0x4C
    pub comen:   u32,  // 0x50: COM使能
    pub segen0:  u32,  // 0x54: SEG使能0
    pub segen1:  u32,  // 0x58: SEG使能1 ★
    pub bstcr:   u32,  // 0x5C: 升压控制 ★
}

pub mod lcd_cr {
    pub const EN:          u32 = 1 << 0;
    pub const LMUX_SHIFT:  u32 = 1;
    pub const LMUX_MASK:   u32 = 0x03;   // 00=4COM, 01=6COM, 10/11=8COM
    pub const WFT:         u32 = 1 << 3;
    pub const ANTIPOLAR:   u32 = 1 << 4;
    pub const BIASMD:      u32 = 1 << 5;
    pub const BIAS_SHIFT:  u32 = 8;
    pub const BIAS_MASK:   u32 = 0x0F;
    pub const FLICK:       u32 = 1 << 14;
    pub const ENMODE:      u32 = 1 << 15;
    pub const IC_CTRL_SHIFT: u32 = 16;
    pub const IC_CTRL_MASK:  u32 = 0x03;
}

// ══════════════════════════════════════════════════════════════════
// IWDT — 独立看门狗 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct IwdtRegs {
    pub serv:  u32,    // 0x00: 喂狗 (WO, 写 0x12345678)
    pub cfgr:  u32,    // 0x04: 配置
    pub cntr:  u32,    // 0x08: 计数值 (RO)
}

// ══════════════════════════════════════════════════════════════════
// WWDT — 窗口看门狗 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct WwdtRegs {
    pub cr:    u32,    // 0x00: 控制 (WO)
    pub cfgr:  u32,    // 0x04: 配置
    pub cntr:  u32,    // 0x08: 计数 (RO)
    pub ier:   u32,    // 0x0C: 中断使能
    pub isr:   u32,    // 0x10: 中断标志
    pub pscr:  u32,    // 0x14: 预分频 (RO)
}

// ══════════════════════════════════════════════════════════════════
// RTC — 实时时钟 (FM33A0XXEV.h)
// BCD 格式，有亚秒、校准、虚拟校准
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct RtcRegs {
    pub wer:     u32,  // 0x00: 写使能
    pub ier:     u32,  // 0x04: 中断使能
    pub isr:     u32,  // 0x08: 中断标志
    pub bcdsec:  u32,  // 0x0C: 秒 (BCD)
    pub bcdmin:  u32,  // 0x10: 分 (BCD)
    pub bcdhour: u32,  // 0x14: 时 (BCD)
    pub bcddate: u32,  // 0x18: 日 (BCD)
    pub bcdweek: u32,  // 0x1C: 星期 (BCD)
    pub bcdmonth:u32,  // 0x20: 月 (BCD)
    pub bcdyear: u32,  // 0x24: 年 (BCD)
    pub alarm:   u32,  // 0x28: 闹钟
    pub tmsel:   u32,  // 0x2C: 时标选择
    pub adjust:  u32,  // 0x30: 校准值
    pub adsign:  u32,  // 0x34: 校准符号
    pub vcal:    u32,  // 0x38: 虚拟校准
    pub mscnt:   u32,  // 0x3C: 毫秒计数
    pub calstep: u32,  // 0x40: 校准步进
    pub adcnt:   u32,  // 0x44: 校准计数 (RO)
    pub ssr:     u32,  // 0x48: 亚秒 (RO)
    pub ssa:     u32,  // 0x4C: 亚秒闹钟
    pub dtr:     u32,  // 0x50: 时标占空比
    _res1:      [u32; 10], // 0x54~0x78
    pub cr:      u32,  // 0x7C: 控制
}

// ══════════════════════════════════════════════════════════════════
// AES — 硬件加密 (FM33A0XXEV.h)
// 支持 AES-128/192/256 + GCM 模式
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct AesRegs {
    pub cr:   u32,     // 0x00: 控制
    pub ier:  u32,     // 0x04: 中断使能
    pub isr:  u32,     // 0x08: 中断标志
    pub dir:  u32,     // 0x0C: 数据输入
    pub dor:  u32,     // 0x10: 数据输出 (RO)
    pub key:  [u32; 8], // 0x14~0x30: KEY0~KEY7 (WO)
    pub iv:   [u32; 4], // 0x34~0x40: IV0~IV3
    pub h:    [u32; 4], // 0x44~0x50: H0~H3 (GCM MultH) ★
}

// ══════════════════════════════════════════════════════════════════
// TRNG — 真随机数 + CRC (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct TrngRegs {
    pub cr:     u32,   // 0x00: 控制
    pub dor:    u32,   // 0x04: 数据输出 (RO)
    _res1:     [u32; 2], // 0x08~0x0C
    pub sr:     u32,   // 0x10: 状态
    pub crc_cr: u32,   // 0x14: CRC控制
    pub crc_dir:u32,   // 0x18: CRC数据输入
    pub crc_sr: u32,   // 0x1C: CRC状态
}

// ══════════════════════════════════════════════════════════════════
// ADC — 模数转换 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct AdcRegs {
    pub cr:   u32,     // 0x00: 控制
    pub trim: u32,     // 0x04: 调校
    pub dr:   u32,     // 0x08: 数据 (RO)
    pub isr:  u32,     // 0x0C: 中断标志
    pub cfgr: u32,     // 0x10: 配置
}

// ══════════════════════════════════════════════════════════════════
// I2C — (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct I2cRegs {
    pub cfgr:   u32,   // 0x00: 配置
    pub cr:     u32,   // 0x04: 控制
    pub ier:    u32,   // 0x08: 中断使能
    pub isr:    u32,   // 0x0C: 中断标志
    pub sr:     u32,   // 0x10: 状态
    pub brg:    u32,   // 0x14: 波特率
    pub buf:    u32,   // 0x18: 数据缓冲
    pub timing: u32,   // 0x1C: 时序
    pub to_reg: u32,   // 0x20: 超时
}

// ══════════════════════════════════════════════════════════════════
// DMA — 12通道 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct DmaRegs {
    pub gcr:    u32,   // 0x00: 全局控制
    pub ch:     [DmaChannel; 12], // 0x04: CH0~CH11 (每通道 8 bytes: CR + MAR)
    pub isr:    u32,   // 0x64+4=0x68: 中断标志 (offset after 12 channels)
    // Shadow registers at 0x100+
}

#[repr(C)]
pub struct DmaChannel {
    pub cr:  u32,     // 通道控制
    pub mar: u32,     // 存储器地址
}

// ══════════════════════════════════════════════════════════════════
// U7816 — ISO7816智能卡接口 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct U7816Regs {
    pub cr:   u32,    // 0x00: 控制
    pub ffr:  u32,    // 0x04: 帧格式
    pub egtr: u32,    // 0x08: 额外保护时间
    pub psc:  u32,    // 0x0C: 预分频
    pub bgr:  u32,    // 0x10: 波特率
    pub rxbuf: u32,   // 0x14: 接收缓冲 (RO)
    pub txbuf: u32,   // 0x18: 发送缓冲 (WO)
    pub ier:  u32,    // 0x1C: 中断使能
    pub isr:  u32,    // 0x20: 中断标志
}

// ══════════════════════════════════════════════════════════════════
// BSTIM — 基本定时器 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct BstimRegs {
    pub cr1:  u32,    // 0x00: 控制1
    pub cr2:  u32,    // 0x04: 控制2
    _res1:   u32,     // 0x08
    pub ier:  u32,    // 0x0C: 中断使能
    pub isr:  u32,    // 0x10: 中断标志
    pub egr:  u32,    // 0x14: 事件生成 (WO)
    _res2:   [u32; 3], // 0x18~0x20
    pub cntr: u32,    // 0x24: 计数器
    pub pscr: u32,    // 0x28: 预分频
    pub arr:  u32,    // 0x2C: 自动重载
}

// ══════════════════════════════════════════════════════════════════
// LPTIM — 低功耗定时器 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct LptimRegs {
    pub cfgr: u32,    // 0x00: 配置
    pub cntr: u32,    // 0x04: 计数器 (RO)
    pub ccsr: u32,    // 0x08: 捕获/比较控制状态
    pub arr:  u32,    // 0x0C: 自动重载
    pub ier:  u32,    // 0x10: 中断使能
    pub isr:  u32,    // 0x14: 中断标志
    pub cr:   u32,    // 0x18: 控制
    _res1:   u32,     // 0x1C
    pub ccr:  [u32; 4], // 0x20~0x2C: 捕获/比较1~4
}

// ══════════════════════════════════════════════════════════════════
// 中断号 (FM33A0XXEV.h)
// ══════════════════════════════════════════════════════════════════

pub mod irqn {
    pub const WWDT:     isize = 0;
    pub const SVD:      isize = 1;
    pub const RTC:      isize = 2;
    pub const FLASH:    isize = 3;
    pub const CMU:      isize = 4;
    pub const ADC:      isize = 5;
    pub const SPI0:     isize = 6;
    pub const SPI1:     isize = 7;
    pub const SPI2:     isize = 8;
    pub const UART0:    isize = 9;
    pub const UART1:    isize = 10;
    pub const UART2:    isize = 11;
    pub const UART3:    isize = 12;
    pub const UART4:    isize = 13;
    pub const UART5:    isize = 14;
    pub const U7816:    isize = 15;
    pub const LPUART0:  isize = 16;
    pub const I2CX:     isize = 17;
    pub const CRYPTO:   isize = 19;  // AES/PAE/HASH/TRNG
    pub const LPTIM:    isize = 20;
    pub const DMA:      isize = 21;
    pub const WKUPX:    isize = 22;
    pub const COMP:     isize = 23;
    pub const BTX:      isize = 24;
    pub const QSPI:     isize = 25;
    pub const ETX:      isize = 26;
    pub const BSTIM:    isize = 27;
    pub const SPI3:     isize = 28;
    pub const SPI4:     isize = 29;
    pub const GPIO:     isize = 30;
    pub const LPUART1:  isize = 31;
}

// ══════════════════════════════════════════════════════════════════
// 寄存器访问
// ══════════════════════════════════════════════════════════════════

macro_rules! reg {
    ($addr:expr, $t:ty) => {
        unsafe { &*($addr as *const $t) }
    };
}

pub fn gpioa()      -> &'static GpioRegs       { reg!(base::GPIOA, GpioRegs) }
pub fn gpiob()      -> &'static GpioRegs       { reg!(base::GPIOB, GpioRegs) }
pub fn gpioc()      -> &'static GpioRegs       { reg!(base::GPIOC, GpioRegs) }
pub fn gpiod()      -> &'static GpioRegs       { reg!(base::GPIOD, GpioRegs) }
pub fn gpio_common()-> &'static GpioCommonRegs { reg!(base::GPIO_COMMON, GpioCommonRegs) }

pub fn uart0()      -> &'static UartRegs       { reg!(base::UART0, UartRegs) }
pub fn uart1()      -> &'static UartRegs       { reg!(base::UART1, UartRegs) }
pub fn uart4()      -> &'static UartRegs       { reg!(base::UART4, UartRegs) }
pub fn uart5()      -> &'static UartRegs       { reg!(base::UART5, UartRegs) }
pub fn uartir()     -> &'static UartirRegs     { reg!(base::UARTIR, UartirRegs) }
pub fn lpuart0()    -> &'static LpuartRegs     { reg!(base::LPUART0, LpuartRegs) }
pub fn lpuart1()    -> &'static LpuartRegs     { reg!(base::LPUART1, LpuartRegs) }

pub fn spi0()       -> &'static SpiRegs        { reg!(base::SPI0, SpiRegs) }
pub fn spi1()       -> &'static SpiRegs        { reg!(base::SPI1, SpiRegs) }
pub fn spi2()       -> &'static SpiRegs        { reg!(base::SPI2, SpiRegs) }

pub fn i2c0()       -> &'static I2cRegs        { reg!(base::I2C0, I2cRegs) }
pub fn lcd()        -> &'static LcdRegs        { reg!(base::LCD, LcdRegs) }
pub fn rtc()        -> &'static RtcRegs        { reg!(base::RTC, RtcRegs) }
pub fn iwdt()       -> &'static IwdtRegs       { reg!(base::IWDT, IwdtRegs) }
pub fn wwdt()       -> &'static WwdtRegs       { reg!(base::WWDT, WwdtRegs) }
pub fn aes()        -> &'static AesRegs        { reg!(base::AES, AesRegs) }
pub fn trng()       -> &'static TrngRegs       { reg!(base::TRNG, TrngRegs) }
pub fn adc()        -> &'static AdcRegs        { reg!(base::ADC, AdcRegs) }
pub fn dma()        -> &'static DmaRegs        { reg!(base::DMA, DmaRegs) }
pub fn cmu()        -> &'static CmuRegs        { reg!(base::CMU, CmuRegs) }
pub fn rmu()        -> &'static RmuRegs        { reg!(base::RMU, RmuRegs) }
pub fn pmu()        -> &'static PmuRegs        { reg!(base::PMU, PmuRegs) }
pub fn bstim()      -> &'static BstimRegs      { reg!(base::BSTIM, BstimRegs) }
pub fn lptim()      -> &'static LptimRegs      { reg!(base::LPTIM, LptimRegs) }

// ══════════════════════════════════════════════════════════════════
// 辅助常量
// ══════════════════════════════════════════════════════════════════

pub const SRAM_SIZE: usize = 24 * 1024;
pub const FLASH_SIZE: usize = 256 * 1024;
pub const SYSCLK_MAX: u32 = 64_000_000;

/// 波特率计算: SPBRG = SYSCLK / BAUDRATE
pub fn calc_spbrg(sysclk: u32, baudrate: u32) -> u16 {
    let div = sysclk / baudrate;
    if div < 0x0010 { 0x0010 } else { div as u16 }
}
