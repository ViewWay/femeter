//! Interactive Shell — box-drawing table format

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

// ── Box-drawing table helpers ──

fn table_top(widths: &[usize]) -> String {
    let inner: Vec<String> = widths.iter().map(|w| "─".repeat(*w + 2)).collect();
    format!("┌{}┐", inner.join("┬"))
}

fn table_separator(widths: &[usize]) -> String {
    let inner: Vec<String> = widths.iter().map(|w| "─".repeat(*w + 2)).collect();
    format!("├{}┤", inner.join("┼"))
}

fn table_bottom(widths: &[usize]) -> String {
    let inner: Vec<String> = widths.iter().map(|w| "─".repeat(*w + 2)).collect();
    format!("└{}┘", inner.join("┴"))
}

/// Center text within width
fn pad_center(text: &str, width: usize) -> String {
    let tlen = text.display_width();
    if tlen >= width {
        return text.to_string();
    }
    let left = (width - tlen) / 2;
    let right = width - tlen - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

/// Left-align text within width
fn pad_left(text: &str, width: usize) -> String {
    let tlen = text.display_width();
    if tlen >= width {
        return text.to_string();
    }
    format!("{}{}", text, " ".repeat(width - tlen))
}

/// Right-align text within width
fn pad_right(text: &str, width: usize) -> String {
    let tlen = text.display_width();
    if tlen >= width {
        return text.to_string();
    }
    format!("{}{}", " ".repeat(width - tlen), text)
}

fn table_header(widths: &[usize], headers: &[&str]) -> String {
    let cells: Vec<String> = widths
        .iter()
        .zip(headers.iter())
        .map(|(w, h)| format!(" {} ", pad_center(h, *w)))
        .collect();
    format!("│{}│", cells.join("│"))
}

fn table_row(widths: &[usize], values: &[&str], aligns: &[Align]) -> String {
    let cells: Vec<String> = widths
        .iter()
        .zip(values.iter())
        .zip(aligns.iter())
        .map(|((w, v), a)| {
            let padded = match a {
                Align::Left => pad_left(v, *w),
                Align::Right => pad_right(v, *w),
            };
            format!(" {} ", padded)
        })
        .collect();
    format!("│{}│", cells.join("│"))
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Right,
}

/// Simple single-box: top + content lines + bottom
fn single_box(width: usize, lines: &[&str]) -> String {
    let mut out = table_top(&[width]) + nl();
    for line in lines {
        out.push_str(&format!("│ {} │{}", pad_left(line, width), nl()));
    }
    out.push_str(&table_bottom(&[width]));
    out
}

/// Box with centered title spanning full width, then multi-column rows
fn total_width(widths: &[usize]) -> usize {
    widths.iter().sum::<usize>() + widths.len() * 3 - 1
}

fn titled_table(
    title: &str,
    widths: &[usize],
    headers: &[&str],
    rows: &[Vec<String>],
    aligns: &[Align],
) -> String {
    let tw = total_width(widths);
    let mut out = table_top(widths) + nl();
    if !title.is_empty() {
        out.push_str(&format!("│ {} │{}", pad_left(title, tw), nl()));
        out.push_str(&table_separator(widths));
        out.push_str(nl());
    }
    out.push_str(&table_header(widths, headers));
    out.push_str(nl());
    out.push_str(&table_separator(widths));
    out.push_str(nl());
    for row in rows {
        let refs: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
        out.push_str(&table_row(widths, &refs, aligns));
        out.push_str(nl());
    }
    out.push_str(&table_bottom(widths));
    out
}

/// Multi-section table: title spanning full width, then column sections separated by ┼ separators
fn multi_section_table(
    title: &str,
    widths: &[usize],
    headers: &[&str],
    sections: &[(Vec<Vec<String>>, bool)], // (rows, has_separator_after)
    footer: &[&str],
) -> String {
    let aligns_left: Vec<Align> = widths.iter().map(|_| Align::Left).collect();
    let tw = total_width(widths);
    let mut out = table_top(widths) + nl();
    out.push_str(&format!("│ {} │{}", pad_left(title, tw), nl()));
    out.push_str(&table_separator(widths));
    out.push_str(nl());
    out.push_str(&table_header(widths, headers));
    out.push_str(nl());
    for (rows, _has_sep) in sections {
        out.push_str(&table_separator(widths));
        out.push_str(nl());
        for row in rows {
            let refs: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
            out.push_str(&table_row(widths, &refs, &aligns_left));
            out.push_str(nl());
        }
    }
    out.push_str(&table_separator(widths));
    out.push_str(nl());
    for line in footer {
        out.push_str(&format!("│ {} │{}", pad_left(line, tw), nl()));
    }
    out.push_str(&table_bottom(widths));
    out
}

trait DisplayWidth {
    fn display_width(&self) -> usize;
}

impl DisplayWidth for str {
    fn display_width(&self) -> usize {
        self.chars().count() // All content is ASCII, so char count = display width
    }
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
        queue!(stdout, style::Print(cyan("> ")))?;
        stdout.flush()?;
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            self.execute_command(line.trim(), &mut stdout, false)?;
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            queue!(stdout, style::Print(cyan("> ")))?;
            stdout.flush()?;
        }
        Ok(())
    }

    fn run_raw_mode(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        let mut input = String::new();
        let mut history: Vec<String> = Vec::new();
        let mut history_index = 0;

        queue!(stdout, style::Print(cyan("> ")))?;
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
                let out = format!("{}{}{}", red("  unknown: "), parts[0], nl());
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
        let w: [usize; 2] = [18, 30];
        let set_cmds = vec![
            vec!["set ua/ub/uc <V>".into(), "Voltage".into()],
            vec!["set ia/ib/ic <A>".into(), "Current".into()],
            vec!["set angle-a/b/c".into(), "Phase angle (deg)".into()],
            vec!["set freq <Hz>".into(), "Frequency".into()],
            vec!["set pf <0~1>".into(), "Power factor".into()],
            vec!["set 3p <V A H P>".into(), "Three-phase combo".into()],
            vec!["set noise on/off".into(), "Noise simulation".into()],
            vec!["set accel <rate>".into(), "Time acceleration".into()],
        ];
        let get_cmds = vec![
            vec!["get voltage".into(), "Phase + line voltage".into()],
            vec!["get current".into(), "Phase + neutral current".into()],
            vec!["get power".into(), "Active/reactive/apparent".into()],
            vec!["get energy".into(), "Cumulative energy".into()],
            vec!["get freq".into(), "Frequency".into()],
            vec!["get pf".into(), "Power factor".into()],
            vec!["get status".into(), "Status word".into()],
        ];
        let other_cmds = vec![
            vec!["status".into(), "Full status table".into()],
            vec!["scenario <name>".into(), "Preset scenario".into()],
            vec!["event <type>".into(), "Inject event".into()],
            vec!["event list".into(), "Event history".into()],
            vec!["watch [ms]".into(), "Real-time monitor".into()],
            vec!["reset".into(), "Reset energy".into()],
            vec!["quit".into(), "Exit".into()],
        ];

        let out = multi_section_table(
            "FeMeter v0.2",
            &w,
            &["Command", "Description"],
            &[(set_cmds, true), (get_cmds, true), (other_cmds, false)],
            &[],
        );
        queue!(stdout, style::Print(format!("{}{}", out, nl())))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_scenario(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            let out = format!("{}{}", grey("  usage: scenario <name>"), nl());
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
                let out = format!("{}{}{}", red("  unknown: "), args[0], nl());
                queue!(stdout, style::Print(out))?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.load_scenario(sc);
        let out = format!("  -> scenario: {}", yellow(&format!("{:?}", sc)));
        queue!(stdout, style::Print(format!("{}{}", out, nl())))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_event(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.first().map(|s| s.to_lowercase()).as_deref() == Some("list") || args.is_empty() {
            let meter = self.meter.lock().expect("mutex poisoned");
            let events = meter.events();
            if events.is_empty() {
                let out = format!("{}{}", grey("  no events"), nl());
                queue!(stdout, style::Print(out))?;
            } else {
                let w: [usize; 2] = [10, 18];
                let rows: Vec<Vec<String>> = events
                    .iter()
                    .rev()
                    .take(20)
                    .map(|e| {
                        vec![
                            e.timestamp.format("%H:%M:%S").to_string(),
                            format!("{:?}", e.event),
                        ]
                    })
                    .collect();
                let out = titled_table(
                    &format!("Events ({})", events.len()),
                    &w,
                    &["Time", "Event"],
                    &rows,
                    &[Align::Left, Align::Left],
                );
                queue!(stdout, style::Print(format!("{}{}", out, nl())))?;
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
                let out = format!("{}{}{}", red("  unknown: "), args[0], nl());
                queue!(stdout, style::Print(out))?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.trigger_event(ev);
        let out = format!("  -> event: {}", yellow(&format!("{:?}", ev)));
        queue!(stdout, style::Print(format!("{}{}", out, nl())))?;
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

        let accel = cfg.time_accel;
        let w: [usize; 5] = [8, 10, 9, 9, 6];
        let tw = total_width(&w);

        let title = format!(
            "FeMeter v0.2  {:?}  {:.2}Hz  Accel:{:.0}x",
            snap.chip, snap.freq, accel
        );

        let aligns = [
            Align::Left,
            Align::Right,
            Align::Right,
            Align::Right,
            Align::Right,
        ];
        let rows = vec![
            vec![
                "A".into(),
                format!("{:.2}", snap.phase_a.voltage),
                format!("{:.2}", snap.phase_a.current),
                format!("{:.1}", snap.phase_a.angle),
                format!("{:.3}", snap.computed.pf_a),
            ],
            vec![
                "B".into(),
                format!("{:.2}", snap.phase_b.voltage),
                format!("{:.2}", snap.phase_b.current),
                format!("{:.1}", snap.phase_b.angle),
                format!("{:.3}", snap.computed.pf_b),
            ],
            vec![
                "C".into(),
                format!("{:.2}", snap.phase_c.voltage),
                format!("{:.2}", snap.phase_c.current),
                format!("{:.1}", snap.phase_c.angle),
                format!("{:.3}", snap.computed.pf_c),
            ],
        ];

        let footer: Vec<String> = vec![
            format!(
                "P: {:.1}W  Q: {:.1}var  S: {:.1}VA",
                snap.computed.p_total, snap.computed.q_total, snap.computed.s_total
            ),
            format!(
                "E: {:.3} kWh / {:.3} kvarh",
                snap.energy.wh_total / 1000.0,
                snap.energy.varh_total / 1000.0
            ),
        ];
        let footer_refs: Vec<&str> = footer.iter().map(|s| s.as_str()).collect();

        let mut out = table_top(&w) + nl();
        out.push_str(&format!("│ {} │{}", pad_left(&title, tw), nl()));
        out.push_str(&table_separator(&w));
        out.push_str(nl());
        out.push_str(&table_header(
            &w,
            &["Phase", "V(V)", "I(A)", "Angle°", "PF"],
        ));
        out.push_str(nl());
        out.push_str(&table_separator(&w));
        out.push_str(nl());
        for row in &rows {
            let refs: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
            out.push_str(&table_row(&w, &refs, &aligns));
            out.push_str(nl());
        }
        out.push_str(&table_separator(&w));
        out.push_str(nl());
        for line in &footer_refs {
            out.push_str(&format!("│ {} │{}", pad_left(line, tw), nl()));
        }
        out.push_str(&table_bottom(&w));

        queue!(stdout, style::Print(format!("{}{}", out, nl())))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_set(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            let out = format!(
                "{}{}",
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
                let out = format!("{}{}", grey("  usage: set 3p <V> <A> <Hz> <PF>"), nl());
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
                "  3P: {}{}  {}{}  {}{}  PF={}",
                white(&format!("{:.1}", v)),
                grey("V"),
                white(&format!("{:.2}", i)),
                grey("A"),
                white(&format!("{:.1}", freq)),
                grey("Hz"),
                white(&format!("{:.2}", pf)),
            );
            queue!(stdout, style::Print(format!("{}{}", out, nl())))?;
            stdout.flush()?;
            return Ok(());
        }

        if args.len() < 2 {
            let out = format!(
                "{}{}",
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
                    "  A-phase voltage: {}{}",
                    white(&format!("{:.2}", v)),
                    grey("V")
                )
            }
            "ub" => {
                let v = value_str.parse().unwrap_or(220.0);
                meter.set_voltage('b', v);
                format!(
                    "  B-phase voltage: {}{}",
                    white(&format!("{:.2}", v)),
                    grey("V")
                )
            }
            "uc" => {
                let v = value_str.parse().unwrap_or(220.0);
                meter.set_voltage('c', v);
                format!(
                    "  C-phase voltage: {}{}",
                    white(&format!("{:.2}", v)),
                    grey("V")
                )
            }
            "ia" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_current('a', v);
                format!(
                    "  A-phase current: {}{}",
                    white(&format!("{:.3}", v)),
                    grey("A")
                )
            }
            "ib" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_current('b', v);
                format!(
                    "  B-phase current: {}{}",
                    white(&format!("{:.3}", v)),
                    grey("A")
                )
            }
            "ic" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_current('c', v);
                format!(
                    "  C-phase current: {}{}",
                    white(&format!("{:.3}", v)),
                    grey("A")
                )
            }
            "angle-a" | "angle_a" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_angle('a', v);
                format!(
                    "  A-phase angle: {}{}",
                    white(&format!("{:.1}", v)),
                    grey("°")
                )
            }
            "angle-b" | "angle_b" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_angle('b', v);
                format!(
                    "  B-phase angle: {}{}",
                    white(&format!("{:.1}", v)),
                    grey("°")
                )
            }
            "angle-c" | "angle_c" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_angle('c', v);
                format!(
                    "  C-phase angle: {}{}",
                    white(&format!("{:.1}", v)),
                    grey("°")
                )
            }
            "freq" => {
                let v = value_str.parse().unwrap_or(50.0);
                meter.set_freq(v);
                format!("  Frequency: {}{}", white(&format!("{:.2}", v)), grey("Hz"))
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
                        "  Power factor: {} (angle {}{} )",
                        white(&format!("{:.3}", pf)),
                        white(&format!("{:.1}", angle)),
                        grey("°"),
                    )
                }
            }
            "noise" => {
                let e = ["on", "1", "true"].contains(&value_str.to_lowercase().as_str());
                meter.set_noise(e);
                format!("  Noise: {}", if e { green("on") } else { dim("off") })
            }
            "chip" => match value_str.to_lowercase().as_str() {
                "att7022e" | "att7022" => {
                    meter.set_chip(ChipType::ATT7022E);
                    format!("  Chip: {}", yellow("ATT7022E"))
                }
                "rn8302b" | "rn8302" => {
                    meter.set_chip(ChipType::RN8302B);
                    format!("  Chip: {}", yellow("RN8302B"))
                }
                _ => format!("{} unknown: {}", red("  ✗"), value_str),
            },
            "accel" => {
                let a: f64 = value_str.parse().unwrap_or(1.0);
                meter.set_time_accel(a);
                format!("  Accel: {}{}", white(&format!("{:.0}", a)), grey("x"))
            }
            _ => format!("{} unknown: {}", red("  ✗"), param),
        };
        queue!(stdout, style::Print(format!("{}{}", result, nl())))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_get(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            let out = format!(
                "{}{}",
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

                let w: [usize; 3] = [8, 10, 10];
                let aligns = [Align::Left, Align::Right, Align::Right];
                let rows = vec![
                    vec![
                        "A".into(),
                        format!("{:.2}", snap.phase_a.voltage),
                        format!("{:.2}", v_ab),
                    ],
                    vec![
                        "B".into(),
                        format!("{:.2}", snap.phase_b.voltage),
                        format!("{:.2}", v_bc),
                    ],
                    vec![
                        "C".into(),
                        format!("{:.2}", snap.phase_c.voltage),
                        format!("{:.2}", v_ca),
                    ],
                ];
                titled_table("", &w, &["Phase", "Phase(V)", "Line(V)"], &rows, &aligns)
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

                let w: [usize; 2] = [8, 11];
                let aligns = [Align::Left, Align::Right];
                let rows = vec![
                    vec!["A".into(), format!("{:.3}", snap.phase_a.current)],
                    vec!["B".into(), format!("{:.3}", snap.phase_b.current)],
                    vec!["C".into(), format!("{:.3}", snap.phase_c.current)],
                    vec!["N".into(), format!("{:.3}", i_n)],
                ];
                titled_table("", &w, &["Phase", "Current(A)"], &rows, &aligns)
            }
            "power" | "pow" | "p" => {
                let w: [usize; 5] = [8, 10, 11, 10, 7];
                let aligns = [
                    Align::Left,
                    Align::Right,
                    Align::Right,
                    Align::Right,
                    Align::Right,
                ];
                let rows = vec![
                    vec![
                        "A".into(),
                        format!("{:.2}", snap.computed.p_a),
                        format!("{:.2}", snap.computed.q_a),
                        format!("{:.2}", snap.computed.s_a),
                        format!("{:.3}", snap.computed.pf_a),
                    ],
                    vec![
                        "B".into(),
                        format!("{:.2}", snap.computed.p_b),
                        format!("{:.2}", snap.computed.q_b),
                        format!("{:.2}", snap.computed.s_b),
                        format!("{:.3}", snap.computed.pf_b),
                    ],
                    vec![
                        "C".into(),
                        format!("{:.2}", snap.computed.p_c),
                        format!("{:.2}", snap.computed.q_c),
                        format!("{:.2}", snap.computed.s_c),
                        format!("{:.3}", snap.computed.pf_c),
                    ],
                    vec![
                        "Total".into(),
                        format!("{:.2}", snap.computed.p_total),
                        format!("{:.2}", snap.computed.q_total),
                        format!("{:.2}", snap.computed.s_total),
                        format!("{:.3}", snap.computed.pf_total),
                    ],
                ];
                titled_table(
                    "",
                    &w,
                    &["Phase", "W(W)", "var(var)", "VA(VA)", "PF"],
                    &rows,
                    &aligns,
                )
            }
            "energy" | "en" | "e" => {
                let w: [usize; 3] = [8, 11, 12];
                let aligns = [Align::Left, Align::Right, Align::Right];
                let rows = vec![
                    vec![
                        "A".into(),
                        format!("{:.4}", snap.energy.wh_a / 1000.0),
                        format!("{:.4}", snap.energy.varh_a / 1000.0),
                    ],
                    vec![
                        "B".into(),
                        format!("{:.4}", snap.energy.wh_b / 1000.0),
                        format!("{:.4}", snap.energy.varh_b / 1000.0),
                    ],
                    vec![
                        "C".into(),
                        format!("{:.4}", snap.energy.wh_c / 1000.0),
                        format!("{:.4}", snap.energy.varh_c / 1000.0),
                    ],
                    vec![
                        "Total".into(),
                        format!("{:.4}", snap.energy.wh_total / 1000.0),
                        format!("{:.4}", snap.energy.varh_total / 1000.0),
                    ],
                ];
                titled_table("", &w, &["Phase", "kWh", "kvarh"], &rows, &aligns)
            }
            "frequency" | "freq" | "f" => {
                format!(
                    "  {} {}{}",
                    grey("Freq:"),
                    white(&format!("{:.3}", snap.freq)),
                    grey("Hz")
                )
            }
            "power-factor" | "pf" => {
                let w: [usize; 2] = [8, 10];
                let aligns = [Align::Left, Align::Right];
                let rows = vec![
                    vec!["A".into(), format!("{:.4}", snap.computed.pf_a)],
                    vec!["B".into(), format!("{:.4}", snap.computed.pf_b)],
                    vec!["C".into(), format!("{:.4}", snap.computed.pf_c)],
                    vec!["Total".into(), format!("{:.4}", snap.computed.pf_total)],
                ];
                titled_table("", &w, &["Phase", "PF"], &rows, &aligns)
            }
            "status-word" | "status" | "sw" => {
                let sw = self.compute_status_word(&snap);
                if sw == 0 {
                    single_box(18, &[&format!("Status: 0x{:08X}", sw), "OK"])
                } else {
                    let mut errors = Vec::new();
                    if sw & 0x4000 != 0 {
                        errors.push("! Phase seq error");
                    }
                    if sw & 0x01 != 0 {
                        errors.push("! A-phase loss");
                    }
                    if sw & 0x02 != 0 {
                        errors.push("! B-phase loss");
                    }
                    if sw & 0x04 != 0 {
                        errors.push("! C-phase loss");
                    }
                    if sw & 0x08 != 0 {
                        errors.push("! A overvoltage");
                    }
                    if sw & 0x10 != 0 {
                        errors.push("! B overvoltage");
                    }
                    if sw & 0x20 != 0 {
                        errors.push("! C overvoltage");
                    }
                    if sw & 0x40 != 0 {
                        errors.push("! A undervoltage");
                    }
                    if sw & 0x80 != 0 {
                        errors.push("! B undervoltage");
                    }
                    if sw & 0x100 != 0 {
                        errors.push("! C undervoltage");
                    }
                    if sw & 0x200 != 0 {
                        errors.push("! A overcurrent");
                    }
                    if sw & 0x400 != 0 {
                        errors.push("! B overcurrent");
                    }
                    if sw & 0x800 != 0 {
                        errors.push("! C overcurrent");
                    }
                    if sw & 0x1000 != 0 {
                        errors.push("! Current imbalance");
                    }
                    if sw & 0x2000 != 0 {
                        errors.push("! Voltage imbalance");
                    }
                    if sw & 0x8000 != 0 {
                        errors.push("! Reverse power");
                    }
                    let max_w = 18.max(errors.iter().map(|e| e.len()).max().unwrap_or(0));
                    let mut lines = vec![format!("Status: 0x{:08X}", sw)];
                    lines.extend(errors.iter().map(|s| s.to_string()));
                    let lines_ref: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
                    single_box(max_w, &lines_ref)
                }
            }
            _ => {
                format!("{}{}", red("  unknown:"), sub)
            }
        };
        queue!(stdout, style::Print(format!("{}{}", out, nl())))?;
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
        let out = "  -> energy reset".to_string();
        queue!(stdout, style::Print(format!("{}{}", out, nl())))?;
        stdout.flush()?;
        Ok(())
    }
}
