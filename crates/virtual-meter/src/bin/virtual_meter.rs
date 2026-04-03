//! FeMeter 虚拟电表主程序
//!
//! 提供交互式 Shell、串口服务和 TCP 服务

use anyhow::Result;
use clap::Parser;
use virtual_meter::{
    create_meter, list_ports, ChipType, SerialConfig, SerialService, Shell, TcpServer,
};

/// FeMeter 虚拟电表 - 模拟 ATT7022E/RN8302B 计量芯片
#[derive(Parser, Debug)]
#[command(name = "femeter-meter")]
#[command(author = "FeMeter Team")]
#[command(version = "0.1.0")]
#[command(about = "虚拟电表，模拟计量芯片用于测试和开发")]
struct Args {
    /// TCP 端口 (DLMS over TCP)
    #[arg(long, default_value = "4059")]
    tcp: u16,

    /// 文本协议 TCP 端口
    #[arg(long, default_value = "8888")]
    text_port: u16,

    /// 串口设备路径 (如 /dev/ttyUSB0)
    #[arg(long)]
    serial: Option<String>,

    /// 创建虚拟串口对，打印 slave 路径
    #[arg(long)]
    virtual_serial: bool,

    /// 波特率 (默认 9600)
    #[arg(long, default_value = "9600")]
    baud: u32,

    /// 电表地址 (默认 1)
    #[arg(long, default_value = "1")]
    meter_id: u16,

    /// 模拟数据文件路径
    #[arg(long)]
    data_file: Option<String>,

    /// 日志级别 (off/error/info/debug)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// 初始化芯片类型 (att7022e 或 rn8302b)
    #[arg(short, long, default_value = "att7022e")]
    chip: String,

    /// 初始化频率 (Hz)
    #[arg(short = 'f', long, default_value = "50.0")]
    freq: f64,

    /// 启用噪声模拟
    #[arg(short, long)]
    noise: bool,

    /// 列出可用串口并退出
    #[arg(short, long)]
    list_ports: bool,

    /// 非交互模式 (仅启动服务，不进入 shell)
    #[arg(short = 'n', long)]
    non_interactive: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 设置日志级别
    let log_filter = match args.log_level.to_lowercase().as_str() {
        "off" => tracing::Level::ERROR,
        "error" => tracing::Level::ERROR,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        _ => tracing::Level::INFO,
    };
    tracing_subscriber::fmt().with_max_level(log_filter).init();

    // 列出串口模式
    if args.list_ports {
        println!("可用串口:");
        let ports = list_ports();
        if ports.is_empty() {
            println!("  (没有找到串口设备)");
        } else {
            for port in ports {
                println!("  {}", port);
            }
        }
        return Ok(());
    }

    // Shell prints its own banner

    // 创建电表
    let meter = create_meter();

    // 应用初始配置
    {
        let mut meter = meter.lock().expect("mutex poisoned");

        // 设置芯片类型
        match args.chip.to_lowercase().as_str() {
            "att7022e" | "att7022" => meter.set_chip(ChipType::ATT7022E),
            "rn8302b" | "rn8302" => meter.set_chip(ChipType::RN8302B),
            _ => {
                eprintln!("警告: 未知芯片类型 '{}', 使用默认 ATT7022E", args.chip);
                meter.set_chip(ChipType::ATT7022E);
            }
        }

        // 设置频率
        meter.set_freq(args.freq);

        // 设置噪声
        meter.set_noise(args.noise);
    }

    // TCP 服务器
    let mut tcp_server = TcpServer::new(meter.clone());
    if args.tcp > 0 {
        if let Err(e) = tcp_server.start_dlms(args.tcp) {
            eprintln!("TCP DLMS server failed: {}", e);
        }
    }
    if args.text_port > 0 {
        if let Err(e) = tcp_server.start_text(args.text_port) {
            eprintln!("TCP text server failed: {}", e);
        }
    }

    // 串口服务
    let mut serial_service =
        SerialService::new(meter.clone()).with_config(SerialConfig::new_8n1(args.baud));

    if args.virtual_serial {
        #[cfg(unix)]
        {
            match serial_service.start_virtual() {
                Ok(slave_path) => {
                    eprintln!("Serial port: {}", slave_path);
                }
                Err(e) => {
                    eprintln!("Serial failed: {}", e);
                }
            }
        }
        #[cfg(not(unix))]
        {
            eprintln!("[Serial] Virtual serial not supported on this platform");
        }
    } else if let Some(port_name) = &args.serial {
        if let Err(e) = serial_service.start(port_name) {
            eprintln!("Serial failed: {}", e);
        }
    }

    // 加载数据文件
    if let Some(data_file) = &args.data_file {
        eprintln!("Loading: {}", data_file);
        // TODO: 实现数据文件加载
    }

    // 非交互模式
    if args.non_interactive {
        eprintln!("Non-interactive. Ctrl+C to exit.");

        // 等待 Ctrl+C
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let r = running.clone();

        ctrlc::set_handler(move || {
            r.store(false, std::sync::atomic::Ordering::Relaxed);
        })
        .ok();

        while running.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        println!("\n[Shutdown] Stopping services...");
        let _ = serial_service.stop();
        tcp_server.stop();
        println!("[Shutdown] Done.");
        return Ok(());
    }

    // 运行交互式 shell
    println!();
    let mut shell = Shell::new(meter);
    shell.run()?;

    // 清理
    let _ = serial_service.stop();
    tcp_server.stop();

    Ok(())
}
