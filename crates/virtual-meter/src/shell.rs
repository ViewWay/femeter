//! Interactive Shell — clean borderless format

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

fn nl() -> &'static str {
    "\n\r"
}

fn pad_right(text: &str, width: usize) -> String {
    let tlen = text.len();
    if tlen >= width {
        return text.to_string();
    }
    format!("{}{}", " ".repeat(width - tlen), text)
}

fn pad_left(text: &str, width: usize) -> String {
    let tlen = text.len();
    if tlen >= width {
        return text.to_string();
    }
    format!("{}{}", text, " ".repeat(width - tlen))
}

/// Simple borderless table: title (blank = skip), header row, data rows
fn simple_table(title: &str, widths: &[usize], headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(title);
        out.push_str(nl());
        out.push_str(nl());
    }
    // header
    let hdr: Vec<String> = widths
        .iter()
        .zip(headers.iter())
        .map(|(w, h)| pad_left(h, *w))
        .collect();
    out.push_str(&format!("  {}\n\r", hdr.join("  ")));
    // rows
    for row in rows {
        let cells: Vec<String> = widths
            .iter()
            .zip(row.iter())
            .map(|(w, v)| {
                // first column left-align, rest right-align
                if widths.first() == Some(w) {
                    pad_left(v, *w)
                } else {
                    pad_right(v, *w)
                }
            })
            .collect();
        out.push_str(&format!("  {}\n\r", cells.join("  ")));
    }
    out
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
        queue!(stdout, style::Print("> "))?;
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
            queue!(stdout, style::Print("> "))?;
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

        queue!(stdout, style::Print("> "))?;
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
                queue!(stdout, style::Print(format!("{}{}", nl(), "> ")))?;
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
        queue!(stdout, style::Print("> "))?;
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
            "freeze" => self.cmd_freeze(stdout),
            "watch" | "w" => {
                self.cmd_watch(&parts[1..], stdout);
                Ok(())
            }
            "quit" | "exit" | "q" => {
                self.running.store(false, Ordering::Relaxed);
                Ok(())
            }
            _ => {
                let out = format!("  unknown: {}{}", parts[0], nl());
                queue!(stdout, style::Print(out))?;
                if raw {
                    self.print_prompt(stdout)?;
                }
                Ok(())
            }
        }
    }

    fn cmd_help(&self, stdout: &mut impl Write) -> Result<()> {
        let out = concat!(
            " FeMeter v0.2\n\r",
            "\n\r",
            " set ua/ub/uc <V>        Voltage\n\r",
            " set ia/ib/ic <A>        Current\n\r",
            " set angle-a/b/c <deg>   Phase angle\n\r",
            " set freq <Hz>           Frequency\n\r",
            " set pf <0~1>            Power factor\n\r",
            " set 3p <V A Hz PF>      Three-phase combo\n\r",
            " set noise on/off        Noise simulation\n\r",
            " set accel <rate>        Time acceleration\n\r",
            "\n\r",
            " get voltage             Phase + line voltage\n\r",
            " get current             Phase + neutral current\n\r",
            " get power               Active/reactive/apparent\n\r",
            " get energy              Cumulative energy\n\r",
            " get freq                Frequency\n\r",
            " get pf                  Power factor\n\r",
            " get status              Status word\n\r",
            "\n\r",
            " status                  Full status table\n\r",
            " get tou / tariff / demand / profile / freeze\n\r",
            " set tou <preset>        Load TOU preset\n\r",
            " set demand-reset         Reset max demand\n\r",
            " set profile-interval <s> Load profile interval\n\r",
            " freeze                   Manual freeze\n\r",
            " scenario <name>         Preset scenario\n\r",
            " event <type>            Inject event\n\r",
            " event list              Event history\n\r",
            " watch [ms]              Real-time monitor\n\r",
            " reset                   Reset energy\n\r",
            " quit                    Exit\n\r",
        );
        queue!(stdout, style::Print(out))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_status(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();
        let cfg = meter.config();
        let accel = cfg.time_accel;

        let title = format!(
            " FeMeter v0.2  {:?}  {:.2}Hz  Accel:{:.0}x",
            snap.chip, snap.freq, accel
        );

        let w: [usize; 5] = [6, 8, 7, 8, 6];
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

        let mut out = simple_table(
            &title,
            &w,
            &["Phase", "V(V)", "I(A)", "Angle(°)", "PF"],
            &rows,
        );
        out.push_str(nl());
        out.push_str(&format!(
            "  P: {:.1}W  Q: {:.1}var  S: {:.1}VA  PF: {:.3}\n\r",
            snap.computed.p_total,
            snap.computed.q_total,
            snap.computed.s_total,
            snap.computed.pf_total
        ));
        out.push_str(&format!(
            "  E: {:.3} kWh / {:.3} kvarh\n\r",
            snap.energy.wh_total / 1000.0,
            snap.energy.varh_total / 1000.0
        ));
        out.push_str(nl());

        queue!(stdout, style::Print(out))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_set(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            let out = format!("  usage: set <param> <value> | set 3p <V A Hz PF>{}", nl());
            queue!(stdout, style::Print(out))?;
            stdout.flush()?;
            return Ok(());
        }
        let param = args[0].to_lowercase();
        let mut meter = self.meter.lock().expect("mutex poisoned");

        if param == "three-phase" || param == "threephase" || param == "3p" {
            if args.len() < 5 {
                let out = format!("  usage: set 3p <V> <A> <Hz> <PF>{}", nl());
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
                " -> 3P: {:.1}V {:.2}A {:.1}Hz PF={:.2}{}",
                v,
                i,
                freq,
                pf,
                nl()
            );
            queue!(stdout, style::Print(out))?;
            stdout.flush()?;
            return Ok(());
        }

        if args.len() < 2 {
            let out = format!("  usage: set {} <value>{}", param, nl());
            queue!(stdout, style::Print(out))?;
            stdout.flush()?;
            return Ok(());
        }

        let value_str = args[1];
        let result = match param.as_str() {
            "ua" => {
                let v = value_str.parse().unwrap_or(220.0);
                meter.set_voltage('a', v);
                format!(" -> A-phase voltage: {:.2}V{}", v, nl())
            }
            "ub" => {
                let v = value_str.parse().unwrap_or(220.0);
                meter.set_voltage('b', v);
                format!(" -> B-phase voltage: {:.2}V{}", v, nl())
            }
            "uc" => {
                let v = value_str.parse().unwrap_or(220.0);
                meter.set_voltage('c', v);
                format!(" -> C-phase voltage: {:.2}V{}", v, nl())
            }
            "ia" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_current('a', v);
                format!(" -> A-phase current: {:.3}A{}", v, nl())
            }
            "ib" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_current('b', v);
                format!(" -> B-phase current: {:.3}A{}", v, nl())
            }
            "ic" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_current('c', v);
                format!(" -> C-phase current: {:.3}A{}", v, nl())
            }
            "angle-a" | "angle_a" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_angle('a', v);
                format!(" -> A-phase angle: {:.1}deg{}", v, nl())
            }
            "angle-b" | "angle_b" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_angle('b', v);
                format!(" -> B-phase angle: {:.1}deg{}", v, nl())
            }
            "angle-c" | "angle_c" => {
                let v = value_str.parse().unwrap_or(0.0);
                meter.set_angle('c', v);
                format!(" -> C-phase angle: {:.1}deg{}", v, nl())
            }
            "freq" => {
                let v = value_str.parse().unwrap_or(50.0);
                meter.set_freq(v);
                format!(" -> Frequency: {:.2}Hz{}", v, nl())
            }
            "pf" | "power-factor" => {
                let pf: f64 = value_str.parse().unwrap_or(0.95);
                if !(-1.0..=1.0).contains(&pf) {
                    format!("  PF must be -1~1{}", nl())
                } else {
                    let angle = pf.acos() * 180.0 / std::f64::consts::PI;
                    meter.set_angle('a', angle);
                    meter.set_angle('b', angle);
                    meter.set_angle('c', angle);
                    format!(
                        " -> Power factor: {:.3} (angle {:.1}deg){}",
                        pf,
                        angle,
                        nl()
                    )
                }
            }
            "noise" => {
                let e = ["on", "1", "true"].contains(&value_str.to_lowercase().as_str());
                meter.set_noise(e);
                format!(" -> Noise: {}{}", if e { "on" } else { "off" }, nl())
            }
            "chip" => match value_str.to_lowercase().as_str() {
                "att7022e" | "att7022" => {
                    meter.set_chip(ChipType::ATT7022E);
                    format!(" -> Chip: ATT7022E{}", nl())
                }
                "rn8302b" | "rn8302" => {
                    meter.set_chip(ChipType::RN8302B);
                    format!(" -> Chip: RN8302B{}", nl())
                }
                _ => format!("  unknown chip: {}{}", value_str, nl()),
            },
            "accel" => {
                let a: f64 = value_str.parse().unwrap_or(1.0);
                meter.set_time_accel(a);
                format!(" -> Accel: {:.0}x{}", a, nl())
            }
            "tou" => {
                let preset = match value_str.to_lowercase().as_str() {
                    "single" | "1" => crate::tou::TouPreset::SingleRate,
                    "two" | "2" => crate::tou::TouPreset::TwoRateTimeOfDay,
                    "three" | "3" => crate::tou::TouPreset::ThreeRatePeakFlatValley,
                    "four" | "4" => crate::tou::TouPreset::FourRatePeakFlatValleySharp,
                    _ => {
                        let out = format!(
                            "  unknown preset: {} (single/two/three/four){}",
                            value_str,
                            nl()
                        );
                        queue!(stdout, style::Print(out))?;
                        stdout.flush()?;
                        return Ok(());
                    }
                };
                meter.tou.load_preset(preset);
                format!(" -> TOU preset: {:?}{}", preset, nl())
            }
            "demand-reset" => {
                let clock = meter.clock;
                meter.demand_calc.reset_max(&clock);
                format!(" -> Max demand reset{}", nl())
            }
            "profile-interval" => {
                let s: u32 = value_str.parse().unwrap_or(900);
                meter.load_profile.set_interval(s);
                format!(" -> Profile interval: {}s{}", s, nl())
            }
            _ => format!("  unknown: {}{}", param, nl()),
        };
        queue!(stdout, style::Print(result))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_get(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            let out = format!(
                "  usage: get <voltage|current|power|energy|freq|pf|status>{}",
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

                simple_table(
                    " Voltage",
                    &[6, 10, 10],
                    &["Phase", "Phase(V)", "Line(V)"],
                    &[
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
                    ],
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

                let mut tbl = simple_table(
                    " Current",
                    &[6, 11],
                    &["Phase", "Current(A)"],
                    &[
                        vec!["A".into(), format!("{:.3}", snap.phase_a.current)],
                        vec!["B".into(), format!("{:.3}", snap.phase_b.current)],
                        vec!["C".into(), format!("{:.3}", snap.phase_c.current)],
                        vec!["N".into(), format!("{:.3}", i_n)],
                    ],
                );
                // Append "(neutral)" note for N row
                tbl = tbl.trim_end_matches("\n\r").to_string() + "  (neutral)\n\r";
                tbl
            }
            "power" | "pow" | "p" => simple_table(
                " Power",
                &[6, 10, 10, 10, 7],
                &["Phase", "W(var)", "var(var)", "VA(VA)", "PF"],
                &[
                    vec![
                        "A".into(),
                        format!("{:.1}", snap.computed.p_a),
                        format!("{:.1}", snap.computed.q_a),
                        format!("{:.1}", snap.computed.s_a),
                        format!("{:.3}", snap.computed.pf_a),
                    ],
                    vec![
                        "B".into(),
                        format!("{:.1}", snap.computed.p_b),
                        format!("{:.1}", snap.computed.q_b),
                        format!("{:.1}", snap.computed.s_b),
                        format!("{:.3}", snap.computed.pf_b),
                    ],
                    vec![
                        "C".into(),
                        format!("{:.1}", snap.computed.p_c),
                        format!("{:.1}", snap.computed.q_c),
                        format!("{:.1}", snap.computed.s_c),
                        format!("{:.3}", snap.computed.pf_c),
                    ],
                    vec![
                        "Total".into(),
                        format!("{:.1}", snap.computed.p_total),
                        format!("{:.1}", snap.computed.q_total),
                        format!("{:.1}", snap.computed.s_total),
                        format!("{:.3}", snap.computed.pf_total),
                    ],
                ],
            ),
            "energy" | "en" | "e" => simple_table(
                " Energy",
                &[6, 10, 10],
                &["Phase", "kWh", "kvarh"],
                &[
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
                ],
            ),
            "frequency" | "freq" | "f" => {
                format!(" Frequency: {:.3} Hz{}", snap.freq, nl())
            }
            "power-factor" | "pf" => simple_table(
                " Power Factor",
                &[6, 10],
                &["Phase", "PF"],
                &[
                    vec!["A".into(), format!("{:.4}", snap.computed.pf_a)],
                    vec!["B".into(), format!("{:.4}", snap.computed.pf_b)],
                    vec!["C".into(), format!("{:.4}", snap.computed.pf_c)],
                    vec!["Total".into(), format!("{:.4}", snap.computed.pf_total)],
                ],
            ),
            "tou" => {
                let m = &meter.tou;
                let rate = m.current_rate();
                let mut out = format!(
                    " TOU Engine  Current: {} ({} changes){}",
                    rate.label(),
                    m.rate_change_count,
                    nl()
                );
                if let Some(dp) = m.calendar.day_profiles.first() {
                    out.push_str("\n\r Time Table:\n\r");
                    for seg in &dp.segments {
                        out.push_str(&format!(
                            "   {:02}:{:02} -> {}{}",
                            seg.start_hour,
                            seg.start_min,
                            seg.rate.label(),
                            nl()
                        ));
                    }
                }
                out
            }
            "tariff" => {
                let te = &meter.tariff_energy;
                let rate_names = [
                    "T1(Sharp)",
                    "T2(Peak)",
                    "T3(Normal)",
                    "T4(Valley)",
                    "T5",
                    "T6",
                    "T7",
                    "T8",
                ];
                let mut rows = Vec::new();
                for (i, &total) in te.active_total.iter().enumerate() {
                    if total > 0.0 {
                        rows.push(vec![rate_names[i].into(), format!("{:.4}", total / 1000.0)]);
                    }
                }
                if rows.is_empty() {
                    format!(" No tariff energy recorded{}", nl())
                } else {
                    simple_table(
                        " Tariff Energy",
                        &[12usize, 14usize],
                        &["Rate", "kWh"],
                        &rows,
                    )
                }
            }
            "demand" => {
                let dc = &meter.demand_calc;
                format!(
                    " Demand  Current: {:.3} kW  Max: {:.3} kW{}",
                    dc.current_demand_kw(),
                    dc.max_demand_kw(),
                    nl()
                )
            }
            "profile" | "loadprofile" => {
                let n = args
                    .get(1)
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(20);
                let lp = &meter.load_profile;
                let entries = lp.query_last(n);
                if entries.is_empty() {
                    format!(
                        " No load profile entries (interval: {}s, captured: {}){}",
                        lp.capture_interval_s,
                        lp.capture_count,
                        nl()
                    )
                } else {
                    let mut out = format!(
                        " Load Profile ({} entries, showing {}){}",
                        lp.buffer.len(),
                        entries.len(),
                        nl()
                    );
                    for e in entries.iter().rev().take(n) {
                        out.push_str(&format!("   {} |", e.timestamp.format("%m-%d %H:%M")));
                        // Show P_total (index 9)
                        if e.values.len() > 9 {
                            out.push_str(&format!(" P={:.1}W", e.values[9]));
                        }
                        if e.values.len() > 15 {
                            out.push_str(&format!(" F={:.1}Hz", e.values[15]));
                        }
                        out.push_str(nl());
                    }
                    out
                }
            }
            "freeze" => {
                let ftype_str = args.get(1).map(|s| s.to_lowercase());
                let records = match ftype_str.as_deref() {
                    Some("monthly") => &meter.freeze.monthly_records,
                    _ => &meter.freeze.daily_records,
                };
                if records.is_empty() {
                    format!(" No freeze records{}", nl())
                } else {
                    let mut out = format!(" Freeze Records ({}){}", records.len(), nl());
                    for r in records.iter().rev().take(20) {
                        out.push_str(&format!(
                            "   {} | {:?} | Wh={:.2} | Demand={:.3}kW{}
\r",
                            r.timestamp.format("%Y-%m-%d %H:%M"),
                            r.freeze_type,
                            r.energy.total_active_import_wh,
                            r.demand.max_demand_kw,
                            nl()
                        ));
                    }
                    out
                }
            }
            "status-word" | "status" | "sw" => {
                let sw = self.compute_status_word(&snap);
                if sw == 0 {
                    format!(" Status: 0x{:08X}  OK{}", sw, nl())
                } else {
                    let mut lines = format!(" Status: 0x{:08X}  ALARM{}", sw, nl());
                    if sw & 0x4000 != 0 {
                        lines.push_str(&format!("   Phase sequence error{}\n\r", nl()));
                    }
                    if sw & 0x01 != 0 {
                        lines.push_str(&format!("   A-phase voltage loss (<10V){}\n\r", nl()));
                    }
                    if sw & 0x02 != 0 {
                        lines.push_str(&format!("   B-phase voltage loss (<10V){}\n\r", nl()));
                    }
                    if sw & 0x04 != 0 {
                        lines.push_str(&format!("   C-phase voltage loss (<10V){}\n\r", nl()));
                    }
                    if sw & 0x08 != 0 {
                        lines.push_str(&format!("   A-phase overvoltage (>264V){}\n\r", nl()));
                    }
                    if sw & 0x10 != 0 {
                        lines.push_str(&format!("   B-phase overvoltage (>264V){}\n\r", nl()));
                    }
                    if sw & 0x20 != 0 {
                        lines.push_str(&format!("   C-phase overvoltage (>264V){}\n\r", nl()));
                    }
                    if sw & 0x40 != 0 {
                        lines.push_str(&format!("   A-phase undervoltage (<198V){}\n\r", nl()));
                    }
                    if sw & 0x80 != 0 {
                        lines.push_str(&format!("   B-phase undervoltage (<198V){}\n\r", nl()));
                    }
                    if sw & 0x100 != 0 {
                        lines.push_str(&format!("   C-phase undervoltage (<198V){}\n\r", nl()));
                    }
                    if sw & 0x200 != 0 {
                        lines.push_str(&format!("   A-phase overcurrent (>60A){}\n\r", nl()));
                    }
                    if sw & 0x400 != 0 {
                        lines.push_str(&format!("   B-phase overcurrent (>60A){}\n\r", nl()));
                    }
                    if sw & 0x800 != 0 {
                        lines.push_str(&format!("   C-phase overcurrent (>60A){}\n\r", nl()));
                    }
                    if sw & 0x1000 != 0 {
                        lines.push_str(&format!("   Current imbalance{}\n\r", nl()));
                    }
                    if sw & 0x2000 != 0 {
                        lines.push_str(&format!("   Voltage imbalance{}\n\r", nl()));
                    }
                    if sw & 0x8000 != 0 {
                        lines.push_str(&format!("   Reverse power{}\n\r", nl()));
                    }
                    lines
                }
            }
            _ => format!("  unknown: {}{}", sub, nl()),
        };
        queue!(stdout, style::Print(out))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_scenario(&self, args: &[&str], stdout: &mut impl Write) -> Result<()> {
        if args.is_empty() {
            let out = format!("  usage: scenario <name>{}", nl());
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
                let out = format!("  unknown: {}{}", args[0], nl());
                queue!(stdout, style::Print(out))?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.load_scenario(sc);
        let out = format!(
            " -> scenario: {}{}",
            format!("{:?}", sc).to_lowercase(),
            nl()
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
                let out = format!("  no events{}", nl());
                queue!(stdout, style::Print(out))?;
            } else {
                let mut out = format!(" Events ({}){}\n\r", events.len(), nl());
                for e in events.iter().rev().take(20) {
                    out.push_str(&format!(
                        "   {:>10}  {:?}{}\n\r",
                        e.timestamp.format("%H:%M:%S"),
                        e.event,
                        nl()
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
                let out = format!("  unknown: {}{}", args[0], nl());
                queue!(stdout, style::Print(out))?;
                stdout.flush()?;
                return Ok(());
            }
        };
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.trigger_event(ev);
        let out = format!(" -> event: {}{}", format!("{:?}", ev).to_lowercase(), nl());
        queue!(stdout, style::Print(out))?;
        stdout.flush()?;
        Ok(())
    }

    fn cmd_freeze(&self, stdout: &mut impl Write) -> Result<()> {
        use crate::freeze::{DemandSnapshot, EnergySnapshot, FreezeRecord, FreezeType};
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let clock = meter.clock;
        let energy_wh_total = meter.energy().wh_total;
        let energy_varh_total = meter.energy().varh_total;
        let max_demand_kw = meter.demand_calc.max_demand_kw();
        let max_demand_time = meter.demand_calc.max_demand_timestamp;
        let phase_max = meter.demand_calc.phase_max_demand_w;
        let record = FreezeRecord {
            freeze_type: FreezeType::OnDemand,
            timestamp: clock,
            energy: EnergySnapshot {
                active_import_wh: meter.tariff_energy.active_total,
                active_export_wh: 0.0,
                reactive_import_varh: meter.tariff_energy.reactive_total,
                reactive_export_varh: 0.0,
                total_active_import_wh: energy_wh_total,
                total_reactive_import_varh: energy_varh_total,
            },
            demand: DemandSnapshot {
                max_demand_kw,
                max_demand_time,
                max_demand_phase_kw: [
                    phase_max[0] / 1000.0,
                    phase_max[1] / 1000.0,
                    phase_max[2] / 1000.0,
                ],
            },
            voltage: [0.0; 3],
            current: [0.0; 3],
            power_factor: 0.0,
            status_word: 0,
            tariff_rate: 0,
        };
        meter.freeze.do_freeze(FreezeType::OnDemand, record);
        let out = format!(" -> manual freeze executed{}", nl());
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
                " [{}] A:{:.1}V {:.2}A | B:{:.1}V {:.2}A | C:{:.1}V {:.2}A",
                snap.timestamp.format("%H:%M:%S"),
                snap.phase_a.voltage,
                snap.phase_a.current,
                snap.phase_b.voltage,
                snap.phase_b.current,
                snap.phase_c.voltage,
                snap.phase_c.current,
            );
            let line2 = format!(
                "          P:{:.1}W Q:{:.1}var S:{:.1}VA PF:{:.3}",
                snap.computed.p_total,
                snap.computed.q_total,
                snap.computed.s_total,
                snap.computed.pf_total,
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
        queue!(stdout, style::Print(format!("{}{}", nl(), "> "))).ok();
        stdout.flush().ok();
    }

    fn cmd_reset(&self, stdout: &mut impl Write) -> Result<()> {
        let mut meter = self.meter.lock().expect("mutex poisoned");
        meter.reset_energy();
        let out = format!(" -> energy reset{}", nl());
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
}
