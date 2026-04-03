//! Test helper binary that starts TCP text server + DLMS HDLC server
//!
//! - Port 8888: TCP text protocol (for pytest)
//! - Port 4059: TCP DLMS HDLC service (for DLMS client testing)
//!
//! Usage: test_server [--text-port PORT] [--dlms-port PORT]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use virtual_meter::{create_dlms_processor, create_meter, tcp_server::TcpServer};

fn main() {
    let meter = create_meter();
    let meter_clone = meter.clone();

    // --- TCP Text Server (port 8888) ---
    let text_port: u16 = std::env::args()
        .position(|a| a == "--text-port")
        .and_then(|p| std::env::args().nth(p + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(8888);

    let text_handle = thread::spawn(move || {
        let mut server = TcpServer::new(meter);
        if let Err(e) = server.start_text(text_port) {
            eprintln!("Failed to start TCP text server: {}", e);
        }
    });

    // --- DLMS HDLC Server (port 4059) ---
    let dlms_port: u16 = std::env::args()
        .position(|a| a == "--dlms-port")
        .and_then(|p| std::env::args().nth(p + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(4059);

    let dlms_processor = Arc::new(create_dlms_processor(meter_clone));

    let dlms_handle = thread::spawn(move || {
        start_dlms_server(dlms_processor, dlms_port);
    });

    println!(
        "Virtual meter servers started: text={}, dlms={}",
        text_port, dlms_port
    );

    // Block until Ctrl+C
    let _ = text_handle.join();
    let _ = dlms_handle.join();
}

/// Start a TCP server that accepts DLMS HDLC connections
fn start_dlms_server(processor: Arc<virtual_meter::dlms::DlmsProcessor>, port: u16) {
    let listener = match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind DLMS server on port {}: {}", port, e);
            return;
        }
    };

    eprintln!("DLMS HDLC server listening on 127.0.0.1:{}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let proc = processor.clone();
                thread::spawn(move || {
                    handle_dlms_client(proc, &mut stream);
                });
            }
            Err(e) => {
                eprintln!("DLMS accept error: {}", e);
            }
        }
    }
}

/// Handle a single DLMS client connection
fn handle_dlms_client(
    processor: Arc<virtual_meter::dlms::DlmsProcessor>,
    stream: &mut (impl Read + Write),
) {
    let mut buf = [0u8; 2048];
    let mut frame_buf: Vec<u8> = Vec::new();

    loop {
        // Read available bytes
        let n = match stream.read(&mut buf) {
            Ok(0) => {
                // Connection closed
                return;
            }
            Ok(n) => n,
            Err(_) => return,
        };

        // Feed bytes into frame assembler
        let mut escaped = false;
        for &b in &buf[..n] {
            match b {
                0x7E => {
                    if !frame_buf.is_empty() {
                        // End of frame detected — wrap with flags for HdlcFrame::decode
                        let mut frame = Vec::with_capacity(frame_buf.len() + 2);
                        frame.push(0x7E);
                        frame.extend_from_slice(&frame_buf);
                        frame.push(0x7E);
                        frame_buf.clear();
                        match processor.process_hdlc(&frame) {
                            Ok(response) => {
                                let _ = stream.write_all(&response);
                                let _ = stream.flush();
                            }
                            Err(e) => {
                                eprintln!("DLMS processing error: {}", e);
                            }
                        }
                    }
                    escaped = false;
                }
                0x7D => {
                    escaped = true;
                }
                _ => {
                    if escaped {
                        frame_buf.push(b ^ 0x20);
                        escaped = false;
                    } else {
                        frame_buf.push(b);
                    }
                }
            }
        }

        // Safety: don't let buffer grow unbounded
        if frame_buf.len() > 1500 {
            frame_buf.clear();
        }
    }
}
