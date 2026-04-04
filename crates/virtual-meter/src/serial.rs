//! 串口服务 - 支持真实串口和虚拟串口(PTY)
//!
//! 提供:
//! - 真实串口连接(/dev/ttyUSB0 等)
//! - 虚拟串口对(PTY master/slave)
//! - HDLC over serial 支持
//! - 波特率配置(9600/19200/38400/115200)

use crate::protocol::create_protocol_handler;
use crate::{dlms::create_dlms_processor, MeterHandle};
use anyhow::{anyhow, Result};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// 串口服务配置
#[derive(Debug, Clone)]
pub struct SerialConfig {
    /// 波特率
    pub baud_rate: u32,
    /// 数据位
    pub data_bits: serialport::DataBits,
    /// 停止位
    pub stop_bits: serialport::StopBits,
    /// 校验位
    pub parity: serialport::Parity,
    /// 流控制
    pub flow_control: serialport::FlowControl,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            baud_rate: 9600,
            data_bits: serialport::DataBits::Eight,
            stop_bits: serialport::StopBits::One,
            parity: serialport::Parity::None,
            flow_control: serialport::FlowControl::None,
        }
    }
}

impl SerialConfig {
    /// 创建 8N1 配置
    pub fn new_8n1(baud_rate: u32) -> Self {
        Self {
            baud_rate,
            ..Default::default()
        }
    }
}

/// 串口服务
pub struct SerialService {
    /// 电表句柄
    meter: MeterHandle,
    /// 运行标志
    running: Arc<AtomicBool>,
    /// 服务线程句柄
    handle: Option<JoinHandle<()>>,
    /// 当前端口名
    port_name: Option<String>,
    /// 配置
    config: SerialConfig,
}
impl SerialService {
    /// 创建串口服务
    pub fn new(meter: MeterHandle) -> Self {
        Self {
            meter,
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
            port_name: None,
            config: SerialConfig::default(),
        }
    }

    /// 设置配置
    pub fn with_config(mut self, config: SerialConfig) -> Self {
        self.config = config;
        self
    }

    /// 启动真实串口服务
    pub fn start(&mut self, port_name: &str) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Err(anyhow!("Serial service already running"));
        }

        let port_name = port_name.to_string();
        let meter = self.meter.clone();
        let running = self.running.clone();
        let config = self.config.clone();

        running.store(true, Ordering::Relaxed);

        let port_name_clone = port_name.clone();
        let handle = thread::spawn(move || {
            if let Err(e) = Self::run_real_serial(&port_name_clone, meter, running.clone(), config)
            {
                eprintln!("Serial server error: {}", e);
            }
        });

        self.handle = Some(handle);
        self.port_name = Some(port_name);
        Ok(())
    }

    /// 启动虚拟串口服务(PTY)
    /// 返回 slave 端设备路径(如 /dev/ttys001)
    #[cfg(unix)]
    pub fn start_virtual(&mut self) -> Result<String> {
        if self.running.load(Ordering::Relaxed) {
            return Err(anyhow!("Serial service already running"));
        }

        let meter = self.meter.clone();
        let running = self.running.clone();
        let config = self.config.clone();

        running.store(true, Ordering::Relaxed);

        // 创建 PTY 对
        let (master, slave_path) = Self::create_pty_pair()?;

        println!("[Serial] Virtual serial port created: {}", slave_path);
        println!(
            "[Serial] Connect with: screen {} {} 8N1",
            slave_path, config.baud_rate
        );
        println!(
            "[Serial] Or: minicom -D {} -b {}",
            slave_path, config.baud_rate
        );

        let handle = thread::spawn(move || {
            if let Err(e) = Self::run_virtual_serial(master, meter, running.clone(), config) {
                eprintln!("Virtual serial server error: {}", e);
            }
        });

        self.handle = Some(handle);
        self.port_name = Some(slave_path.clone());
        Ok(slave_path)
    }

    /// 停止串口服务
    pub fn stop(&mut self) -> Result<()> {
        if !self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        self.port_name = None;
        Ok(())
    }

    /// 检查是否运行中
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// 获取当前端口名
    pub fn port_name(&self) -> Option<&str> {
        self.port_name.as_deref()
    }

    /// 运行真实串口服务器
    fn run_real_serial(
        port_name: &str,
        meter: MeterHandle,
        running: Arc<AtomicBool>,
        config: SerialConfig,
    ) -> Result<()> {
        let port = serialport::new(port_name, config.baud_rate)
            .data_bits(config.data_bits)
            .stop_bits(config.stop_bits)
            .parity(config.parity)
            .flow_control(config.flow_control)
            .timeout(Duration::from_millis(100))
            .open()?;

        println!(
            "[Serial] Port {} opened at {} baud",
            port_name, config.baud_rate
        );
        Self::handle_serial_port(port, meter, running, config)
    }

    /// 处理串口连接（真实串口）
    fn handle_serial_port(
        mut port: Box<dyn serialport::SerialPort>,
        meter: MeterHandle,
        running: Arc<AtomicBool>,
        _config: SerialConfig,
    ) -> Result<()> {
        // 支持两种协议：
        // 1. 简单文本协议（换行分隔）
        // 2. HDLC 帧协议（0x7E 分隔）

        let handler = create_protocol_handler(meter.clone());
        let dlms_processor = create_dlms_processor(meter);

        let mut line_buf = Vec::new();
        let mut hdlc_buf = Vec::new();
        let mut hdlc_mode = false;

        while running.load(Ordering::Relaxed) {
            let mut byte = [0u8; 1];
            match port.read(&mut byte) {
                Ok(1) => {
                    let b = byte[0];

                    // 检测 HDLC 模式(0x7E 标志)
                    if b == 0x7E {
                        hdlc_mode = true;
                        if !hdlc_buf.is_empty() {
                            // 完整帧
                            let mut frame = Vec::with_capacity(hdlc_buf.len() + 2);
                            frame.push(0x7E);
                            frame.extend_from_slice(&hdlc_buf);
                            frame.push(0x7E);

                            match dlms_processor.process_hdlc(&frame) {
                                Ok(resp) => {
                                    if !resp.is_empty() {
                                        port.write_all(&resp)?;
                                        port.flush()?;
                                    }
                                }
                                Err(e) => eprintln!("[Serial HDLC] Error: {}", e),
                            }
                            hdlc_buf.clear();
                        }
                    } else if hdlc_mode {
                        // HDLC 帧数据
                        hdlc_buf.push(b);
                        // 防止缓冲区无限增长
                        if hdlc_buf.len() > 2048 {
                            hdlc_buf.clear();
                        }
                    } else {
                        // 文本协议
                        line_buf.push(b);
                        if b == b'\n' {
                            // 移除 \r\n 或 \n
                            if line_buf.ends_with(b"\r\n") {
                                line_buf.pop();
                                line_buf.pop();
                            } else if line_buf.ends_with(b"\n") {
                                line_buf.pop();
                            }

                            let line = String::from_utf8_lossy(&line_buf);
                            let response = handler.handle_line(&line);
                            let response_bytes = format!("{}\r\n", response);

                            port.write_all(response_bytes.as_bytes())?;
                            port.flush()?;
                            line_buf.clear();
                        }
                    }
                }
                Ok(_) => continue,
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    eprintln!("[Serial] Read error: {}", e);
                    break;
                }
            }
        }

        println!("[Serial] Server stopped");
        Ok(())
    }

    // ============== Unix PTY 支持 ==============

    #[cfg(unix)]
    fn create_pty_pair() -> Result<(std::fs::File, String)> {
        use std::os::fd::{FromRawFd, IntoRawFd};

        // 使用 nix crate 创建 PTY
        let result = nix::pty::openpty(None, None);
        match result {
            Ok(pty_result) => {
                // pty_result.master 和 pty_result.slave 是 OwnedFd
                // 将 OwnedFd 转换为 RawFd 再转换为 File
                let master_raw = pty_result.master.into_raw_fd();
                let master = unsafe { std::fs::File::from_raw_fd(master_raw) };

                // 获取 slave 路径
                let slave_path = {
                    let slave_raw = pty_result.slave.into_raw_fd();
                    // 读取 ttyname - 需要借用 slave_fd
                    let slave_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(slave_raw) };
                    let path_buf = nix::unistd::ttyname(slave_fd)
                        .map_err(|e| anyhow!("ttyname failed: {}", e))?;
                    // slave 需要保持打开 - 转换为 File 并 forget
                    let _slave_file = unsafe { std::fs::File::from_raw_fd(slave_raw) };
                    std::mem::forget(_slave_file);
                    path_buf.to_string_lossy().into_owned()
                };

                Ok((master, slave_path))
            }
            Err(e) => Err(anyhow!("openpty failed: {}", e)),
        }
    }

    #[cfg(unix)]
    fn run_virtual_serial(
        mut master: std::fs::File,
        meter: MeterHandle,
        running: Arc<AtomicBool>,
        config: SerialConfig,
    ) -> Result<()> {
        use std::os::fd::AsFd;

        println!(
            "[Serial PTY] Virtual serial started ({} baud 8N1)",
            config.baud_rate
        );

        let handler = create_protocol_handler(meter.clone());
        let dlms_processor = create_dlms_processor(meter);

        let mut line_buf = Vec::new();
        let mut hdlc_buf = Vec::new();
        let mut hdlc_mode = false;
        let mut escaped = false;

        while running.load(Ordering::Relaxed) {
            let fd = master.as_fd();
            let mut pfd = [nix::poll::PollFd::new(fd, nix::poll::PollFlags::POLLIN)];
            let result = nix::poll::poll(&mut pfd, Some(100u16)); // 100ms timeout

            match result {
                Ok(0) => continue, // timeout
                Ok(_) => {
                    // 有数据可读
                    let mut byte = [0u8; 1];
                    match master.read(&mut byte) {
                        Ok(0) => {
                            println!("[Serial PTY] EOF");
                            break;
                        }
                        Ok(n) => {
                            // 处理读取到的 n 个字节
                            for &b in byte.iter().take(n) {
                                // HDLC 帧处理
                                if b == 0x7E {
                                    hdlc_mode = true;
                                    if !hdlc_buf.is_empty() {
                                        let mut frame = Vec::with_capacity(hdlc_buf.len() + 2);
                                        frame.push(0x7E);
                                        frame.extend_from_slice(&hdlc_buf);
                                        frame.push(0x7E);

                                        match dlms_processor.process_hdlc(&frame) {
                                            Ok(resp) => {
                                                if !resp.is_empty() {
                                                    let _ = master.write(&resp);
                                                }
                                            }
                                            Err(e) => eprintln!("[Serial PTY HDLC] Error: {}", e),
                                        }
                                        hdlc_buf.clear();
                                    }
                                    escaped = false;
                                } else if hdlc_mode {
                                    // 处理转义
                                    if b == 0x7D {
                                        escaped = true;
                                    } else {
                                        let data = if escaped { b ^ 0x20 } else { b };
                                        hdlc_buf.push(data);
                                        escaped = false;

                                        if hdlc_buf.len() > 2048 {
                                            hdlc_buf.clear();
                                        }
                                    }
                                } else {
                                    // 文本协议
                                    line_buf.push(b);
                                    if b == b'\n' {
                                        if line_buf.ends_with(b"\r\n") {
                                            line_buf.pop();
                                            line_buf.pop();
                                        } else if line_buf.ends_with(b"\n") {
                                            line_buf.pop();
                                        }

                                        let line = String::from_utf8_lossy(&line_buf);
                                        let response = handler.handle_line(&line);
                                        let response_bytes = format!("{}\r\n", response);

                                        let _ = master.write(response_bytes.as_bytes());
                                        line_buf.clear();
                                    }
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                        Err(e) => {
                            eprintln!("[Serial PTY] Read error: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Serial PTY] Poll error: {}", e);
                    break;
                }
            }
        }

        println!("[Serial PTY] Server stopped");
        Ok(())
    }
}
impl Drop for SerialService {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// 列出可用串口
pub fn list_ports() -> Vec<String> {
    serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.port_name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_config_default() {
        let config = SerialConfig::default();
        assert_eq!(config.baud_rate, 9600);
        assert_eq!(config.data_bits, serialport::DataBits::Eight);
        assert_eq!(config.stop_bits, serialport::StopBits::One);
        assert_eq!(config.parity, serialport::Parity::None);
    }

    #[test]
    fn test_serial_config_8n1() {
        let config = SerialConfig::new_8n1(115200);
        assert_eq!(config.baud_rate, 115200);
    }

    #[test]
    fn test_list_ports() {
        let ports = list_ports();
        // Should not panic
        println!("Available ports: {:?}", ports);
    }
}
