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
            rotate_total: 8,
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
        // TODO: 根据符号类型写入对应 SEG 位
        let _ = (symbol, on);
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
                self.write_number(4, content.frequency as i32, 2); // xx.xx Hz
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
            let dp = if dp_pos > 0 && (7 - i) == dp_pos as usize {
                true
            } else {
                false
            };
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

    #[test]
    fn test_lcd_panel_new() {
        let panel = LcdPanel::new();
        assert!(!panel.enabled);
        assert_eq!(panel.rotate_total, 8);
        assert_eq!(panel.rotate_page, 0);
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
        panel.rotate_page = 7;
        panel.next_page();
        assert_eq!(panel.current_page(), 0);
    }

    #[test]
    fn test_segment_patterns_length() {
        assert_eq!(SEGMENT_PATTERNS.len(), 16);
    }

    #[test]
    fn test_segment_patterns_basic() {
        assert!(SEGMENT_PATTERNS[0] != 0);
        assert!(SEGMENT_PATTERNS[1] < SEGMENT_PATTERNS[0]);
        assert_eq!(SEGMENT_PATTERNS[8], 0xFE);
    }

    #[test]
    fn test_symbol_values() {
        assert_eq!(symbol::MINUS, 0x02);
        assert_eq!(symbol::ALL_ON, 0xFF);
        assert_eq!(symbol::ALL_OFF, 0x00);
    }

    #[test]
    fn test_lcd_symbol_debug() {
        assert_eq!(format!("{:?}", LcdSymbol::PhaseA), "PhaseA");
        assert_eq!(format!("{:?}", LcdSymbol::UnitKWh), "UnitKWh");
        assert_eq!(format!("{:?}", LcdSymbol::Alarm), "Alarm");
    }

    #[test]
    fn test_rotate_timer_resets() {
        let mut panel = LcdPanel::new();
        panel.rotate_timer = 3000;
        panel.next_page();
        assert_eq!(panel.rotate_timer, 0);
    }
}
