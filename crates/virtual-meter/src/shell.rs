//! Interactive Shell — compact, colored output

use crate::{ChipType, MeterEvent, MeterHandle, Scenario};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::style;
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, queue};
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct Shell {
    meter: MeterHandle,
    running: Arc<AtomicBool>,
}

// ── Color helpers ──

fn cyan(text: &str) -> String {
    format!("\x1b[36m{}\x1b[0m", text)
}
fn yellow(text: &str) -> String {
    format!("\x1b[33m{}\x1b[0m", text)
}
fn green(text: &str) -> String {
    format!("\x1b[32m{}\x1b[0m", text)
}
fn red(text: &str) -> String {
    format!("\x1b[31m{}\x1b[0m", text)
}
fn grey(text: &str) -> String {
    format!("\x1b[90m{}\x1b[0m", text)
}
fn white(text: &str) -> String {
    format!("\x1b[97m{}\x1b[0m", text)
}
fn dim(text: &str) -> String {
    format!("\x1b[2m{}\x1b[0m", text)
}

fn nl() -> &'static str {
    "\n\r"
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
        println!("{} {}", cyan("⚡ FeMeter v0.2"), dim("| type help"));
        print!("{} ", cyan(">"));
        io::stdout().flush()?;
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            self.execute_command(line.trim(), &mut stdout, false)?;
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            print!("{} ", cyan(">"));
            io::stdout().flush()?;
        }
        Ok(())
    }

    fn run_raw_mode(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        let mut input = String::new();
        let mut history: Vec<String> = Vec::new();
        let mut history_index = 0;

        // Startup banner
        queue!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0),
            style::Print(format!(
                "{} {}{}",
                cyan("⚡ FeMeter v0.2"),
                dim("| ATT7022E"),
                nl()
            )),
            style::Print(cyan("> ")),
        )?;
        stdout.flush()?;

        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
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
                queue!(stdout, style::Print(format!("{}{}", nl(), cyan("> "))))?;
                stdout.flush()?;
                continue;
            }
            if !history.contains(&cmd.to_string()) {
                history.push(cmd.to_string());
                history_index = history.len();
            }
            self.execute_command(cmd, &mut stdout, true)?;
        }

        terminal::disable_raw_mode()?;
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

    fn print_prompt(&self, stdout: &mut impl Write) -> Result<()> {
        queue!(stdout, style::Print(cyan("> ")))?;
        stdout.flush()?;
        Ok(())
    }

    fn execute_command(&self, input: &str, stdout: &mut impl Write, raw: bool) -> Result<()> {
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
                let out = format!("{}{}{}", red("  ✗ unknown: "), parts[0], nl());
                queue!(stdout, style::Print(out))?;
                if raw {
                    self.print_prompt(stdout)?;
                }
                Ok(())
            }
        }
    }

    // ── Commands ──

    fn cmd_help(&self, stdout: &mut impl Write) -> Result<()> {
        let out = format!(
            r#"{g} ⚡ set {d}─────────────────────
   ua/ub/uc <V>     voltage
   ia/ib/ic <A>     current
   angle-a/b/c <°>  phase angle
   freq <Hz>        frequency
   pf <0~1>         power factor
   3p <V A Hz PF>   three-phase combo
   noise on/off     noise sim
   accel <rate>     time accel
{g} ⚡ get {d}─────────────────────
   voltage   phase + line V
   current   phase + neutral I
   power     P / Q / S
   energy    kWh / kvarh
   freq      Hz
   pf        power factor
   status    status word
{g} ⚡ other {d}──────────────────
   status          full table
   scenario <name> normal/full/noload/overv/loss/overi/reverse
   event <type>    cover/terminal/magnetic/battery
   watch [ms]      live monitor
   reset           reset energy
   quit            exit
"#,
            g = cyan("⚡"),
            d = grey("─"),
        );
        queue!(stdout, style::Print(out))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_scenario(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            let out = format!("{}{}\n", grey("  usage: scenario <name>"), nl());
            queue!(stdout, style::Print(out))?;
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
                let out = format!("{}{}{}\n", red("  ✗ unknown: "), args[0], nl());
                queue!(stdout, style::Print(out))?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.load_scenario(sc);
        let out = format!(
            "{} {}\n",
            grey("  → scenario:"),
            yellow(&format!("{:?}", sc))
        );
        queue!(stdout, style::Print(out))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_event(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.first().map(|s| s.to_lowercase()).as_deref() == Some("list") || args.is_empty() {
            let meter = self.meter.lock().expect("mutex poisoned");
            let events = meter.events();
            if events.is_empty() {
                let out = format!("{}{}\n", grey("  no events"), nl());
                queue!(stdout, style::Print(out))?;
            } else {
                let mut out = format!(
                    "{}{}\n",
                    cyan("  Events"),
                    grey(&format!(" ({}) ──", events.len()))
                );
                for e in events.iter().rev().take(20) {
                    out.push_str(&format!(
                        "   {}  {:<16} {}\n",
                        grey(&e.timestamp.format("%H:%M:%S").to_string()),
                        white(&format!("{:?}", e.event)),
                        &e.description,
                    ));
                }
                queue!(stdout, style::Print(out))?;
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
                let out = format!("{}{}{}\n", red("  ✗ unknown: "), args[0], nl());
                queue!(stdout, style::Print(out))?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.trigger_event(ev);
        let out = format!("{} {}\n", grey("  → event:"), yellow(&format!("{:?}", ev)));
        queue!(stdout, style::Print(out))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_watch(&self, args: &[&str], stdout: &mut impl Write) {
        let interval = args
            .first()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(500);
        for _i in 0..10 {
            std::thread::sleep(Duration::from_millis(interval));
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            let mut meter = self.meter.lock().expect("mutex poisoned");
            let snap = meter.snapshot();
            drop(meter);

            let line1 = format!(
                " {}{} A:{}{} {}{} | B:{}{} {}{} | C:{}{} {}{}",
                grey("["),
                grey(&snap.timestamp.format("%H:%M:%S").to_string()),
                white(&format!("{:.1}", snap.phase_a.voltage)),
                grey("V"),
                white(&format!("{:.2}", snap.phase_a.current)),
                grey("A"),
                white(&format!("{:.1}", snap.phase_b.voltage)),
                grey("V"),
                white(&format!("{:.2}", snap.phase_b.current)),
                grey("A"),
                white(&format!("{:.1}", snap.phase_c.voltage)),
                grey("V"),
                white(&format!("{:.2}", snap.phase_c.current)),
                grey("A"),
            );
            let line2 = format!(
                "  P:{}{} Q:{}{} S:{}{} PF:{}",
                white(&format!("{:.1}", snap.computed.p_total)),
                grey("W"),
                white(&format!("{:.1}", snap.computed.q_total)),
                grey("var"),
                white(&format!("{:.1}", snap.computed.s_total)),
                grey("VA"),
                white(&format!("{:.3}", snap.computed.pf_total)),
            );

            queue!(
                stdout,
                terminal::Clear(ClearType::All),
                cursor::MoveTo(0, 0),
                style::Print(line1),
                style::Print(nl()),
                style::Print(line2),
            )
            .ok();
            stdout.flush().ok();
        }
        queue!(stdout, style::Print(format!("{}{}", nl(), cyan("> ")))).ok();
        stdout.flush().ok();
    }

    fn cmd_status(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();
        let cfg = meter.config();

        let ev_str = if snap.active_events.is_empty() {
            green("✓").to_string()
        } else {
            red(&format!(
                "! {}",
                snap.active_events
                    .iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<_>>()
                    .join(",")
            ))
            .to_string()
        };

        let noise = if cfg.noise_enabled { "on" } else { "off" };
        let accel = cfg.time_accel;

        let out = format!(
            "{}\n   {}  {:.2}{}  noise:{}  accel:{}{}\n{}\n        {}    {}    {}    {}\n   A  {:>8.2}  {:>6.3}  {:>6.1}  {:>6.3}\n   B  {:>8.2}  {:>6.3}  {:>6.1}  {:>6.3}\n   C  {:>8.2}  {:>6.3}  {:>6.1}  {:>6.3}\n{}\n   P {:>8.1}{}   Q {:>8.1}{}   S {:>8.1}{}   PF {}\n   E {}{} / {}{}\n{}\n",
            cyan("  ══ FeMeter ═══════════════════════════════"),
            yellow(&format!("{:?}", snap.chip)),
            white(&format!("{:.2}", snap.freq)),
            grey("Hz"),
            grey(noise),
            grey(&format!("{:.0}x", accel)),
            ev_str,
            grey("  ──────────────────────────────────────────"),
            grey("V"), grey("A"), grey("°"), grey("PF"),
            white(&format!("{:.2}", snap.phase_a.voltage)),
            white(&format!("{:.3}", snap.phase_a.current)),
            white(&format!("{:.1}", snap.phase_a.angle)),
            white(&format!("{:.3}", snap.computed.pf_a)),
            white(&format!("{:.2}", snap.phase_b.voltage)),
            white(&format!("{:.3}", snap.phase_b.current)),
            white(&format!("{:.1}", snap.phase_b.angle)),
            white(&format!("{:.3}", snap.computed.pf_b)),
            white(&format!("{:.2}", snap.phase_c.voltage)),
            white(&format!("{:.3}", snap.phase_c.current)),
            white(&format!("{:.1}", snap.phase_c.angle)),
            white(&format!("{:.3}", snap.computed.pf_c)),
            grey("  ──────────────────────────────────────────"),
            white(&format!("{:.1}", snap.computed.p_total)),
            grey("W"),
            white(&format!("{:.1}", snap.computed.q_total)),
            grey("var"),
            white(&format!("{:.1}", snap.computed.s_total)),
            grey("VA"),
            white(&format!("{:.3}", snap.computed.pf_total)),
            white(&format!("{:.3}", snap.energy.wh_total / 1000.0)),
            grey("kWh"),
            white(&format!("{:.3}", snap.energy.varh_total / 1000.0)),
            grey("kvarh"),
            grey("  ══════════════════════════════════════════"),
        );
        queue!(stdout, style::Print(out))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_set(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            let out = format!(
                "{}{}\n",
                grey("  usage: set <param> <value> | set 3p <V A Hz PF>"),
                nl()
            );
            queue!(stdout, style::Print(out))?;
            stdout.flush()?;
            return Ok(());
        }
        let param = args[0].to_lowercase();
        let mut meter = self.meter.lock().expect("mutex poisoned");

        if param == "three-phase" || param == "threephase" || param == "3p" {
            if args.len() < 5 {
                let out = format!("{}{}\n", grey("  usage: set 3p <V> <A> <Hz> <PF>"), nl());
                queue!(stdout, style::Print(out))?;
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

            let out = format!(
                "{} {}{} {}{} {}{} {}={}\n",
                grey("  → 3P:"),
                white(&format!("{:.1}", v)),
                grey("V"),
                white(&format!("{:.2}", i)),
                grey("A"),
                white(&format!("{:.1}", freq)),
                grey("Hz"),
                grey("PF"),
                white(&format!("{:.2}", pf)),
            );
            queue!(stdout, style::Print(out))?;
            stdout.flush()?;
            return Ok(());
        }

        if args.len() < 2 {
            let out = format!(
                "{}{}\n",
                grey(&format!("  usage: set {} <value>", param)),
                nl()
            );
            queue!(stdout, style::Print(out))?;
            stdout.flush()?;
            return Ok(());
        }

        let value_str = args[1];
        let result = match param.as_str() {
            "ua" => {
                let v = value_str.parse().unwrap_or(220.0);
                meter.set_voltage('a', v);
                format!(
                    "{} A-phase: {}{}",
                    grey("  →"),
                    white(&format!("{:.2}", v)),
                    grey("V")
                )
            }
            "ub" => {
                let v = value_str.parse().unwrap_or(220.0);
                meter.set_voltage('b', v);
                format!(
                    "{} B-phase: {}{}",
                    grey("  →"),
                    white(&format!("{:.2}", v)),
                    grey("V")
                )
            }
            "uc" => {
                let v = value_str.parse().unwrap_or(220.0);
                meter.set_voltage('c', v);
                format!(
                    "{} C-phase: {}{}",
                    grey("  →"),
                    white(&format!("{:.2}", v)),
                    grey("V")
                )
            }
            "ia" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_current('a', v);
                format!(
                    "{} A-phase: {}{}",
                    grey("  →"),
                    white(&format!("{:.3}", v)),
                    grey("A")
                )
            }
            "ib" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_current('b', v);
                format!(
                    "{} B-phase: {}{}",
                    grey("  →"),
                    white(&format!("{:.3}", v)),
                    grey("A")
                )
            }
            "ic" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_current('c', v);
                format!(
                    "{} C-phase: {}{}",
                    grey("  →"),
                    white(&format!("{:.3}", v)),
                    grey("A")
                )
            }
            "angle-a" | "angle_a" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_angle('a', v);
                format!(
                    "{} A-angle: {}{}",
                    grey("  →"),
                    white(&format!("{:.1}", v)),
                    grey("°")
                )
            }
            "angle-b" | "angle_b" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_angle('b', v);
                format!(
                    "{} B-angle: {}{}",
                    grey("  →"),
                    white(&format!("{:.1}", v)),
                    grey("°")
                )
            }
            "angle-c" | "angle_c" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_angle('c', v);
                format!(
                    "{} C-angle: {}{}",
                    grey("  →"),
                    white(&format!("{:.1}", v)),
                    grey("°")
                )
            }
            "freq" => {
                let v = value_str.parse().unwrap_or(50.0);
                meter.set_freq(v);
                format!(
                    "{} freq: {}{}",
                    grey("  →"),
                    white(&format!("{:.2}", v)),
                    grey("Hz")
                )
            }
            "pf" | "power-factor" => {
                let pf: f64 = value_str.parse().unwrap_or(0.95);
                if !(-1.0..=1.0).contains(&pf) {
                    format!("{} PF must be -1~1", red("  ✗"))
                } else {
                    let angle = pf.acos() * 180.0 / std::f64::consts::PI;
                    meter.set_angle('a', angle);
                    meter.set_angle('b', angle);
                    meter.set_angle('c', angle);
                    format!(
                        "{} PF: {} ({}{}{})",
                        grey("  →"),
                        white(&format!("{:.3}", pf)),
                        grey("angle"),
                        white(&format!("{:.1}", angle)),
                        grey("°")
                    )
                }
            }
            "noise" => {
                let e = ["on", "1", "true"].contains(&value_str.to_lowercase().as_str());
                meter.set_noise(e);
                format!(
                    "{} noise: {}",
                    grey("  →"),
                    if e { green("on") } else { dim("off") }
                )
            }
            "chip" => match value_str.to_lowercase().as_str() {
                "att7022e" | "att7022" => {
                    meter.set_chip(ChipType::ATT7022E);
                    format!("{} chip: {}", grey("  →"), yellow("ATT7022E"))
                }
                "rn8302b" | "rn8302" => {
                    meter.set_chip(ChipType::RN8302B);
                    format!("{} chip: {}", grey("  →"), yellow("RN8302B"))
                }
                _ => format!("{} unknown: {}", red("  ✗"), value_str),
            },
            "accel" => {
                let a: f64 = value_str.parse().unwrap_or(1.0);
                meter.set_time_accel(a);
                format!(
                    "{} accel: {}{}",
                    grey("  →"),
                    white(&format!("{:.0}", a)),
                    grey("x")
                )
            }
            _ => format!("{} unknown: {}", red("  ✗"), param),
        };
        queue!(stdout, style::Print(format!("{}\n", result)))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_get(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            let out = format!(
                "{}{}\n",
                grey("  usage: get <voltage|current|power|energy|freq|pf|status>"),
                nl()
            );
            queue!(stdout, style::Print(out))?;
            stdout.flush()?;
            return Ok(());
        }

        let sub = args[0].to_lowercase();
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();

        let out = match sub.as_str() {
            "voltage" | "volt" | "v" => {
                let v_ab = (snap.phase_a.voltage.powi(2) + snap.phase_b.voltage.powi(2)
                    - 2.0
                        * snap.phase_a.voltage
                        * snap.phase_b.voltage
                        * ((snap.phase_a.angle - snap.phase_b.angle) * std::f64::consts::PI
                            / 180.0)
                            .cos())
                .sqrt();
                let v_bc = (snap.phase_b.voltage.powi(2) + snap.phase_c.voltage.powi(2)
                    - 2.0
                        * snap.phase_b.voltage
                        * snap.phase_c.voltage
                        * ((snap.phase_b.angle - snap.phase_c.angle) * std::f64::consts::PI
                            / 180.0)
                            .cos())
                .sqrt();
                let v_ca = (snap.phase_c.voltage.powi(2) + snap.phase_a.voltage.powi(2)
                    - 2.0
                        * snap.phase_c.voltage
                        * snap.phase_a.voltage
                        * ((snap.phase_c.angle - snap.phase_a.angle) * std::f64::consts::PI
                            / 180.0)
                            .cos())
                .sqrt();

                format!(
                    "{}{}\n   {}  {}{}  {}  {}{}\n   {}  {}{}  {}  {}{}\n   {}  {}{}  {}  {}{}\n",
                    cyan("  Voltage "),
                    grey("──────────────"),
                    grey("A"),
                    white(&format!("{:>8.2}", snap.phase_a.voltage)),
                    grey("V"),
                    grey("AB"),
                    white(&format!("{:>8.1}", v_ab)),
                    grey("V"),
                    grey("B"),
                    white(&format!("{:>8.2}", snap.phase_b.voltage)),
                    grey("V"),
                    grey("BC"),
                    white(&format!("{:>8.1}", v_bc)),
                    grey("V"),
                    grey("C"),
                    white(&format!("{:>8.2}", snap.phase_c.voltage)),
                    grey("V"),
                    grey("CA"),
                    white(&format!("{:>8.1}", v_ca)),
                    grey("V"),
                )
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

                format!(
                    "{}{}\n   {}  {}{}\n   {}  {}{}\n   {}  {}{}\n   {}  {}{}",
                    cyan("  Current "),
                    grey("──────────────"),
                    grey("A"),
                    white(&format!("{:>8.3}", snap.phase_a.current)),
                    grey("A"),
                    grey("B"),
                    white(&format!("{:>8.3}", snap.phase_b.current)),
                    grey("A"),
                    grey("C"),
                    white(&format!("{:>8.3}", snap.phase_c.current)),
                    grey("A"),
                    grey("N"),
                    white(&format!("{:>8.3}", i_n)),
                    grey("A"),
                )
            }
            "power" | "pow" | "p" => {
                format!(
                    "{}{}\n         {}        {}       {}      {}\n   A  {:>8.1}  {:>8.1}  {:>8.1}  {:>6.3}\n   B  {:>8.1}  {:>8.1}  {:>8.1}  {:>6.3}\n   C  {:>8.1}  {:>8.1}  {:>8.1}  {:>6.3}\n   {}  {:>8.1}  {:>8.1}  {:>8.1}  {:>6.3}\n",
                    cyan("  Power "),
                    grey("────────────────────────────"),
                    grey("W"), grey("var"), grey("VA"), grey("PF"),
                    white(&format!("{:.1}", snap.computed.p_a)),
                    white(&format!("{:.1}", snap.computed.q_a)),
                    white(&format!("{:.1}", snap.computed.s_a)),
                    white(&format!("{:.3}", snap.computed.pf_a)),
                    white(&format!("{:.1}", snap.computed.p_b)),
                    white(&format!("{:.1}", snap.computed.q_b)),
                    white(&format!("{:.1}", snap.computed.s_b)),
                    white(&format!("{:.3}", snap.computed.pf_b)),
                    white(&format!("{:.1}", snap.computed.p_c)),
                    white(&format!("{:.1}", snap.computed.q_c)),
                    white(&format!("{:.1}", snap.computed.s_c)),
                    white(&format!("{:.3}", snap.computed.pf_c)),
                    grey("Σ"),
                    white(&format!("{:.1}", snap.computed.p_total)),
                    white(&format!("{:.1}", snap.computed.q_total)),
                    white(&format!("{:.1}", snap.computed.s_total)),
                    white(&format!("{:.3}", snap.computed.pf_total)),
                )
            }
            "energy" | "en" | "e" => {
                format!(
                    "{}{}\n       {}      {}\n   A  {}  {}\n   B  {}  {}\n   C  {}  {}\n   {}  {}  {}\n",
                    cyan("  Energy "),
                    grey("────────────"),
                    grey("kWh"), grey("kvarh"),
                    white(&format!("{:>8.4}", snap.energy.wh_a / 1000.0)),
                    white(&format!("{:>8.4}", snap.energy.varh_a / 1000.0)),
                    white(&format!("{:>8.4}", snap.energy.wh_b / 1000.0)),
                    white(&format!("{:>8.4}", snap.energy.varh_b / 1000.0)),
                    white(&format!("{:>8.4}", snap.energy.wh_c / 1000.0)),
                    white(&format!("{:>8.4}", snap.energy.varh_c / 1000.0)),
                    grey("Σ"),
                    white(&format!("{:>8.4}", snap.energy.wh_total / 1000.0)),
                    white(&format!("{:>8.4}", snap.energy.varh_total / 1000.0)),
                )
            }
            "frequency" | "freq" | "f" => {
                format!(
                    "  {} {}{}\n",
                    grey("Freq:"),
                    white(&format!("{:.3}", snap.freq)),
                    grey("Hz")
                )
            }
            "power-factor" | "pf" => {
                format!(
                    "{}{}\n   {}  {:>7.4}\n   {}  {:>7.4}\n   {}  {:>7.4}\n   {}  {:>7.4}\n",
                    cyan("  PF "),
                    grey("────────────"),
                    grey("A"),
                    white(&format!("{:.4}", snap.computed.pf_a)),
                    grey("B"),
                    white(&format!("{:.4}", snap.computed.pf_b)),
                    grey("C"),
                    white(&format!("{:.4}", snap.computed.pf_c)),
                    grey("Σ"),
                    white(&format!("{:.4}", snap.computed.pf_total)),
                )
            }
            "status-word" | "status" | "sw" => {
                let sw = self.compute_status_word(&snap);
                if sw == 0 {
                    format!(
                        "  {} 0x{:08X} {} {}\n",
                        grey("Status:"),
                        sw,
                        green("✓"),
                        green("OK")
                    )
                } else {
                    let mut s =
                        format!("  {} 0x{:08X} {}{}\n", grey("Status:"), sw, red("✗"), nl());
                    if sw & 0x01 != 0 {
                        s.push_str(&format!("    {} A-phase loss (<10V)\n", red("!")));
                    }
                    if sw & 0x02 != 0 {
                        s.push_str(&format!("    {} B-phase loss (<10V)\n", red("!")));
                    }
                    if sw & 0x04 != 0 {
                        s.push_str(&format!("    {} C-phase loss (<10V)\n", red("!")));
                    }
                    if sw & 0x08 != 0 {
                        s.push_str(&format!("    {} A-phase overvoltage (>264V)\n", red("!")));
                    }
                    if sw & 0x10 != 0 {
                        s.push_str(&format!("    {} B-phase overvoltage (>264V)\n", red("!")));
                    }
                    if sw & 0x20 != 0 {
                        s.push_str(&format!("    {} C-phase overvoltage (>264V)\n", red("!")));
                    }
                    if sw & 0x40 != 0 {
                        s.push_str(&format!("    {} A-phase undervoltage (<198V)\n", red("!")));
                    }
                    if sw & 0x80 != 0 {
                        s.push_str(&format!("    {} B-phase undervoltage (<198V)\n", red("!")));
                    }
                    if sw & 0x100 != 0 {
                        s.push_str(&format!("    {} C-phase undervoltage (<198V)\n", red("!")));
                    }
                    if sw & 0x200 != 0 {
                        s.push_str(&format!("    {} A-phase overcurrent (>60A)\n", red("!")));
                    }
                    if sw & 0x400 != 0 {
                        s.push_str(&format!("    {} B-phase overcurrent (>60A)\n", red("!")));
                    }
                    if sw & 0x800 != 0 {
                        s.push_str(&format!("    {} C-phase overcurrent (>60A)\n", red("!")));
                    }
                    if sw & 0x1000 != 0 {
                        s.push_str(&format!("    {} Current imbalance (>20%)\n", red("!")));
                    }
                    if sw & 0x2000 != 0 {
                        s.push_str(&format!("    {} Voltage imbalance (>2%)\n", red("!")));
                    }
                    if sw & 0x4000 != 0 {
                        s.push_str(&format!("    {} Phase sequence error\n", red("!")));
                    }
                    if sw & 0x8000 != 0 {
                        s.push_str(&format!("    {} Reverse power\n", red("!")));
                    }
                    s
                }
            }
            _ => {
                format!("{}{}\n", red("  ✗ unknown:"), sub)
            }
        };
        queue!(stdout, style::Print(out))?;
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
        let out = format!("{} energy reset\n", grey("  →"));
        queue!(stdout, style::Print(out))?;
        stdout.flush()?;
        Ok(())
    }
}
