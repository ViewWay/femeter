//! FeMeter 虚拟电表主程序
//!
//! 提供交互式 Shell 和串口服务

use anyhow::Result;
use clap::Parser;
use virtual_meter::{create_meter, list_ports, ChipType, Shell};

/// FeMeter 虚拟电表 - 模拟 ATT7022E/RN8302B 计量芯片
#[derive(Parser, Debug)]
#[command(name = "virtual-meter")]
#[command(author = "FeMeter Team")]
#[command(version = "0.1.0")]
#[command(about = "虚拟电表，模拟计量芯片用于测试和开发")]
struct Args {
    /// 启动后自动设置串口服务
    #[arg(short, long)]
    serial: Option<String>,

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
}

fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let args = Args::parse();

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

    // 如果指定了串口，启动串口服务
    if let Some(port_name) = &args.serial {
        println!("注意: 串口服务需要手动启动");
        println!("在 shell 中使用: serial start {}", port_name);
    }

    // 运行交互式 shell
    let mut shell = Shell::new(meter);
    shell.run()?;

    Ok(())
}
