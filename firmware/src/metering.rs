//! 计量芯片驱动 — ATT7022 / BL6523 / BL0937
//!
//! 通过 SPI0 与计量芯片通信，读取电压、电流、功率、电能等数据
//!
//! ATT7022: 三相高精度计量芯片, SPI, 24-bit ADC
//! BL6523: 单相防窃电计量芯片, SPI, 24-bit ADC
//! BL0937: 单相计量芯片 (脉冲输出, 不需要 SPI)

/// 计量芯片类型
#[derive(Clone, Copy)]
pub enum MeteringChip {
    Att7022,
    Bl6523,
    Bl0937,  // 脉冲输出方式，不需要 SPI
}

/// ATT7022 寄存器地址
pub mod att7022 {
    // 参数寄存器
    pub const WAVE_IA:    u8 = 0x01; // A相电流波形
    pub const WAVE_IB:    u8 = 0x02;
    pub const WAVE_IC:    u8 = 0x03;
    pub const WAVE_UA:    u8 = 0x04; // A相电压波形
    pub const WAVE_UB:    u8 = 0x05;
    pub const WAVE_UC:    u8 = 0x06;

    // 功率/能量寄存器
    pub const PA:         u8 = 0x10; // A相有功功率
    pub const PB:         u8 = 0x11;
    pub const PC:         u8 = 0x12;
    pub const PTOTAL:     u8 = 0x13; // 总有功功率
    pub const QA:         u8 = 0x14; // A相无功功率
    pub const QB:         u8 = 0x15;
    pub const QC:         u8 = 0x16;
    pub const QTOTAL:     u8 = 0x17;
    pub const SA:         u8 = 0x18; // A相视在功率
    pub const STOTAL:     u8 = 0x1B;

    // 有效值寄存器
    pub const URMS_A:     u8 = 0x20; // A相电压RMS
    pub const URMS_B:     u8 = 0x21;
    pub const URMS_C:     u8 = 0x22;
    pub const IRMS_A:     u8 = 0x23; // A相电流RMS
    pub const IRMS_B:     u8 = 0x24;
    pub const IRMS_C:     u8 = 0x25;

    // 能量寄存器
    pub const ACTIVE_IMPORT:   u8 = 0x30; // 正向有功电能
    pub const ACTIVE_EXPORT:   u8 = 0x31; // 反向有功电能
    pub const REACTIVE_IMPORT: u8 = 0x32; // 正向无功电能
    pub const REACTIVE_EXPORT: u8 = 0x33; // 反向无功电能

    // 频率
    pub const FREQ:       u8 = 0x40;
    pub const PFA:        u8 = 0x41; // A相功率因数

    // 校准寄存器
    pub const UGAIN_A:    u8 = 0x60; // A相电压增益
    pub const IGAIN_A:    u8 = 0x61; // A相电流增益
    pub const PGAIN_A:    u8 = 0x62; // A相有功功率增益
    pub const QGAIN_A:    u8 = 0x63; // A相无功功率增益
    pub const PHCAL_A:    u8 = 0x64; // A相角差校正
}

/// BL6523 寄存器地址
pub mod bl6523 {
    pub const WAVE_I:     u8 = 0x01; // 电流波形
    pub const WAVE_U:     u8 = 0x02; // 电压波形
    pub const IRMS:       u8 = 0x10; // 电流RMS
    pub const URMS:       u8 = 0x11; // 电压RMS
    pub const POWER_A:    u8 = 0x12; // 有功功率
    pub const POWER_R:    u8 = 0x13; // 无功功率
    pub const POWER_S:    u8 = 0x14; // 视在功率
    pub const FREQ:       u8 = 0x15; // 频率
    pub const PF:         u8 = 0x16; // 功率因数
    pub const ENERGY_P:   u8 = 0x20; // 有功电能
    pub const ENERGY_N:   u8 = 0x21; // 反向有功电能
    pub const ENERGY_Q:   u8 = 0x22; // 无功电能
    pub const UGAIN:      u8 = 0x30;
    pub const IGAIN:      u8 = 0x31;
    pub const PGAIN:      u8 = 0x32;
}

/// 计量数据 (工程单位)
#[derive(Clone, Copy, Default)]
pub struct MeteringData {
    pub voltage: u16,      // 0.1V
    pub current: u16,      // mA
    pub active_power: i32, // W (signed)
    pub reactive_power: i32,// var
    pub frequency: u16,    // 0.01Hz
    pub power_factor: u16, // 0-1000
}

/// 计量驱动
pub struct MeteringDriver {
    chip: MeteringChip,
    /// 电压校准系数
    u_coeff: f32,
    /// 电流校准系数
    i_coeff: f32,
    /// 有功功率校准系数
    p_coeff: f32,
    /// 无功功率校准系数
    q_coeff: f32,
}

impl MeteringDriver {
    pub fn new(chip: MeteringChip) -> Self {
        let (u_coeff, i_coeff, p_coeff, q_coeff) = match chip {
            // 默认校准系数 — 需要根据实际硬件标定
            MeteringChip::Att7022 => (0.000322, 0.001032, 0.000332, 0.000332),
            MeteringChip::Bl6523  => (0.000215, 0.000945, 0.000203, 0.000203),
            MeteringChip::Bl0937  => (1.0, 1.0, 1.0, 1.0), // BL0937 用脉冲
        };
        Self { chip, u_coeff, i_coeff, p_coeff, q_coeff }
    }

    /// SPI 读取计量芯片寄存器 (24-bit)
    fn spi_read(&self, reg: u8) -> u32 {
        // FM33LG0xx SPI0 操作:
        // 1. CS 低
        // 2. 写入: 0x80 | (reg & 0x3F) (读命令)
        // 3. 读取 3 字节 (MSB first)
        // 4. CS 高
        let spi = crate::fm33lg0::spi0();
        // 实际实现需要 GPIO CS 控制
        let _ = spi;
        0 // placeholder
    }

    /// SPI 写入计量芯片寄存器
    fn spi_write(&self, reg: u8, val: u32) {
        let _ = (reg, val);
    }

    /// 读取全部计量数据 (单相, BL6523)
    pub fn read_bl6523(&self) -> MeteringData {
        let u_raw = self.spi_read(bl6523::URMS);
        let i_raw = self.spi_read(bl6523::IRMS);
        let p_raw = self.spi_read(bl6523::POWER_A) as i32;
        let q_raw = self.spi_read(bl6523::POWER_R) as i32;
        let f_raw = self.spi_read(bl6523::FREQ);
        let pf_raw = self.spi_read(bl6523::PF);

        MeteringData {
            voltage: (u_raw as f32 * self.u_coeff * 10.0) as u16,
            current: (i_raw as f32 * self.i_coeff * 1000.0) as u16,
            active_power: (p_raw as f32 * self.p_coeff) as i32,
            reactive_power: (q_raw as f32 * self.q_coeff) as i32,
            frequency: if f_raw > 0 { (3579545.0 / (2.0 * f_raw as f32) * 100.0) as u16 } else { 5000 },
            power_factor: (pf_raw as f32 * 1000.0 / 32768.0) as u16,
        }
    }

    /// 读取全部计量数据 (三相, ATT7022)
    pub fn read_att7022(&self) -> MeteringData {
        // 三相计量 — 取 A 相或总和
        let u_raw = self.spi_read(att7022::URMS_A);
        let i_raw = self.spi_read(att7022::IRMS_A);
        let p_raw = self.spi_read(att7022::PTOTAL) as i32;
        let q_raw = self.spi_read(att7022::QTOTAL) as i32;
        let f_raw = self.spi_read(att7022::FREQ);

        MeteringData {
            voltage: (u_raw as f32 * self.u_coeff * 10.0) as u16,
            current: (i_raw as f32 * self.i_coeff * 1000.0) as u16,
            active_power: (p_raw as f32 * self.p_coeff) as i32,
            reactive_power: (q_raw as f32 * self.q_coeff) as i32,
            frequency: if f_raw > 0 { (3579545.0 / (2.0 * f_raw as f32) * 100.0) as u16 } else { 5000 },
            power_factor: 1000, // 需要计算
        }
    }

    /// 读取能量 (有功正向)
    pub fn read_energy_import(&self) -> u64 {
        let raw = match self.chip {
            MeteringChip::Att7022 => self.spi_read(att7022::ACTIVE_IMPORT),
            MeteringChip::Bl6523  => self.spi_read(bl6523::ENERGY_P),
            MeteringChip::Bl0937  => 0, // BL0937 用脉冲计数
        };
        // 能量脉冲累计: 每个 pulse = 1 Wh (或由脉冲常数决定)
        raw as u64
    }

    /// 写入校准系数到计量芯片
    pub fn apply_calibration(&self, u_gain: u32, i_gain: u32, p_gain: u32) {
        match self.chip {
            MeteringChip::Att7022 => {
                self.spi_write(att7022::UGAIN_A, u_gain);
                self.spi_write(att7022::IGAIN_A, i_gain);
                self.spi_write(att7022::PGAIN_A, p_gain);
            }
            MeteringChip::Bl6523 => {
                self.spi_write(bl6523::UGAIN, u_gain);
                self.spi_write(bl6523::IGAIN, i_gain);
                self.spi_write(bl6523::PGAIN, p_gain);
            }
            MeteringChip::Bl0937 => {}
        }
    }
}
