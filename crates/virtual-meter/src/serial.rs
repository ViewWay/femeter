//! 串口服务
//!
//! 提供虚拟串口服务，外部工具可通过串口连接

use crate::MeterHandle;
use anyhow::Result;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::protocol::create_protocol_handler;

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
}

impl SerialService {
    /// 创建串口服务
    pub fn new(meter: MeterHandle) -> Self {
        Self {
            meter,
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
            port_name: None,
        }
    }

    /// 启动串口服务
    pub fn start(&mut self, port_name: &str) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Serial service already running"));
        }

        let port_name = port_name.to_string();
        let meter = self.meter.clone();
        let running = self.running.clone();
        let port_clone = port_name.clone();

        running.store(true, Ordering::Relaxed);

        let handle = thread::spawn(move || {
            if let Err(e) = Self::run_server(&port_clone, meter, running.clone()) {
                eprintln!("Serial server error: {}", e);
            }
        });

        self.handle = Some(handle);
        self.port_name = Some(port_name);
        Ok(())
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

    /// 运行串口服务器
    fn run_server(port_name: &str, meter: MeterHandle, running: Arc<AtomicBool>) -> Result<()> {
        // 在 Unix 系统上尝试打开串口
        // 注意：macOS/Linux 上真正的虚拟串口需要特殊驱动 (如 socat)
        // 这里我们实现一个简化的 TCP 串口桥接

        #[cfg(unix)]
        {
            // 尝试打开真实串口
            if let Ok(port) = serialport::new(port_name, 9600)
                .timeout(Duration::from_millis(100))
                .open()
            {
                println!("Serial port {} opened", port_name);
                Self::handle_serial_port(port, meter, running)?;
                return Ok(());
            }

            // 如果串口打开失败，提示用户使用 socat 创建虚拟串口
            eprintln!("Note: Could not open {}. For virtual serial port, try:", port_name);
            eprintln!("  socat -d -d pty,link=/tmp/vmeter0,raw pty,link=/tmp/vmeter1,raw");
            eprintln!("Then connect to /tmp/vmeter0");
        }

        #[cfg(windows)]
        {
            if let Ok(port) = serialport::new(port_name, 9600)
                .timeout(Duration::from_millis(100))
                .open()
            {
                println!("Serial port {} opened", port_name);
                Self::handle_serial_port(port, meter, running)?;
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "Could not open serial port {}. Use 'serial list' to see available ports.",
            port_name
        ))
    }

    /// 处理串口连接
    fn handle_serial_port(
        mut port: Box<dyn serialport::SerialPort>,
        meter: MeterHandle,
        running: Arc<AtomicBool>,
    ) -> Result<()> {
        let handler = create_protocol_handler(meter);
        let mut line_buf = Vec::new();

        while running.load(Ordering::Relaxed) {
            let mut byte = [0u8; 1];
            match port.read(&mut byte) {
                Ok(1) => {
                    line_buf.push(byte[0]);

                    // 检测行结束 (\n 或 \r\n)
                    if byte[0] == b'\n' {
                        // 移除 \r\n 或 \n
                        if line_buf.ends_with(&[b'\r', b'\n']) {
                            line_buf.pop();
                            line_buf.pop();
                        } else if line_buf.ends_with(&[b'\n']) {
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
                Ok(_) => continue,  // 读取到多个字节或其他情况
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    eprintln!("Serial read error: {}", e);
                    break;
                }
            }
        }

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
