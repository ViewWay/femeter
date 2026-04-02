//! LCD 段码显示模拟
//!
//! 4COM×44SEG, OBIS 短码显示, 自动轮显, ASCII art

use serde::{Deserialize, Serialize};

/// OBIS 短码 -> 显示项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayItem {
    pub obis_short: String,
    pub label: String,
    pub unit: String,
}

/// 显示状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayMode {
    Normal,
    Battery, // 断电, 只显示电能
    Calibration,
}

pub struct LcdDisplay {
    /// 轮显列表
    items: Vec<DisplayItem>,
    /// 轮显间隔 (秒)
    cycle_interval: u64,
    /// 当前索引
    current_idx: usize,
    /// 显示模式
    mode: DisplayMode,
    /// 按键翻页偏移
    key_offset: i32,
}

impl Default for LcdDisplay {
    fn default() -> Self {
        Self {
            items: default_display_items(),
            cycle_interval: 5,
            current_idx: 0,
            mode: DisplayMode::Normal,
            key_offset: 0,
        }
    }
}

fn default_display_items() -> Vec<DisplayItem> {
    vec![
        DisplayItem {
            obis_short: "1.0.0.0.0".into(),
            label: "有功电能".into(),
            unit: "kWh".into(),
        },
        DisplayItem {
            obis_short: "1.0.1.0.0".into(),
            label: "无功电能".into(),
            unit: "kvarh".into(),
        },
        DisplayItem {
            obis_short: "1.0.12.7.0".into(),
            label: "A相电压".into(),
            unit: "V".into(),
        },
        DisplayItem {
            obis_short: "1.0.13.7.0".into(),
            label: "A相电流".into(),
            unit: "A".into(),
        },
        DisplayItem {
            obis_short: "1.0.14.7.0".into(),
            label: "有功功率".into(),
            unit: "W".into(),
        },
        DisplayItem {
            obis_short: "1.0.15.7.0".into(),
            label: "无功功率".into(),
            unit: "var".into(),
        },
        DisplayItem {
            obis_short: "1.0.21.7.0".into(),
            label: "功率因数".into(),
            unit: "".into(),
        },
        DisplayItem {
            obis_short: "1.0.1.7.0".into(),
            label: "频率".into(),
            unit: "Hz".into(),
        },
    ]
}

impl LcdDisplay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_cycle_interval(&mut self, secs: u64) {
        self.cycle_interval = secs;
    }
    pub fn set_mode(&mut self, mode: DisplayMode) {
        self.mode = mode;
    }
    pub fn mode(&self) -> DisplayMode {
        self.mode
    }

    /// 按键翻页
    pub fn next_page(&mut self) {
        self.key_offset += 1;
        self.current_idx = ((self.current_idx as i32 + 1) as usize) % self.items.len();
    }

    pub fn prev_page(&mut self) {
        self.current_idx = if self.current_idx == 0 {
            self.items.len() - 1
        } else {
            self.current_idx - 1
        };
    }

    /// 获取当前显示项
    pub fn current_item(&self) -> &DisplayItem {
        &self.items[self.current_idx]
    }

    /// 生成 ASCII art 显示
    pub fn render_ascii(&self, value: f64, battery: bool) -> String {
        let item = self.current_item();
        let mut art = String::new();

        if battery {
            art.push_str("╔═══════════════════════════════════╗\n");
            art.push_str("║  [BAT] FeMeter                    ║\n");
            art.push_str("╠═══════════════════════════════════╣\n");
            art.push_str(&format!(
                "║  {:>12} {:>10} {:>4}   ║\n",
                item.label,
                format!("{:.2}", value),
                item.unit
            ));
            art.push_str("╚═══════════════════════════════════╝\n");
        } else {
            art.push_str("╔══════════════════════════════════════════╗\n");
            art.push_str("║  FeMeter Virtual Meter v1.0              ║\n");
            art.push_str("╠══════════════════════════════════════════╣\n");
            art.push_str(&format!(
                "║  OBIS: {:14}  {:>4}/{:<4}      ║\n",
                item.obis_short,
                self.current_idx + 1,
                self.items.len()
            ));
            art.push_str(&format!(
                "║  {:>10}: {:>12} {:>4}     ║\n",
                item.label, value, item.unit
            ));
            art.push_str("╚══════════════════════════════════════════╝\n");
        }
        art
    }

    /// 模拟 4COM×44SEG 段码映射
    pub fn segment_map(&self, text: &str) -> Vec<u8> {
        // Simplified: map ASCII to 7-segment representation
        let mut segments = vec![0u8; 44]; // 44 segments
        let seg_chars: &[(char, u8)] = &[
            ('0', 0x3F),
            ('1', 0x06),
            ('2', 0x5B),
            ('3', 0x4F),
            ('4', 0x66),
            ('5', 0x6D),
            ('6', 0x7D),
            ('7', 0x07),
            ('8', 0x7F),
            ('9', 0x6F),
            ('-', 0x40),
            (' ', 0x00),
        ];
        for (i, c) in text.chars().take(6).enumerate() {
            let seg = seg_chars
                .iter()
                .find(|(ch, _)| *ch == c)
                .map(|(_, s)| *s)
                .unwrap_or(0x00);
            if i * 2 + 1 < segments.len() {
                segments[i * 2] = seg & 0x0F;
                segments[i * 2 + 1] = (seg >> 4) & 0x0F;
            }
        }
        segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_cycle() {
        let mut disp = LcdDisplay::new();
        let initial = disp.current_item().obis_short.clone();
        disp.next_page();
        assert_ne!(disp.current_item().obis_short, initial);
    }

    #[test]
    fn test_ascii_render() {
        let disp = LcdDisplay::new();
        let art = disp.render_ascii(12345.67, false);
        assert!(art.contains("FeMeter"));
        assert!(art.contains("12345.67"));
    }

    #[test]
    fn test_battery_mode() {
        let disp = LcdDisplay::new();
        let art = disp.render_ascii(100.0, true);
        assert!(art.contains("[BAT]"));
    }

    #[test]
    fn test_segment_map() {
        let disp = LcdDisplay::new();
        let segs = disp.segment_map("123456");
        assert!(segs.iter().any(|&s| s != 0));
    }
}
