//! Interactive Shell (ASCII-only output)

use crate::{ChipType, MeterEvent, MeterHandle, Scenario};
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
    running: Arc<AtomicBool>,
}

impl Shell {
    pub fn new(meter: MeterHandle) -> Self {
        Self {
            meter,
            running: Arc::new(AtomicBool::new(true)),
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
        writeln!(stdout, "FeMeter v0.2 | type help")?;
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

            queue!(stdout, style::Print("\n\r> "), cursor::Show)?;
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
            style::Print("FeMeter v0.2 | type help\n\r"),
            style::Print("> "),
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
            "reset" => self.cmd_reset(stdout),
            "scenario" | "sc" => self.cmd_scenario(&parts[1..], stdout),
            "event" | "ev" | "events" => self.cmd_event(&parts[1..], stdout),
            "watch" | "w" => {
                self.cmd_watch(&parts[1..], stdout);
                Ok(())
            }
            "quit" | "exit" | "q" => {
                self.running.store(false, Ordering::Relaxed);
                Ok(())
            }
            _ => {
                queue!(
                    stdout,
                    style::Print(format!("Unknown command: {} (type help)\n\r", parts[0]))
                )?;
                let _ = stdout.flush();
                Ok(())
            }
        }
    }

    fn cmd_help(&self, stdout: &mut impl Write) -> Result<()> {
        let help = r#"
FeMeter v0.2
=========================================
 set ua/ub/uc <V>       voltage
 set ia/ib/ic <A>       current
 set angle-a/b/c <deg>  phase angle
 set freq <Hz>          frequency
 set pf <0~1>           power factor
 set 3p <V> <A> <Hz> <PF>  three-phase combo
 set noise on/off       noise
 set accel <rate>       time acceleration

 get voltage            phase + line voltage
 get current            phase + neutral current
 get power              active/reactive/apparent
 get energy             cumulative energy
 get freq               frequency
 get pf                 power factor
 get status             status word

 status                 full status table
 scenario <name>        normal/full/noload/overv/loss/overi/reverse
 event <type>           cover/terminal/magnetic/battery
 event list             event history
 watch [ms]             real-time monitor
 reset                  reset energy
 quit                   exit
"#;
        queue!(stdout, style::Print(help))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_scenario(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(stdout, style::Print("Usage: scenario <normal|full|noload|overv|underv|loss|overi|reverse|unbalanced>\n\r"))?;
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
                queue!(
                    stdout,
                    style::Print(format!("Unknown scenario: {}\n\r", args[0]))
                )?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.load_scenario(sc);
        queue!(
            stdout,
            style::Print(format!("Loaded scenario: {:?}\n\r", sc))
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_event(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.first().map(|s| s.to_lowercase()).as_deref() == Some("list") || args.is_empty() {
            let meter = self.meter.lock().expect("mutex poisoned");
            let events = meter.events();
            if events.is_empty() {
                queue!(stdout, style::Print("No events\n\r"))?;
            } else {
                queue!(
                    stdout,
                    style::Print(format!("Events ({} total):\n\r", events.len()))
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
            return Ok(());
        }

        let ev = match args[0].to_lowercase().as_str() {
            "cover" => MeterEvent::CoverOpen,
            "terminal" => MeterEvent::TerminalCoverOpen,
            "magnetic" => MeterEvent::MagneticTamper,
            "battery" => MeterEvent::BatteryLow,
            _ => {
                queue!(
                    stdout,
                    style::Print(format!("Unknown event: {}\n\r", args[0]))
                )?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.trigger_event(ev);
        queue!(
            stdout,
            style::Print(format!("Triggered event: {:?}\n\r", ev))
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_watch(&self, args: &[&str], stdout: &mut impl Write) {
        let interval = args
            .first()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(500);
        queue!(
            stdout,
            style::Print(format!(
                "Watch ({}ms interval, 10 rounds, Ctrl+C to stop):\n\r",
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
        queue!(stdout, style::Print("Watch ended\n\r")).ok();
        stdout.flush().ok();
    }

    fn cmd_status(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();
        let ev_str = if snap.active_events.is_empty() {
            "none".to_string()
        } else {
            snap.active_events
                .iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let status = format!(
            r#"
==========================================
  Chip: {:?} ({}-bit)  Freq: {:.2} Hz  Noise: {}  Accel: {:.0}x
  Events: {}
------------------------------------------
  Phase    V(V)      I(A)     Angle(deg)    PF
  -----  --------  --------  ----------  -----
  A       {:>8.2}  {:>8.2}  {:>10.1}  {:>5.3}
  B       {:>8.2}  {:>8.2}  {:>10.1}  {:>5.3}
  C       {:>8.2}  {:>8.2}  {:>10.1}  {:>5.3}
------------------------------------------
  P: {:>10.1} W   Q: {:>10.1} var   PF: {:.3}
  S: {:>10.1} VA
------------------------------------------
  Energy: {:.3} kWh / {:.3} kvarh
==========================================
"#,
            snap.chip,
            snap.chip.bits(),
            snap.freq,
            if meter.config().noise_enabled {
                "ON"
            } else {
                "OFF"
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
            queue!(
                stdout,
                style::Print("Usage: set <param> <value> | set 3p <V> <A> <Hz> <PF>\n\r")
            )?;
            stdout.flush()?;
            return Ok(());
        }
        let param = args[0].to_lowercase();
        let mut meter = self.meter.lock().expect("mutex poisoned");

        if param == "three-phase" || param == "threephase" || param == "3p" {
            if args.len() < 5 {
                queue!(
                    stdout,
                    style::Print("Usage: set 3p <Voltage> <Current> <Freq> <PF>\n\r")
                )?;
                queue!(stdout, style::Print("Example: set 3p 230 5 50 0.95\n\r"))?;
                stdout.flush()?;
                return Ok(());
            }
            let v: f64 = args[1].parse().unwrap_or(220.0);
            let i: f64 = args[2].parse().unwrap_or(5.0);
            let freq: f64 = args[3].parse().unwrap_or(50.0);
            let pf: f64 = args[4].parse().unwrap_or(0.95);

            let angle = pf.acos() * 180.0 / std::f64::consts::PI;

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

            queue!(
                stdout,
                style::Print(format!(
                    "3-phase set: {:.1}V {:.2}A {:.2}Hz PF={:.3} (angle={:.1}deg)\n\r",
                    v, i, freq, pf, angle
                ))
            )?;
            stdout.flush()?;
            return Ok(());
        }

        if args.len() < 2 {
            queue!(
                stdout,
                style::Print(format!("Usage: set {} <value>\n\r", param))
            )?;
            stdout.flush()?;
            return Ok(());
        }

        let value_str = args[1];
        let result = match param.as_str() {
            "ua" => {
                meter.set_voltage('a', value_str.parse().unwrap_or(220.0));
                format!(
                    "A-phase voltage = {:.2} V",
                    value_str.parse::<f64>().unwrap_or(220.0)
                )
            }
            "ub" => {
                meter.set_voltage('b', value_str.parse().unwrap_or(220.0));
                format!(
                    "B-phase voltage = {:.2} V",
                    value_str.parse::<f64>().unwrap_or(220.0)
                )
            }
            "uc" => {
                meter.set_voltage('c', value_str.parse().unwrap_or(220.0));
                format!(
                    "C-phase voltage = {:.2} V",
                    value_str.parse::<f64>().unwrap_or(220.0)
                )
            }
            "ia" => {
                meter.set_current('a', value_str.parse().unwrap_or(0.0));
                format!(
                    "A-phase current = {:.2} A",
                    value_str.parse::<f64>().unwrap_or(0.0)
                )
            }
            "ib" => {
                meter.set_current('b', value_str.parse().unwrap_or(0.0));
                format!(
                    "B-phase current = {:.2} A",
                    value_str.parse::<f64>().unwrap_or(0.0)
                )
            }
            "ic" => {
                meter.set_current('c', value_str.parse().unwrap_or(0.0));
                format!(
                    "C-phase current = {:.2} A",
                    value_str.parse::<f64>().unwrap_or(0.0)
                )
            }
            "angle-a" | "angle_a" => {
                meter.set_angle('a', value_str.parse().unwrap_or(0.0));
                format!(
                    "A-phase angle = {:.1} deg",
                    value_str.parse::<f64>().unwrap_or(0.0)
                )
            }
            "angle-b" | "angle_b" => {
                meter.set_angle('b', value_str.parse().unwrap_or(0.0));
                format!(
                    "B-phase angle = {:.1} deg",
                    value_str.parse::<f64>().unwrap_or(0.0)
                )
            }
            "angle-c" | "angle_c" => {
                meter.set_angle('c', value_str.parse().unwrap_or(0.0));
                format!(
                    "C-phase angle = {:.1} deg",
                    value_str.parse::<f64>().unwrap_or(0.0)
                )
            }
            "freq" => {
                meter.set_freq(value_str.parse().unwrap_or(50.0));
                format!(
                    "Frequency = {:.2} Hz",
                    value_str.parse::<f64>().unwrap_or(50.0)
                )
            }
            "pf" | "power-factor" => {
                let pf: f64 = value_str.parse().unwrap_or(0.95);
                if !(-1.0..=1.0).contains(&pf) {
                    "Error: PF must be -1.0 to 1.0".to_string()
                } else {
                    let angle = pf.acos() * 180.0 / std::f64::consts::PI;
                    meter.set_angle('a', angle);
                    meter.set_angle('b', angle);
                    meter.set_angle('c', angle);
                    format!("PF = {:.3} (angle auto-set to {:.1} deg)", pf, angle)
                }
            }
            "noise" => {
                let e = ["on", "1", "true"].contains(&value_str.to_lowercase().as_str());
                meter.set_noise(e);
                format!("Noise {}", if e { "ON" } else { "OFF" })
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
                _ => format!("Unknown: {}", value_str),
            },
            "accel" => {
                let a: f64 = value_str.parse().unwrap_or(1.0);
                meter.set_time_accel(a);
                format!("Time accel = {:.0}x", a)
            }
            _ => format!("Unknown param: {}", param),
        };
        queue!(stdout, style::Print(format!("{}\n\r", result)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_get(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            queue!(
                stdout,
                style::Print("Usage: get <voltage|current|angle|power|energy|freq|pf|status>\n\r")
            )?;
            stdout.flush()?;
            return Ok(());
        }

        let sub = args[0].to_lowercase();
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();

        match sub.as_str() {
            "voltage" | "volt" | "v" => {
                let v_ab = (snap.phase_a.voltage * snap.phase_a.voltage
                    + snap.phase_b.voltage * snap.phase_b.voltage
                    - 2.0
                        * snap.phase_a.voltage
                        * snap.phase_b.voltage
                        * ((snap.phase_a.angle - snap.phase_b.angle) * std::f64::consts::PI
                            / 180.0)
                            .cos())
                .sqrt();
                let v_bc = (snap.phase_b.voltage * snap.phase_b.voltage
                    + snap.phase_c.voltage * snap.phase_c.voltage
                    - 2.0
                        * snap.phase_b.voltage
                        * snap.phase_c.voltage
                        * ((snap.phase_b.angle - snap.phase_c.angle) * std::f64::consts::PI
                            / 180.0)
                            .cos())
                .sqrt();
                let v_ca = (snap.phase_c.voltage * snap.phase_c.voltage
                    + snap.phase_a.voltage * snap.phase_a.voltage
                    - 2.0
                        * snap.phase_c.voltage
                        * snap.phase_a.voltage
                        * ((snap.phase_c.angle - snap.phase_a.angle) * std::f64::consts::PI
                            / 180.0)
                            .cos())
                .sqrt();

                let output = format!(
                    "\n\r  [Voltage]\n\r  Phase  Phase(V)  Line(V)\n\r  -----  --------  --------\n\r  A        {:>8.2}  {:>8.2}\n\r  B        {:>8.2}  {:>8.2}\n\r  C        {:>8.2}  {:>8.2}\n\r  (Uab/Ubc/Uca)\n\r\n",
                    snap.phase_a.voltage,
                    v_ab,
                    snap.phase_b.voltage,
                    v_bc,
                    snap.phase_c.voltage,
                    v_ca
                );
                queue!(stdout, style::Print(output))?;
            }
            "current" | "curr" | "i" => {
                let i_n = ((snap.phase_a.current
                    * (snap.phase_a.angle * std::f64::consts::PI / 180.0).cos()
                    + snap.phase_b.current
                        * (snap.phase_b.angle * std::f64::consts::PI / 180.0).cos()
                    + snap.phase_c.current
                        * (snap.phase_c.angle * std::f64::consts::PI / 180.0).cos())
                .powi(2)
                    + (snap.phase_a.current
                        * (snap.phase_a.angle * std::f64::consts::PI / 180.0).sin()
                        + snap.phase_b.current
                            * (snap.phase_b.angle * std::f64::consts::PI / 180.0).sin()
                        + snap.phase_c.current
                            * (snap.phase_c.angle * std::f64::consts::PI / 180.0).sin())
                    .powi(2))
                .sqrt();

                let output = format!(
                    "\n\r  [Current]\n\r  Phase  Current(A)\n\r  -----  ----------\n\r  A        {:>8.3}\n\r  B        {:>8.3}\n\r  C        {:>8.3}\n\r  N        {:>8.3}  (neutral)\n\r\n",
                    snap.phase_a.current, snap.phase_b.current, snap.phase_c.current, i_n
                );
                queue!(stdout, style::Print(output))?;
            }
            "angle" | "ang" | "a" => {
                let output = format!(
                    "\n\r  [Angle]\n\r  Phase  Angle(deg)\n\r  -----  ----------\n\r  A        {:>8.1}\n\r  B        {:>8.1}\n\r  C        {:>8.1}\n\r  (Normal: A=0, B=-120, C=120)\n\r\n",
                    snap.phase_a.angle, snap.phase_b.angle, snap.phase_c.angle
                );
                queue!(stdout, style::Print(output))?;
            }
            "power" | "pow" | "p" => {
                let output = format!(
                    "\n\r  [Power]\n\r  Phase        W(W)     var(var)    VA(VA)      PF\n\r  -----  ----------  ----------  ---------  -------\n\r  A        {:>8.1}  {:>10.1}  {:>9.1}  {:>7.3}\n\r  B        {:>8.1}  {:>10.1}  {:>9.1}  {:>7.3}\n\r  C        {:>8.1}  {:>10.1}  {:>9.1}  {:>7.3}\n\r  Total    {:>8.1}  {:>10.1}  {:>9.1}  {:>7.3}\n\r\n",
                    snap.computed.p_a,
                    snap.computed.q_a,
                    snap.computed.s_a,
                    snap.computed.pf_a,
                    snap.computed.p_b,
                    snap.computed.q_b,
                    snap.computed.s_b,
                    snap.computed.pf_b,
                    snap.computed.p_c,
                    snap.computed.q_c,
                    snap.computed.s_c,
                    snap.computed.pf_c,
                    snap.computed.p_total,
                    snap.computed.q_total,
                    snap.computed.s_total,
                    snap.computed.pf_total
                );
                queue!(stdout, style::Print(output))?;
            }
            "energy" | "en" | "e" => {
                let output = format!(
                    "\n\r  [Energy]\n\r  Phase       kWh       kvarh\n\r  -----  ---------  ---------\n\r  A        {:>9.4}  {:>9.4}\n\r  B        {:>9.4}  {:>9.4}\n\r  C        {:>9.4}  {:>9.4}\n\r  Total    {:>9.4}  {:>9.4}\n\r\n",
                    snap.energy.wh_a / 1000.0,
                    snap.energy.varh_a / 1000.0,
                    snap.energy.wh_b / 1000.0,
                    snap.energy.varh_b / 1000.0,
                    snap.energy.wh_c / 1000.0,
                    snap.energy.varh_c / 1000.0,
                    snap.energy.wh_total / 1000.0,
                    snap.energy.varh_total / 1000.0
                );
                queue!(stdout, style::Print(output))?;
            }
            "frequency" | "freq" | "f" => {
                let output = format!("\n\r  Freq: {:.3} Hz\n\r\n", snap.freq);
                queue!(stdout, style::Print(output))?;
            }
            "power-factor" | "pf" => {
                let output = format!(
                    "\n\r  [Power Factor]\n\r  Phase      PF\n\r  -----  -------\n\r  A       {:>7.4}\n\r  B       {:>7.4}\n\r  C       {:>7.4}\n\r  Total   {:>7.4}\n\r\n",
                    snap.computed.pf_a,
                    snap.computed.pf_b,
                    snap.computed.pf_c,
                    snap.computed.pf_total
                );
                queue!(stdout, style::Print(output))?;
            }
            "status-word" | "status" | "sw" => {
                let sw = self.compute_status_word(&snap);
                let mut output = format!("\n\r  [Status Word] 0x{:08X}\n\r", sw);

                if sw == 0 {
                    output.push_str("  OK - All parameters normal\n\r");
                } else {
                    if sw & 0x01 != 0 {
                        output.push_str("  ! A-phase voltage loss (< 10V)\n\r");
                    }
                    if sw & 0x02 != 0 {
                        output.push_str("  ! B-phase voltage loss (< 10V)\n\r");
                    }
                    if sw & 0x04 != 0 {
                        output.push_str("  ! C-phase voltage loss (< 10V)\n\r");
                    }
                    if sw & 0x08 != 0 {
                        output.push_str("  ! A-phase overvoltage (> 264V)\n\r");
                    }
                    if sw & 0x10 != 0 {
                        output.push_str("  ! B-phase overvoltage (> 264V)\n\r");
                    }
                    if sw & 0x20 != 0 {
                        output.push_str("  ! C-phase overvoltage (> 264V)\n\r");
                    }
                    if sw & 0x40 != 0 {
                        output.push_str("  ! A-phase undervoltage (< 198V)\n\r");
                    }
                    if sw & 0x80 != 0 {
                        output.push_str("  ! B-phase undervoltage (< 198V)\n\r");
                    }
                    if sw & 0x100 != 0 {
                        output.push_str("  ! C-phase undervoltage (< 198V)\n\r");
                    }
                    if sw & 0x200 != 0 {
                        output.push_str("  ! A-phase overcurrent (> 60A)\n\r");
                    }
                    if sw & 0x400 != 0 {
                        output.push_str("  ! B-phase overcurrent (> 60A)\n\r");
                    }
                    if sw & 0x800 != 0 {
                        output.push_str("  ! C-phase overcurrent (> 60A)\n\r");
                    }
                    if sw & 0x1000 != 0 {
                        output.push_str("  ! Current imbalance (> 20%)\n\r");
                    }
                    if sw & 0x2000 != 0 {
                        output.push_str("  ! Voltage imbalance (> 2%)\n\r");
                    }
                    if sw & 0x4000 != 0 {
                        output.push_str("  ! Phase sequence error\n\r");
                    }
                    if sw & 0x8000 != 0 {
                        output.push_str("  ! Reverse power\n\r");
                    }
                }
                output.push('\n');
                queue!(stdout, style::Print(output))?;
            }
            _ => {
                queue!(stdout, style::Print(format!("Unknown query: {}\n\r", sub)))?;
                queue!(
                    stdout,
                    style::Print(
                        "Available: voltage, current, angle, power, energy, freq, pf, status\n\r"
                    )
                )?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn compute_status_word(&self, snap: &crate::MeterSnapshot) -> u32 {
        let mut sw: u32 = 0;

        if snap.phase_a.voltage < 10.0 {
            sw |= 0x01;
        }
        if snap.phase_b.voltage < 10.0 {
            sw |= 0x02;
        }
        if snap.phase_c.voltage < 10.0 {
            sw |= 0x04;
        }

        if snap.phase_a.voltage > 264.0 {
            sw |= 0x08;
        }
        if snap.phase_b.voltage > 264.0 {
            sw |= 0x10;
        }
        if snap.phase_c.voltage > 264.0 {
            sw |= 0x20;
        }

        if snap.phase_a.voltage < 198.0 && snap.phase_a.voltage >= 10.0 {
            sw |= 0x40;
        }
        if snap.phase_b.voltage < 198.0 && snap.phase_b.voltage >= 10.0 {
            sw |= 0x80;
        }
        if snap.phase_c.voltage < 198.0 && snap.phase_c.voltage >= 10.0 {
            sw |= 0x100;
        }

        if snap.phase_a.current > 60.0 {
            sw |= 0x200;
        }
        if snap.phase_b.current > 60.0 {
            sw |= 0x400;
        }
        if snap.phase_c.current > 60.0 {
            sw |= 0x800;
        }

        let i_max = snap
            .phase_a
            .current
            .max(snap.phase_b.current)
            .max(snap.phase_c.current);
        let i_min = snap
            .phase_a
            .current
            .min(snap.phase_b.current)
            .min(snap.phase_c.current);
        let i_avg = (snap.phase_a.current + snap.phase_b.current + snap.phase_c.current) / 3.0;
        if i_avg > 0.0 && (i_max - i_min) / i_avg > 0.2 {
            sw |= 0x1000;
        }

        let v_max = snap
            .phase_a
            .voltage
            .max(snap.phase_b.voltage)
            .max(snap.phase_c.voltage);
        let v_min = snap
            .phase_a
            .voltage
            .min(snap.phase_b.voltage)
            .min(snap.phase_c.voltage);
        let v_avg = (snap.phase_a.voltage + snap.phase_b.voltage + snap.phase_c.voltage) / 3.0;
        if v_avg > 0.0 && (v_max - v_min) / v_avg > 0.02 {
            sw |= 0x2000;
        }

        let a_ok = snap.phase_a.angle.abs() < 10.0;
        let b_ok = (snap.phase_b.angle + 120.0).abs() < 10.0;
        let c_ok = (snap.phase_c.angle - 120.0).abs() < 10.0;
        if !(a_ok && b_ok && c_ok) {
            sw |= 0x4000;
        }

        if snap.computed.p_total < 0.0 {
            sw |= 0x8000;
        }

        sw
    }

    fn cmd_reset(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.reset_energy();
        queue!(stdout, style::Print("Energy reset\n\r"))?;
        stdout.flush()?;
        Ok(())
    }
}
