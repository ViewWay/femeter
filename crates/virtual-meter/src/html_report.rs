//! HTML 电表运行报告生成器
//!
//! 生成包含 SVG 电能趋势图、事件日志、负荷曲线、费率统计的 HTML 报告。

use std::fmt::Write;

/// 报告数据
pub struct ReportData {
    pub meter_id: String,
    pub report_time: String,
    /// 电压趋势 [A, B, C] (各 phase 一组)
    pub voltage_series: [Vec<f32>; 3],
    /// 电流趋势
    pub current_series: [Vec<f32>; 3],
    /// 有功功率趋势
    pub power_series: Vec<f32>,
    /// 事件日志行
    pub event_log: Vec<EventLogEntry>,
    /// 费率统计
    pub tariff_stats: Vec<TariffStat>,
}

#[derive(Clone)]
pub struct EventLogEntry {
    pub timestamp: String,
    pub event_type: String,
    pub description: String,
    pub severity: String,
}

#[derive(Clone)]
pub struct TariffStat {
    pub tariff_name: String,
    pub energy_kwh: f64,
    pub cost_yuan: f64,
    pub percentage: f64,
}

impl ReportData {
    pub fn new(meter_id: &str, report_time: &str) -> Self {
        Self {
            meter_id: meter_id.to_string(),
            report_time: report_time.to_string(),
            voltage_series: [Vec::new(), Vec::new(), Vec::new()],
            current_series: [Vec::new(), Vec::new(), Vec::new()],
            power_series: Vec::new(),
            event_log: Vec::new(),
            tariff_stats: Vec::new(),
        }
    }
}

/// 生成 SVG 折线图
fn svg_line_chart(
    title: &str,
    series: &[(&str, &Vec<f32>)],
    width: u32,
    height: u32,
    y_label: &str,
) -> String {
    let padding_top = 30u32;
    let padding_bottom = 30u32;
    let padding_left = 50u32;
    let padding_right = 20u32;
    let chart_w = width - padding_left - padding_right;
    let chart_h = height - padding_top - padding_bottom;

    // 找全局 min/max
    let mut all_min = f32::MAX;
    let mut all_max = f32::MIN;
    for (_, data) in series {
        for &v in *data {
            all_min = all_min.min(v);
            all_max = all_max.max(v);
        }
    }
    if all_max <= all_min {
        all_max = all_min + 1.0;
    }
    let range = all_max - all_min;

    let colors = ["#e74c3c", "#2ecc71", "#3498db", "#f39c12", "#9b59b6"];

    let mut svg = format!(
        r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">"#,
        width, height
    );
    // Title
    svg.push_str(&format!(
        r#"<text x="{}" y="20" font-size="14" font-weight="bold" text-anchor="middle">{}</text>"#,
        width / 2,
        title
    ));

    // Y axis
    svg.push_str(&format!(
        r#"<text x="10" y="{}" font-size="10" text-anchor="start">{}</text>"#,
        padding_top + chart_h / 2,
        y_label
    ));

    // Grid lines (5 lines)
    for i in 0..=4 {
        let y = padding_top + (i as f32 / 4.0 * chart_h as f32) as u32;
        let val = all_max - (i as f32 / 4.0) * range;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ddd\" stroke-width=\"0.5\"/>",
            padding_left,
            y,
            padding_left + chart_w,
            y
        ));
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" text-anchor="end">{:.1}</text>"#,
            padding_left - 5,
            y + 3,
            val
        ));
    }

    // Data lines
    for (si, (name, data)) in series.iter().enumerate() {
        if data.len() < 2 {
            continue;
        }
        let color = colors[si % colors.len()];
        let mut points = String::new();
        for (i, v) in data.iter().enumerate() {
            let x =
                padding_left + (i as f32 / (data.len() - 1).max(1) as f32 * chart_w as f32) as u32;
            let y = padding_top + ((all_max - v) / range * chart_h as f32) as u32;
            let _ = write!(points, "{} {} ", x, y);
        }
        svg.push_str(&format!(
            r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="1.5"/>"#,
            points, color
        ));
        // Legend
        let lx = padding_left + 10 + (si as u32 * 80);
        let ly = height - 8;
        svg.push_str(&format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2"/>"#,
            lx,
            ly,
            lx + 15,
            ly,
            color
        ));
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9">{}</text>"#,
            lx + 18,
            ly + 3,
            name
        ));
    }

    svg.push_str("</svg>");
    svg
}

/// 生成费率统计 SVG 柱状图
fn svg_tariff_bar(stats: &[TariffStat], width: u32, height: u32) -> String {
    let padding_top = 30u32;
    let padding_bottom = 40u32;
    let padding_left = 50u32;
    let padding_right = 20u32;
    let chart_w = width - padding_left - padding_right;
    let chart_h = height - padding_top - padding_bottom;

    let max_val = stats.iter().map(|s| s.energy_kwh).fold(0.0, f64::max);
    let bar_width = if stats.is_empty() {
        0
    } else {
        (chart_w as usize / stats.len()).min(60) as u32
    };
    let gap = if stats.len() > 1 {
        (chart_w - bar_width * stats.len() as u32) / (stats.len() as u32 - 1)
    } else {
        0
    };

    let colors = ["#3498db", "#e74c3c", "#2ecc71", "#f39c12", "#9b59b6"];

    let mut svg = format!(
        r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">"#,
        width, height
    );
    svg.push_str(&format!(
        r#"<text x="{}" y="20" font-size="14" font-weight="bold" text-anchor="middle">费率统计</text>"#,
        width / 2
    ));

    for (i, stat) in stats.iter().enumerate() {
        let x = padding_left + i as u32 * (bar_width + gap);
        let bar_h = if max_val > 0.0 {
            (stat.energy_kwh / max_val * chart_h as f64) as u32
        } else {
            0
        };
        let y = padding_top + chart_h - bar_h;
        let color = colors[i % colors.len()];

        svg.push_str(&format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" rx="2"/>"#,
            x, y, bar_width, bar_h, color
        ));
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" text-anchor="middle">{:.1}kWh</text>"#,
            x + bar_width / 2,
            y - 5,
            stat.energy_kwh
        ));
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="8" text-anchor="middle" transform="rotate(-45,{},{})">{}</text>"#,
            x + bar_width / 2,
            padding_top + chart_h + 15,
            x + bar_width / 2,
            padding_top + chart_h + 15,
            stat.tariff_name
        ));
    }

    svg.push_str("</svg>");
    svg
}

/// 生成完整 HTML 报告
pub fn generate_html_report(data: &ReportData) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html><html><head><meta charset='UTF-8'>");
    html.push_str("<style>");
    html.push_str("body{font-family:'Segoe UI',Arial,sans-serif;margin:20px;background:#f5f5f5;}");
    html.push_str(".container{max-width:900px;margin:0 auto;background:#fff;padding:20px;border-radius:8px;box-shadow:0 2px 8px rgba(0,0,0,0.1);}");
    html.push_str("h1{color:#2c3e50;border-bottom:2px solid #3498db;padding-bottom:10px;}");
    html.push_str("h2{color:#34495e;margin-top:30px;}");
    html.push_str(".meta{color:#7f8c8d;font-size:14px;margin-bottom:20px;}");
    html.push_str("table{width:100%;border-collapse:collapse;margin:10px 0;}");
    html.push_str("th,td{border:1px solid #ddd;padding:8px;text-align:left;font-size:13px;}");
    html.push_str("th{background:#3498db;color:white;}");
    html.push_str("tr:nth-child(even){background:#f2f2f2;}");
    html.push_str(".severity-high{color:#e74c3c;font-weight:bold;}");
    html.push_str(".severity-medium{color:#f39c12;}");
    html.push_str(".severity-low{color:#27ae60;}");
    html.push_str(".chart{margin:20px 0;text-align:center;}");
    html.push_str("</style></head><body>");

    html.push_str("<div class='container'>");
    html.push_str("<h1>电表运行报告</h1>");
    html.push_str(&format!(
        "<div class='meta'>表号: <strong>{}</strong> | 报告时间: <strong>{}</strong></div>",
        data.meter_id, data.report_time
    ));

    // 电压趋势图
    html.push_str("<h2>电压趋势</h2><div class='chart'>");
    let v_series: Vec<(&str, &Vec<f32>)> = vec![
        ("A相", &data.voltage_series[0]),
        ("B相", &data.voltage_series[1]),
        ("C相", &data.voltage_series[2]),
    ];
    html.push_str(&svg_line_chart("三相电压 (V)", &v_series, 800, 250, "V"));
    html.push_str("</div>");

    // 电流趋势图
    html.push_str("<h2>电流趋势</h2><div class='chart'>");
    let i_series: Vec<(&str, &Vec<f32>)> = vec![
        ("A相", &data.current_series[0]),
        ("B相", &data.current_series[1]),
        ("C相", &data.current_series[2]),
    ];
    html.push_str(&svg_line_chart("三相电流 (A)", &i_series, 800, 250, "A"));
    html.push_str("</div>");

    // 功率趋势图
    html.push_str("<h2>有功功率趋势</h2><div class='chart'>");
    let p_series: Vec<(&str, &Vec<f32>)> = vec![("有功功率", &data.power_series)];
    html.push_str(&svg_line_chart("有功功率 (W)", &p_series, 800, 200, "W"));
    html.push_str("</div>");

    // 费率统计
    if !data.tariff_stats.is_empty() {
        html.push_str("<h2>费率统计</h2><div class='chart'>");
        html.push_str(&svg_tariff_bar(&data.tariff_stats, 800, 250));
        html.push_str("</div>");

        html.push_str(
            "<table><tr><th>费率</th><th>电量 (kWh)</th><th>费用 (元)</th><th>占比</th></tr>",
        );
        for s in &data.tariff_stats {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{:.1}%</td></tr>",
                s.tariff_name, s.energy_kwh, s.cost_yuan, s.percentage
            ));
        }
        html.push_str("</table>");
    }

    // 事件日志
    html.push_str("<h2>事件日志</h2>");
    if data.event_log.is_empty() {
        html.push_str("<p style='color:#27ae60;'>无异常事件</p>");
    } else {
        html.push_str("<table><tr><th>时间</th><th>类型</th><th>描述</th><th>严重度</th></tr>");
        for e in &data.event_log {
            let cls = match e.severity.as_str() {
                "高" => "severity-high",
                "中" => "severity-medium",
                _ => "severity-low",
            };
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td class='{}'>{}</td></tr>",
                e.timestamp, e.event_type, e.description, cls, e.severity
            ));
        }
        html.push_str("</table>");
    }

    html.push_str("</div></body></html>");
    html
}

// ══════════════════════════════════════════════════════════════════
//  单元测试
// ══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> ReportData {
        let mut d = ReportData::new("FM001-TEST", "2026-04-03 08:00:00");
        for i in 0..24 {
            let t = i as f32;
            d.voltage_series[0].push(220.0 + (t * 0.1).sin() * 2.0);
            d.voltage_series[1].push(219.0 + (t * 0.1 + 1.0).sin() * 2.0);
            d.voltage_series[2].push(221.0 + (t * 0.1 + 2.0).sin() * 2.0);
            d.current_series[0].push(5.0 + (t * 0.3).sin() * 2.0);
            d.current_series[1].push(4.8 + (t * 0.3 + 1.0).sin() * 2.0);
            d.current_series[2].push(5.2 + (t * 0.3 + 2.0).sin() * 2.0);
            d.power_series.push(3300.0 + (t * 0.3).sin() * 500.0);
        }
        d.event_log.push(EventLogEntry {
            timestamp: "2026-04-03 02:15:00".into(),
            event_type: "电压暂降".into(),
            description: "A相电压降至 195V".into(),
            severity: "中".into(),
        });
        d.tariff_stats.push(TariffStat {
            tariff_name: "尖峰".into(),
            energy_kwh: 120.5,
            cost_yuan: 145.0,
            percentage: 25.0,
        });
        d.tariff_stats.push(TariffStat {
            tariff_name: "高峰".into(),
            energy_kwh: 200.0,
            cost_yuan: 180.0,
            percentage: 42.0,
        });
        d.tariff_stats.push(TariffStat {
            tariff_name: "平段".into(),
            energy_kwh: 100.0,
            cost_yuan: 55.0,
            percentage: 21.0,
        });
        d.tariff_stats.push(TariffStat {
            tariff_name: "低谷".into(),
            energy_kwh: 55.0,
            cost_yuan: 16.5,
            percentage: 12.0,
        });
        d
    }

    #[test]
    fn test_generate_report_contains_html() {
        let data = sample_data();
        let html = generate_html_report(&data);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn test_report_contains_meter_id() {
        let data = sample_data();
        let html = generate_html_report(&data);
        assert!(html.contains("FM001-TEST"));
    }

    #[test]
    fn test_report_contains_svg() {
        let data = sample_data();
        let html = generate_html_report(&data);
        assert!(html.contains("<svg"));
    }

    #[test]
    fn test_report_contains_event_log() {
        let data = sample_data();
        let html = generate_html_report(&data);
        assert!(html.contains("电压暂降"));
    }

    #[test]
    fn test_report_contains_tariff() {
        let data = sample_data();
        let html = generate_html_report(&data);
        assert!(html.contains("尖峰"));
        assert!(html.contains("低谷"));
    }

    #[test]
    fn test_report_empty() {
        let data = ReportData::new("EMPTY", "2026-01-01");
        let html = generate_html_report(&data);
        assert!(html.contains("无异常事件"));
    }

    #[test]
    fn test_svg_line_chart_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let series = vec![("test", &data)];
        let svg = svg_line_chart("Test", &series, 400, 200, "Y");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Test"));
    }

    #[test]
    fn test_svg_bar_chart_basic() {
        let stats = vec![TariffStat {
            tariff_name: "A".into(),
            energy_kwh: 100.0,
            cost_yuan: 50.0,
            percentage: 50.0,
        }];
        let svg = svg_tariff_bar(&stats, 400, 200);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("A"));
    }
}
