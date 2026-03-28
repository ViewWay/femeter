//! LCD 段码显示驱动 — FM33LG0xx 内置 LCD 控制器
//!
//! 支持: 4COM x 40SEG 段码 LCD
//! 显示内容: 电压、电流、功率、电能、费率、状态图标
//!
//! 段码编码 (7段 + DP):
//!    aaa
//!   f   b
//!   f   b
//!    ggg
//!   e   c
//!   e   c
//!    ddd  DP
//!
//! 编码位: DP g f e d c b a (MSB to LSB)

/// 7段码数字 0-9 编码表 (a=bit0, b=bit1, ..., g=bit6, DP=bit7)
pub const DIGIT_PATTERNS: [u8; 16] = [
    0x3F, // 0: abcdef
    0x06, // 1: bc
    0x5B, // 2: abdeg
    0x4F, // 3: abcdg
    0x66, // 4: bcfg
    0x6D, // 5: acdfg
    0x7D, // 6: acdefg
    0x07, // 7: abc
    0x7F, // 8: abcdefg
    0x6F, // 9: abcdfg
    0x77, // A: abcefg
    0x7C, // b: cdefg
    0x39, // C: adef
    0x5E, // d: bcdeg
    0x79, // E: adefg
    0x71, // F: aefg
];

/// LCD 显示页
#[derive(Clone, Copy, PartialEq)]
pub enum DisplayPage {
    /// 电压 + 电流: "220.0V 05.23A"
    VoltageCurrent,
    /// 有功功率: " 1234 W"
    Power,
    /// 总电能: "12345.6kWh"
    EnergyTotal,
    /// 按费率的电能: "T1:12345.6"
    EnergyTariff(u8),
    /// 频率 + PF: "50.00Hz 0.998"
    FrequencyPf,
    /// 费率时段: "T1 P:1234W"
    TariffInfo,
    /// 错误码: "Err 0x12"
    Error(u8),
}

/// LCD 驱动
pub struct LcdDriver {
    current_page: DisplayPage,
    auto_cycle: bool,
    cycle_counter: u32,
    /// 4COM x 40SEG 显示缓存 (每个 COM 一个 u32, 每 bit 对应一个 SEG)
    display_ram: [u32; 4],
}

impl LcdDriver {
    pub const fn new() -> Self {
        Self {
            current_page: DisplayPage::VoltageCurrent,
            auto_cycle: true,
            cycle_counter: 0,
            display_ram: [0; 4],
        }
    }

    /// 初始化 LCD 控制器硬件
    pub fn init_hw(&mut self) {
        // FM33LG0xx LCD 控制器初始化:
        // 1. 使能 LCD 时钟 (RCC.APBENR2.LCDEN)
        // 2. 配置 LCD_CR:
        //    - COM 数量: 4
        //    - SEG 数量: 40
        //    - 时钟源: LSE (32.768kHz)
        //    - 偏置: 1/3 bias (4COM)
        //    - 占空比: 1/4 duty
        // 3. 配置对比度 (LCD_CR.CONTRAST)
        // 4. 使能 LCD
        // 5. 等待 LCD 就绪
    }

    /// 刷新显示
    pub fn update(
        &mut self,
        voltage: u16,   // 0.1V
        current: u16,   // mA
        power: i32,     // W
        energy: u64,    // Wh
        tariff: u8,     // 0-7
        frequency: u16, // 0.01Hz
        pf: u16,        // 0-1000
    ) {
        if self.auto_cycle {
            self.cycle_counter += 1;
            if self.cycle_counter >= 200 {
                self.cycle_counter = 0;
                self.current_page = match self.current_page {
                    DisplayPage::VoltageCurrent => DisplayPage::Power,
                    DisplayPage::Power => DisplayPage::EnergyTotal,
                    DisplayPage::EnergyTotal => DisplayPage::FrequencyPf,
                    DisplayPage::FrequencyPf => DisplayPage::VoltageCurrent,
                    _ => DisplayPage::VoltageCurrent,
                };
            }
        }

        self.display_ram = [0; 4]; // Clear

        match self.current_page {
            DisplayPage::VoltageCurrent => {
                self.write_voltage_current(voltage, current);
            }
            DisplayPage::Power => {
                self.write_power(power);
            }
            DisplayPage::EnergyTotal => {
                self.write_energy_kwh(energy);
            }
            DisplayPage::FrequencyPf => {
                self.write_freq_pf(frequency, pf);
            }
            DisplayPage::EnergyTariff(t) => {
                self.write_tariff_energy(t, energy);
            }
            DisplayPage::TariffInfo => {
                self.write_tariff_info(tariff, power);
            }
            DisplayPage::Error(code) => {
                self.write_error(code);
            }
        }

        // 费率指示图标 (T1-T8)
        self.set_tariff_icon(tariff);
        // 继电器状态图标
        // self.set_relay_icon(relay_closed);

        // 写入 LCD RAM
        self.flush_to_hw();
    }

    /// 写入 "220.0V 05.23A"
    fn write_voltage_current(&mut self, voltage: u16, current: u16) {
        let v_int = voltage / 10;
        let v_dec = voltage % 10;
        let a_int = current / 1000;
        let a_dec = (current % 1000) / 10;

        // 数字写入到段码 RAM (假设段码映射)
        self.write_digit(0, (v_int / 100) as u8, false);
        self.write_digit(1, ((v_int / 10) % 10) as u8, false);
        self.write_digit(2, (v_int % 10) as u8, true); // 小数点
        self.write_digit(3, v_dec as u8, false);

        self.write_digit(5, (a_int / 10) as u8, false);
        self.write_digit(6, (a_int % 10) as u8, true);
        self.write_digit(7, (a_dec / 10) as u8, false);
        self.write_digit(8, (a_dec % 10) as u8, false);
    }

    fn write_power(&mut self, power: i32) {
        let abs_p = if power < 0 { -power } else { power };
        self.write_digit(0, (abs_p / 10000) as u8, false);
        self.write_digit(1, ((abs_p / 1000) % 10) as u8, false);
        self.write_digit(2, ((abs_p / 100) % 10) as u8, false);
        self.write_digit(3, ((abs_p / 10) % 10) as u8, false);
        self.write_digit(4, (abs_p % 10) as u8, false);
    }

    fn write_energy_kwh(&mut self, energy_wh: u64) {
        let kwh = energy_wh / 1000;
        let wh = (energy_wh % 1000) / 100; // 1 decimal
        self.write_digit(0, ((kwh / 10000) % 10) as u8, false);
        self.write_digit(1, ((kwh / 1000) % 10) as u8, false);
        self.write_digit(2, ((kwh / 100) % 10) as u8, false);
        self.write_digit(3, ((kwh / 10) % 10) as u8, false);
        self.write_digit(4, (kwh % 10) as u8, true); // 小数点
        self.write_digit(5, wh as u8, false);
    }

    fn write_freq_pf(&mut self, freq: u16, pf: u16) {
        // freq: 5000 = 50.00Hz
        self.write_digit(0, (freq / 1000) as u8, false);
        self.write_digit(1, ((freq / 100) % 10) as u8, true);
        self.write_digit(2, ((freq / 10) % 10) as u8, false);
        self.write_digit(3, (freq % 10) as u8, false);
        // pf: 998 = 0.998
        self.write_digit(5, ((pf / 100) % 10) as u8, false);
        self.write_digit(6, ((pf / 10) % 10) as u8, false);
        self.write_digit(7, (pf % 10) as u8, false);
    }

    fn write_tariff_energy(&mut self, tariff: u8, energy_wh: u64) {
        self.write_digit(0, tariff + 1, false);
        self.write_energy_kwh(energy_wh);
    }

    fn write_tariff_info(&mut self, tariff: u8, power: i32) {
        self.write_digit(0, tariff + 1, false);
        self.write_power(power);
    }

    fn write_error(&mut self, code: u8) {
        self.write_digit(4, (code >> 4) as u8, false);
        self.write_digit(5, (code & 0x0F) as u8, false);
    }

    /// 写入单个数字到段码 RAM
    fn write_digit(&mut self, pos: usize, digit: u8, decimal_point: bool) {
        if digit > 0xF || pos > 12 {
            return;
        }
        let mut pattern = DIGIT_PATTERNS[digit as usize];
        if decimal_point {
            pattern |= 0x80; // DP bit
        }
        // Map digit position to LCD RAM segments
        // Each digit occupies 8 segments (7 + DP) across 4 COM lines
        // Real mapping depends on PCB routing — this is placeholder
        let seg_base = pos * 8;
        for (com, bits) in [(0, 0x0F), (1, 0x30), (2, 0x0F), (3, 0x30)].iter() {
            for bit in 0..7 {
                if pattern & (1 << bit) != 0 {
                    let seg = seg_base + bit;
                    if seg < 40 {
                        self.display_ram[*com] |= 1 << seg;
                    }
                }
            }
        }
    }

    /// 费率指示图标
    fn set_tariff_icon(&mut self, tariff: u8) {
        if tariff < 8 {
            // 费率图标通常在固定 SEG 位置
            self.display_ram[0] |= 1 << (36 + tariff as usize);
        }
    }

    /// 写入 LCD 控制器 RAM
    fn flush_to_hw(&self) {
        // 实际固件:
        // let lcd = crate::fm33lg0::lcd();
        // lcd.ram[0] = self.display_ram[0];
        // lcd.ram[1] = self.display_ram[1];
        // lcd.ram[2] = self.display_ram[2];
        // lcd.ram[3] = self.display_ram[3];
    }
}
