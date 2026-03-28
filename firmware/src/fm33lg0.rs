//! FM33LG0xx / FM33LC0xx 寄存器定义
//!
//! 基于: FM33LC0xx 产品说明书 V2.9.1 (复旦微电子)
//! ARM Cortex-M0+, 最高 64MHz, 256KB Flash, 24KB RAM
//! FM33LG0xx 与 FM33LC0xx 属同一系列, 外设寄存器兼容
//!
//! 地址映射来源: 表 7-1 外设模块总线地址列表

/// 外设基地址 — 来自 FM33LC0xx 产品说明书 表7-1
pub mod base {
    // ── AHB 外设 ──
    pub const GPIO:    usize = 0x4000_0000;  // GPIO 基地址 (各 port 偏移见下方)
    pub const SCU:     usize = 0x4000_0400;  // SCU + PMU + CMU + RMU
    pub const DMA:     usize = 0x4000_0800;  // DMA
    pub const NVMIF:   usize = 0x4000_1000;  // Flash 接口
    pub const USB:     usize = 0x5000_0000;  // USB Controller

    // ── GPIO 各 Port 偏移 (每组 0x40) ──
    // GPIO 基地址 = 0x4000_0C00, 但文档说 GPIO 在 0x4000_0000
    // 实际: GPIOx = GPIO_BASE + port_index * 0x40
    // PA=0x4000_0C00, PB=0x4000_1000? 不对...
    // 从寄存器描述: PA,y=0: 0x00000000+y*0x40 (相对于 GPIO 基地址)
    // 所以 GPIOA = 0x4000_0C00 + 0*0x40 = 0x4000_0C00
    // 但这与地址表冲突... 让我重新核对
    //
    // 地址表: GPIO 在 0x4000_0C00~0x4000_0FFF (1KB)
    // 寄存器: GPIOx_INEN offset = 0x00000000 + y*0x40 (y=PA:0, PB:1, PC:2, PD:3)
    // 所以: GPIOA = 0x4000_0C00, GPIOB = 0x4000_0C40, GPIOC = 0x4000_0C80, GPIOD = 0x4000_0CC0
    //
    // 但是 0x4000_0000~0x4000_03FF 是 SCU/PMU/CMU/RMU (1KB)
    // 而 0x4000_0C00~0x4000_0FFF 是 GPIO (1KB, 包含4个port各64字节)
    //
    // 等等... GPIO 在 0x4000_0C00 但 INEN offset 写的是 "0x00000000 + y*0x40"
    // 这意味着 GPIO 寄存器是绝对地址, 不是相对于某个基?
    // 不, 这应该是相对于 GPIO 基地址的偏移
    // GPIO 基地址可能在别的地方...
    //
    // 实际查看: 文档中 GPIO 寄存器地址格式是 "PA,y=0: 0x00000000 + y*0x40"
    // 这表示 GPIO 基地址 = 0x4000_0C00 (从地址表), 然后:
    // GPIOA = base + 0*0x40 = 0x4000_0C00
    // GPIOB = base + 1*0x40 = 0x4000_0C40
    // GPIOC = base + 2*0x40 = 0x4000_0C80
    // GPIOD = base + 3*0x40 = 0x4000_0CC0

    pub const GPIOA: usize = 0x4000_0C00;
    pub const GPIOB: usize = 0x4000_0C40;
    pub const GPIOC: usize = 0x4000_0C80;
    pub const GPIOD: usize = 0x4000_0CC0;

    // ── APB1 外设 ──
    pub const ISO7816_0: usize = 0x4001_0000;
    pub const LPUART0:  usize = 0x4001_0400;
    pub const SPI2:     usize = 0x4001_0800;
    pub const LCD:      usize = 0x4001_0C00;
    pub const RTC:      usize = 0x4001_1000;
    pub const IWDT:     usize = 0x4001_1400;
    pub const WWDT:     usize = 0x4001_1800;
    pub const UART0:    usize = 0x4001_1C00;
    pub const UART1:    usize = 0x4001_2000;
    pub const I2C:      usize = 0x4001_2400;
    // 0x4001_2800 Reserved
    pub const RAMBIST:  usize = 0x4001_3000;
    // 0x4001_3400 Reserved
    pub const LPTIM32:  usize = 0x4001_3800;
    pub const GTIMER0:  usize = 0x4001_3C00;

    // ── APB2 外设 ──
    pub const GTIMER1:  usize = 0x4001_8000;
    pub const CRC:      usize = 0x4001_8400;
    pub const LPUART1:  usize = 0x4001_8800;
    // 0x4001_8C00 Reserved
    pub const SPI1:     usize = 0x4001_9400;
    // 0x4001_9800~A000 Reserved
    pub const UART4:    usize = 0x4001_A000;
    pub const UART5:    usize = 0x4001_A400;
    pub const SVD_OPA_COMP: usize = 0x4001_A800;
    pub const ADC:      usize = 0x4001_AC00;
    pub const ATIM:     usize = 0x4001_B000;  // Advanced Timer
    pub const BSTIM32:  usize = 0x4001_B400;  // Basic Timer 32-bit
    pub const AES:      usize = 0x4001_B800;
    pub const TRNG:     usize = 0x4001_BC00;

    // ── 存储器 ──
    pub const FLASH_MAIN: usize = 0x0000_0000;  // Flash 起始
    pub const FLASH_OPT:  usize = 0x1FFF_F000;  // Option bytes
    pub const SRAM:       usize = 0x2000_0000;  // SRAM 24KB
}

// ══════════════════════════════════════════════════════════════════
// GPIO — 第34章
// 每组(GPIOA~GPIOD)各占 0x40 字节
// ══════════════════════════════════════════════════════════════════

/// GPIO 寄存器结构 (每组 Port, 偏移间隔 0x40)
/// 来源: 34.8 寄存器
#[repr(C)]
pub struct GpioRegs {
    pub inen:  u32,    // 0x00: 输入使能 (INEN[15:0])
    pub puen:  u32,    // 0x04: 上拉使能 (PUEN[15:0])
    pub oden:  u32,    // 0x08: 开漏使能 (ODEN[15:0])
    pub fcr:   u32,    // 0x0C: 功能选择 (PxxFCR, 每位2bit: 00=输入,01=输出,10=数字功能,11=模拟)
    pub do_reg: u32,   // 0x10: 输出数据 (DO[15:0])
    pub dset:  u32,    // 0x14: 输出置位 (写1置位, WO)
    pub drst:  u32,    // 0x18: 输出复位 (写1复位, WO)
    pub din:   u32,    // 0x1C: 输入数据 (DIN[15:0], RO)
    _reserved: [u32; 6], // 0x20~0x37
    pub atomic_set: u32, // 0x38: 快速GPIO输出置位映射 (alias of DSET)
    pub atomic_clr: u32, // 0x3C: 快速GPIO输出复位映射 (alias of DRST)
}

// ══════════════════════════════════════════════════════════════════
// UART — 第20章 (UART0, UART1, UART4, UART5)
// LPUART — 第21章 (LPUART0, LPUART1)
// ══════════════════════════════════════════════════════════════════

/// UART 寄存器结构 (UART0/1/4/5)
/// 来源: 20.11 寄存器
#[repr(C)]
pub struct UartRegs {
    pub ircr:  u32,    // 0x00: 红外调制配置 (IRCR)
    pub csr:   u32,    // 0x04: 控制状态 (CSR)
    pub ier:   u32,    // 0x08: 中断使能 (IER)
    pub isr:   u32,    // 0x0C: 中断标志 (ISR, 写1清零)
    pub todr:  u32,    // 0x10: 超时和延迟 (TODR: TXDLY_LEN[15:8] + RXTO_LEN[7:0])
    pub rxbuf: u32,    // 0x14: 接收缓冲 (RXBUF[8:0], RO)
    pub txbuf: u32,    // 0x18: 发送缓冲 (TXBUF[8:0], WO)
    pub bgr:   u32,    // 0x1C: 波特率 (SPBRG[15:0])
}

/// UART CSR 位域
pub mod uart_csr {
    pub const TXEN:    u32 = 1 << 0;   // 发送使能
    pub const RXEN:    u32 = 1 << 1;   // 接收使能
    pub const TXPOL:   u32 = 1 << 2;   // 发送极性反转
    pub const RXPOL:   u32 = 1 << 3;   // 接收极性反转
    pub const PARITY_SHIFT: u32 = 4;   // 校验: 00=无, 01=偶, 10=奇
    pub const PARITY_MASK:  u32 = 0x03;
    pub const PDSEL_SHIFT:  u32 = 6;   // 数据长度: 00=7bit, 01=8bit, 10=9bit, 11=6bit
    pub const PDSEL_MASK:   u32 = 0x03;
    pub const STOPCFG:  u32 = 1 << 8;  // 停止位: 0=1位, 1=2位
    pub const BITORD:   u32 = 1 << 9;  // 位序: 0=LSB, 1=MSB
    pub const IOSWAP:   u32 = 1 << 12; // RX/TX 引脚交换
    pub const RXTOEN:   u32 = 1 << 16; // 接收超时使能
    pub const TXIREN:   u32 = 1 << 17; // 红外调制发送使能
    pub const BUSY:     u32 = 1 << 24; // 忙标志 (RO)
}

/// UART ISR 位域
pub mod uart_isr {
    pub const TXSE:    u32 = 1 << 0;   // 发送移位寄存器空
    pub const TXBE:    u32 = 1 << 1;   // 发送缓存空
    pub const RXBF:    u32 = 1 << 8;   // 接收缓存满
    pub const OERR:    u32 = 1 << 16;  // 溢出错误 (写1清零)
    pub const FERR:    u32 = 1 << 17;  // 帧错误 (写1清零)
    pub const PERR:    u32 = 1 << 18;  // 校验错误 (写1清零)
    pub const RXTO:    u32 = 1 << 11;  // 接收超时 (写1清零)
}

/// UART IER 位域
pub mod uart_ier {
    pub const TXSE_IE: u32 = 1 << 0;   // 发送完成中断
    pub const TXBE_IE: u32 = 1 << 1;   // 发送空中断
    pub const RXBF_IE: u32 = 1 << 8;   // 接收满中断
    pub const RXERR_IE: u32 = 1 << 9;  // 接收错误中断
    pub const RXTO_IE: u32 = 1 << 11;  // 接收超时中断
}

/// LPUART 寄存器结构 (LPUART0/1)
/// 来源: 21.7 寄存器 (与 UART 类似但有额外寄存器)
#[repr(C)]
pub struct LpuartRegs {
    pub csr:   u32,    // 0x00: 控制状态
    pub ier:   u32,    // 0x04: 中断使能
    pub isr:   u32,    // 0x08: 中断标志
    pub bmr:   u32,    // 0x0C: 波特率调制
    _reserved: u32,    // 0x10
    pub rxbuf: u32,    // 0x14: 接收缓冲
    pub txbuf: u32,    // 0x18: 发送缓冲
    pub dmr:   u32,    // 0x1C: 数据匹配
}

// ══════════════════════════════════════════════════════════════════
// SPI — 第23章 (SPI1, SPI2)
// ══════════════════════════════════════════════════════════════════

/// SPI 寄存器结构 (SPI1/SPI2)
/// 来源: 23.7 寄存器
#[repr(C)]
pub struct SpiRegs {
    pub cr1:    u32,   // 0x00: 控制寄存器1
    pub cr2:    u32,   // 0x04: 控制寄存器2 (含 SSN 控制)
    pub cr3:    u32,   // 0x08: 控制寄存器3 (DMA, 中断)
    pub ier:    u32,   // 0x0C: 中断使能
    pub isr:    u32,   // 0x10: 中断标志
    pub sr:     u32,   // 0x14: 状态
    pub rxbuf:  u32,   // 0x18: 接收缓冲 (RO)
    pub txbuf:  u32,   // 0x1C: 发送缓冲 (WO)
}

/// SPI CR1 位域
pub mod spi_cr1 {
    pub const CPHA:    u32 = 1 << 0;   // 时钟相位
    pub const CPOL:    u32 = 1 << 1;   // 时钟极性
    pub const LSBF:    u32 = 1 << 2;   // LSB first
    pub const MM:      u32 = 1 << 8;   // Master模式 (默认1=主机)
    pub const SSPA:    u32 = 1 << 9;   // SSN空闲电平
    pub const MSPA:    u32 = 1 << 10;  // SCLK空闲电平
    pub const BAUD_SHIFT: u32 = 3;     // 波特率分频 [5:3]
    pub const BAUD_MASK:  u32 = 0x07;
    pub const WAIT_SHIFT: u32 = 6;     // 等待时间 [7:6]
    pub const IOSWAP: u32 = 1 << 11;   // 引脚交换
}

/// SPI CR2 位域
pub mod spi_cr2 {
    pub const SSN:     u32 = 1 << 0;   // SSN 引脚电平
    pub const SSNSEN:  u32 = 1 << 1;   // SSN 软件控制
    pub const TXOEN:   u32 = 1 << 2;   // 发送输出使能
    pub const RXOEN:   u32 = 1 << 3;   // 接收输出使能
    pub const DLEN_SHIFT: u32 = 4;     // 数据长度 [7:4]
    pub const DLEN_MASK:  u32 = 0x0F;
}

// ══════════════════════════════════════════════════════════════════
// LCD — 第31章
// ══════════════════════════════════════════════════════════════════

/// LCD 控制器寄存器
/// 来源: 31.5 寄存器
/// 最大支持 4COM×44SEG / 6COM×42SEG / 8COM×40SEG
#[repr(C)]
pub struct LcdRegs {
    pub cr:      u32,  // 0x00: 显示控制 (EN, LMUX, WFT, BIAS等)
    pub test:    u32,  // 0x04: 测试控制
    pub fcr:     u32,  // 0x08: 显示频率 (DF[7:0])
    pub flkt:    u32,  // 0x0C: 闪烁时间 (TOFF[15:8] + TON[7:0])
    _reserved1: u32,  // 0x10
    pub ier:     u32,  // 0x14: 中断使能 (DONIE, DOFFIE)
    pub isr:     u32,  // 0x18: 中断标志 (DONIF, DOFFIF)
    _reserved2: [u32; 2], // 0x1C~0x24
    pub data:    [u32; 8], // 0x24~0x40: LCD_DATA0~7 显示数据缓存
    // 注意: 实际偏移是 0x24 + x*0x04
    pub comen:   u32,  // 0x44: COM 使能 (COMEN[7:0])
    pub segen0:  u32,  // 0x48: SEG 使能0 (SEGEN0[31:0])
    // 以下可能还有 segen1 等, 取决于型号
}

/// LCD CR 位域
pub mod lcd_cr {
    pub const EN:        u32 = 1 << 0;   // LCD 使能
    pub const LMUX_SHIFT: u32 = 1;       // COM数: 00=4COM, 01=6COM, 10/11=8COM
    pub const LMUX_MASK:  u32 = 0x03;
    pub const WFT:       u32 = 1 << 3;   // 波形: 0=A类, 1=B类
    pub const ANTIPOLAR: u32 = 1 << 4;   // 防极化
    pub const BIASMD:    u32 = 1 << 5;   // 偏置: 1=1/3bias, 0=1/4bias
    pub const BIAS_SHIFT: u32 = 8;       // 偏置电压 [11:8]
    pub const BIAS_MASK:  u32 = 0x0F;
    pub const FLICK:     u32 = 1 << 14;  // 闪烁使能
    pub const ENMODE:    u32 = 1 << 15;  // 驱动模式: 1=片内电阻型
    pub const IC_CTRL_SHIFT: u32 = 16;   // 偏置电流 [17:16]
    pub const IC_CTRL_MASK:  u32 = 0x03;
}

// ══════════════════════════════════════════════════════════════════
// IWDT — 第9章 独立看门狗
// ══════════════════════════════════════════════════════════════════

/// IWDT 寄存器
/// 来源: 9.6 寄存器
#[repr(C)]
pub struct IwdtRegs {
    pub serv:  u32,    // 0x00: 清除/喂狗寄存器 (写 0x1234_5678 喂狗)
    pub cr:    u32,    // 0x04: 配置寄存器 (WDGEN, WINEN, PRS等)
    pub cnt:   u32,    // 0x08: 计数值 (RO)
    pub win:   u32,    // 0x0C: 窗口值
    pub ier:   u32,    // 0x10: 中断使能
    pub isr:   u32,    // 0x14: 中断标志
}

// ══════════════════════════════════════════════════════════════════
// WWDT — 第10章 窗口看门狗
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct WwdtRegs {
    pub cr:    u32,    // 0x00: 控制寄存器
    pub cfgr:  u32,    // 0x04: 配置寄存器
    pub cnt:   u32,    // 0x08: 计数寄存器
    pub ier:   u32,    // 0x0C: 中断使能
    pub isr:   u32,    // 0x10: 中断标志
    pub psc:   u32,    // 0x14: 预分频
}

// ══════════════════════════════════════════════════════════════════
// RTC — 第28章 实时时钟日历
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct RtcRegs {
    pub time:  u32,    // 0x00: 时间 (BCD)
    pub date:  u32,    // 0x04: 日期 (BCD)
    pub cr:    u32,    // 0x08: 控制
    pub almh:  u32,    // 0x0C: 闹钟高位
    pub alml:  u32,    // 0x10: 闹钟低位
    pub isr:   u32,    // 0x14: 中断标志
    pub ier:   u32,    // 0x18: 中断使能
}

// ══════════════════════════════════════════════════════════════════
// CMU/RCC — 第11章 时钟管理单元
// 寄存器分散在多个偏移
// ══════════════════════════════════════════════════════════════════

/// CMU/RCC 寄存器 (关键字段)
/// 来源: 11.12 寄存器
#[repr(C)]
pub struct CmuRegs {
    pub lkpcr:     u32, // 0x00: LOCKUP复位控制
    pub softrst:   u32, // 0x04: 软件复位
    pub rstfr:     u32, // 0x08: 复位标志
    pub sysclkcr:  u32, // 0x0C: 系统时钟控制
    pub rchfcr:    u32, // 0x10: RCHF控制
    pub rchftr:    u32, // 0x14: RCHF调校
    pub pllcr:     u32, // 0x18: PLL控制
    pub lposccr:   u32, // 0x1C: LPOSC控制
    pub lposctr:   u32, // 0x20: LPOSC调校
    pub xthfcr:    u32, // 0x24: XTHF控制 (未用: reserved, 在后面)
    pub pclken1:   u32, // 0x28: 外设总线时钟使能1 (APB1)
    pub pclken2:   u32, // 0x2C: 外设总线时钟使能2 (APB2)
    pub pclken3:   u32, // 0x30: 外设总线时钟使能3
    pub pclken4:   u32, // 0x34: 外设总线时钟使能4
    pub lsclksel:  u32, // 0x38: LSCLK选择
    pub ahbmcr:    u32, // 0x3C: AHB Master控制
    pub prsten:    u32, // 0x40: 外设复位使能
    pub ahbrstcr:  u32, // 0x44: AHB外设复位
    pub apbrstcr1: u32, // 0x48: APB外设复位1
    pub apbrstcr2: u32, // 0x4C: APB外设复位2
    pub xthfcr2:   u32, // 0x50: XTHF控制
    pub rcmfcr:    u32, // 0x54: RCMF控制
    pub rcmftr:    u32, // 0x58: RCMF调校
}

// ══════════════════════════════════════════════════════════════════
// PMU — 第4章 电源管理单元
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct PmuRegs {
    pub cr:    u32,    // 0x00: 低功耗控制
    pub wktr:  u32,    // 0x04: 唤醒时间控制
    pub wkfr:  u32,    // 0x08: 唤醒源标志
    pub ier:   u32,    // 0x0C: 中断使能
    pub isr:   u32,    // 0x10: 中断标志
}

// ══════════════════════════════════════════════════════════════════
// AES — 硬件加密
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct AesRegs {
    pub cr:    u32,    // 0x00: 控制
    pub ier:   u32,    // 0x04: 中断使能
    pub isr:   u32,    // 0x08: 中断标志
    pub key:   [u32; 8], // 0x0C~0x28: 密钥 (128/192/256bit = 4/6/8 words)
    pub iv:    [u32; 4], // 0x2C~0x38: 初始化向量
    pub din:   [u32; 4], // 0x3C~0x48: 输入数据
    pub dout:  [u32; 4], // 0x4C~0x58: 输出数据
}

// ══════════════════════════════════════════════════════════════════
// ADC — 第27章 模数转换
// ══════════════════════════════════════════════════════════════════

#[repr(C)]
pub struct AdcRegs {
    pub cr:      u32,  // 0x00: 控制
    pub cfgr:    u32,  // 0x04: 配置
    pub ier:     u32,  // 0x08: 中断使能
    pub isr:     u32,  // 0x0C: 中断标志
    pub chselr:  u32,  // 0x10: 通道选择
    pub smpr:    u32,  // 0x14: 采样时间
    pub dr:      u32,  // 0x18: 数据寄存器
    pub calib:   u32,  // 0x1C: 校准
}

// ══════════════════════════════════════════════════════════════════
// 寄存器访问宏与辅助函数
// ══════════════════════════════════════════════════════════════════

macro_rules! reg {
    ($addr:expr, $t:ty) => {
        unsafe { &*($addr as *const $t) }
    };
}

/// GPIOA 寄存器
pub fn gpioa() -> &'static GpioRegs { reg!(base::GPIOA, GpioRegs) }
/// GPIOB 寄存器
pub fn gpiob() -> &'static GpioRegs { reg!(base::GPIOB, GpioRegs) }
/// GPIOC 寄存器
pub fn gpioc() -> &'static GpioRegs { reg!(base::GPIOC, GpioRegs) }
/// GPIOD 寄存器
pub fn gpiod() -> &'static GpioRegs { reg!(base::GPIOD, GpioRegs) }

/// UART0 寄存器 (RS-485)
pub fn uart0() -> &'static UartRegs { reg!(base::UART0, UartRegs) }
/// UART1 寄存器 (红外)
pub fn uart1() -> &'static UartRegs { reg!(base::UART1, UartRegs) }
/// UART4 寄存器
pub fn uart4() -> &'static UartRegs { reg!(base::UART4, UartRegs) }
/// UART5 寄存器
pub fn uart5() -> &'static UartRegs { reg!(base::UART5, UartRegs) }

/// LPUART0 寄存器 (低功耗串口)
pub fn lpuart0() -> &'static LpuartRegs { reg!(base::LPUART0, LpuartRegs) }
/// LPUART1 寄存器
pub fn lpuart1() -> &'static LpuartRegs { reg!(base::LPUART1, LpuartRegs) }

/// SPI1 寄存器 (计量芯片)
pub fn spi1() -> &'static SpiRegs { reg!(base::SPI1, SpiRegs) }
/// SPI2 寄存器
pub fn spi2() -> &'static SpiRegs { reg!(base::SPI2, SpiRegs) }

/// LCD 控制器寄存器
pub fn lcd() -> &'static LcdRegs { reg!(base::LCD, LcdRegs) }
/// RTC 寄存器
pub fn rtc() -> &'static RtcRegs { reg!(base::RTC, RtcRegs) }
/// IWDT 寄存器
pub fn iwdt() -> &'static IwdtRegs { reg!(base::IWDT, IwdtRegs) }
/// WWDT 寄存器
pub fn wwdt() -> &'static WwdtRegs { reg!(base::WWDT, WwdtRegs) }
/// AES 寄存器
pub fn aes() -> &'static AesRegs { reg!(base::AES, AesRegs) }
/// ADC 寄存器
pub fn adc() -> &'static AdcRegs { reg!(base::ADC, AdcRegs) }

// ══════════════════════════════════════════════════════════════════
// 辅助常量
// ══════════════════════════════════════════════════════════════════

/// RAM 大小: 24KB
pub const SRAM_SIZE: usize = 24 * 1024;
/// Flash 大小: 256KB
pub const FLASH_SIZE: usize = 256 * 1024;
/// 系统最高频率
pub const SYSCLK_MAX: u32 = 64_000_000;

/// 波特率计算: SPBRG = SYSCLK / BAUDRATE (近似)
/// 当 SPBRG <= 0x000F 时, UARTDIV = 0x000F
/// 当 SPBRG > 0x000F 时, UARTDIV = SPBRG
/// 实际波特率 = SYSCLK / UARTDIV
pub fn calc_spbrg(sysclk: u32, baudrate: u32) -> u16 {
    let div = sysclk / baudrate;
    if div < 0x0010 {
        0x0010
    } else {
        div as u16
    }
}
