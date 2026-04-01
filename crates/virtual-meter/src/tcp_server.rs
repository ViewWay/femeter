//! TCP 服务器
//!
//! DLMS over TCP (port 4059) + 简单文本协议 (port 8888)

use crate::{MeterHandle, ProtocolHandler, create_protocol_handler};
use anyhow::Result;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::io::{Read, Write};

pub struct TcpServer {
    meter: MeterHandle,
    running: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
}

impl TcpServer {
    pub fn new(meter: MeterHandle) -> Self {
        Self { meter, running: Arc::new(AtomicBool::new(false)), handles: Vec::new() }
    }

    pub fn start_text(&mut self, port: u16) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        let meter = self.meter.clone();
        let running = self.running.clone();
        self.running.store(true, Ordering::Relaxed);
        let h = thread::spawn(move || {
            listener.set_nonblocking(false).ok();
            while running.load(Ordering::Relaxed) {
                if let Ok((stream, addr)) = listener.accept() {
                    println!("[TCP Text] Connection from {}", addr);
                    let handler = create_protocol_handler(meter.clone());
                    thread::spawn(move || {
                        handle_text_client(stream, handler);
                    });
                }
            }
        });
        self.handles.push(h);
        println!("[TCP] Text protocol server on port {}", port);
        Ok(())
    }

    #[cfg(feature = "tcp")]
    pub fn start_dlms(&mut self, port: u16) -> Result<()> {
        use crate::dlms::create_dlms_processor;
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        let meter = self.meter.clone();
        let running = self.running.clone();
        self.running.store(true, Ordering::Relaxed);
        let h = thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                if let Ok((stream, addr)) = listener.accept() {
                    println!("[TCP DLMS] Connection from {}", addr);
                    let processor = create_dlms_processor(meter.clone());
                    thread::spawn(move || {
                        handle_dlms_client(stream, processor);
                    });
                }
            }
        });
        self.handles.push(h);
        println!("[TCP] DLMS server on port {}", port);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // Connect to self to unblock accept()
        let _ = std::net::TcpStream::connect("127.0.0.1:8888");
        for h in self.handles.drain(..) { let _ = h.join(); }
    }
}

fn handle_text_client(mut stream: TcpStream, handler: ProtocolHandler) {
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let line = String::from_utf8_lossy(&buf[..n]);
                for l in line.lines() {
                    let response = handler.handle_line(l.trim());
                    let _ = stream.write_all(format!("{}\n", response).as_bytes());
                    let _ = stream.flush();
                }
            }
            Err(_) => break,
        }
    }
}

#[cfg(feature = "tcp")]
fn handle_dlms_client(mut stream: TcpStream, processor: crate::dlms::DlmsProcessor) {
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                match processor.process_hdlc(&buf[..n]) {
                    Ok(resp) => { let _ = stream.write_all(&resp); let _ = stream.flush(); }
                    Err(e) => eprintln!("[DLMS] Error: {}", e),
                }
            }
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tcp_bind() {
        // Just test we can bind
        let result = std::net::TcpListener::bind("0.0.0.0:0");
        assert!(result.is_ok());
    }
}
