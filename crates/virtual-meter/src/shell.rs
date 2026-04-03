//! 交互式 Shell (增强版)
//!
//! 新增命令: log, scenario, event, replay, pulse

use crate::{
    is_log_enabled, list_ports, set_log_enabled, ChipType, MeterEvent, MeterHandle, Scenario,
    SerialService,
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, queue, style};
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct Shell {
    meter: MeterHandle,
    #[allow(dead_code)]
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
            if line.trim().is_empty() {
                continue;
            }
            self.execute_command(line.trim(), &mut stdout)?;
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
        }
        println!("Goodbye!");
        Ok(())
    }

    fn print_welcome_simple(&self, stdout: &mut impl Write) -> Result<()> {
        writeln!(
            stdout,
            "FeMeter Virtual Meter v0.2 (line mode, log={})",
            if is_log_enabled() { "ON" } else { "OFF" }
        )?;
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
            if !self.running.load(Ordering::Relaxed) {
                break;
            }

            // 自动刷新模式
            if self.auto_refresh {
                let mut meter = self.meter.lock().expect("mutex poisoned");
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
                                queue!(
                                    stdout,
                                    cursor::MoveLeft(1),
                                    terminal::Clear(ClearType::UntilNewLine)
                                )?;
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
                            queue!(
                                stdout,
                                cursor::MoveLeft(1),
                                terminal::Clear(ClearType::UntilNewLine)
                            )?;
                            stdout.flush()?;
                        }
                        KeyCode::Enter => {
                            break;
                        }
                        KeyCode::Up if !history.is_empty() && history_index > 0 => {
                            history_index -= 1;
                            self.replace_input(&mut stdout, &input, &history[history_index])?;
                            input = history[history_index].clone();
                        }
                        KeyCode::Down
                            if !history.is_empty() && history_index < history.len() - 1 =>
                        {
                            history_index += 1;
                            self.replace_input(&mut stdout, &input, &history[history_index])?;
                            input = history[history_index].clone();
                        }
                        _ => {}
                    }
                }
            }

            let cmd = input.trim();
            if cmd.is_empty() {
                continue;
            }
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

    fn print_welcome(&self, stdout: &mut impl Write) -> Result<()> {
        queue!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0),
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
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0].to_lowercase().as_str() {
            "help" | "h" | "?" => self.cmd_help(stdout),
            "status" | "st" => self.cmd_status(stdout),
            "set" => self.cmd_set(&parts[1..], stdout),
            "get" => self.cmd_get(&parts[1..], stdout),
            "energy" | "en" => self.cmd_energy(stdout),
            "reset" => self.cmd_reset(stdout),
            "snapshot" | "ss" => self.cmd_snapshot(stdout),
            "log" => self.cmd_log(&parts[1..], stdout),
            "scenario" | "sc" => self.cmd_scenario(&parts[1..], stdout),
            "event" | "ev" => self.cmd_event(&parts[1..], stdout),
            "events" => self.cmd_events(stdout),
            "watch" | "w" => {
                self.cmd_watch(&parts[1..], stdout);
                Ok(())
            }
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
            "obis" => self.cmd_obis(&parts[1..], stdout),
            "run" => self.cmd_run(&parts[1..], stdout),
            "quit" | "exit" | "q" => {
                self.running.store(false, Ordering::Relaxed);
                Ok(())
            }
            _ => {
                queue!(
                    stdout,
                    style::Print(format!("未知命令: {} (输入 help)\n\r", parts[0]))
                )?;
                let _ = stdout.flush();
                Ok(())
            }
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

  三相设置:
    set ua/ub/uc <V>      电压 (V)
    set ia/ib/ic <A>      电流 (A)
    set angle-a/b/c <°>   相角 (如 angle-a 0, angle-b -120, angle-c 120)
    set freq <Hz>         频率
    set pf <0.0-1.0>      功率因数 (自动计算相角)
    set three-phase <V> <A> <Hz> <PF>  三相组合设置
    set noise on/off      噪声模拟
    set chip att7022e     切换芯片
    set accel <倍率>      时间加速 (如 3600 = 1秒=1小时)

  查询:
    get voltage           显示三相电压 + 线电压
    get current           显示三相电流 + 中性线电流
    get angle             显示三相角度
    get power             显示有功/无功/视在功率
    get energy            显示累计电能
    get frequency         显示频率
    get power-factor      显示功率因数
    get status-word       显示状态字及异常检测

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

  协议调试:
    dlms send <hex>       发送原始 DLMS APDU
    dlms get <OBIS>       读取 COSEM 对象
    dlms assoc            发送关联请求
    obis <OBIS>           查询 OBIS 码值
    obis                  列出常用 OBIS 码
    iec                   IEC 62056-21 状态

  场景自动化:
    run <script>          执行脚本文件

  退出:
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
                queue!(
                    stdout,
                    style::Print(format!(
                        "日志状态: {}\n\r",
                        if is_log_enabled() { "ON" } else { "OFF" }
                    ))
                )?;
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
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.load_scenario(sc);
        queue!(stdout, style::Print(format!("已加载场景: {:?}\n\r", sc)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_event(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(
                stdout,
                style::Print("用法: event <cover|terminal|magnetic|battery>\n\r")
            )?;
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
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.trigger_event(ev);
        queue!(stdout, style::Print(format!("已触发事件: {:?}\n\r", ev)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_events(&self, stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().expect("mutex poisoned");
        let events = meter.events();
        if events.is_empty() {
            queue!(stdout, style::Print("无事件记录\n\r"))?;
        } else {
            queue!(
                stdout,
                style::Print(format!("事件记录 (共 {} 条):\n\r", events.len()))
            )?;
            for e in events.iter().rev().take(20) {
                queue!(
                    stdout,
                    style::Print(format!(
                        "  [{}] {:?} - {}\n\r",
                        e.timestamp.format("%H:%M:%S"),
                        e.event,
                        e.description
                    ))
                )?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn cmd_watch(&self, args: &[&str], stdout: &mut impl Write) {
        // 注意: auto_refresh 需要可变引用, 这里通过 Shell 自身方法设置
        // 简化: 直接打印 10 次然后退出
        let interval = args
            .first()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(500);
        queue!(
            stdout,
            style::Print(format!(
                "实时监控 (每 {}ms, 共 10 次, Ctrl+C 退出):\n\r",
                interval
            ))
        )
        .ok();
        stdout.flush().ok();
        for _i in 0..10 {
            std::thread::sleep(Duration::from_millis(interval));
            let mut meter = self.meter.lock().expect("mutex poisoned");
            meter.print_status(stdout);
            drop(meter);
            stdout.flush().ok();
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
        }
        queue!(stdout, style::Print("监控结束\n\r")).ok();
        stdout.flush().ok();
    }

    fn cmd_pulse(&self, stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().expect("mutex poisoned");
        queue!(
            stdout,
            style::Print(format!(
                "脉冲常数: {} imp/kWh, 累计脉冲: {}\n\r",
                meter.pulse_count(), // TODO: add getter
                0                    // placeholder
            ))
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_replay(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        // 简单回放: 依次加载多个场景
        let scenarios = match args.first().copied() {
            Some("all") | None => vec![
                ("normal", 3),
                ("full", 3),
                ("overv", 2),
                ("underv", 2),
                ("loss", 2),
                ("overi", 2),
                ("reverse", 2),
                ("normal", 2),
            ],
            Some("stress") => vec![
                ("normal", 1),
                ("full", 1),
                ("overv", 1),
                ("loss", 1),
                ("normal", 1),
                ("overi", 1),
                ("reverse", 1),
                ("normal", 1),
            ],
            _ => {
                queue!(stdout, style::Print("用法: replay [all|stress]\n\r"))?;
                stdout.flush()?;
                return Ok(());
            }
        };

        queue!(
            stdout,
            style::Print(format!("回放 {} 个场景 (每个 3s):\n\r", scenarios.len()))
        )?;
        stdout.flush()?;

        for (name, secs) in &scenarios {
            let mut meter = self.meter.lock().expect("mutex poisoned");
            let sc = match *name {
                "normal" => Scenario::Normal,
                "full" => Scenario::FullLoad,
                "noload" => Scenario::NoLoad,
                "overv" => Scenario::OverVoltage,
                "underv" => Scenario::UnderVoltage,
                "loss" => Scenario::PhaseLoss,
                "overi" => Scenario::OverCurrent,
                "reverse" => Scenario::ReversePower,
                "unbalanced" => Scenario::Unbalanced,
                _ => Scenario::Normal,
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
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();
        let ev_str = if snap.active_events.is_empty() {
            "无".to_string()
        } else {
            snap.active_events
                .iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join(", ")
        };

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
            snap.chip,
            snap.chip.bits(),
            snap.freq,
            if meter.config().noise_enabled {
                "开"
            } else {
                "关"
            },
            meter.config().time_accel,
            ev_str,
            snap.phase_a.voltage,
            snap.phase_a.current,
            snap.phase_a.angle,
            snap.computed.pf_a,
            snap.phase_b.voltage,
            snap.phase_b.current,
            snap.phase_b.angle,
            snap.computed.pf_b,
            snap.phase_c.voltage,
            snap.phase_c.current,
            snap.phase_c.angle,
            snap.computed.pf_c,
            snap.computed.p_total,
            snap.computed.q_total,
            snap.computed.pf_total,
            snap.computed.s_total,
            snap.energy.wh_total / 1000.0,
            snap.energy.varh_total / 1000.0,
        );
        queue!(stdout, style::Print(status))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_set(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(stdout, style::Print("用法: set <param> <value> | set three-phase <V> <A> <Hz> <PF>\n\r"))?;
            stdout.flush()?;
            return Ok(());
        }
        let param = args[0].to_lowercase();
        let mut meter = self.meter.lock().expect("mutex poisoned");
        
        // 处理组合命令: set three-phase <V> <A> <Hz> <PF>
        if param == "three-phase" || param == "threephase" || param == "3p" {
            if args.len() < 5 {
                queue!(stdout, style::Print("用法: set three-phase <电压V> <电流A> <频率Hz> <功率因数PF>\n\r"))?;
                queue!(stdout, style::Print("示例: set three-phase 230 5 50 0.95\n\r"))?;
                stdout.flush()?;
                return Ok(());
            }
            let v: f64 = args[1].parse().unwrap_or(220.0);
            let i: f64 = args[2].parse().unwrap_or(5.0);
            let freq: f64 = args[3].parse().unwrap_or(50.0);
            let pf: f64 = args[4].parse().unwrap_or(0.95);
            
            // 根据功率因数计算角度 (cos⁻¹(pf))
            let angle = pf.acos() * 180.0 / std::f64::consts::PI;
            
            // 设置三相
            meter.set_voltage('a', v);
            meter.set_voltage('b', v);
            meter.set_voltage('c', v);
            meter.set_current('a', i);
            meter.set_current('b', i);
            meter.set_current('c', i);
            meter.set_angle('a', angle);
            meter.set_angle('b', angle);
            meter.set_angle('c', angle);
            meter.set_freq(freq);
            
            queue!(stdout, style::Print(format!(
                "三相设置: {:.1}V {:.2}A {:.2}Hz PF={:.3} (角度={:.1}°)\n\r",
                v, i, freq, pf, angle
            )))?;
            stdout.flush()?;
            return Ok(());
        }
        
        if args.len() < 2 {
            queue!(stdout, style::Print(format!("用法: set {} <value>\n\r", param)))?;
            stdout.flush()?;
            return Ok(());
        }
        
        let value_str = args[1];
        let result = match param.as_str() {
            "ua" => {
                meter.set_voltage('a', value_str.parse().unwrap_or(220.0));
                format!("A相电压 = {:.2} V", value_str.parse::<f64>().unwrap_or(220.0))
            }
            "ub" => {
                meter.set_voltage('b', value_str.parse().unwrap_or(220.0));
                format!("B相电压 = {:.2} V", value_str.parse::<f64>().unwrap_or(220.0))
            }
            "uc" => {
                meter.set_voltage('c', value_str.parse().unwrap_or(220.0));
                format!("C相电压 = {:.2} V", value_str.parse::<f64>().unwrap_or(220.0))
            }
            "ia" => {
                meter.set_current('a', value_str.parse().unwrap_or(0.0));
                format!("A相电流 = {:.2} A", value_str.parse::<f64>().unwrap_or(0.0))
            }
            "ib" => {
                meter.set_current('b', value_str.parse().unwrap_or(0.0));
                format!("B相电流 = {:.2} A", value_str.parse::<f64>().unwrap_or(0.0))
            }
            "ic" => {
                meter.set_current('c', value_str.parse().unwrap_or(0.0));
                format!("C相电流 = {:.2} A", value_str.parse::<f64>().unwrap_or(0.0))
            }
            "angle-a" | "angle_a" => {
                meter.set_angle('a', value_str.parse().unwrap_or(0.0));
                format!("A相角度 = {:.1}°", value_str.parse::<f64>().unwrap_or(0.0))
            }
            "angle-b" | "angle_b" => {
                meter.set_angle('b', value_str.parse().unwrap_or(0.0));
                format!("B相角度 = {:.1}°", value_str.parse::<f64>().unwrap_or(0.0))
            }
            "angle-c" | "angle_c" => {
                meter.set_angle('c', value_str.parse().unwrap_or(0.0));
                format!("C相角度 = {:.1}°", value_str.parse::<f64>().unwrap_or(0.0))
            }
            "freq" => {
                meter.set_freq(value_str.parse().unwrap_or(50.0));
                format!("频率 = {:.2} Hz", value_str.parse::<f64>().unwrap_or(50.0))
            }
            "pf" | "power-factor" => {
                let pf: f64 = value_str.parse().unwrap_or(0.95);
                if !(-1.0..=1.0).contains(&pf) {
                    "错误: 功率因数必须在 -1.0 到 1.0 之间".to_string()
                } else {
                    // 根据功率因数计算角度 (cos⁻¹(pf))
                    let angle = pf.acos() * 180.0 / std::f64::consts::PI;
                    meter.set_angle('a', angle);
                    meter.set_angle('b', angle);
                    meter.set_angle('c', angle);
                    format!("功率因数 = {:.3} (角度自动设为 {:.1}°)", pf, angle)
                }
            }
            "noise" => {
                let e = ["on", "1", "true"].contains(&value_str.to_lowercase().as_str());
                meter.set_noise(e);
                format!("噪声 {}", if e { "开" } else { "关" })
            }
            "chip" => match value_str.to_lowercase().as_str() {
                "att7022e" | "att7022" => {
                    meter.set_chip(ChipType::ATT7022E);
                    "ATT7022E".to_string()
                }
                "rn8302b" | "rn8302" => {
                    meter.set_chip(ChipType::RN8302B);
                    "RN8302B".to_string()
                }
                _ => format!("未知: {}", value_str),
            },
            "accel" => {
                let a: f64 = value_str.parse().unwrap_or(1.0);
                meter.set_time_accel(a);
                format!("时间加速 = {:.0}x", a)
            }
            _ => format!("未知参数: {}", param),
        };
        queue!(stdout, style::Print(format!("{}\n\r", result)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_get(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(stdout, style::Print(
                "用法: get <voltage|current|angle|power|energy|frequency|power-factor|status-word>\n\r"
            ))?;
            stdout.flush()?;
            return Ok(());
        }
        
        let sub = args[0].to_lowercase();
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();
        
        match sub.as_str() {
            "voltage" | "volt" | "v" => {
                // 计算线电压 (相电压 × √3)
                let v_ab = (snap.phase_a.voltage * snap.phase_a.voltage 
                    + snap.phase_b.voltage * snap.phase_b.voltage 
                    - 2.0 * snap.phase_a.voltage * snap.phase_b.voltage * ((snap.phase_a.angle - snap.phase_b.angle) * std::f64::consts::PI / 180.0).cos()).sqrt();
                let v_bc = (snap.phase_b.voltage * snap.phase_b.voltage 
                    + snap.phase_c.voltage * snap.phase_c.voltage 
                    - 2.0 * snap.phase_b.voltage * snap.phase_c.voltage * ((snap.phase_b.angle - snap.phase_c.angle) * std::f64::consts::PI / 180.0).cos()).sqrt();
                let v_ca = (snap.phase_c.voltage * snap.phase_c.voltage 
                    + snap.phase_a.voltage * snap.phase_a.voltage 
                    - 2.0 * snap.phase_c.voltage * snap.phase_a.voltage * ((snap.phase_c.angle - snap.phase_a.angle) * std::f64::consts::PI / 180.0).cos()).sqrt();
                
                let output = format!(
                    "\n\r┌────────────────────────────────────────┐\n\r\
                     │           电压测量                     │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ 相位    相电压(V)    线电压(V)         │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ A        {:>8.2}      Uab: {:>8.2}     │\n\r\
                     │ B        {:>8.2}      Ubc: {:>8.2}     │\n\r\
                     │ C        {:>8.2}      Uca: {:>8.2}     │\n\r\
                     └────────────────────────────────────────┘\n\r\n"
                    , snap.phase_a.voltage, v_ab
                    , snap.phase_b.voltage, v_bc
                    , snap.phase_c.voltage, v_ca
                );
                queue!(stdout, style::Print(output))?;
            }
            "current" | "curr" | "i" => {
                // 计算中性线电流 (简化为三相电流矢量和)
                let i_n = ((snap.phase_a.current * (snap.phase_a.angle * std::f64::consts::PI / 180.0).cos()
                    + snap.phase_b.current * (snap.phase_b.angle * std::f64::consts::PI / 180.0).cos()
                    + snap.phase_c.current * (snap.phase_c.angle * std::f64::consts::PI / 180.0).cos()).powi(2)
                    + (snap.phase_a.current * (snap.phase_a.angle * std::f64::consts::PI / 180.0).sin()
                    + snap.phase_b.current * (snap.phase_b.angle * std::f64::consts::PI / 180.0).sin()
                    + snap.phase_c.current * (snap.phase_c.angle * std::f64::consts::PI / 180.0).sin()).powi(2)).sqrt();
                
                let output = format!(
                    "\n\r┌────────────────────────────────────────┐\n\r\
                     │           电流测量                     │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ 相位    电流(A)                       │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ A        {:>8.3}                     │\n\r\
                     │ B        {:>8.3}                     │\n\r\
                     │ C        {:>8.3}                     │\n\r\
                     │ N        {:>8.3}  (中性线)           │\n\r\
                     └────────────────────────────────────────┘\n\r\n"
                    , snap.phase_a.current
                    , snap.phase_b.current
                    , snap.phase_c.current
                    , i_n
                );
                queue!(stdout, style::Print(output))?;
            }
            "angle" | "ang" | "a" => {
                let output = format!(
                    "\n\r┌────────────────────────────────────────┐\n\r\
                     │           相角测量                     │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ 相位    角度(°)                       │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ A        {:>8.1}°                    │\n\r\
                     │ B        {:>8.1}°                    │\n\r\
                     │ C        {:>8.1}°                    │\n\r\
                     └────────────────────────────────────────┘\n\r\
                     注: 正常三相角度 A=0°, B=-120°, C=120°\n\r\n"
                    , snap.phase_a.angle
                    , snap.phase_b.angle
                    , snap.phase_c.angle
                );
                queue!(stdout, style::Print(output))?;
            }
            "power" | "pow" | "p" => {
                let output = format!(
                    "\n\r┌────────────────────────────────────────────────┐\n\r\
                     │                功率测量                        │\n\r\
                     ├────────────────────────────────────────────────┤\n\r\
                     │ 相位  有功(W)  无功(var)  视在(VA)    PF    │\n\r\
                     ├────────────────────────────────────────────────┤\n\r\
                     │ A    {:>7.1}  {:>8.1}  {:>8.1}  {:>5.3}│\n\r\
                     │ B    {:>7.1}  {:>8.1}  {:>8.1}  {:>5.3}│\n\r\
                     │ C    {:>7.1}  {:>8.1}  {:>8.1}  {:>5.3}│\n\r\
                     ├────────────────────────────────────────────────┤\n\r\
                     │ 总  {:>8.1}  {:>8.1}  {:>8.1}  {:>5.3}│\n\r\
                     └────────────────────────────────────────────────┘\n\r\n"
                    , snap.computed.p_a, snap.computed.q_a, snap.computed.s_a, snap.computed.pf_a
                    , snap.computed.p_b, snap.computed.q_b, snap.computed.s_b, snap.computed.pf_b
                    , snap.computed.p_c, snap.computed.q_c, snap.computed.s_c, snap.computed.pf_c
                    , snap.computed.p_total, snap.computed.q_total, snap.computed.s_total, snap.computed.pf_total
                );
                queue!(stdout, style::Print(output))?;
            }
            "energy" | "en" | "e" => {
                let output = format!(
                    "\n\r┌────────────────────────────────────────┐\n\r\
                     │           电能累计                     │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ 相位    有功(kWh)    无功(kvarh)      │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ A        {:>10.4}    {:>10.4}      │\n\r\
                     │ B        {:>10.4}    {:>10.4}      │\n\r\
                     │ C        {:>10.4}    {:>10.4}      │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ 总      {:>10.4}    {:>10.4}      │\n\r\
                     └────────────────────────────────────────┘\n\r\n"
                    , snap.energy.wh_a / 1000.0, snap.energy.varh_a / 1000.0
                    , snap.energy.wh_b / 1000.0, snap.energy.varh_b / 1000.0
                    , snap.energy.wh_c / 1000.0, snap.energy.varh_c / 1000.0
                    , snap.energy.wh_total / 1000.0, snap.energy.varh_total / 1000.0
                );
                queue!(stdout, style::Print(output))?;
            }
            "frequency" | "freq" | "f" => {
                let output = format!("\n\r  频率: {:.3} Hz\n\r\n", snap.freq);
                queue!(stdout, style::Print(output))?;
            }
            "power-factor" | "pf" => {
                let output = format!(
                    "\n\r┌────────────────────────────────────────┐\n\r\
                     │           功率因数                     │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ 相位    功率因数                      │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ A        {:>8.4}                    │\n\r\
                     │ B        {:>8.4}                    │\n\r\
                     │ C        {:>8.4}                    │\n\r\
                     │ 总       {:>8.4}                    │\n\r\
                     └────────────────────────────────────────┘\n\r\n"
                    , snap.computed.pf_a
                    , snap.computed.pf_b
                    , snap.computed.pf_c
                    , snap.computed.pf_total
                );
                queue!(stdout, style::Print(output))?;
            }
            "status-word" | "status" | "sw" => {
                // 生成状态字
                let sw = self.compute_status_word(&snap);
                let mut output = format!(
                    "\n\r┌────────────────────────────────────────┐\n\r\
                     │           状态字分析                   │\n\r\
                     ├────────────────────────────────────────┤\n\r\
                     │ 状态字: 0x{:08X}                    │\n\r\
                     ├────────────────────────────────────────┤\n\r"
                    , sw
                );
                
                // 解析各状态位
                if sw == 0 {
                    output.push_str("│ ✓ 所有参数正常                        │\n\r");
                } else {
                    if sw & 0x01 != 0 { output.push_str("│ ⚠ A相失压 (< 10V)                    │\n\r"); }
                    if sw & 0x02 != 0 { output.push_str("│ ⚠ B相失压 (< 10V)                    │\n\r"); }
                    if sw & 0x04 != 0 { output.push_str("│ ⚠ C相失压 (< 10V)                    │\n\r"); }
                    if sw & 0x08 != 0 { output.push_str("│ ⚠ A相过压 (> 264V)                  │\n\r"); }
                    if sw & 0x10 != 0 { output.push_str("│ ⚠ B相过压 (> 264V)                  │\n\r"); }
                    if sw & 0x20 != 0 { output.push_str("│ ⚠ C相过压 (> 264V)                  │\n\r"); }
                    if sw & 0x40 != 0 { output.push_str("│ ⚠ A相欠压 (< 198V)                  │\n\r"); }
                    if sw & 0x80 != 0 { output.push_str("│ ⚠ B相欠压 (< 198V)                  │\n\r"); }
                    if sw & 0x100 != 0 { output.push_str("│ ⚠ C相欠压 (< 198V)                  │\n\r"); }
                    if sw & 0x200 != 0 { output.push_str("│ ⚠ A相过流 (> 60A)                   │\n\r"); }
                    if sw & 0x400 != 0 { output.push_str("│ ⚠ B相过流 (> 60A)                   │\n\r"); }
                    if sw & 0x800 != 0 { output.push_str("│ ⚠ C相过流 (> 60A)                   │\n\r"); }
                    if sw & 0x1000 != 0 { output.push_str("│ ⚠ 电流不平衡 (> 20%)                │\n\r"); }
                    if sw & 0x2000 != 0 { output.push_str("│ ⚠ 电压不平衡 (> 2%)                 │\n\r"); }
                    if sw & 0x4000 != 0 { output.push_str("│ ⚠ 相序错误                          │\n\r"); }
                    if sw & 0x8000 != 0 { output.push_str("│ ⚠ 反向功率                          │\n\r"); }
                }
                output.push_str("└────────────────────────────────────────┘\n\r\n");
                queue!(stdout, style::Print(output))?;
            }
            _ => {
                queue!(stdout, style::Print(format!("未知查询: {}\n\r", sub)))?;
                queue!(stdout, style::Print("可用: voltage, current, angle, power, energy, frequency, power-factor, status-word\n\r"))?;
            }
        }
        stdout.flush()?;
        Ok(())
    }
    
    /// 计算状态字
    fn compute_status_word(&self, snap: &crate::MeterSnapshot) -> u32 {
        let mut sw: u32 = 0;
        
        // 失压检测 (< 10V)
        if snap.phase_a.voltage < 10.0 { sw |= 0x01; }
        if snap.phase_b.voltage < 10.0 { sw |= 0x02; }
        if snap.phase_c.voltage < 10.0 { sw |= 0x04; }
        
        // 过压检测 (> 264V, 220V+20%)
        if snap.phase_a.voltage > 264.0 { sw |= 0x08; }
        if snap.phase_b.voltage > 264.0 { sw |= 0x10; }
        if snap.phase_c.voltage > 264.0 { sw |= 0x20; }
        
        // 欠压检测 (< 198V, 220V-10%)
        if snap.phase_a.voltage < 198.0 && snap.phase_a.voltage >= 10.0 { sw |= 0x40; }
        if snap.phase_b.voltage < 198.0 && snap.phase_b.voltage >= 10.0 { sw |= 0x80; }
        if snap.phase_c.voltage < 198.0 && snap.phase_c.voltage >= 10.0 { sw |= 0x100; }
        
        // 过流检测 (> 60A 额定 1.2 倍 = 72A, 这里简化为 60A)
        if snap.phase_a.current > 60.0 { sw |= 0x200; }
        if snap.phase_b.current > 60.0 { sw |= 0x400; }
        if snap.phase_c.current > 60.0 { sw |= 0x800; }
        
        // 电流不平衡检测 ((max-min)/avg > 20%)
        let i_max = snap.phase_a.current.max(snap.phase_b.current).max(snap.phase_c.current);
        let i_min = snap.phase_a.current.min(snap.phase_b.current).min(snap.phase_c.current);
        let i_avg = (snap.phase_a.current + snap.phase_b.current + snap.phase_c.current) / 3.0;
        if i_avg > 0.0 && (i_max - i_min) / i_avg > 0.2 { sw |= 0x1000; }
        
        // 电压不平衡检测 ((max-min)/avg > 2%)
        let v_max = snap.phase_a.voltage.max(snap.phase_b.voltage).max(snap.phase_c.voltage);
        let v_min = snap.phase_a.voltage.min(snap.phase_b.voltage).min(snap.phase_c.voltage);
        let v_avg = (snap.phase_a.voltage + snap.phase_b.voltage + snap.phase_c.voltage) / 3.0;
        if v_avg > 0.0 && (v_max - v_min) / v_avg > 0.02 { sw |= 0x2000; }
        
        // 相序错误检测 (角度不在正常范围)
        // 正常情况下: A=0°, B≈-120°, C≈120° (允许 ±10° 偏差)
        let a_ok = snap.phase_a.angle.abs() < 10.0;
        let b_ok = (snap.phase_b.angle + 120.0).abs() < 10.0;
        let c_ok = (snap.phase_c.angle - 120.0).abs() < 10.0;
        if !(a_ok && b_ok && c_ok) { sw |= 0x4000; }
        
        // 反向功率检测
        if snap.computed.p_total < 0.0 { sw |= 0x8000; }
        
        sw
    }

    fn cmd_energy(&self, stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().expect("mutex poisoned");
        let e = meter.energy();
        let r = format!(
            "\n  有功电能 (Wh): A={:.3} B={:.3} C={:.3} 总={:.3}\n  无功电能 (varh): A={:.3} B={:.3} C={:.3} 总={:.3}\n\n",
            e.wh_a, e.wh_b, e.wh_c, e.wh_total, e.varh_a, e.varh_b, e.varh_c, e.varh_total);
        queue!(stdout, style::Print(r))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_reset(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.reset_energy();
        queue!(stdout, style::Print("电能已重置\n\r"))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_snapshot(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().expect("mutex poisoned");
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
                if ports.is_empty() {
                    queue!(stdout, style::Print("无可用串口\n\r"))?;
                } else {
                    for p in ports {
                        queue!(stdout, style::Print(format!("  {}\n\r", p)))?;
                    }
                }
            }
            _ => {
                queue!(stdout, style::Print("用法: serial list\n\r"))?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn cmd_tariff(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().expect("mutex poisoned");
        let tou = meter.tou();
        queue!(
            stdout,
            style::Print(format!("当前费率: {:?}\n\r", tou.current_tariff()))
        )?;
        queue!(
            stdout,
            style::Print(format!(
                "累计电能: 尖={:.3} 峰={:.3} 平={:.3} 谷={:.3}\n\r",
                tou.energy
                    .get(&crate::tariff::TariffType::Sharp)
                    .unwrap_or(&0.0)
                    / 1000.0,
                tou.energy
                    .get(&crate::tariff::TariffType::Peak)
                    .unwrap_or(&0.0)
                    / 1000.0,
                tou.energy
                    .get(&crate::tariff::TariffType::Normal)
                    .unwrap_or(&0.0)
                    / 1000.0,
                tou.energy
                    .get(&crate::tariff::TariffType::Valley)
                    .unwrap_or(&0.0)
                    / 1000.0,
            ))
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_profile(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        queue!(stdout, style::Print("负荷曲线: 请使用 save/load 管理\n\r"))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_demand(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().expect("mutex poisoned");
        let demand = meter.demand();
        queue!(
            stdout,
            style::Print(format!(
                "当前需量: P={:.2}W Q={:.2}var\n\r",
                demand.current_p(),
                demand.current_q()
            ))
        )?;
        queue!(
            stdout,
            style::Print(format!(
                "最大需量: P={:.2}W ({})\n\r",
                demand.max_p().value,
                demand.max_p().timestamp.format("%Y-%m-%d %H:%M")
            ))
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_display(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let battery = args.first().map(|s| *s == "bat").unwrap_or(false);
        let disp = crate::display::LcdDisplay::new();
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();
        let value = snap.energy.wh_total / 1000.0;
        drop(meter);
        let art = disp.render_ascii(value, battery);
        queue!(stdout, style::Print(art))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_stats(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let meter = self.meter.lock().expect("mutex poisoned");
        let stats = meter.statistics();
        queue!(stdout, style::Print("统计记录:\n\r"))?;
        let va_min = stats.daily.last().map(|d| d.va.min).unwrap_or(0.0);
        let va_max = stats.daily.last().map(|d| d.va.max).unwrap_or(0.0);
        let va_avg = stats.daily.last().map(|d| d.va.avg()).unwrap_or(0.0);
        queue!(
            stdout,
            style::Print(format!(
                "  电压A: min={:.1} max={:.1} avg={:.1}\n\r",
                va_min, va_max, va_avg
            ))
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_cal(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        let _meter = self.meter.lock().expect("mutex poisoned");
        let cal = crate::calibration::CalibrationParams::default();
        queue!(
            stdout,
            style::Print(format!("校准参数: 脉冲常数={}\n\r", cal.pulse_constant))
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_save(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        use crate::persistence::PersistedState;
        let mut meter = self.meter.lock().expect("mutex poisoned");
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
                queue!(
                    stdout,
                    style::Print(format!("加载状态: {}\n\r", state.saved_at))
                )?;
            }
            Err(e) => {
                queue!(stdout, style::Print(format!("加载失败: {}\n\r", e)))?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn cmd_tcp(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        queue!(
            stdout,
            style::Print("TCP: 使用虚拟电表可执行程序启动 TCP 服务\n\r")
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_iec(&self, _args: &[&str], stdout: &mut impl Write) -> Result<()> {
        use crate::iec62056::Iec62056Processor;
        let mut proc = Iec62056Processor::new();
        let id = proc.handle_identification();
        queue!(
            stdout,
            style::Print(format!(
                "IEC 62056-21 协议状态\n\r  波特率: {}\n\r  标识: {}\n\r",
                proc.current_baud_rate().value(),
                id.trim()
            ))
        )?;
        stdout.flush()?;
        Ok(())
    }

    /// OBIS 码查询命令: obis <OBIS>
    /// 例: obis 1.0.1.8.0.255  查询总有功电能
    ///     obis 1.0.32.7.0.255  查询A相电压
    fn cmd_obis(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            // 列出常用 OBIS 码
            let common = [
                ("1.0.1.8.0.255", "总有功电能 (kWh)"),
                ("1.0.1.8.1.255", "有功电能-费率1"),
                ("1.0.1.8.2.255", "有功电能-费率2"),
                ("1.0.3.8.0.255", "总无功电能 (kvarh)"),
                ("1.0.32.7.0.255", "A相电压 (V)"),
                ("1.0.52.7.0.255", "B相电压 (V)"),
                ("1.0.72.7.0.255", "C相电压 (V)"),
                ("1.0.31.7.0.255", "A相电流 (A)"),
                ("1.0.51.7.0.255", "B相电流 (A)"),
                ("1.0.71.7.0.255", "C相电流 (A)"),
                ("1.0.1.7.0.255", "总有功功率 (W)"),
                ("1.0.3.7.0.255", "总无功功率 (var)"),
                ("1.0.14.7.0.255", "频率 (Hz)"),
                ("1.0.13.7.0.255", "功率因数"),
                ("0.0.96.1.0.255", "时钟"),
                ("0.0.96.10.1.255", "当前费率"),
            ];
            queue!(stdout, style::Print("\n\r常用 OBIS 码:\n\r".to_string()))?;
            for (code, desc) in &common {
                queue!(stdout, style::Print(format!("  {} - {}\n\r", code, desc)))?;
            }
            queue!(
                stdout,
                style::Print("\n\r用法: obis <OBIS>  例: obis 1.0.1.8.0.255\n\r".to_string())
            )?;
            stdout.flush()?;
            return Ok(());
        }

        let obis_str = args.join(".");
        let processor = crate::dlms::create_dlms_processor(self.meter.clone());
        match processor.query_obis(&obis_str) {
            Ok(result) => {
                queue!(stdout, style::Print(format!("{}\n\r", result)))?;
            }
            Err(e) => {
                queue!(stdout, style::Print(format!("查询失败: {}\n\r", e)))?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    /// DLMS 协议调试命令
    fn cmd_dlms(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(stdout, style::Print(
                "DLMS 协议调试命令:\n\r  dlms send <hex>  发送原始 APDU\n\r  dlms get <OBIS>  发送 GetRequest\n\r  dlms assoc      发送关联请求\n\r"
                .to_string()
            ))?;
            stdout.flush()?;
            return Ok(());
        }

        let sub = args[0].to_lowercase();
        match sub.as_str() {
            "send" => {
                // dlms send <hex bytes>
                if args.len() < 2 {
                    queue!(
                        stdout,
                        style::Print("用法: dlms send <hex>\n\r".to_string())
                    )?;
                    stdout.flush()?;
                    return Ok(());
                }
                let hex_str = args[1..].join("");
                let apdu = hex_decode(&hex_str);
                if apdu.is_empty() {
                    queue!(stdout, style::Print("无效的十六进制数据\n\r".to_string()))?;
                    stdout.flush()?;
                    return Ok(());
                }
                let processor = crate::dlms::create_dlms_processor(self.meter.clone());
                match processor.raw_apdu(&apdu) {
                    Ok(resp) => {
                        let hex = hex_encode(&resp);
                        queue!(
                            stdout,
                            style::Print(format!("响应 ({} bytes): {}\n\r", resp.len(), hex))
                        )?;
                    }
                    Err(e) => {
                        queue!(stdout, style::Print(format!("错误: {}\n\r", e)))?;
                    }
                }
            }
            "get" => {
                // dlms get <OBIS>
                if args.len() < 2 {
                    queue!(
                        stdout,
                        style::Print("用法: dlms get <OBIS>\n\r".to_string())
                    )?;
                    stdout.flush()?;
                    return Ok(());
                }
                let obis_str = args[1..].join(".");
                // 构造 GetRequest APDU (LN mode)
                let obis_parts: Vec<u8> = obis_str
                    .split('.')
                    .filter_map(|s| s.parse::<u8>().ok())
                    .collect();
                if obis_parts.len() != 6 {
                    queue!(
                        stdout,
                        style::Print("OBIS 需要 6 个字节 (a.b.c.d.e.f)\n\r".to_string())
                    )?;
                    stdout.flush()?;
                    return Ok(());
                }
                let apdu = [
                    0xC0,
                    0x01, // GetRequest-Normal
                    0x01, // invoke_id
                    0x00,
                    0x03, // class_id = Register(3)
                    obis_parts[0],
                    obis_parts[1],
                    obis_parts[2],
                    obis_parts[3],
                    obis_parts[4],
                    obis_parts[5],
                    0x02, // attribute_id = 2 (value)
                    0x01, // access = GET
                ];
                let processor = crate::dlms::create_dlms_processor(self.meter.clone());
                match processor.raw_apdu(&apdu) {
                    Ok(resp) => {
                        let hex = hex_encode(&resp);
                        queue!(stdout, style::Print(format!("响应: {}\n\r", hex)))?;
                    }
                    Err(e) => {
                        queue!(stdout, style::Print(format!("错误: {}\n\r", e)))?;
                    }
                }
            }
            "assoc" => {
                // 发送 AARQ
                let apdu = [0xE0, 0x00, 0x00, 0x00, 0x00];
                let processor = crate::dlms::create_dlms_processor(self.meter.clone());
                match processor.raw_apdu(&apdu) {
                    Ok(resp) => {
                        let hex = hex_encode(&resp);
                        queue!(stdout, style::Print(format!("AARE 响应: {}\n\r", hex)))?;
                    }
                    Err(e) => {
                        queue!(stdout, style::Print(format!("错误: {}\n\r", e)))?;
                    }
                }
            }
            _ => {
                queue!(
                    stdout,
                    style::Print(format!("未知 DLMS 子命令: {}\n\r", sub))
                )?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    /// 场景自动化: 从脚本文件批量执行命令
    /// 用法: run <script_file>
    /// 脚本格式: 每行一个命令, # 开头为注释
    fn cmd_run(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(stdout, style::Print(
                "场景自动化: run <script_file>\n\r  每行一个命令, # 开头为注释\n\r  支持 delay <ms> 命令\n\r"
                .to_string()
            ))?;
            stdout.flush()?;
            return Ok(());
        }

        let path = args.join(" ");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                queue!(
                    stdout,
                    style::Print(format!("无法读取文件 {}: {}\n\r", path, e))
                )?;
                stdout.flush()?;
                return Ok(());
            }
        };

        let mut cmd_count = 0;
        let mut err_count = 0;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // 支持 delay 命令
            if line.starts_with("delay ") {
                if let Some(ms_str) = line.strip_prefix("delay ") {
                    if let Ok(ms) = ms_str.parse::<u64>() {
                        queue!(stdout, style::Print(format!("  等待 {}ms...\n\r", ms)))?;
                        stdout.flush()?;
                        std::thread::sleep(std::time::Duration::from_millis(ms));
                        continue;
                    }
                }
            }

            queue!(stdout, style::Print(format!("> {}\n\r", line)))?;
            stdout.flush()?;
            cmd_count += 1;

            if let Err(e) = self.execute_command(line, stdout) {
                err_count += 1;
                queue!(stdout, style::Print(format!("  错误: {}\n\r", e)))?;
            }
        }

        queue!(
            stdout,
            style::Print(format!(
                "\n\r脚本执行完成: {} 命令, {} 错误\n\r",
                cmd_count, err_count
            ))
        )?;
        stdout.flush()?;
        Ok(())
    }
}

// ============================================================
// 辅助函数
// ============================================================

/// 十六进制编码
fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect()
}

/// 十六进制解码 (支持空格分隔)
fn hex_decode(s: &str) -> Vec<u8> {
    let hex_str: String = s.split_whitespace().collect();
    (0..hex_str.len())
        .step_by(2)
        .filter_map(|i| {
            if i + 2 <= hex_str.len() {
                u8::from_str_radix(&hex_str[i..i + 2], 16).ok()
            } else {
                None
            }
        })
        .collect()
}
