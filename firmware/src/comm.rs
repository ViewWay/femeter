//! 多通道通信驱动 — FM33LG0xx
//!
//! UART0: RS-485 (HDLC/DLMS), 9600-115200 bps, 8N1
//! UART1: 红外串口 (IEC 62056-21), 300-9600 bps, 8N1
//! UART2: 模块通信, 38400 bps, 8N1
//!
//! RS-485 收发控制: GPIO 控制 DE/RE 引脚
//! 红外: GPIO 控制 IR 发射/接收使能

/// UART 通道
#[derive(Clone, Copy, PartialEq)]
pub enum UartChannel {
    /// RS-485: UART0, HDLC/DLMS 协议
    Rs485,
    /// 红外: UART1, IEC 62056-21 协议
    Infrared,
    /// 模块: UART2, 38400 bps
    Module,
}

/// 通信驱动
pub struct CommDriver {
    /// RS-485 HDLC 接收缓冲区
    rs485_rx_buf: [u8; 256],
    rs485_rx_len: usize,
    rs485_tx_pending: bool,

    /// 红外接收缓冲区
    ir_rx_buf: [u8; 64],
    ir_rx_len: usize,

    /// 模块通信接收缓冲区
    module_rx_buf: [u8; 128],
    module_rx_len: usize,

    /// HDLC 帧分隔符计数 (检测连续 0x7E)
    flag_count: u8,
}

impl CommDriver {
    pub fn new() -> Self {
        Self {
            rs485_rx_buf: [0u8; 256],
            rs485_rx_len: 0,
            rs485_tx_pending: false,
            ir_rx_buf: [0u8; 64],
            ir_rx_len: 0,
            module_rx_buf: [0u8; 128],
            module_rx_len: 0,
            flag_count: 0,
        }
    }

    /// 初始化所有 UART 外设
    pub fn init_hw(&self) {
        // UART0 (RS-485): 9600 bps, 8N1
        // UART1 (红外):   2400 bps, 8N1
        // UART2 (模块):   38400 bps, 8N1
        //
        // 实际配置:
        // let uart0 = crate::fm33lg0::uart0();
        // BRR = SystemClock / BaudRate
        // 64MHz / 9600 = 6666.67 → BRR=6667
        // 64MHz / 2400 = 26666.67 → BRR=26667
        // 64MHz / 38400 = 1666.67 → BRR=1667
    }

    /// 设置 RS-485 波特率 (支持动态切换)
    pub fn set_rs485_baud(&self, baud: u32) {
        let _ = baud;
        // uart0.brr = 64_000_000 / baud;
    }

    /// 设置红外波特率
    pub fn set_ir_baud(&self, baud: u32) {
        let _ = baud;
    }

    /// RS-485 接收字节 (UART0 中断调用)
    /// 返回 true 表示收到完整 HDLC 帧
    pub fn rs485_feed(&mut self, byte: u8) -> bool {
        if byte == 0x7E {
            self.flag_count += 1;
            if self.flag_count >= 2 && self.rs485_rx_len > 1 {
                // 完整帧: flag + data + flag
                self.flag_count = 0;
                return true;
            }
        } else {
            self.flag_count = 0;
        }

        if self.rs485_rx_len < self.rs485_rx_buf.len() {
            self.rs485_rx_buf[self.rs485_rx_len] = byte;
            self.rs485_rx_len += 1;
        }
        false
    }

    /// 获取 RS-485 接收到的 HDLC 帧
    pub fn rs485_frame(&self) -> &[u8] {
        &self.rs485_rx_buf[..self.rs485_rx_len]
    }

    /// 清空 RS-485 接收缓冲区
    pub fn rs485_reset(&mut self) {
        self.rs485_rx_len = 0;
        self.flag_count = 0;
    }

    /// RS-485 发送 HDLC 帧
    pub fn rs485_send(&mut self, frame: &[u8]) {
        // 1. GPIO DE=HIGH (RS-485 发送模式)
        // 2. 逐字节发送 (或 DMA)
        // 3. 等待发送完成
        // 4. GPIO DE=LOW (回到接收模式)
        self.rs485_tx_pending = true;
        let _ = frame;
    }

    /// 红外接收字节
    pub fn ir_feed(&mut self, byte: u8) -> bool {
        if self.ir_rx_len < self.ir_rx_buf.len() {
            self.ir_rx_buf[self.ir_rx_len] = byte;
            self.ir_rx_len += 1;
        }
        // IEC 62056-21: 以 <CR><LF> 结束
        byte == b'\n' && self.ir_rx_len > 2
    }

    pub fn ir_frame(&self) -> &[u8] {
        &self.ir_rx_buf[..self.ir_rx_len]
    }

    pub fn ir_reset(&mut self) {
        self.ir_rx_len = 0;
    }

    /// 模块 UART 接收字节
    pub fn module_feed(&mut self, byte: u8) -> bool {
        if self.module_rx_len < self.module_rx_buf.len() {
            self.module_rx_buf[self.module_rx_len] = byte;
            self.module_rx_len += 1;
        }
        self.module_rx_len >= self.module_rx_buf.len()
    }

    pub fn module_data(&self) -> &[u8] {
        &self.module_rx_buf[..self.module_rx_len]
    }

    pub fn module_reset(&mut self) {
        self.module_rx_len = 0;
    }
}
