//! 交互式 Shell (增强版)
//!
//! 新增命令: log, scenario, event, replay, pulse

use crate::{list_ports, ChipType, MeterHandle, MeterEvent, Scenario, SerialService, set_log_enabled, is_log_enabled};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, queue, style};
use std::io::{self, Write, IsTerminal};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct Shell {
    meter: MeterHandle,
    serial_service: SerialService,
    running: Arc<AtomicBool>,
    /// 自动刷新 (log 模式)
    auto_refresh: bool,
    refresh_interval_ms: u64,
}

impl Shell {
    pub fn new(meter: MeterHandle) -> Self {
        let serial_service = SerialService::new(meter.clone());
        Self {
            meter,
            serial_service,
            running: Arc::new(AtomicBool::new(true)),
            auto_refresh: false,
            refresh_interval_ms: 500,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        if io::stdin().is_terminal() {
            self.run_raw_mode()
        } else {
            self.run_line_mode()
        }
    }

    fn run_line_mode(&mut self) -> Result<()> {
        use std::io::BufRead;
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        self.print_welcome_simple(&mut stdout)?;
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() { continue; }
            self.execute_command(line.trim(), &mut stdout)?;
            if !self.running.load(Ordering::Relaxed) { break; }
        }
        println!("Goodbye!");
        Ok(())
    }

    fn print_welcome_simple(&self, stdout: &mut impl Write) -> Result<()> {
        writeln!(stdout, "FeMeter Virtual Meter v0.2 (line mode, log={})", if is_log_enabled() { "ON" } else { "OFF" })?;
        writeln!(stdout, "输入 'help' 查看命令 | 'log on' 开启日志")?;
        stdout.flush()?;
        Ok(())
    }

    fn run_raw_mode(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        let mut input = String::new();
        let mut history: Vec<String> = Vec::new();
        let mut history_index = 0;

        self.print_welcome(&mut stdout)?;

        loop {
            if !self.running.load(Ordering::Relaxed) { break; }

            // 自动刷新模式
            if self.auto_refresh {
                let mut meter = self.meter.lock().unwrap();
                meter.print_status(&mut stdout);
                drop(meter);
                stdout.flush()?;

                // 非阻塞读键盘
                if event::poll(Duration::from_millis(self.refresh_interval_ms))? {
                    if let Event::Key(key) = event::read()? {
                        match key.code {
                            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.running.store(false, Ordering::Relaxed);
                                break;
                            }
                            KeyCode::Char(c) => {
                                input.push(c);
                                queue!(stdout, style::Print(c))?;
                                stdout.flush()?;
                            }
                            KeyCode::Backspace if !input.is_empty() => {
                                input.pop();
                                queue!(stdout, cursor::MoveLeft(1), terminal::Clear(ClearType::UntilNewLine))?;
                                stdout.flush()?;
                            }
                            KeyCode::Enter => {
                                let cmd = input.trim().to_string();
                                input.clear();
                                if !cmd.is_empty() {
                                    self.execute_command(&cmd, &mut stdout)?;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                continue;
            }

            queue!(stdout, style::Print("\n\r⚡> "), cursor::Show)?;
            stdout.flush()?;
            input.clear();

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
                        KeyCode::Backspace if !input.is_empty() => {
                            input.pop();
                            queue!(stdout, cursor::MoveLeft(1), terminal::Clear(ClearType::UntilNewLine))?;
                            stdout.flush()?;
                        }
                        KeyCode::Enter => { break; }
                        KeyCode::Up if !history.is_empty() && history_index > 0 => {
                            history_index -= 1;
                            self.replace_input(&mut stdout, &input, &history[history_index])?;
                            input = history[history_index].clone();
                        }
                        KeyCode::Down if !history.is_empty() && history_index < history.len() - 1 => {
                            history_index += 1;
                            self.replace_input(&mut stdout, &input, &history[history_index])?;
                            input = history[history_index].clone();
                        }
                        _ => {}
                    }
                }
            }

            let cmd = input.trim();
            if cmd.is_empty() { continue; }
            if !history.contains(&cmd.to_string()) {
                history.push(cmd.to_string());
                history_index = history.len();
            }
            self.execute_command(cmd, &mut stdout)?;
        }

        terminal::disable_raw_mode()?;
        println!("\nGoodbye!");
        Ok(())
    }

    fn replace_input(&self, stdout: &mut impl Write, old: &str, new: &str) -> Result<()> {
        let old_len = old.len() as u16;
        if old_len > 0 {
            queue!(stdout, cursor::MoveLeft(old_len), terminal::Clear(ClearType::UntilNewLine))?;
        }
        queue!(stdout, style::Print(new))?;
        stdout.flush()?;
        Ok(())
    }

    fn print_welcome(&self, stdout: &mut impl Write) -> Result<()> {
        queue!(stdout,
            terminal::Clear(ClearType::All), cursor::MoveTo(0, 0),
            style::Print("╔══════════════════════════════════════════════╗\n\r"),
            style::Print("║    FeMeter Virtual Meter v0.2                ║\n\r"),
            style::Print("║    模拟 ATT7022E / RN8302B | log="),
            style::Print(if is_log_enabled() { "ON" } else { "OFF" }),
            style::Print("                ║\n\r"),
            style::Print("╚══════════════════════════════════════════════╝\n\r"),
            style::Print("\n\r输入 'help' 查看命令 | 'watch' 进入实时监控\n\r"),
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn execute_command(&self, input: &str, stdout: &mut impl Write) -> Result<()> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() { return Ok(()); }

        match parts[0].to_lowercase().as_str() {
            "help" | "h" | "?" => self.cmd_help(stdout),
            "status" | "st" => self.cmd_status(stdout),
            "set" => self.cmd_set(&parts[1..], stdout),
            "energy" | "en" => self.cmd_energy(stdout),
            "reset" => self.cmd_reset(stdout),
            "snapshot" | "ss" => self.cmd_snapshot(stdout),
            "log" => self.cmd_log(&parts[1..], stdout),
            "scenario" | "sc" => self.cmd_scenario(&parts[1..], stdout),
            "event" | "ev" => self.cmd_event(&parts[1..], stdout),
            "events" => self.cmd_events(stdout),
            "watch" | "w" => { self.cmd_watch(&parts[1..], stdout); Ok(()) },
            "pulse" => self.cmd_pulse(stdout),
            "serial" => self.cmd_serial(&parts[1..], stdout),
            "replay" => self.cmd_replay(&parts[1..], stdout),
            "tariff" => self.cmd_tariff(&parts[1..], stdout),
            "profile" => self.cmd_profile(&parts[1..], stdout),
            "demand" => self.cmd_demand(&parts[1..], stdout),
            "dlms" => self.cmd_dlms(&parts[1..], stdout),
            "display" => self.cmd_display(&parts[1..], stdout),
            "stats" => self.cmd_stats(&parts[1..], stdout),
            "cal" => self.cmd_cal(&parts[1..], stdout),
            "save" => self.cmd_save(&parts[1..], stdout),
            "load" => self.cmd_load(&parts[1..], stdout),
            "tcp" => self.cmd_tcp(&parts[1..], stdout),
            "iec" => self.cmd_iec(&parts[1..], stdout),
            "quit" | "exit" | "q" => { self.running.store(false, Ordering::Relaxed); Ok(()) }
            _ => { queue!(stdout, style::Print(format!("未知命令: {} (输入 help)\n\r", parts[0])))?; let _ = stdout.flush(); Ok(()) }
        }
    }

    fn cmd_help(&self, stdout: &mut impl Write) -> Result<()> {
        let help = r#"
════════════════════════════════════════════════
  FeMeter Virtual Meter 命令列表
════════════════════════════════════════════════
  基础:
    help / h              显示帮助
    status / st           显示完整状态表
    snapshot / ss         JSON 快照

  设置:
    set ua/ub/uc <V>      电压 (V)
    set ia/ib/ic <A>      电流 (A)
    set angle_a/b/c <°>   相角
    set freq <Hz>         频率
    set noise on/off      噪声模拟
    set chip att7022e     切换芯片
    set accel <倍率>      时间加速 (如 3600 = 1秒=1小时)

  日志:
    log on/off            开关日志打印
    log status            查看日志状态

  场景:
    scenario normal       正常运行 (220V 5A)
    scenario full         满载 (60A)
    scenario noload       空载
    scenario overv        A相过压 (280V)
    scenario underv       A相欠压 (170V)
    scenario loss         A相断相 (0V)
    scenario overi        过流 (70A)
    scenario reverse      反向功率
    scenario unbalanced   三相不平衡

  事件:
    event cover           上盖打开
    event terminal        端子盖打开
    event magnetic        磁场干扰
    event battery         电池低电压
    events                查看事件历史

  监控:
    watch [ms]            实时监控 (默认 500ms, q 退出)
    pulse                 查看脉冲计数

  其他:
    energy / en           电能累计
    reset                 重置电能
    serial list/start/stop 串口
    quit / q              退出
════════════════════════════════════════════════
"#;
        queue!(stdout, style::Print(help))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_log(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        match args.first().map(|s| s.to_lowercase()).as_deref() {
            Some("on" | "1" | "true") => {
                set_log_enabled(true);
                queue!(stdout, style::Print("日志已开启 ON\n\r"))?;
            }
            Some("off" | "0" | "false") => {
                set_log_enabled(false);
                queue!(stdout, style::Print("日志已关闭 OFF\n\r"))?;
            }
            Some("status") | None => {
                queue!(stdout, style::Print(format!("日志状态: {}\n\r", if is_log_enabled() { "ON" } else { "OFF" })))?;
            }
            _ => {
                queue!(stdout, style::Print("用法: log on|off|status\n\r"))?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn cmd_scenario(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(stdout, style::Print("用法: scenario <normal|full|noload|overv|underv|loss|overi|reverse|unbalanced>\n\r"))?;
            stdout.flush()?;
            return Ok(());
        }
        let sc = match args[0].to_lowercase().as_str() {
            "normal" => Scenario::Normal,
            "full" | "fullload" => Scenario::FullLoad,
            "noload" | "no" | "empty" => Scenario::NoLoad,
            "overv" | "overvoltage" => Scenario::OverVoltage,
            "underv" | "undervoltage" => Scenario::UnderVoltage,
            "loss" | "phaseloss" => Scenario::PhaseLoss,
            "overi" | "overcurrent" => Scenario::OverCurrent,
            "reverse" | "reversepower" => Scenario::ReversePower,
            "unbalanced" => Scenario::Unbalanced,
            _ => {
                queue!(stdout, style::Print(format!("未知场景: {}\n\r", args[0])))?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().unwrap();
        meter.load_scenario(sc);
        queue!(stdout, style::Print(format!("已加载场景: {:?}\n\r", sc)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_event(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(stdout, style::Print("用法: event <cover|terminal|magnetic|battery>\n\r"))?;
            stdout.flush()?;
            return Ok(());
        }
        let ev = match args[0].to_lowercase().as_str() {
            "cover" => MeterEvent::CoverOpen,
            "terminal" => MeterEvent::TerminalCoverOpen,
            "magnetic" => MeterEvent::MagneticTamper,
            "battery" => MeterEvent::BatteryLow,
            _ => {
                queue!(stdout, style::Print(format!("未知事件: {}\n\r", args[0])))?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().unwrap();
        meter.trigger_event(ev);
        queue!(stdout, style::Print(format!("已触发事件: {:?}\n\r", ev)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_events(&self, stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().unwrap();
        let events = meter.events();
        if events.is_empty() {
            queue!(stdout, style::Print("无事件记录\n\r"))?;
        } else {
            queue!(stdout, style::Print(format!("事件记录 (共 {} 条):\n\r", events.len())))?;
            for e in events.iter().rev().take(20) {
                queue!(stdout, style::Print(format!(
                    "  [{}] {:?} - {}\n\r", e.timestamp.format("%H:%M:%S"), e.event, e.description
                )))?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn cmd_watch(&self, args: &[&str], stdout: &mut impl Write) {
        // 注意: auto_refresh 需要可变引用, 这里通过 Shell 自身方法设置
        // 简化: 直接打印 10 次然后退出
        let interval = args.first().and_then(|s| s.parse::<u64>().ok()).unwrap_or(500);
        queue!(stdout, style::Print(format!("实时监控 (每 {}ms, 共 10 次, Ctrl+C 退出):\n\r", interval))).ok();
        stdout.flush().ok();
        for _i in 0..10 {
            std::thread::sleep(Duration::from_millis(interval));
            let mut meter = self.meter.lock().unwrap();
            meter.print_status(stdout);
            drop(meter);
            stdout.flush().ok();
            if !self.running.load(Ordering::Relaxed) { break; }
        }
        queue!(stdout, style::Print("监控结束\n\r")).ok();
        stdout.flush().ok();
    }

    fn cmd_pulse(&self, stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().unwrap();
        queue!(stdout, style::Print(format!(
            "脉冲常数: {} imp/kWh, 累计脉冲: {}\n\r",
            meter.pulse_count(), // TODO: add getter
            0 // placeholder
        )))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_replay(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        // 简单回放: 依次加载多个场景
        let scenarios = match args.first().copied() {
            Some("all") | None => vec![
                ("normal", 3), ("full", 3), ("overv", 2), ("underv", 2),
                ("loss", 2), ("overi", 2), ("reverse", 2), ("normal", 2),
            ],
            Some("stress") => vec![
                ("normal", 1), ("full", 1), ("overv", 1), ("loss", 1),
                ("normal", 1), ("overi", 1), ("reverse", 1), ("normal", 1),
            ],
            _ => {
                queue!(stdout, style::Print("用法: replay [all|stress]\n\r"))?;
                stdout.flush()?;
                return Ok(());
            }
        };

        queue!(stdout, style::Print(format!("回放 {} 个场景 (每个 3s):\n\r", scenarios.len())))?;
        stdout.flush()?;

        for (name, secs) in &scenarios {
            let mut meter = self.meter.lock().unwrap();
            let sc = match *name {
                "normal" => Scenario::Normal, "full" => Scenario::FullLoad,
                "noload" => Scenario::NoLoad, "overv" => Scenario::OverVoltage,
                "underv" => Scenario::UnderVoltage, "loss" => Scenario::PhaseLoss,
                "overi" => Scenario::OverCurrent, "reverse" => Scenario::ReversePower,
                "unbalanced" => Scenario::Unbalanced, _ => Scenario::Normal,
            };
            meter.load_scenario(sc);
            meter.print_status(stdout);
            drop(meter);
            stdout.flush()?;
            std::thread::sleep(Duration::from_secs(*secs));
        }

        queue!(stdout, style::Print("回放完成\n\r"))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_status(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().unwrap();
        let snap = meter.snapshot();
        let ev_str = if snap.active_events.is_empty() { "无".to_string() }
            else { snap.active_events.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>().join(", ") };

        let status = format!(
            r#"
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  芯片: {:?} ({}-bit)  频率: {:.2} Hz  噪声: {}  加速: {:.0}x
  活动事件: {}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  相位    电压(V)    电流(A)    角度(°)    PF
  ───────────────────────────────────────
  A       {:>8.2}   {:>8.2}   {:>8.1}   {:>6.3}
  B       {:>8.2}   {:>8.2}   {:>8.1}   {:>6.3}
  C       {:>8.2}   {:>8.2}   {:>8.1}   {:>6.3}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  P总: {:>10.2} W    Q总: {:>10.2} var    PF: {:.3}
  S总: {:>10.2} VA
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  有功电能: {:.3} kWh    无功电能: {:.3} kvarh
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"#,
            snap.chip, snap.chip.bits(), snap.freq,
            if meter.config().noise_enabled { "开" } else { "关" },
            meter.config().time_accel,
            ev_str,
            snap.phase_a.voltage, snap.phase_a.current, snap.phase_a.angle, snap.computed.pf_a,
            snap.phase_b.voltage, snap.phase_b.current, snap.phase_b.angle, snap.computed.pf_b,
            snap.phase_c.voltage, snap.phase_c.current, snap.phase_c.angle, snap.computed.pf_c,
            snap.computed.p_total, snap.computed.q_total, snap.computed.pf_total,
            snap.computed.s_total,
            snap.energy.wh_total / 1000.0, snap.energy.varh_total / 1000.0,
        );
        queue!(stdout, style::Print(status))?;
        stdout.flush()?;
        Ok(())
    }

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
            "ua" => { meter.set_voltage('a', value_str.parse().unwrap_or(220.0)); format!("A相电压 = {:.2} V", value_str.parse::<f64>().unwrap_or(220.0)) }
            "ub" => { meter.set_voltage('b', value_str.parse().unwrap_or(220.0)); format!("B相电压 = {:.2} V", value_str.parse::<f64>().unwrap_or(220.0)) }
            "uc" => { meter.set_voltage('c', value_str.parse().unwrap_or(220.0)); format!("C相电压 = {:.2} V", value_str.parse::<f64>().unwrap_or(220.0)) }
            "ia" => { meter.set_current('a', value_str.parse().unwrap_or(0.0)); format!("A相电流 = {:.2} A", value_str.parse::<f64>().unwrap_or(0.0)) }
            "ib" => { meter.set_current('b', value_str.parse().unwrap_or(0.0)); format!("B相电流 = {:.2} A", value_str.parse::<f64>().unwrap_or(0.0)) }
            "ic" => { meter.set_current('c', value_str.parse().unwrap_or(0.0)); format!("C相电流 = {:.2} A", value_str.parse::<f64>().unwrap_or(0.0)) }
            "angle_a" => { meter.set_angle('a', value_str.parse().unwrap_or(0.0)); format!("A相角度 = {:.1}°", value_str.parse::<f64>().unwrap_or(0.0)) }
            "angle_b" => { meter.set_angle('b', value_str.parse().unwrap_or(0.0)); format!("B相角度 = {:.1}°", value_str.parse::<f64>().unwrap_or(0.0)) }
            "angle_c" => { meter.set_angle('c', value_str.parse().unwrap_or(0.0)); format!("C相角度 = {:.1}°", value_str.parse::<f64>().unwrap_or(0.0)) }
            "freq" => { meter.set_freq(value_str.parse().unwrap_or(50.0)); format!("频率 = {:.2} Hz", value_str.parse::<f64>().unwrap_or(50.0)) }
            "noise" => { let e = ["on","1","true"].contains(&value_str.to_lowercase().as_str()); meter.set_noise(e); format!("噪声 {}", if e {"开"} else {"关"}) }
            "chip" => match value_str.to_lowercase().as_str() {
                "att7022e" | "att7022" => { meter.set_chip(ChipType::ATT7022E); "ATT7022E".to_string() }
                "rn8302b" | "rn8302" => { meter.set_chip(ChipType::RN8302B); "RN8302B".to_string() }
                _ => format!("未知: {}", value_str),
            },
            "accel" => { let a: f64 = value_str.parse().unwrap_or(1.0); meter.set_time_accel(a); format!("时间加速 = {:.0}x", a) }
            _ => format!("未知参数: {}", param),
        };
        queue!(stdout, style::Print(format!("{}\n\r", result)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_energy(&self, stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().unwrap();
        let e = meter.energy();
        let r = format!(
            "\n  有功电能 (Wh): A={:.3} B={:.3} C={:.3} 总={:.3}\n  无功电能 (varh): A={:.3} B={:.3} C={:.3} 总={:.3}\n\n",
            e.wh_a, e.wh_b, e.wh_c, e.wh_total, e.varh_a, e.varh_b, e.varh_c, e.varh_total);
        queue!(stdout, style::Print(r))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_reset(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().unwrap();
        meter.reset_energy();
        queue!(stdout, style::Print("电能已重置\n\r"))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_snapshot(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().unwrap();
        let snap = meter.snapshot();
        let json = serde_json::to_string_pretty(&snap)?;
        queue!(stdout, style::Print(format!("{}\n\r", json)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_serial(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        match args.first().map(|s| s.to_lowercase()).as_deref() {
            Some("list") => {
                let ports = list_ports();
                if ports.is_empty() { queue!(stdout, style::Print("无可用串口\n\r"))?; }
                else { for p in ports { queue!(stdout, style::Print(format!("  {}\n\r", p)))?; } }
            }
            _ => { queue!(stdout, style::Print("用法: serial list\n\r"))?; }
        }
        stdout.flush()?;
        Ok(())
    }

    fn cmd_tariff(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().unwrap();
        let tou = meter.tou();
        queue!(stdout, style::Print(format!("当前费率: {:?}\n\r", tou.current_tariff())))?;
        queue!(stdout, style::Print(format!("累计电能: 尖={:.3} 峰={:.3} 平={:.3} 谷={:.3}\n\r",
            tou.energy.get(&crate::tariff::TariffType::Sharp).unwrap_or(&0.0) / 1000.0,
            tou.energy.get(&crate::tariff::TariffType::Peak).unwrap_or(&0.0) / 1000.0,
            tou.energy.get(&crate::tariff::TariffType::Normal).unwrap_or(&0.0) / 1000.0,
            tou.energy.get(&crate::tariff::TariffType::Valley).unwrap_or(&0.0) / 1000.0,
        )))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_profile(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        queue!(stdout, style::Print("负荷曲线: 请使用 save/load 管理\n\r"))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_demand(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().unwrap();
        let demand = meter.demand();
        queue!(stdout, style::Print(format!("当前需量: P={:.2}W Q={:.2}var\n\r", demand.current_p(), demand.current_q())))?;
        queue!(stdout, style::Print(format!("最大需量: P={:.2}W ({})\n\r", demand.max_p().value, demand.max_p().timestamp.format("%Y-%m-%d %H:%M"))))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_dlms(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        queue!(stdout, style::Print("DLMS: 协议已启用 (port 4059)\n\r"))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_display(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let battery = args.first().map(|s| *s == "bat").unwrap_or(false);
        let disp = crate::display::LcdDisplay::new();
        let mut meter = self.meter.lock().unwrap();
        let snap = meter.snapshot();
        let value = snap.energy.wh_total / 1000.0;
        drop(meter);
        let art = disp.render_ascii(value, battery);
        queue!(stdout, style::Print(art))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_stats(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().unwrap();
        let stats = meter.statistics();
        queue!(stdout, style::Print("统计记录:\n\r"))?;
        let va_min = stats.daily.last().map(|d| d.va.min).unwrap_or(0.0);
        let va_max = stats.daily.last().map(|d| d.va.max).unwrap_or(0.0);
        let va_avg = stats.daily.last().map(|d| d.va.avg()).unwrap_or(0.0);
        queue!(stdout, style::Print(format!("  电压A: min={:.1} max={:.1} avg={:.1}\n\r", va_min, va_max, va_avg)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_cal(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let _meter = self.meter.lock().unwrap();
        let cal = crate::calibration::CalibrationParams::default();
        queue!(stdout, style::Print(format!("校准参数: 脉冲常数={}\n\r", cal.pulse_constant)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_save(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        use crate::persistence::PersistedState;
        let mut meter = self.meter.lock().unwrap();
        let snap = meter.snapshot();
        let state = PersistedState {
            energy: snap.energy.clone(),
            saved_at: snap.timestamp,
            ..Default::default()
        };
        let p = crate::persistence::Persistence::new("meter_state.json");
        if let Err(e) = p.save(&state) {
            queue!(stdout, style::Print(format!("保存失败: {}\n\r", e)))?;
        } else {
            queue!(stdout, style::Print("状态已保存到 meter_state.json\n\r"))?;
        }
        stdout.flush()?;
        Ok(())
    }

    fn cmd_load(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let p = crate::persistence::Persistence::new("meter_state.json");
        if !p.exists() {
            queue!(stdout, style::Print("未找到保存的文件\n\r"))?;
            stdout.flush()?;
            return Ok(());
        }
        match p.load() {
            Ok(state) => {
                queue!(stdout, style::Print(format!("加载状态: {}\n\r", state.saved_at)))?;
            }
            Err(e) => {
                queue!(stdout, style::Print(format!("加载失败: {}\n\r", e)))?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn cmd_tcp(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        queue!(stdout, style::Print("TCP: 使用虚拟电表可执行程序启动 TCP 服务\n\r"))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_iec(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        use crate::iec62056::Iec62056Processor;
        let _proc = Iec62056Processor::new();
        queue!(stdout, style::Print("IEC 62056-21: "))?;
        stdout.flush()?;
        Ok(())
    }
}
