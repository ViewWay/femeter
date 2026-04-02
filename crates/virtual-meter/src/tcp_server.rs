//! TCP 服务器 (增强版)
//!
//! - DLMS over TCP (wrapping HDLC), port 4059
//! - IEC 62056-21 over TCP, port 4059
//! - 简单文本协议, port 8888
//! - 多客户端连接
//! - 心跳/超时处理

use crate::{create_protocol_handler, MeterHandle, ProtocolHandler};
use anyhow::Result;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// 最大同时连接数
const MAX_CONNECTIONS: usize = 16;

/// 客户端超时时间 (秒)
const CLIENT_TIMEOUT_SECS: u64 = 300;

/// 心跳间隔 (秒)
#[allow(dead_code)]
const HEARTBEAT_INTERVAL_SECS: u64 = 60;
#[allow(dead_code)]

pub struct TcpServer {
    meter: MeterHandle,
    running: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
    /// 当前活跃连接数
    active_connections: Arc<AtomicUsize>,
}

impl TcpServer {
    pub fn new(meter: MeterHandle) -> Self {
        Self {
            meter,
            running: Arc::new(AtomicBool::new(false)),
            handles: Vec::new(),
            active_connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 启动文本协议服务器
    pub fn start_text(&mut self, port: u16) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        let meter = self.meter.clone();
        let running = self.running.clone();
        let conn_count = self.active_connections.clone();
        self.running.store(true, Ordering::Relaxed);

        let h = thread::spawn(move || {
            listener.set_nonblocking(false).ok();
            while running.load(Ordering::Relaxed) {
                if let Ok((stream, addr)) = listener.accept() {
                    let current = conn_count.load(Ordering::Relaxed);
                    if current >= MAX_CONNECTIONS {
                        eprintln!("[TCP Text] Rejected {}: max connections reached", addr);
                        continue;
                    }
                    println!(
                        "[TCP Text] Connection from {} ({} active)",
                        addr,
                        current + 1
                    );
                    conn_count.fetch_add(1, Ordering::Relaxed);

                    let handler = create_protocol_handler(meter.clone());
                    let cc = conn_count.clone();
                    thread::spawn(move || {
                        handle_text_client(stream, handler);
                        cc.fetch_sub(1, Ordering::Relaxed);
                        println!(
                            "[TCP Text] Disconnected ({} active)",
                            cc.load(Ordering::Relaxed)
                        );
                    });
                }
            }
        });
        self.handles.push(h);
        println!(
            "[TCP] Text protocol server on port {} (max {} connections)",
            port, MAX_CONNECTIONS
        );
        Ok(())
    }

    /// 启动 DLMS over TCP 服务器 (wrapping HDLC)
    pub fn start_dlms(&mut self, port: u16) -> Result<()> {
        use crate::dlms::create_dlms_processor;
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        let meter = self.meter.clone();
        let running = self.running.clone();
        let conn_count = self.active_connections.clone();
        self.running.store(true, Ordering::Relaxed);

        let h = thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                if let Ok((stream, addr)) = listener.accept() {
                    let current = conn_count.load(Ordering::Relaxed);
                    if current >= MAX_CONNECTIONS {
                        eprintln!("[TCP DLMS] Rejected {}: max connections", addr);
                        continue;
                    }
                    println!(
                        "[TCP DLMS] Connection from {} ({} active)",
                        addr,
                        current + 1
                    );
                    conn_count.fetch_add(1, Ordering::Relaxed);

                    let processor = create_dlms_processor(meter.clone());
                    let cc = conn_count.clone();
                    thread::spawn(move || {
                        handle_dlms_client(stream, processor);
                        cc.fetch_sub(1, Ordering::Relaxed);
                        println!(
                            "[TCP DLMS] Disconnected ({} active)",
                            cc.load(Ordering::Relaxed)
                        );
                    });
                }
            }
        });
        self.handles.push(h);
        println!(
            "[TCP] DLMS server on port {} (HDLC wrapping, max {} connections)",
            port, MAX_CONNECTIONS
        );
        Ok(())
    }

    /// 启动 IEC 62056-21 over TCP 服务器
    #[allow(dead_code)]
    pub fn start_iec(&mut self, port: u16) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        let running = self.running.clone();
        let conn_count = self.active_connections.clone();
        self.running.store(true, Ordering::Relaxed);

        let h = thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                if let Ok((stream, addr)) = listener.accept() {
                    let current = conn_count.load(Ordering::Relaxed);
                    if current >= MAX_CONNECTIONS {
                        eprintln!("[TCP IEC] Rejected {}", addr);
                        continue;
                    }
                    println!("[TCP IEC] Connection from {}", addr);
                    conn_count.fetch_add(1, Ordering::Relaxed);

                    let cc = conn_count.clone();
                    thread::spawn(move || {
                        handle_iec_client(stream);
                        cc.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            }
        });
        self.handles.push(h);
        println!("[TCP] IEC 62056-21 server on port {}", port);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // Connect to self to unblock accept()
        let _ = std::net::TcpStream::connect_timeout(
            &"127.0.0.1:8888".parse().unwrap(),
            Duration::from_secs(1),
        );
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
        println!("[TCP] Server stopped");
    }

    /// 获取当前活跃连接数
    #[allow(dead_code)]
    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }
}

fn handle_text_client(mut stream: TcpStream, handler: ProtocolHandler) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(CLIENT_TIMEOUT_SECS)));
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let line = String::from_utf8_lossy(&buf[..n]);
                for l in line.lines() {
                    let response = handler.handle_line(l.trim());
                    if let Err(e) = stream.write_all(format!("{}\n", response).as_bytes()) {
                        eprintln!("[TCP Text] Write error: {}", e);
                        return;
                    }
                    let _ = stream.flush();
                }
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::TimedOut
                    && e.kind() != std::io::ErrorKind::WouldBlock
                {
                    eprintln!("[TCP Text] Read error: {}", e);
                }
                break;
            }
        }
    }
}

fn handle_dlms_client(mut stream: TcpStream, processor: crate::dlms::DlmsProcessor) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(CLIENT_TIMEOUT_SECS)));
    let mut buf = [0u8; 4096];
    // HDLC over TCP: 帧由 0x7E 分隔, 需要积累完整帧
    let mut frame_buf = Vec::new();

    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                frame_buf.extend_from_slice(&buf[..n]);

                // 查找完整 HDLC 帧 (以 0x7E 开头和结尾)
                while let Some(end) = frame_buf.iter().position(|&b| b == 0x7E) {
                    if end == 0 {
                        // 跳过起始 flag
                        frame_buf.remove(0);
                        continue;
                    }
                    let frame_data: Vec<u8> = frame_buf[..=end].to_vec();
                    frame_buf.drain(..=end);

                    if frame_data.len() < 4 {
                        continue;
                    }

                    match processor.process_hdlc(&frame_data) {
                        Ok(resp) => {
                            if let Err(e) = stream.write_all(&resp) {
                                eprintln!("[TCP DLMS] Write error: {}", e);
                                return;
                            }
                            let _ = stream.flush();
                        }
                        Err(e) => eprintln!("[DLMS] Error: {}", e),
                    }
                }

                // 防止缓冲区无限增长
                if frame_buf.len() > 65536 {
                    frame_buf.clear();
                }
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::TimedOut
                    && e.kind() != std::io::ErrorKind::WouldBlock
                {
                    eprintln!("[TCP DLMS] Read error: {}", e);
                }
                break;
            }
        }
    }
}

fn handle_iec_client(mut stream: TcpStream) {
    use crate::iec62056::Iec62056Processor;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(CLIENT_TIMEOUT_SECS)));
    let mut proc = Iec62056Processor::new();
    let mut buf = [0u8; 1024];

    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let line = String::from_utf8_lossy(&buf[..n]);
                for l in line.lines() {
                    let responses = proc.process_input(l);
                    for resp in responses {
                        if let Err(e) = stream.write_all(resp.as_bytes()) {
                            eprintln!("[TCP IEC] Write error: {}", e);
                            return;
                        }
                        let _ = stream.flush();
                    }
                }
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::TimedOut
                    && e.kind() != std::io::ErrorKind::WouldBlock
                {
                    eprintln!("[TCP IEC] Read error: {}", e);
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tcp_bind() {
        let result = std::net::TcpListener::bind("0.0.0.0:0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_connections_const() {
        use super::{CLIENT_TIMEOUT_SECS, HEARTBEAT_INTERVAL_SECS, MAX_CONNECTIONS};
        assert!(MAX_CONNECTIONS > 0);
        assert!(CLIENT_TIMEOUT_SECS > 0);
        assert!(HEARTBEAT_INTERVAL_SECS > 0);
    }
}
