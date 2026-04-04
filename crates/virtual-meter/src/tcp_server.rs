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
                    // TCP text connection
                    conn_count.fetch_add(1, Ordering::Relaxed);

                    let handler = create_protocol_handler(meter.clone());
                    let cc = conn_count.clone();
                    thread::spawn(move || {
                        handle_text_client(stream, handler);
                        cc.fetch_sub(1, Ordering::Relaxed);
                        // TCP text disconnect
                    });
                }
            }
        });
        self.handles.push(h);
        // TCP text started
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
                    // TCP DLMS connection
                    conn_count.fetch_add(1, Ordering::Relaxed);

                    let processor = create_dlms_processor(meter.clone());
                    let cc = conn_count.clone();
                    thread::spawn(move || {
                        handle_dlms_client(stream, processor);
                        cc.fetch_sub(1, Ordering::Relaxed);
                        // TCP DLMS disconnect
                    });
                }
            }
        });
        self.handles.push(h);
        // TCP DLMS started
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
                    // TCP IEC connection
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
        // TCP IEC started
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // Connect to self to unblock accept()
        let _ = std::net::TcpStream::connect_timeout(
            &"127.0.0.1:8888".parse().expect("valid socket addr"),
            Duration::from_secs(1),
        );
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
        // TCP stopped
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
    eprintln!("[TCP DLMS] New client connection from {:?}", stream.peer_addr());
    let _ = stream.set_read_timeout(Some(Duration::from_secs(CLIENT_TIMEOUT_SECS)));
    let mut buf = [0u8; 4096];
    // HDLC over TCP: 帧由 0x7E 分隔, 需要积累完整帧
    let mut frame_buf = Vec::new();

    loop {
        match stream.read(&mut buf) {
            Ok(0) => {
                eprintln!("[TCP DLMS] Client disconnected");
                break;
            }
            Ok(n) => {
                eprintln!("[TCP DLMS] Received {} bytes: {:02X?}", n, &buf[..n]);
                frame_buf.extend_from_slice(&buf[..n]);
                eprintln!("[TCP DLMS] Received {} bytes: {:02X?}", n, &buf[..n]);

                // 查找完整 HDLC 帧 (以 0x7E 开头和结尾)
                // 跳过起始的连续 0x7E
                while !frame_buf.is_empty() && frame_buf[0] == 0x7E {
                    frame_buf.remove(0);
                }
                
                // 查找下一个 0x7E (结束 flag)
                if let Some(end) = frame_buf.iter().position(|&b| b == 0x7E) {
                    // 完整帧数据 = 0x7E + content + 0x7E
                    let mut frame_data = vec![0x7E];
                    frame_data.extend_from_slice(&frame_buf[..end]);
                    frame_data.push(0x7E);
                    frame_buf.drain(..=end);

                    eprintln!("[TCP DLMS] Processing frame: {:02X?}", frame_data);

                    if frame_data.len() < 4 {
                        eprintln!("[TCP DLMS] Frame too short: {}", frame_data.len());
                        continue;
                    }

                    match processor.process_hdlc(&frame_data) {
                        Ok(resp) => {
                            eprintln!("[TCP DLMS] Sending response: {:02X?}", resp);
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
