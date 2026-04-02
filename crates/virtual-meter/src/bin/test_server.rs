//! Test helper binary that starts TCP text server for pytest
use std::process;
use virtual_meter::{create_meter, tcp_server::TcpServer};

fn main() {
    let meter = create_meter();
    let mut server = TcpServer::new(meter);
    if let Err(e) = server.start_text(8888) {
        eprintln!("Failed to start TCP server: {}", e);
        process::exit(1);
    }
    // Block forever
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
