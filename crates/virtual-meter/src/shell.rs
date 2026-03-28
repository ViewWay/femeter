//! 交互式 Shell
//!
//! 提供命令行交互界面

use crate::{list_ports, ChipType, MeterHandle, SerialService};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, queue, style};
use std::io::{self, Write,};

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Shell 上下文
pub struct Shell {
    meter: MeterHandle,
    serial_service: SerialService,
    running: Arc<AtomicBool>,
}

impl Shell {
    /// 创建 Shell
    pub fn new(meter: MeterHandle) -> Self {
        let serial_service = SerialService::new(meter.clone());
        let running = Arc::new(AtomicBool::new(true));

        Self {
            meter,
            serial_service,
            running,
        }
    }

    /// 运行 Shell
    pub fn run(&mut self) -> Result<()> {
        // 检测是否有 TTY，没有则用简单行模式
        let is_tty = std::io::stdin().is_terminal(); // Rust 1.70+
        if is_tty {
            self.run_raw_mode()
        } else {
            self.run_line_mode()
        }
    }

    /// 简单行模式（非 TTY / 管道输入）
    fn run_line_mode(&mut self) -> Result<()> {
        use std::io::BufRead;
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        self.print_welcome_simple(&mut stdout)?;

        for line in stdin.lock().lines() {
            let line = line?;
            let input = line.trim();
            if input.is_empty() {
                continue;
            }
            self.execute_command(input, &mut stdout)?;
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
        }
        println!("Goodbye!");
        Ok(())
    }

    /// 简单欢迎信息
    fn print_welcome_simple(&self, stdout: &mut impl Write) -> Result<()> {
        write!(stdout, "FeMeter Virtual Meter v0.1 (line mode)\n")?;
        write!(stdout, "输入 'help' 查看可用命令\n")?;
        stdout.flush()?;
        Ok(())
    }

    /// Raw 模式（TTY 交互）
    fn run_raw_mode(&mut self) -> Result<()> {
        // 初始化终端
        terminal::enable_raw_mode()?;

        let mut stdout = io::stdout();
        let mut input = String::new();
        let mut history: Vec<String> = Vec::new();
        let mut history_index = 0;

        self.print_welcome(&mut stdout)?;

        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }

            // 打印提示符
            queue!(
                stdout,
                style::Print("\n\rlightning> "),
                cursor::Show
            )?;
            stdout.flush()?;

            input.clear();

            // 读取输入
            loop {
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Char(c) => {
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                                self.running.store(false, Ordering::Relaxed);
                                break;
                            }
                            input.push(c);
                            queue!(stdout, style::Print(c))?;
                            stdout.flush()?;
                        }
                        KeyCode::Backspace => {
                            if !input.is_empty() {
                                input.pop();
                                queue!(stdout, cursor::MoveLeft(1), terminal::Clear(ClearType::UntilNewLine))?;
                                stdout.flush()?;
                            }
                        }
                        KeyCode::Enter => {
                            break;
                        }
                        KeyCode::Up => {
                            if !history.is_empty() && history_index > 0 {
                                history_index -= 1;
                                self.replace_input(&mut stdout, &input, &history[history_index])?;
                                input = history[history_index].clone();
                            }
                        }
                        KeyCode::Down => {
                            if !history.is_empty() && history_index < history.len() - 1 {
                                history_index += 1;
                                self.replace_input(&mut stdout, &input, &history[history_index])?;
                                input = history[history_index].clone();
                            }
                        }
                        _ => {}
                    }
                }
            }

            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            // 保存历史
            if !input.is_empty() && (history.is_empty() || history.last() != Some(&input.to_string()))
            {
                history.push(input.to_string());
                history_index = history.len();
            }

            // 执行命令
            self.execute_command(input, &mut stdout)?;
        }

        // 清理
        terminal::disable_raw_mode()?;
        println!("\nGoodbye!");

        Ok(())
    }

    /// 替换输入内容
    fn replace_input(&self, stdout: &mut impl Write, old: &str, new: &str) -> Result<()> {
        let old_len = old.len() as u16;
        if old_len > 0 {
            queue!(
                stdout,
                cursor::MoveLeft(old_len),
                terminal::Clear(ClearType::UntilNewLine)
            )?;
        }
        queue!(stdout, style::Print(new))?;
        stdout.flush()?;
        Ok(())
    }

    /// 打印欢迎信息
    fn print_welcome(&self, stdout: &mut impl Write) -> Result<()> {
        queue!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0),
            style::Print("╔════════════════════════════════════════════╗\n\r"),
            style::Print("║     FeMeter Virtual Meter v0.1             ║\n\r"),
            style::Print("║     模拟 ATT7022E / RN8302B 计量芯片       ║\n\r"),
            style::Print("╚════════════════════════════════════════════╝\n\r"),
            style::Print("\n\r"),
            style::Print("输入 'help' 查看可用命令\n\r"),
        )?;
        stdout.flush()?;
        Ok(())
    }

    /// 执行命令
    fn execute_command(&self, input: &str, stdout: &mut impl Write) -> Result<()> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(());
        }

        match parts[0].to_lowercase().as_str() {
            "help" | "h" | "?" => self.cmd_help(stdout),
            "status" | "st" => self.cmd_status(stdout),
            "set" => self.cmd_set(&parts[1..], stdout),
            "energy" | "en" => self.cmd_energy(stdout),
            "reset" => self.cmd_reset(stdout),
            "snapshot" | "ss" => self.cmd_snapshot(stdout),
            "serial" => self.cmd_serial(&parts[1..], stdout),
            "quit" | "exit" | "q" => {
                self.running.store(false, Ordering::Relaxed);
                Ok(())
            }
            _ => {
                queue!(stdout, style::Print(format!("未知命令: {}\n\r", parts[0])))?;
                stdout.flush()?;
                Ok(())
            }
        }
    }

    /// 帮助命令
    fn cmd_help(&self, stdout: &mut impl Write) -> Result<()> {
        let help = r#"
可用命令:
  help / h / ?          显示此帮助
  status / st           显示所有参数
  set <param> <value>   设置参数
    set ua 220.5        设置A相电压 (V)
    set ub 220.0        设置B相电压
    set uc 219.8        设置C相电压
    set ia 5.2          设置A相电流 (A)
    set ib 5.0          设置B相电流
    set ic 4.9          设置C相电流
    set angle_a 30      设置A相相角 (度)
    set angle_b 25      设置B相相角
    set angle_c 28      设置C相相角
    set freq 50         设置频率 (Hz)
    set noise on/off    开关噪声模拟
    set chip att7022e   切换到 ATT7022E 模式
    set chip rn8302b    切换到 RN8302B 模式

  energy / en           显示电能累计
  reset                 重置电能累计
  snapshot / ss         一次完整读取

  serial list           列出可用串口
  serial start <port>   启动串口服务
  serial stop           停止串口服务
  serial status         显示串口状态

  quit / q / exit       退出程序
"#;
        queue!(stdout, style::Print(help))?;
        stdout.flush()?;
        Ok(())
    }

    /// 状态命令
    fn cmd_status(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().unwrap();
        let snapshot = meter.snapshot();
        let config = meter.config();

        let status = format!(
            r#"
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  芯片: {:?} ({}-bit)
  频率: {:.2} Hz
  噪声: {}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  相位    电压(V)    电流(A)    角度(°)    PF
  ───────────────────────────────────────
  A       {:>8.2}   {:>8.2}   {:>8.1}   {:>6.3}
  B       {:>8.2}   {:>8.2}   {:>8.1}   {:>6.3}
  C       {:>8.2}   {:>8.2}   {:>8.1}   {:>6.3}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  功率 (W):
    P_A: {:>10.2}    P_B: {:>10.2}    P_C: {:>10.2}
    P_总: {:>10.2}
  
  功率 (var):
    Q_A: {:>10.2}    Q_B: {:>10.2}    Q_C: {:>10.2}
    Q_总: {:>10.2}
  
  视在功率 (VA):
    S_总: {:>10.2}
  
  总功率因数: {:.3}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"#,
            config.chip,
            config.chip.bits(),
            snapshot.freq,
            if config.noise_enabled { "开" } else { "关" },
            snapshot.phase_a.voltage, snapshot.phase_a.current, snapshot.phase_a.angle, snapshot.computed.pf_a,
            snapshot.phase_b.voltage, snapshot.phase_b.current, snapshot.phase_b.angle, snapshot.computed.pf_b,
            snapshot.phase_c.voltage, snapshot.phase_c.current, snapshot.phase_c.angle, snapshot.computed.pf_c,
            snapshot.computed.p_a, snapshot.computed.p_b, snapshot.computed.p_c, snapshot.computed.p_total,
            snapshot.computed.q_a, snapshot.computed.q_b, snapshot.computed.q_c, snapshot.computed.q_total,
            snapshot.computed.s_total,
            snapshot.computed.pf_total,
        );

        queue!(stdout, style::Print(status))?;
        stdout.flush()?;
        Ok(())
    }

    /// 设置命令
    fn cmd_set(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.len() < 2 {
            queue!(stdout, style::Print("用法: set <param> <value>\n\r"))?;
            stdout.flush()?;
            return Ok(());
        }

        let param = args[0].to_lowercase();
        let value_str = args[1];

        let mut meter = self.meter.lock().unwrap();
        let result = match param.as_str() {
            "ua" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_voltage('a', v);
                    format!("A相电压设置为 {:.2} V", v)
                } else {
                    format!("无效的电压值: {}", value_str)
                }
            }
            "ub" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_voltage('b', v);
                    format!("B相电压设置为 {:.2} V", v)
                } else {
                    format!("无效的电压值: {}", value_str)
                }
            }
            "uc" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_voltage('c', v);
                    format!("C相电压设置为 {:.2} V", v)
                } else {
                    format!("无效的电压值: {}", value_str)
                }
            }
            "ia" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_current('a', v);
                    format!("A相电流设置为 {:.2} A", v)
                } else {
                    format!("无效的电流值: {}", value_str)
                }
            }
            "ib" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_current('b', v);
                    format!("B相电流设置为 {:.2} A", v)
                } else {
                    format!("无效的电流值: {}", value_str)
                }
            }
            "ic" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_current('c', v);
                    format!("C相电流设置为 {:.2} A", v)
                } else {
                    format!("无效的电流值: {}", value_str)
                }
            }
            "angle_a" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_angle('a', v);
                    format!("A相角度设置为 {:.1}°", v)
                } else {
                    format!("无效的角度值: {}", value_str)
                }
            }
            "angle_b" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_angle('b', v);
                    format!("B相角度设置为 {:.1}°", v)
                } else {
                    format!("无效的角度值: {}", value_str)
                }
            }
            "angle_c" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_angle('c', v);
                    format!("C相角度设置为 {:.1}°", v)
                } else {
                    format!("无效的角度值: {}", value_str)
                }
            }
            "freq" => {
                if let Ok(v) = value_str.parse::<f64>() {
                    meter.set_freq(v);
                    format!("频率设置为 {:.2} Hz", v)
                } else {
                    format!("无效的频率值: {}", value_str)
                }
            }
            "noise" => {
                let enabled = value_str.to_lowercase() == "on" || value_str == "1" || value_str.to_lowercase() == "true";
                meter.set_noise(enabled);
                format!("噪声模拟 {}", if enabled { "已开启" } else { "已关闭" })
            }
            "chip" => {
                match value_str.to_lowercase().as_str() {
                    "att7022e" | "att7022" => {
                        meter.set_chip(ChipType::ATT7022E);
                        "已切换到 ATT7022E 模式".to_string()
                    }
                    "rn8302b" | "rn8302" => {
                        meter.set_chip(ChipType::RN8302B);
                        "已切换到 RN8302B 模式".to_string()
                    }
                    _ => format!("未知芯片类型: {} (支持: att7022e, rn8302b)", value_str),
                }
            }
            _ => format!("未知参数: {}", param),
        };

        queue!(stdout, style::Print(format!("{}\n\r", result)))?;
        stdout.flush()?;
        Ok(())
    }

    /// 电能命令
    fn cmd_energy(&self, stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().unwrap();
        let energy = meter.energy();

        let result = format!(
            r#"
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  电能累计
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  有功电能 (Wh):
    A相: {:>12.3}
    B相: {:>12.3}
    C相: {:>12.3}
    总计: {:>12.3}
  
  无功电能 (varh):
    A相: {:>12.3}
    B相: {:>12.3}
    C相: {:>12.3}
    总计: {:>12.3}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"#,
            energy.wh_a, energy.wh_b, energy.wh_c, energy.wh_total,
            energy.varh_a, energy.varh_b, energy.varh_c, energy.varh_total,
        );

        queue!(stdout, style::Print(result))?;
        stdout.flush()?;
        Ok(())
    }

    /// 重置命令
    fn cmd_reset(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().unwrap();
        meter.reset_energy();

        queue!(stdout, style::Print("电能累计已重置\n\r"))?;
        stdout.flush()?;
        Ok(())
    }

    /// 快照命令
    fn cmd_snapshot(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().unwrap();
        let snapshot = meter.snapshot();

        let json = serde_json::to_string_pretty(&snapshot)?;
        queue!(stdout, style::Print(format!("{}\n\r", json)))?;
        stdout.flush()?;
        Ok(())
    }

    /// 串口命令
    fn cmd_serial(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(stdout, style::Print("用法: serial <list|start|stop|status>\n\r"))?;
            stdout.flush()?;
            return Ok(());
        }

        match args[0].to_lowercase().as_str() {
            "list" => {
                let ports = list_ports();
                if ports.is_empty() {
                    queue!(stdout, style::Print("没有找到可用串口\n\r"))?;
                } else {
                    queue!(stdout, style::Print("可用串口:\n\r"))?;
                    for port in ports {
                        queue!(stdout, style::Print(format!("  {}\n\r", port)))?;
                    }
                }
            }
            "start" => {
                if args.len() < 2 {
                    queue!(stdout, style::Print("用法: serial start <port>\n\r"))?;
                } else {
                    let port_name = args[1];
                    queue!(stdout, style::Print(format!("正在启动串口 {} ...\n\r", port_name)))?;
                    stdout.flush()?;

                    // 这里需要 mutable borrow，但我们在 &self 上
                    // 实际应用中应该重新设计
                    queue!(stdout, style::Print("提示: 串口服务需要在主函数中启动\n\r"))?;
                }
            }
            "stop" => {
                queue!(stdout, style::Print("提示: 串口服务需要在主函数中停止\n\r"))?;
            }
            "status" => {
                queue!(stdout, style::Print("提示: 使用 'serial list' 查看可用串口\n\r"))?;
            }
            _ => {
                queue!(stdout, style::Print(format!("未知串口命令: {}\n\r", args[0])))?;
            }
        }

        stdout.flush()?;
        Ok(())
    }
}
