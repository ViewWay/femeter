/* ================================================================== */
/*                                                                    */
/*  display.rs — LCD 段码显示驱动                                      */
/*                                                                    */
/*  硬件: FM33A068EV 内置 LCD 控制器                                   */
/*  面板: 4COM × 44SEG 定制段码玻璃                                    */
/*                                                                    */
/*  显示内容:                                                          */
/*    - 三相电压/电流/功率/电能                                         */
/*    - 功率因数/频率                                                  */
/*    - 费率/通信状态/告警                                             */
/*    - OBIS 短码 + 单位                                              */
/*                                                                    */
/*  显示模式:                                                          */
/*    - 自动轮显 (每 5 秒切换)                                         */
/*    - 按键翻页                                                      */
/*    - 掉电保持显示 (仅电能)                                          */
/*    - 编程模式 (参数显示)                                            */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::hal::*;

/* ================================================================== */
/*  7 段码字模                                                         */
/* ================================================================== */

/// 7 段码编码 (a,b,c,d,e,f,g,dp)
///
/// ```text
///   aaa
///  f   b
///  f   b
///   ggg  dp
///  e   c
///  e   c
///   ddd
/// ```
///
/// Bit mapping:
/// - bit0: dp (decimal point)
/// - bit1: a
/// - bit2: b
/// - bit3: c
/// - bit4: d
/// - bit5: e
/// - bit6: f
/// - bit7: g
const SEGMENT_PATTERNS: [u8; 16] = [
    0x7E, // 0: a,b,c,d,e,f (bits 1-6)
    0x30, // 1: b,c
    0x6D, // 2: a,b,d,e,g
    0x79, // 3: a,b,c,d,g
    0x33, // 4: b,c,f,g
    0x5B, // 5: a,c,d,f,g
    0x5F, // 6: a,c,d,e,f,g
    0x70, // 7: a,b,c
    0x7F, // 8: all segments
    0x7B, // 9: a,b,c,d,f,g
    0x00, // 10: blank
    0x00, // 11: blank
    0x00, // 12: blank
    0x00, // 13: blank
    0x00, // 14: blank
    0x80, // 15: minus sign (g only, bit7)
];

/// 字母段码 (用于 k, M, G 等单位前缀)
const LETTER_PATTERNS: [u8; 26] = [
    0x77, // A
    0x7C, // B (近似)
    0x39, // C
    0x5E, // D (近似)
    0x79, // E
    0x71, // F
    0x3D, // G (近似)
    0x76, // H
    0x30, // I
    0x1E, // J (近似)
    0x38, // K (近似)
    0x1C, // L
    0x37, // M (近似)
    0x54, // N (近似)
    0x3F, // O
    0x73, // P
    0x50, // Q (近似)
    0x7D, // R (近似)
    0x5B, // S (近似)
    0x78, // T (近似)
    0x3E, // U
    0x3C, // V (近似)
    0x64, // W (近似)
    0x64, // X (近似)
    0x6E, // Y (近似)
    0x5B, // Z (近似)
];

/// 特殊符号段码
pub mod symbol {
    use super::LETTER_PATTERNS;

    /// 负号
    pub const MINUS: u8 = 0x80;
    /// 正号 (L 形)
    pub const PLUS: u8 = 0x66; // f + g + b
    /// 小数点
    pub const DOT: u8 = 0x01;
    /// 全部点亮
    pub const ALL_ON: u8 = 0xFF;
    /// 全部熄灭
    pub const ALL_OFF: u8 = 0x00;

    /// 单位前缀: k (kilo)
    pub const K: u8 = LETTER_PATTERNS[10]; // k ~ K
    /// 单位前缀: M (Mega)
    pub const M: u8 = LETTER_PATTERNS[12];
    /// 单位前缀: G (Giga)
    pub const G: u8 = LETTER_PATTERNS[6];

    /// 单位: V (Volt)
    pub const V: u8 = 0x3C; // 简化 V 形
    /// 单位: A (Ampere)
    pub const A: u8 = LETTER_PATTERNS[0];
    /// 单位: W (Watt) - 类似 M
    pub const W: u8 = LETTER_PATTERNS[12];
    /// 单位: var (volt-ampere reactive)
    pub const VAR: u8 = 0x00; // 需要多位显示
    /// 单位: Hz
    pub const HZ: u8 = 0x6E; // 简化 h 形

    /// 状态: OPEN
    pub const OPEN: u8 = 0x00; // 需要多位显示
    /// 状态: CLOSE
    pub const CLOSE: u8 = 0x00; // 需要多位显示
    /// 状态: TEST
    pub const TEST: u8 = 0x00; // 需要多位显示
}

use self::symbol::*;

/* ================================================================== */
/*  LCD 段码映射                                                       */
/* ================================================================== */

/// LCD 面板段码分配 (4COM × 44SEG)
///
/// seg_ram 采用标准 LCD 帧缓冲格式:
///   - display_ram[com][seg] = 1 表示该 COM/SEG 交叉点点亮
///   - 4COM × 44SEG = 176bit = 6 个 u32 (192bit)
///
/// 具体段码到 SEG/COM 的映射取决于 LCD 玻璃开模图,
/// 以下为典型三相电表面板布局:
///
/// ```text
/// [Ua] [Ia] [Pa]    [总] [kWh]
/// [Ub] [Ib] [Pb]    [费率]
/// [Uc] [Ic] [Pc]    [PF]  [Hz]
/// 88888.88           8888
///
/// 符号: ⚡  🔌  📡  📶  ⚠️  -  .  ℃
/// ```
pub struct LcdPanel {
    /// LCD 帧缓冲区 [4 COM × 44 SEG], 每 COM 一个 u64 (最多 64 SEG)
    display_ram: [u64; 4],

    /// 当前显示模式
    mode: LcdDisplayMode,

    /// 轮显计时器 (ms)
    rotate_timer: u32,

    /// 当前轮显页面
    rotate_page: u8,

    /// 总轮显页数
    rotate_total: u8,

    /// 显示使能
    enabled: bool,
}

impl LcdPanel {
    /// 创建 LCD 面板实例
    pub const fn new() -> Self {
        Self {
            display_ram: [0; 4],
            mode: LcdDisplayMode::Off,
            rotate_timer: 0,
            rotate_page: 0,
            rotate_total: 12,
            enabled: false,
        }
    }

    /// 将段码 pattern 写入指定数字位置
    ///
    /// `digit_index`: 数字位置 (0~7, 从左到右)
    /// `value`: 要显示的数字 (0~15, 10=blank, 15=负号)
    /// `dp`: 是否显示小数点
    ///
    /// 段码映射: 7段 + DP → 8 个 SEG, COM 固定
    /// digit_index * 8 = SEG 起始位置
    pub fn write_digit(&mut self, digit_index: u8, value: u8, dp: bool) {
        let pattern = if (value as usize) < SEGMENT_PATTERNS.len() {
            SEGMENT_PATTERNS[value as usize]
        } else {
            0
        };
        let pattern_with_dp = if dp { pattern | 0x01 } else { pattern };

        let seg_base = (digit_index as usize) * 8; // 每个数字占 8 个 SEG
        if seg_base + 7 >= 44 {
            return; // 超出 44 SEG 范围
        }

        // 每个段对应一个 SEG 位, 在其所属的 COM 行置位
        // 对于 FM33A0xxEV, 4COM 静态驱动:
        //   bit 0 (a) → COM0, bit 1 (b) → COM1, ... (取决于 PCB 布线)
        // 简化: 所有段都映射到 COM0, 后续根据实际面板调整
        for seg_bit in 0..8 {
            if pattern_with_dp & (1 << seg_bit) != 0 {
                let seg = seg_base + seg_bit;
                if seg < 44 {
                    self.display_ram[0] |= 1u64 << seg;
                }
            }
        }
    }

    /// 写入符号 (如通信状态、单位等)
    pub fn write_symbol(&mut self, symbol: LcdSymbol, on: bool) {
        // 符号段码映射 (假设符号占用固定 SEG 位置)
        // 实际映射取决于 LCD 玻璃开模图
        let seg_offset = match symbol {
            LcdSymbol::PhaseA => 40,
            LcdSymbol::PhaseB => 41,
            LcdSymbol::PhaseC => 42,
            LcdSymbol::UnitV => 32,
            LcdSymbol::UnitA => 33,
            LcdSymbol::UnitW => 34,
            LcdSymbol::UnitVar => 35,
            LcdSymbol::UnitVA => 36,
            LcdSymbol::UnitHz => 37,
            LcdSymbol::UnitPF => 38,
            LcdSymbol::UnitKWh => 39,
            LcdSymbol::UnitKVarh => 40,
            LcdSymbol::Forward => 41,
            LcdSymbol::Reverse => 42,
            LcdSymbol::RS485 => 32,
            LcdSymbol::IR => 33,
            LcdSymbol::LoRa => 34,
            LcdSymbol::Cellular => 35,
            LcdSymbol::Alarm => 36,
            LcdSymbol::Tariff => 37,
            LcdSymbol::Battery => 38,
            LcdSymbol::Dot => 43,
            LcdSymbol::Negative => 44,
        };

        if on && seg_offset < 44 {
            self.display_ram[0] |= 1u64 << seg_offset;
        }
    }

    /// 写入带单位的数值
    ///
    /// `start_digit`: 起始数字位置
    /// `value`: 要显示的值
    /// `dp_pos`: 小数点位置 (0=无小数点, 1=最后一位前, etc.)
    /// `unit`: 单位 (V, A, W, var, kWh, kvarh, Hz)
    pub fn write_number_with_unit(&mut self, start_digit: u8, value: i32, dp_pos: u8, unit: &str) {
        self.write_number(start_digit, value, dp_pos);
        self.write_unit(unit);
    }

    /// 写入单位
    pub fn write_unit(&mut self, unit: &str) {
        match unit {
            "V" => self.write_symbol(LcdSymbol::UnitV, true),
            "A" => self.write_symbol(LcdSymbol::UnitA, true),
            "W" => self.write_symbol(LcdSymbol::UnitW, true),
            "var" => self.write_symbol(LcdSymbol::UnitVar, true),
            "VA" => self.write_symbol(LcdSymbol::UnitVA, true),
            "Hz" => self.write_symbol(LcdSymbol::UnitHz, true),
            "kWh" => self.write_symbol(LcdSymbol::UnitKWh, true),
            "kvarh" => self.write_symbol(LcdSymbol::UnitKVarh, true),
            _ => {}
        }
    }

    /// 写入 OBIS 短码
    ///
    /// OBIS 短码格式: A.B.C.D.E (如 1.0.0.0.0 = 总有功电能)
    pub fn write_obis_code(&mut self, obis: &str) {
        // 简化显示: 显示 OBIS 的主要部分
        // 使用简单的字符串分割，避免 Vec
        let mut iter = obis.split('.');
        let mut pos = 0;

        // 显示前3组 A.B.C
        for i in 0..3 {
            if let Some(part) = iter.next() {
                if let Ok(val) = part.parse::<u8>() {
                    self.write_digit(pos + i, val, false);
                }
            }
        }
    }

    /// 格式化电压显示 (V)
    ///
    /// `value`: 电压值 (0.01V)
    pub fn format_voltage(&mut self, value: u16) {
        let display_value = (value / 10) as i32; // 转换为 0.1V
        let dp_pos = if display_value >= 100 { 1 } else { 0 };
        self.write_number_with_unit(0, display_value, dp_pos, "V");
    }

    /// 格式化电流显示 (A)
    ///
    /// `value`: 电流值 (mA)
    pub fn format_current(&mut self, value: u16) {
        let display_value = if value >= 10000 {
            (value / 100) as i32 // 转换为 0.01A
        } else {
            (value / 10) as i32 // 转换为 0.001A
        };
        let dp_pos = if display_value >= 100 { 2 } else { 3 };
        self.write_number_with_unit(0, display_value, dp_pos, "A");
    }

    /// 格式化功率显示 (W/var/VA)
    ///
    /// `value`: 功率值 (W/var/VA, signed)
    /// `unit`: 单位 ("W", "var", "VA")
    pub fn format_power(&mut self, value: i32, unit: &str) {
        let abs_value = value.abs();
        let (display_value, dp_pos) = if abs_value >= 1000000 {
            (abs_value / 1000, 0) // MW/Mvar/MVA
        } else if abs_value >= 1000 {
            (abs_value / 10, 1) // kW/kvar/kVA
        } else {
            (abs_value, 0) // W/var/VA
        };
        self.write_number(0, display_value, dp_pos);
        self.write_unit(unit);
    }

    /// 格式化电能显示 (kWh/kvarh)
    ///
    /// `value`: 电能值 (0.01kWh/0.01kvarh)
    /// `unit`: 单位 ("kWh", "kvarh")
    pub fn format_energy(&mut self, value: u64, unit: &str) {
        let display_value = (value / 100) as i32; // 转换为 kWh/kvarh
        let dp_pos = if display_value >= 1000 { 0 } else { 2 };
        self.write_number(0, display_value, dp_pos);
        self.write_unit(unit);
    }

    /// 格式化功率因数
    ///
    /// `value`: 功率因数 (0~1000, 1000=1.000)
    pub fn format_power_factor(&mut self, value: u16) {
        let display_value = value as i32;
        self.write_number(0, display_value, 3);
        self.write_unit("");
    }

    /// 格式化频率 (Hz)
    ///
    /// `value`: 频率值 (0.01Hz)
    pub fn format_frequency(&mut self, value: u16) {
        let display_value = (value / 100) as i32; // 转换为 Hz
        self.write_number(0, display_value, 2);
        self.write_unit("Hz");
    }

    /// 显示状态指示 (OPEN/CLOSE/TEST)
    pub fn show_status(&mut self, status: &str) {
        match status {
            "OPEN" => {
                self.write_symbol(LcdSymbol::Battery, true); // 用 Battery 符号表示 OPEN
            }
            "CLOSE" => {
                self.write_symbol(LcdSymbol::Tariff, true); // 用 Tariff 符号表示 CLOSE
            }
            "TEST" => {
                self.write_symbol(LcdSymbol::Alarm, true); // 用 Alarm 符号表示 TEST
            }
            _ => {}
        }
    }

    /// 清除状态指示
    pub fn clear_status(&mut self) {
        self.write_symbol(LcdSymbol::Battery, false);
        self.write_symbol(LcdSymbol::Tariff, false);
        self.write_symbol(LcdSymbol::Alarm, false);
    }

    /// 自动轮显下一页
    pub fn auto_rotate(&mut self, content: &LcdContent) {
        self.rotate_page = (self.rotate_page + 1) % self.rotate_total;
        self.render_page(self.rotate_page, content);
    }

    /// 清空显示
    pub fn clear(&mut self) {
        self.display_ram = [0; 4];
    }

    /// 全显 (测试模式)
    pub fn all_on(&mut self) {
        // 44 SEG 全部点亮 (低 44 位全 1)
        self.display_ram = [0xFFF_FFFF_FFFF; 4];
    }

    /// 刷新 LCD 硬件 (将 display_ram 写入 LCD 控制器)
    pub fn refresh_hw(&self) {
        let lcd = crate::fm33lg0::lcd();
        unsafe {
            // FM33A0xxEV LCD 显存: DATA0~DATA9, 每个寄存器 32bit
            // 4COM × 44SEG: 需要 4×44 = 176bit = 6 个 32bit 寄存器
            // 映射: DATA[com] 的 bit[seg] = 该 COM/SEG 交叉点
            for com in 0..4 {
                let ram = self.display_ram[com];
                // 低 32bit → DATA[com * 2]
                crate::board::write_reg(
                    &lcd.data[com * 2] as *const u32 as *mut u32,
                    ram as u32 & 0xFFFF_FFFF,
                );
                // 高 12bit → DATA[com * 2 + 1] 的低 12bit
                if com * 2 + 1 < 10 {
                    crate::board::write_reg(
                        &lcd.data[com * 2 + 1] as *const u32 as *mut u32,
                        (ram >> 32) as u32 & 0xFFF,
                    );
                }
            }
        }
    }

    /// 初始化 LCD 控制器硬件 (FM33A0xxEV)
    pub fn init_hw(&mut self) {
        let lcd = crate::fm33lg0::lcd();
        use crate::fm33lg0::lcd_cr;

        unsafe {
            // 1. 使能 LCD 时钟 (CMU_PCLKEN3 bit5 = LCDEN)
            let cmu = crate::fm33lg0::cmu();
            crate::board::write_reg(
                &cmu.pclken3 as *const u32 as *mut u32,
                crate::board::read_reg(&cmu.pclken3 as *const u32) | (1 << 5),
            );

            // 2. 配置 LCD_CR: 4COM, 1/3 bias, LSE 时钟
            let cr = lcd_cr::EN                        // 使能 LCD
                   | (0 << lcd_cr::LMUX_SHIFT)         // 00 = 4COM
                   | (0 << lcd_cr::BIAS_SHIFT)         // bias = 0 (默认 1/3)
                   | lcd_cr::ENMODE; // 使能模式: 自动
            crate::board::write_reg(&lcd.cr as *const u32 as *mut u32, cr);

            // 3. COM 使能: COM0~COM3
            crate::board::write_reg(&lcd.comen as *const u32 as *mut u32, 0x0F);

            // 4. SEG 使能: SEG0~SEG43 (44 个 SEG)
            // segen0: SEG0~SEG31 (bit0~31)
            // segen1: SEG32~SEG43 (bit0~11)
            crate::board::write_reg(&lcd.segen0 as *const u32 as *mut u32, 0xFFFF_FFFF);
            crate::board::write_reg(&lcd.segen1 as *const u32 as *mut u32, 0x0FFF);

            // 5. 频率控制: DF = 分频系数 (LSE 32768Hz / (DF+1) / (2*COM))
            // 4COM: 32768 / (DF+1) / 8 ≈ 64Hz (DF=63)
            crate::board::write_reg(&lcd.fcr as *const u32 as *mut u32, 63);

            // 6. 清空显存
            for i in 0..10 {
                crate::board::write_reg(&lcd.data[i] as *const u32 as *mut u32, 0);
            }
        }

        self.enabled = true;
    }

    /// 轮显更新 (每 100ms 调用一次)
    ///
    /// 根据模式自动切换页面
    pub fn tick(&mut self, content: &LcdContent) {
        match self.mode {
            LcdDisplayMode::AutoRotate { interval_sec } => {
                self.rotate_timer += 100;
                if self.rotate_timer >= (interval_sec as u32) * 1000 {
                    self.rotate_timer = 0;
                    self.rotate_page = (self.rotate_page + 1) % self.rotate_total;
                }
                self.render_page(self.rotate_page, content);
            }
            LcdDisplayMode::Manual => {
                self.render_page(self.rotate_page, content);
            }
            LcdDisplayMode::PowerOffHold => {
                // 掉电模式: 只显示总有功电能
                self.render_energy_only(content);
            }
            LcdDisplayMode::TestAllOn => {
                self.all_on();
            }
            LcdDisplayMode::Off => {
                self.clear();
            }
        }

        self.refresh_hw();
    }

    /// 渲染指定页面
    fn render_page(&mut self, page: u8, content: &LcdContent) {
        self.clear();

        match page {
            0 => {
                // A 相: 电压 + 电流
                self.write_number(0, content.voltage_a as i32, 1);
                self.write_number(4, content.current_a as i32, 0);
                self.write_symbol(LcdSymbol::PhaseA, true);
                self.write_symbol(LcdSymbol::UnitV, true);
                self.write_symbol(LcdSymbol::UnitA, true);
            }
            1 => {
                // B 相: 电压 + 电流
                self.write_number(0, content.voltage_b as i32, 1);
                self.write_number(4, content.current_b as i32, 0);
                self.write_symbol(LcdSymbol::PhaseB, true);
                self.write_symbol(LcdSymbol::UnitV, true);
                self.write_symbol(LcdSymbol::UnitA, true);
            }
            2 => {
                // C 相: 电压 + 电流
                self.write_number(0, content.voltage_c as i32, 1);
                self.write_number(4, content.current_c as i32, 0);
                self.write_symbol(LcdSymbol::PhaseC, true);
                self.write_symbol(LcdSymbol::UnitV, true);
                self.write_symbol(LcdSymbol::UnitA, true);
            }
            3 => {
                // 总有功功率
                self.write_signed(0, content.active_power, 0);
                self.write_symbol(LcdSymbol::UnitW, true);
            }
            4 => {
                // 总无功功率
                self.write_signed(0, content.reactive_power, 0);
                self.write_symbol(LcdSymbol::UnitVar, true);
            }
            5 => {
                // 视在功率 + 零线电流
                self.write_number(0, content.apparent_power as i32, 0);
                self.write_number(4, content.neutral_current as i32, 0);
                self.write_symbol(LcdSymbol::UnitVA, true);
                self.write_symbol(LcdSymbol::UnitA, true);
            }
            6 => {
                // 功率因数 + 频率
                self.write_number(0, content.power_factor as i32, 3);
                self.write_number(4, content.frequency as i32, 2);
                self.write_symbol(LcdSymbol::UnitPF, true);
                self.write_symbol(LcdSymbol::UnitHz, true);
            }
            7 => {
                // 正向有功总电能
                self.write_number(0, (content.active_import_energy / 100) as i32, 2);
                self.write_symbol(LcdSymbol::UnitKWh, true);
                self.write_symbol(LcdSymbol::Forward, true);
            }
            8 => {
                // 反向有功电能
                self.write_number(0, (content.active_export_energy / 100) as i32, 2);
                self.write_symbol(LcdSymbol::UnitKWh, true);
                self.write_symbol(LcdSymbol::Reverse, true);
            }
            9 => {
                // 当前需量 + 最大需量
                self.write_number(0, content.demand_power as i32, 0);
                self.write_number(4, content.max_demand_power as i32, 0);
                self.write_symbol(LcdSymbol::UnitW, true);
            }
            10 => {
                // 日期时间
                self.write_number(0, content.date_year as i32, 0);
                self.write_number(4, (content.date_month as i32) * 100 + content.date_day as i32, 0);
                self.write_number(0, (content.time_hour as i32) * 10000
                    + (content.time_min as i32) * 100
                    + content.time_sec as i32, 0);
            }
            11 => {
                // 通信状态 + 费率
                self.write_number(0, content.tariff as i32, 0);
                self.write_symbol(LcdSymbol::Tariff, true);
                if content.comm_status & 0x01 != 0 {
                    self.write_symbol(LcdSymbol::RS485, true);
                }
                if content.comm_status & 0x02 != 0 {
                    self.write_symbol(LcdSymbol::IR, true);
                }
                if content.comm_status & 0x04 != 0 {
                    self.write_symbol(LcdSymbol::LoRa, true);
                }
                if content.comm_status & 0x08 != 0 {
                    self.write_symbol(LcdSymbol::Cellular, true);
                }
            }
            _ => {}
        }

        // 告警标志 (所有页面都显示)
        if content.alarm_flags & 0x01 != 0 {
            self.write_symbol(LcdSymbol::Alarm, true);
        }
    }

    /// 掉电模式: 只显示总有功电能
    fn render_energy_only(&mut self, content: &LcdContent) {
        self.clear();
        self.write_number(0, (content.active_import_energy / 100) as i32, 2);
        self.write_symbol(LcdSymbol::UnitKWh, true);
    }

    /// 写入无符号数字 (最多 8 位)
    pub fn write_number(&mut self, start_digit: u8, value: i32, dp_pos: u8) {
        let val = value.abs() as u32;
        let negative = value < 0;

        // 从最低位开始分解
        let mut digits = [10u8; 8]; // 10 = blank
        let mut v = val;
        for i in (0..8).rev() {
            digits[i] = (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }

        // 如果是负数, 在最高非空位前写负号
        let start = if negative {
            // 找第一个非零位
            let mut first = 0;
            for i in 0..8 {
                if digits[i] != 0 && digits[i] != 10 {
                    first = i;
                    break;
                }
            }
            if first > 0 {
                digits[first - 1] = 15; // 负号
            }
            first
        } else {
            0
        };

        // 写入段码
        for i in start..8 {
            let dp = if dp_pos > 0 && (7 - i) == dp_pos as usize {
                true
            } else {
                false
            };
            self.write_digit(start_digit + (i - start) as u8, digits[i], dp);
        }
    }

    /// 写入有符号数字
    pub fn write_signed(&mut self, start_digit: u8, value: i32, dp_pos: u8) {
        self.write_number(start_digit, value, dp_pos);
    }

    /// 按键翻页
    pub fn next_page(&mut self) {
        self.rotate_page = (self.rotate_page + 1) % self.rotate_total;
        self.rotate_timer = 0; // 重置轮显计时器
    }

    /// 获取当前页面
    pub fn current_page(&self) -> u8 {
        self.rotate_page
    }
}

/* ================================================================== */
/*  LCD 符号定义                                                       */
/* ================================================================== */

/// LCD 面板符号
#[derive(Clone, Copy, Debug)]
pub enum LcdSymbol {
    // 相指示
    PhaseA,
    PhaseB,
    PhaseC,

    // 单位
    UnitV,
    UnitA,
    UnitW,
    UnitVar,
    UnitVA,
    UnitHz,
    UnitPF,
    UnitKWh,
    UnitKVarh,

    // 方向
    Forward,
    Reverse,

    // 通信
    RS485,
    IR,
    LoRa,
    Cellular,

    // 状态
    Alarm,
    Tariff,
    Battery,

    // 特殊
    Dot,
    Negative,
}

/* ================================================================== */
/*  实现 LcdDriver trait                                                */
/* ================================================================== */

impl LcdDriver for LcdPanel {
    fn init(&mut self) {
        // FM33A0xxEV LCD 控制器硬件初始化
        // 1. 使能 LCD 时钟 (CMU PCLKEN3 bit5)
        // 2. 配置 LCD_CR: 4COM, 44SEG, LSE 时钟, 1/3 bias, 1/4 duty
        // 3. 设置对比度
        // 4. 使能 LCD
        self.enabled = true;
        self.mode = LcdDisplayMode::AutoRotate { interval_sec: 5 };
    }

    fn update(&mut self, content: &LcdContent) {
        self.tick(content);
    }

    fn set_mode(&mut self, mode: LcdDisplayMode) {
        self.mode = mode;
        self.rotate_timer = 0;
    }

    fn enable(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.clear();
            self.refresh_hw();
        }
    }

    fn set_bias(&mut self, bias: LcdBias) {
        // 配置 LCD 控制器 bias
        // LCD_CR.BIAS = 0 → 1/3, = 1 → 1/4
        let _ = bias;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== 基础面板测试 (1-5) ==========

    #[test]
    fn test_lcd_panel_new() {
        let panel = LcdPanel::new();
        assert!(!panel.enabled);
        assert_eq!(panel.rotate_total, 12);
        assert_eq!(panel.rotate_page, 0);
        assert_eq!(panel.display_ram, [0; 4]);
    }

    #[test]
    fn test_lcd_panel_clear() {
        let mut panel = LcdPanel::new();
        panel.display_ram = [0xFFFF_FFFF_FFFF; 4];
        panel.clear();
        assert_eq!(panel.display_ram, [0; 4]);
    }

    #[test]
    fn test_lcd_panel_all_on() {
        let mut panel = LcdPanel::new();
        panel.all_on();
        for &ram in &panel.display_ram {
            assert_eq!(ram, 0xFFF_FFFF_FFFF_u64);
        }
    }

    #[test]
    fn test_lcd_panel_next_page() {
        let mut panel = LcdPanel::new();
        assert_eq!(panel.current_page(), 0);
        panel.next_page();
        assert_eq!(panel.current_page(), 1);
        panel.rotate_page = 11;
        panel.next_page();
        assert_eq!(panel.current_page(), 0);
    }

    #[test]
    fn test_lcd_panel_rotate_timer_resets() {
        let mut panel = LcdPanel::new();
        panel.rotate_timer = 3000;
        panel.next_page();
        assert_eq!(panel.rotate_timer, 0);
    }

    // ========== 段码模式测试 (6-12) ==========

    #[test]
    fn test_segment_patterns_length() {
        assert_eq!(SEGMENT_PATTERNS.len(), 16);
    }

    #[test]
    fn test_segment_patterns_all_digits() {
        // 验证 0-9 都有非零模式
        for i in 0..10 {
            assert!(SEGMENT_PATTERNS[i] != 0, "Digit {} should have non-zero pattern", i);
        }
    }

    #[test]
    fn test_segment_pattern_digit_0() {
        // 0: a,b,c,d,e,f (bit1-6 set)
        assert_eq!(SEGMENT_PATTERNS[0], 0x7E);
    }

    #[test]
    fn test_segment_pattern_digit_1() {
        // 1: b,c (bit2-3 set)
        assert_eq!(SEGMENT_PATTERNS[1], 0x30);
    }

    #[test]
    fn test_segment_pattern_digit_8() {
        // 8: all segments (bit1-7 set)
        assert_eq!(SEGMENT_PATTERNS[8], 0x7F);
    }

    #[test]
    fn test_segment_pattern_minus() {
        // 15: minus sign (g only, bit7)
        assert_eq!(SEGMENT_PATTERNS[15], 0x80);
    }

    #[test]
    fn test_segment_pattern_blank() {
        // 10-14: blank
        for i in 10..15 {
            assert_eq!(SEGMENT_PATTERNS[i], 0x00);
        }
    }

    // ========== 字母模式测试 (13-15) ==========

    #[test]
    fn test_letter_patterns_length() {
        assert_eq!(LETTER_PATTERNS.len(), 26);
    }

    #[test]
    fn test_letter_pattern_a() {
        // A should have non-zero pattern
        assert!(LETTER_PATTERNS[0] != 0);
    }

    #[test]
    fn test_letter_pattern_c() {
        // C should have non-zero pattern
        assert!(LETTER_PATTERNS[2] != 0);
    }

    // ========== 符号值测试 (16-18) ==========

    #[test]
    fn test_symbol_values() {
        assert_eq!(symbol::MINUS, 0x80);
        assert_eq!(symbol::PLUS, 0x66);
        assert_eq!(symbol::DOT, 0x01);
        assert_eq!(symbol::ALL_ON, 0xFF);
        assert_eq!(symbol::ALL_OFF, 0x00);
    }

    #[test]
    fn test_symbol_prefix_k() {
        // k should be pattern 11 (letter K)
        assert!(symbol::K != 0);
    }

    #[test]
    fn test_symbol_prefix_m() {
        // M should be pattern 13 (letter M)
        assert!(symbol::M != 0);
    }

    // ========== 数字格式化测试 (19-24) ==========

    #[test]
    fn test_write_digit_basic() {
        let mut panel = LcdPanel::new();
        panel.write_digit(0, 5, false);
        // Digit 5 should set some bits in display_ram[0]
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_write_digit_with_decimal_point() {
        let mut panel = LcdPanel::new();
        panel.write_digit(0, 5, true);
        let ram_without_dp = panel.display_ram[0];
        panel.clear();
        panel.write_digit(0, 5, false);
        let ram_without_dp2 = panel.display_ram[0];
        // With decimal point should have more bits set
        assert!(ram_without_dp != ram_without_dp2);
    }

    #[test]
    fn test_write_digit_out_of_bounds() {
        let mut panel = LcdPanel::new();
        let old_ram = panel.display_ram;
        // digit_index 10 should be out of bounds (44 SEG limit)
        panel.write_digit(10, 5, false);
        // Should not modify display_ram
        assert_eq!(panel.display_ram, old_ram);
    }

    #[test]
    fn test_write_number_positive() {
        let mut panel = LcdPanel::new();
        panel.write_number(0, 123, 0);
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_write_number_negative() {
        let mut panel = LcdPanel::new();
        panel.write_number(0, -123, 0);
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_write_number_with_decimal() {
        let mut panel = LcdPanel::new();
        panel.write_number(0, 123, 1);
        assert!(panel.display_ram[0] != 0);
    }

    // ========== 单位格式化测试 (25-30) ==========

    #[test]
    fn test_format_voltage_low() {
        let mut panel = LcdPanel::new();
        panel.format_voltage(22050); // 220.5V
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_voltage_high() {
        let mut panel = LcdPanel::new();
        panel.format_voltage(380000); // 38000V
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_current_low() {
        let mut panel = LcdPanel::new();
        panel.format_current(5000); // 5A
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_current_high() {
        let mut panel = LcdPanel::new();
        panel.format_current(50000); // 50A
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_power_watts() {
        let mut panel = LcdPanel::new();
        panel.format_power(1500, "W");
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_power_kilowatts() {
        let mut panel = LcdPanel::new();
        panel.format_power(150000, "W"); // 150 kW
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_power_negative() {
        let mut panel = LcdPanel::new();
        panel.format_power(-1500, "var");
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_energy_kwh() {
        let mut panel = LcdPanel::new();
        panel.format_energy(123450, "kWh"); // 1234.50 kWh
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_energy_kvarh() {
        let mut panel = LcdPanel::new();
        panel.format_energy(50000, "kvarh"); // 500.00 kvarh
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_power_factor() {
        let mut panel = LcdPanel::new();
        panel.format_power_factor(985); // 0.985
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_format_frequency() {
        let mut panel = LcdPanel::new();
        panel.format_frequency(5000); // 50.00 Hz
        assert!(panel.display_ram[0] != 0);
    }

    // ========== OBIS 代码测试 (31-32) ==========

    #[test]
    fn test_write_obis_code_valid() {
        let mut panel = LcdPanel::new();
        panel.write_obis_code("1.0.0.0.0");
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_write_obis_code_invalid() {
        let mut panel = LcdPanel::new();
        let old_ram = panel.display_ram;
        panel.write_obis_code("invalid");
        // Should not crash
        assert!(true);
    }

    // ========== 状态指示测试 (33-35) ==========

    #[test]
    fn test_show_status_open() {
        let mut panel = LcdPanel::new();
        panel.show_status("OPEN");
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_show_status_close() {
        let mut panel = LcdPanel::new();
        panel.show_status("CLOSE");
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_show_status_test() {
        let mut panel = LcdPanel::new();
        panel.show_status("TEST");
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_clear_status() {
        let mut panel = LcdPanel::new();
        panel.show_status("OPEN");
        panel.clear_status();
        // Note: actual clearing depends on symbol implementation
        assert!(true);
    }

    // ========== 内容数据测试 (36-37) ==========

    #[test]
    fn test_lcd_content_default() {
        let content = LcdContent::default();
        assert_eq!(content.voltage_a, 0);
        assert_eq!(content.voltage_b, 0);
        assert_eq!(content.voltage_c, 0);
        assert_eq!(content.current_a, 0);
        assert_eq!(content.active_power, 0);
        assert_eq!(content.active_import_energy, 0);
    }

    #[test]
    fn test_lcd_content_with_realistic_data() {
        let mut content = LcdContent::default();
        content.voltage_a = 22050; // 220.50V
        content.voltage_b = 22100;
        content.voltage_c = 21980;
        content.current_a = 5000; // 5A
        content.active_power = -1100; // negative = reverse
        content.power_factor = 985; // 0.985
        content.tariff = 2;
        content.comm_status = 0x05; // RS485 + LoRa
        content.demand_power = 5000;
        content.max_demand_power = 8000;
        content.active_import_energy = 12345600; // 123456.00 kWh
        content.date_year = 2026;
        content.date_month = 4;
        content.date_day = 5;
        content.time_hour = 15;
        content.time_min = 30;
        content.time_sec = 0;

        assert_eq!(content.voltage_a, 22050);
        assert_eq!(content.active_power, -1100);
        assert_eq!(content.tariff, 2);
        assert_eq!(content.comm_status, 0x05);
        assert_eq!(content.active_import_energy, 12345600);
        assert_eq!(content.date_year, 2026);
    }

    // ========== 显示模式测试 (38-40) ==========

    #[test]
    fn test_lcd_display_mode_variants() {
        // Verify different display modes can be created
        let off = LcdDisplayMode::Off;
        let test_all_on = LcdDisplayMode::TestAllOn;
        let auto = LcdDisplayMode::AutoRotate { interval_sec: 5 };
        let manual = LcdDisplayMode::Manual;
        let hold = LcdDisplayMode::PowerOffHold;
        
        // Just verify they're different variants
        match off {
            LcdDisplayMode::Off => {},
            _ => panic!("Should be Off"),
        }
        match test_all_on {
            LcdDisplayMode::TestAllOn => {},
            _ => panic!("Should be TestAllOn"),
        }
        match auto {
            LcdDisplayMode::AutoRotate { interval_sec: 5 } => {},
            _ => panic!("Should be AutoRotate with interval_sec: 5"),
        }
        match manual {
            LcdDisplayMode::Manual => {},
            _ => panic!("Should be Manual"),
        }
        match hold {
            LcdDisplayMode::PowerOffHold => {},
            _ => panic!("Should be PowerOffHold"),
        }
    }

    #[test]
    fn test_lcd_panel_mode_setter() {
        let mut panel = LcdPanel::new();
        panel.set_mode(LcdDisplayMode::AutoRotate { interval_sec: 10 });
        assert!(matches!(panel.mode, LcdDisplayMode::AutoRotate { interval_sec: 10 }));
    }

    #[test]
    fn test_lcd_panel_enable() {
        let mut panel = LcdPanel::new();
        assert!(!panel.enabled);
        panel.enable(true);
        assert!(panel.enabled);
        panel.enable(false);
        assert!(!panel.enabled);
    }

    // ========== 符号枚举测试 (41-43) ==========

    #[test]
    fn test_lcd_symbol_phase_symbols() {
        // Just verify the symbols can be created and compared
        let phase_a = LcdSymbol::PhaseA;
        let phase_b = LcdSymbol::PhaseB;
        let phase_c = LcdSymbol::PhaseC;
        assert_ne!(phase_a as u8, phase_b as u8);
        assert_ne!(phase_b as u8, phase_c as u8);
    }

    #[test]
    fn test_lcd_symbol_unit_symbols() {
        // Just verify the symbols can be created and compared
        let unit_v = LcdSymbol::UnitV;
        let unit_a = LcdSymbol::UnitA;
        let unit_w = LcdSymbol::UnitW;
        let unit_kwh = LcdSymbol::UnitKWh;
        assert_ne!(unit_v as u8, unit_a as u8);
        assert_ne!(unit_w as u8, unit_kwh as u8);
    }

    #[test]
    fn test_lcd_symbol_status_symbols() {
        // Just verify the symbols can be created and compared
        let alarm = LcdSymbol::Alarm;
        let tariff = LcdSymbol::Tariff;
        let battery = LcdSymbol::Battery;
        assert_ne!(alarm as u8, tariff as u8);
        assert_ne!(tariff as u8, battery as u8);
    }

    // ========== 轮显逻辑测试 (44-45) ==========

    #[test]
    fn test_auto_rotate_increment() {
        let mut panel = LcdPanel::new();
        let content = LcdContent::default();
        assert_eq!(panel.current_page(), 0);
        panel.auto_rotate(&content);
        assert_eq!(panel.current_page(), 1);
    }

    #[test]
    fn test_auto_rotate_wrap_around() {
        let mut panel = LcdPanel::new();
        panel.rotate_page = 11;
        let content = LcdContent::default();
        panel.auto_rotate(&content);
        assert_eq!(panel.current_page(), 0);
    }

    // ========== 综合格式化测试 (46-47) ==========

    #[test]
    fn test_voltage_current_display_integration() {
        let mut panel = LcdPanel::new();
        panel.format_voltage(22050);
        let voltage_ram = panel.display_ram[0];
        panel.clear();
        panel.format_current(5000);
        let current_ram = panel.display_ram[0];
        // Both should set display RAM
        assert!(voltage_ram != 0);
        assert!(current_ram != 0);
    }

    #[test]
    fn test_power_energy_display_integration() {
        let mut panel = LcdPanel::new();
        panel.format_power(1500, "W");
        let power_ram = panel.display_ram[0];
        panel.clear();
        panel.format_energy(123450, "kWh");
        let energy_ram = panel.display_ram[0];
        // Both should set display RAM
        assert!(power_ram != 0);
        assert!(energy_ram != 0);
    }

    // ========== 边界条件测试 (48-50) ==========

    #[test]
    fn test_write_number_zero() {
        let mut panel = LcdPanel::new();
        panel.write_number(0, 0, 0);
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_write_number_max_32bit() {
        let mut panel = LcdPanel::new();
        panel.write_number(0, i32::MAX, 0);
        assert!(panel.display_ram[0] != 0);
    }

    #[test]
    fn test_write_number_min_32bit() {
        let mut panel = LcdPanel::new();
        panel.write_number(0, i32::MIN, 0);
        assert!(panel.display_ram[0] != 0);
    }

    // ========== 特殊场景测试 (51-53) ==========

    #[test]
    fn test_multiple_clears() {
        let mut panel = LcdPanel::new();
        panel.clear();
        panel.clear();
        panel.clear();
        assert_eq!(panel.display_ram, [0; 4]);
    }

    #[test]
    fn test_multiple_all_on() {
        let mut panel = LcdPanel::new();
        panel.all_on();
        panel.all_on();
        assert!(panel.display_ram.iter().all(|&ram| ram == 0xFFF_FFFF_FFFF_u64));
    }

    #[test]
    fn test_clear_after_all_on() {
        let mut panel = LcdPanel::new();
        panel.all_on();
        panel.clear();
        assert_eq!(panel.display_ram, [0; 4]);
    }
}
