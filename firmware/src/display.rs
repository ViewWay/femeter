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

/// 7 段码编码 (a,b,c,d,e,f,g)
///
/// ```text
///   aaa
///  f   b
///  f   b
///   ggg
///  e   c
///  e   c
///   ddd
/// ```
const SEGMENT_PATTERNS: [u8; 16] = [
    0xFC, // 0: a,b,c,d,e,f
    0x60, // 1: b,c
    0xDA, // 2: a,b,d,e,g
    0xF2, // 3: a,b,c,d,g
    0x66, // 4: b,c,f,g
    0xB6, // 5: a,c,d,f,g
    0xBE, // 6: a,c,d,e,f,g
    0xE0, // 7: a,b,c
    0xFE, // 8: all
    0xF6, // 9: a,b,c,d,f,g
    0x00, // blank
    0x00, // blank
    0x00, // blank
    0x00, // blank
    0x00, // blank
    0x02, // -: g only
];

/// 特殊符号段码
pub mod symbol {
    /// 负号
    pub const MINUS: u8 = 0x02;
    /// 全部点亮
    pub const ALL_ON: u8 = 0xFF;
    /// 全部熄灭
    pub const ALL_OFF: u8 = 0x00;
}

/* ================================================================== */
/*  LCD 段码映射                                                       */
/* ================================================================== */

/// LCD 面板段码分配 (4COM × 44SEG)
///
/// 这是面板布局定义, 每个数字/符号占用哪些 SEG
/// 具体映射取决于 LCD 玻璃开模图
///
/// 典型电表面板布局:
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
    /// SEG 显存 (44 个 SEG, 每个 4 COM → 44 × 4bit = 22 字节)
    seg_ram: [u32; 11], // 每个字 32bit, 11 × 32 = 352bit > 44×4=176bit

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
            seg_ram: [0; 11],
            mode: LcdDisplayMode::Off,
            rotate_timer: 0,
            rotate_page: 0,
            rotate_total: 8,
            enabled: false,
        }
    }

    /// 将数字 (0~9) 写入指定位置
    ///
    /// `digit_index`: 数字位置 (0~7, 从左到右)
    /// `value`: 要显示的数字 (0~15, 10=blank, 15=负号)
    /// `dp`: 是否显示小数点
    pub fn write_digit(&mut self, digit_index: u8, value: u8, dp: bool) {
        let pattern = if (value as usize) < SEGMENT_PATTERNS.len() {
            SEGMENT_PATTERNS[value as usize]
        } else {
            0
        };

        // TODO: 根据 digit_index 计算 SEG 偏移
        // 这需要具体的 LCD 玻璃面板段码映射表
        let pattern_with_dp = if dp { pattern | 0x01 } else { pattern };

        // 写入 seg_ram
        let word_idx = (digit_index as usize * 4) / 32;
        let bit_offset = (digit_index as usize * 4) % 32;
        if word_idx < self.seg_ram.len() {
            let mask = 0xF << bit_offset;
            self.seg_ram[word_idx] = (self.seg_ram[word_idx] & !mask)
                | ((pattern_with_dp as u32 & 0xFF) << bit_offset);
        }
    }

    /// 写入符号 (如通信状态、单位等)
    pub fn write_symbol(&mut self, symbol: LcdSymbol, on: bool) {
        // TODO: 根据符号类型写入对应 SEG 位
        let _ = (symbol, on);
    }

    /// 清空显示
    pub fn clear(&mut self) {
        self.seg_ram = [0; 11];
    }

    /// 全显 (测试模式)
    pub fn all_on(&mut self) {
        self.seg_ram = [0xFFFF_FFFF; 11];
    }

    /// 刷新 LCD 硬件 (将 seg_ram 写入 LCD 控制器)
    pub fn refresh_hw(&self) {
        // TODO: 写入 FM33A0xxEV LCD 显存寄存器
        // LCD_RAM0~LCD_RAM10
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
                self.write_number(0, content.voltage as i32, 1); // xxx.x V
                self.write_number(4, content.current as i32, 0); // xxxx mA
                self.write_symbol(LcdSymbol::PhaseA, true);
                self.write_symbol(LcdSymbol::UnitV, true);
            }
            1 => {
                // B 相: 电压 + 电流
                self.write_number(0, content.voltage as i32, 1);
                self.write_number(4, content.current as i32, 0);
                self.write_symbol(LcdSymbol::PhaseB, true);
                self.write_symbol(LcdSymbol::UnitV, true);
            }
            2 => {
                // C 相: 电压 + 电流
                self.write_number(0, content.voltage as i32, 1);
                self.write_number(4, content.current as i32, 0);
                self.write_symbol(LcdSymbol::PhaseC, true);
                self.write_symbol(LcdSymbol::UnitV, true);
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
                // 功率因数 + 频率
                self.write_number(0, content.power_factor as i32, 3); // x.xxx
                self.write_number(4, content.frequency as i32, 2);    // xx.xx Hz
                self.write_symbol(LcdSymbol::UnitPF, true);
                self.write_symbol(LcdSymbol::UnitHz, true);
            }
            6 => {
                // 正向有功总电能
                self.write_number(0, (content.active_import_energy / 100) as i32, 2);
                self.write_symbol(LcdSymbol::UnitKWh, true);
                self.write_symbol(LcdSymbol::Forward, true);
            }
            7 => {
                // 通信状态 + 费率
                self.write_number(0, content.tariff as i32, 0);
                self.write_symbol(LcdSymbol::Tariff, true);
                if content.comm_status & 0x01 != 0 {
                    self.write_symbol(LcdSymbol::RS485, true);
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
    fn write_number(&mut self, start_digit: u8, value: i32, dp_pos: u8) {
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
            let dp = if dp_pos > 0 && (7 - i) == dp_pos as usize { true } else { false };
            self.write_digit(start_digit + (i - start) as u8, digits[i], dp);
        }
    }

    /// 写入有符号数字
    fn write_signed(&mut self, start_digit: u8, value: i32, dp_pos: u8) {
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
