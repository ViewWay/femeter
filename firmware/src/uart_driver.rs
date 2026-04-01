/* ================================================================== */
/*                                                                    */
/*  uart_driver.rs — 工业级 UART 中断驱动收发模块                      */
/*                                                                    */
/*  特性:                                                             */
/*    - 环形缓冲区中断接收 (heapless::Deque, 256字节)                  */
/*    - 查询式发送 (M0+ 无 DMA)                                       */
/*    - RS485 方向控制 (CON 引脚)                                      */
/*    - 支持 UART0~UART3 四个实例                                       */
/*    - 波特率: 9600/19200/38400/57600/115200                         */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use core::sync::atomic::{AtomicBool, Ordering};

use crate::fm33lg0::{
    base, calc_spbrg,
    uart_csr, uart_ier, uart_isr,
    UartRegs,
};
use crate::hal::{Parity, UartChannel, UartConfig, UartError};

// ── 环形缓冲区容量 ──
const RX_BUF_SIZE: usize = 256;

/// 系统时钟频率 (PLL 初始化后)
const SYSCLK: u32 = 64_000_000;

/// RS485 收发方向切换延时 (μs), 等待电平稳定
const RS485_DIR_DELAY_US: u32 = 50;

/* ================================================================== */
/*  安全注释:                                                          */
/*  - 所有寄存器访问通过 UART 寄存器基地址进行,                         */
/*    基地址来自芯片手册, 在单核 Cortex-M0+ 上安全                     */
/*  - 中断处理函数中的缓冲区操作使用 AtomicBool 做互斥                  */
/*  - RS485 CON 引脚操作通过 board.rs 提供的 GPIO 抽象                  */
/* ================================================================== */

/* ================================================================== */
/*  中断接收缓冲区 (静态, ISR 访问)                                     */
/* ================================================================== */

/// 各 UART 通道的接收缓冲区 (ISR 写入, 主循环读取)
static mut RX_BUF_0: heapless::Deque<u8, RX_BUF_SIZE> = heapless::Deque::new();
static mut RX_BUF_1: heapless::Deque<u8, RX_BUF_SIZE> = heapless::Deque::new();
static mut RX_BUF_2: heapless::Deque<u8, RX_BUF_SIZE> = heapless::Deque::new();
static mut RX_BUF_3: heapless::Deque<u8, RX_BUF_SIZE> = heapless::Deque::new();

/// 发送忙标志 (防止 ISR 中重入), 各通道独立
static TX_BUSY: [AtomicBool; 4] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];

/* ================================================================== */
/*  UartHal Trait                                                      */
/* ================================================================== */

/// UART 硬件抽象接口
pub trait UartHal {
    /// 初始化 UART (波特率、帧格式、中断使能)
    fn init(&mut self, config: &UartConfig) -> Result<(), ()>;

    /// 批量写入数据 (查询式发送, 阻塞直到完成)
    fn write(&mut self, data: &[u8]) -> Result<usize, ()>;

    /// 从接收缓冲区读取数据 (非阻塞)
    fn read(&mut self, buf: &mut [u8]) -> usize;

    /// 写入单个字节 (查询式发送)
    fn write_byte(&mut self, byte: u8);

    /// 从接收缓冲区读取单个字节 (非阻塞)
    fn read_byte(&mut self) -> Option<u8>;

    /// 等待发送完成 (TX 移位寄存器空)
    fn flush(&mut self);
}

/* ================================================================== */
/*  UartInstance — 单个 UART 实例                                       */
/* ================================================================== */

/// UART 实例, 封装寄存器、缓冲区和 RS485 控制
pub struct UartInstance {
    /// 寄存器基址 (来自 base::UARTx)
    regs: *const UartRegs,
    /// 通道编号
    channel: UartChannel,
    /// 通道索引 (0~3), 用于选择静态缓冲区
    index: usize,
    /// NVIC 中断号 (来自 fm33lg0::irqn)
    irqn: i32,
    /// RS485 CON 引脚 (None 表示不使用 RS485 方向控制)
    rs485_con: Option<(u8, u8)>, // (port, pin)
    /// 是否已初始化
    initialized: bool,
}

// Safety: UartInstance 只在单核 Cortex-M0+ 上使用,
// 寄存器基地址是固定的硬件映射地址
unsafe impl Send for UartInstance {}

impl UartInstance {
    /// 创建 UART 实例
    ///
    /// # 参数
    /// - `regs`: UART 寄存器基地址
    /// - `channel`: UART 通道编号
    /// - `irqn`: NVIC 中断号
    /// - `rs485_con`: RS485 方向控制引脚 (port, pin), None 表示无
    #[inline]
    pub const fn new(
        regs: *const UartRegs,
        channel: UartChannel,
        index: usize,
        irqn: i32,
        rs485_con: Option<(u8, u8)>,
    ) -> Self {
        Self {
            regs,
            channel,
            index,
            irqn,
            rs485_con,
            initialized: false,
        }
    }

    /// 获取寄存器引用
    ///
    /// Safety: 仅在已初始化后调用, 寄存器地址来自芯片手册
    #[inline]
    unsafe fn regs(&self) -> &UartRegs {
        // Safety: regs 指向有效的 UART 外设寄存器映射地址
        &*self.regs
    }

    /// 获取接收缓冲区原始指针
    ///
    /// Safety: 仅在临界区内调用, 或确保与 ISR 互斥
    #[inline]
    unsafe fn rx_buf(&self) -> *mut heapless::Deque<u8, RX_BUF_SIZE> {
        match self.index {
            0 => &raw mut RX_BUF_0,
            1 => &raw mut RX_BUF_1,
            2 => &raw mut RX_BUF_2,
            3 => &raw mut RX_BUF_3,
            _ => &raw mut RX_BUF_0,
        }
    }

    /// 设置 RS485 发送方向 (拉高 CON 引脚)
    #[inline]
    fn rs485_tx_enable(&self) {
        if let Some((port, pin)) = self.rs485_con {
            board_gpio_set_high(port, pin);
            // 等待电平稳定
            delay_us(RS485_DIR_DELAY_US);
        }
    }

    /// 设置 RS485 接收方向 (拉低 CON 引脚)
    #[inline]
    fn rs485_tx_disable(&self) {
        if let Some((port, pin)) = self.rs485_con {
            board_gpio_set_low(port, pin);
        }
    }

    /// 等待 TX 移位寄存器空闲 (BUSY 位清零)
    #[inline]
    fn wait_tx_idle(&self) {
        // Safety: 寄存器地址有效, BUSY 位为只读状态位
        unsafe {
            let regs = self.regs();
            while regs.csr & uart_csr::BUSY != 0 {
                cortex_m::asm::nop();
            }
        }
    }

    /// 等待发送缓冲区为空 (可写入下一字节)
    #[inline]
    fn wait_txbe(&self) {
        // Safety: 寄存器地址有效
        unsafe {
            let regs = self.regs();
            // 等待 ISR 的 TXBE (发送缓冲区空) 标志
            while regs.isr & uart_isr::TXBE == 0 {
                cortex_m::asm::nop();
            }
        }
    }
}

impl UartHal for UartInstance {
    fn init(&mut self, config: &UartConfig) -> Result<(), ()> {
        // Safety: 寄存器基地址来自芯片手册, 在初始化阶段安全访问
        unsafe {
            let regs = self.regs();

            // 1. 禁用 TX/RX
            cortex_m::interrupt::free(|_| {
                let csr = regs.csr & !(uart_csr::TXEN | uart_csr::RXEN);
                // Safety: 写入 UART CSR 控制寄存器
                core::ptr::write_volatile(
                    &regs.csr as *const u32 as *mut u32,
                    csr,
                );
            });

            // 2. 配置波特率
            let spbrg = calc_spbrg(SYSCLK, config.baudrate);
            // Safety: 写入波特率分频寄存器
            core::ptr::write_volatile(
                &regs.bgr as *const u32 as *mut u32,
                spbrg as u32,
            );

            // 3. 配置帧格式 (CSR)
            let mut csr: u32 = 0;

            // 数据位: PDSEL[7:6] — 00=7bit, 01=8bit, 10=9bit, 11=6bit
            match config.data_bits {
                7 => {}
                8 => csr |= 0x01 << uart_csr::PDSEL_SHIFT,
                9 => csr |= 0x02 << uart_csr::PDSEL_SHIFT,
                _ => csr |= 0x01 << uart_csr::PDSEL_SHIFT, // 默认 8-bit
            }

            // 校验位: PARITY[5:4] — 00=无, 01=偶, 10=奇
            match config.parity {
                Parity::Even => csr |= 0x01 << uart_csr::PARITY_SHIFT,
                Parity::Odd => csr |= 0x02 << uart_csr::PARITY_SHIFT,
                Parity::None => {}
            }

            // 停止位: STOPCFG bit8 — 0=1bit, 1=2bit
            if config.stop_bits == 2 {
                csr |= uart_csr::STOPCFG;
            }

            // 使能 TX + RX
            csr |= uart_csr::TXEN | uart_csr::RXEN;

            // Safety: 写入 CSR 配置帧格式并使能收发
            core::ptr::write_volatile(
                &regs.csr as *const u32 as *mut u32,
                csr,
            );

            // 4. 清除所有中断标志 (写1清零)
            // Safety: ISR 寄存器写1清零对应标志位
            core::ptr::write_volatile(
                &regs.isr as *const u32 as *mut u32,
                0xFFFFFFFF,
            );

            // 5. 使能接收缓冲区满中断 (RXBF) 和接收错误中断 (RXERR)
            let ier = uart_ier::RXBF_IE | uart_ier::RXERR_IE;
            // Safety: 写入中断使能寄存器
            core::ptr::write_volatile(
                &regs.ier as *const u32 as *mut u32,
                ier,
            );

            // 6. 使能 NVIC 中断 (使用 cortex_m::peripheral::NVIC::unmask)
            // 注意: cortex_m::NVIC::unmask 需要 InterruptNumber trait 实现
            // 对于自定义芯片, 直接使用 cortex-m 的 NVIC 函数
            let nvic = cortex_m::peripheral::NVIC::PTR;
            unsafe {
                (*nvic).iser[0].write(1 << self.irqn);
            }
        }

        // 7. 如果有 RS485 方向控制引脚, 初始化为接收模式
        self.rs485_tx_disable();

        self.initialized = true;
        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, ()> {
        if !self.initialized {
            return Err(());
        }

        // 检查发送忙标志
        if TX_BUSY[self.index].load(Ordering::Acquire) {
            return Err(());
        }
        TX_BUSY[self.index].store(true, Ordering::Release);

        // RS485: 切换到发送方向
        self.rs485_tx_enable();

        // Safety: 寄存器访问在初始化后是安全的
        unsafe {
            let regs = self.regs();

            for &byte in data {
                // 等待发送缓冲区空
                while regs.isr & uart_isr::TXBE == 0 {
                    cortex_m::asm::nop();
                }
                // Safety: 写入发送数据寄存器
                core::ptr::write_volatile(
                    &regs.txbuf as *const u32 as *mut u32,
                    byte as u32,
                );
            }
        }

        // 等待所有数据发送完成
        self.flush();

        // RS485: 切换回接收方向
        self.rs485_tx_disable();

        TX_BUSY[self.index].store(false, Ordering::Release);
        Ok(data.len())
    }

    fn read(&mut self, buf: &mut [u8]) -> usize {
        if !self.initialized {
            return 0;
        }

        let mut count = 0;
        // 在临界区内访问缓冲区, 防止与 ISR 竞争
        cortex_m::interrupt::free(|_| {
            // Safety: 在临界区内访问静态缓冲区, 与 ISR 互斥
            let rx_buf = unsafe { self.rx_buf() };
            for slot in buf.iter_mut() {
                // Safety: rx_buf 是有效指针, 在临界区内独占访问
                if let Some(byte) = unsafe { (*rx_buf).pop_front() } {
                    *slot = byte;
                    count += 1;
                } else {
                    break;
                }
            }
        });
        count
    }

    fn write_byte(&mut self, byte: u8) {
        if !self.initialized {
            return;
        }

        // Safety: 寄存器访问在初始化后安全
        unsafe {
            let regs = self.regs();

            // 等待发送缓冲区空
            while regs.isr & uart_isr::TXBE == 0 {
                cortex_m::asm::nop();
            }
            // Safety: 写入发送数据寄存器
            core::ptr::write_volatile(
                &regs.txbuf as *const u32 as *mut u32,
                byte as u32,
            );
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if !self.initialized {
            return None;
        }

        cortex_m::interrupt::free(|_| {
            // Safety: 在临界区内访问静态缓冲区
            let rx_buf = unsafe { self.rx_buf() };
            // Safety: rx_buf 是有效指针, 在临界区内独占访问
            unsafe { (*rx_buf).pop_front() }
        })
    }

    fn flush(&mut self) {
        // 等待 TX 移位寄存器发送完毕
        self.wait_tx_idle();
        // 额外等待一个字节时间确保最后一位也发送出去了
        // (粗略延时: 115200bps 下约 87μs, 取 100μs 保底)
        delay_us(100);
    }
}

/* ================================================================== */
/*  预定义 UART 实例 (UART0~UART3)                                      */
/* ================================================================== */

/// UART0 → RS485 (DLMS/COSEM), CON 引脚由板级定义
pub static mut UART0: UartInstance = UartInstance::new(
    base::UART0 as *const UartRegs,
    UartChannel::Uart0,
    0,
    9i32, // fm33lg0::irqn::UART0 = 9
    Some((5, 2)), // RS485 CON: PF2 (board.rs: RS485_DE)
);

/// UART1 → 红外 (IEC 62056-21), 无 RS485 方向控制
pub static mut UART1: UartInstance = UartInstance::new(
    base::UART1 as *const UartRegs,
    UartChannel::Uart1,
    1,
    10i32, // fm33lg0::irqn::UART1 = 10
    None, // 红外无需方向控制
);

/// UART2 → LoRaWAN (ASR6601)
pub static mut UART2: UartInstance = UartInstance::new(
    base::UART2 as *const UartRegs,
    UartChannel::Uart2,
    2,
    11i32, // fm33lg0::irqn::UART2 = 11
    None,
);

/// UART3 → 蜂窝模组 (EC800N/BC260Y)
pub static mut UART3: UartInstance = UartInstance::new(
    base::UART3 as *const UartRegs,
    UartChannel::Uart3,
    3,
    12i32, // fm33lg0::irqn::UART3 = 12
    None,
);

/* ================================================================== */
/*  NVIC 中断处理函数                                                   */
/* ================================================================== */

/// UART0 中断服务程序 — 从 RXBUF 读取字节放入接收缓冲区
///
/// # Safety
/// 这是中断上下文函数, 由硬件自动调用. 必须确保 UART0 寄存器已初始化.
#[no_mangle]
unsafe extern "C" fn UART0_IRQHandler() {
    uart_isr_handler(0, base::UART0 as *const UartRegs);
}

/// UART1 中断服务程序
#[no_mangle]
unsafe extern "C" fn UART1_IRQHandler() {
    uart_isr_handler(1, base::UART1 as *const UartRegs);
}

/// UART2 中断服务程序
#[no_mangle]
unsafe extern "C" fn UART2_IRQHandler() {
    uart_isr_handler(2, base::UART2 as *const UartRegs);
}

/// UART3 中断服务程序
#[no_mangle]
unsafe extern "C" fn UART3_IRQHandler() {
    uart_isr_handler(3, base::UART3 as *const UartRegs);
}

/// 通用 UART ISR 处理逻辑
///
/// 在 ISR 上下文中执行, 注意:
/// - 不使用临界区 (ISR 本身就是最高优先级)
/// - 只做最少的工作: 读取数据 + 清除中断标志
#[inline(always)]
fn uart_isr_handler(index: usize, regs_ptr: *const UartRegs) {
    // Safety: 寄存器地址有效, 在 ISR 中访问
    let regs = unsafe { &*regs_ptr };
    let isr_status = regs.isr;

    // 接收缓冲区满中断 (有数据可读)
    if isr_status & uart_isr::RXBF != 0 {
        // Safety: 在 ISR 中独占访问缓冲区 (主循环通过临界区访问)
        // 使用 &raw mut 避免 static_mut_refs 警告
        let buf: *mut heapless::Deque<u8, RX_BUF_SIZE> = unsafe {
            match index {
                0 => &raw mut RX_BUF_0,
                1 => &raw mut RX_BUF_1,
                2 => &raw mut RX_BUF_2,
                3 => &raw mut RX_BUF_3,
                _ => &raw mut RX_BUF_0,
            }
        };

        // 循环读取直到 RXBF 清零
        loop {
            // 检查是否还有数据
            if regs.isr & uart_isr::RXBF == 0 {
                break;
            }
            // 读取接收数据
            // Safety: RXBUF 是只读寄存器, 读操作安全
            let data = regs.rxbuf as u8;
            // 尝试放入缓冲区, 满则丢弃 (溢出保护)
            // Safety: buf 是有效指针, ISR 上下文中独占访问
            if unsafe { (*buf).push_back(data) }.is_err() {
                // 缓冲区溢出, 丢弃数据
                // 可在此处增加溢出计数器用于调试
            }
        }

        // 清除 RXBF 中断标志 (写1清零)
        // Safety: ISR 写1清零对应标志位
        unsafe {
            core::ptr::write_volatile(
                &regs.isr as *const u32 as *mut u32,
                uart_isr::RXBF,
            );
        }
    }

    // 接收错误处理 (帧错误/校验错误/溢出错误)
    if isr_status & (uart_isr::FERR | uart_isr::PERR | uart_isr::OERR) != 0 {
        // 清除所有错误标志 (写1清零)
        // Safety: 写1清零错误标志位
        unsafe {
            core::ptr::write_volatile(
                &regs.isr as *const u32 as *mut u32,
                uart_isr::FERR | uart_isr::PERR | uart_isr::OERR,
            );
        }
        // 注意: 如果有 OERR, 需要连续读 RXBUF 两次来清除硬件状态
        if isr_status & uart_isr::OERR != 0 {
            let _ = regs.rxbuf;
            let _ = regs.rxbuf;
        }
    }
}

/* ================================================================== */
/*  辅助函数                                                           */
/* ================================================================== */

/// 粗略微秒延时 (SYSCLK=64MHz, 每次循环约 4 cycles)
#[inline(always)]
fn delay_us(us: u32) {
    let loops = us * 16;
    let mut i = 0u32;
    while i < loops {
        cortex_m::asm::nop();
        i += 1;
    }
}

/// 通过 board.rs 的 GPIO 抽象设置引脚高电平
///
/// 使用直接寄存器操作避免依赖 board.rs 的 GpioDriver trait (减少耦合)
#[inline]
fn board_gpio_set_high(port: u8, pin: u8) {
    // Safety: GPIO 寄存器基地址来自芯片手册, 在单核上安全
    unsafe {
        let gpio = gpio_port(port);
        // Safety: DSET 寄存器写1置位对应引脚
        let addr = &gpio.dset as *const u32 as *mut u32;
        core::ptr::write_volatile(addr, 1u32 << pin);
    }
}

/// 设置引脚低电平
#[inline]
fn board_gpio_set_low(port: u8, pin: u8) {
    // Safety: 同上
    unsafe {
        let gpio = gpio_port(port);
        // Safety: DRST 寄存器写1复位对应引脚
        let addr = &gpio.drst as *const u32 as *mut u32;
        core::ptr::write_volatile(addr, 1u32 << pin);
    }
}

/// 获取 GPIO 端口寄存器块
///
/// Safety: GPIO 寄存器基地址来自芯片手册
#[inline]
unsafe fn gpio_port(port: u8) -> &'static crate::fm33lg0::GpioRegs {
    use crate::fm33lg0::base;
    let addr = match port {
        0 => base::GPIOA,
        1 => base::GPIOB,
        2 => base::GPIOC,
        3 => base::GPIOD,
        _ => base::GPIOA,
    };
    // Safety: 地址是有效的 GPIO 外设映射地址
    &*(addr as *const crate::fm33lg0::GpioRegs)
}
