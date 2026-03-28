//! ATT7022E 三相多功能电能计量芯片驱动
//!
//! 钜泉光电 (HitrendTech) ATT7022E — 三相三线/四线多功能计量芯片
//! - 7 路 19-bit sigma-delta ADC (3×电压 + 3×电流 + 1×零线电流)
//! - 有功/无功/视在功率和电能计量
//! - 基波有功功率/电能/电压/电流有效值
//! - 功率因数、相角、电压夹角、线频率
//! - SPI 从模式接口，固定 8bit 命令 + 24bit 数据，最高 4Mbps
//! - 晶振 5.5296MHz, 3.3V 供电, LQFP44 封装
//!
//! 参考: ATT7022E 用户手册 Rev1.3 (210-SD-138)

use crate::fm33lg0::{self, SpiRegs, spi_cr1, spi_cr2};
use core::ops::Deref;

// ══════════════════════════════════════════════════════════════════
// SPI 协议
// ══════════════════════════════════════════════════════════════════
//
// CS:  低有效，下降沿=开始，上升沿=结束
// SCLK: 上升沿放数据(DOUT)，下降沿取数据(DIN)
// 数据格式: MSB first, 8bit 命令 + 24bit 数据 = 32bit 固定长度
//
// 命令字节:
//   Bit7=0: 读寄存器 (Bit6..0 = 地址)
//   Bit7=1,Bit6=0: 写寄存器 (Bit6..0 = 地址)
//   Bit7=1,Bit6=1: 特殊命令 (Bit5..0 = 命令类型)

/// 构造读命令字节
fn cmd_read(addr: u8) -> u8 {
    addr & 0x7F // Bit7=0
}

/// 构造写命令字节
fn cmd_write(addr: u8) -> u8 {
    (addr & 0x3F) | 0x80 // Bit7=1, Bit6=0
}

/// 特殊命令 (Bit7=1, Bit6=1)
fn cmd_special(subcmd: u8) -> u8 {
    (subcmd & 0x3F) | 0xC0
}

// ══════════════════════════════════════════════════════════════════
// 特殊命令
// ══════════════════════════════════════════════════════════════════

pub mod special_cmd {
    /// 启动波形数据缓冲, data = 0x00CCCX (X=通道号)
    pub const WAVE_START: u8 = 0xC0;
    /// 设置缓冲数据读指针, data = 0~1023
    pub const WAVE_PTR: u8 = 0xC1;
    /// 清校表数据 (恢复默认值)
    pub const CLEAR_CALIB: u8 = 0xC3;
    /// 同步数据系数设置
    pub const SYNC_COEFF: u8 = 0xC4;
    /// 同步数据启动/停止 (0x02=自动, 0x03=手动, 0x00=停止)
    pub const SYNC_START: u8 = 0xC5;
    /// 切换读出校表数据 (data=0x00005A → 校表数据, else → 计量数据)
    pub const READ_CALIB: u8 = 0xC6;
    /// 校表数据写使能 (data=0x00005A)
    pub const WRITE_EN: u8 = 0xC9;
    /// 软件复位
    pub const SOFT_RESET: u8 = 0xD3;
}

// ══════════════════════════════════════════════════════════════════
// 计量参数寄存器地址 (只读)
// ══════════════════════════════════════════════════════════════════

pub mod reg {
    // ── 功率 ──
    pub const PA: u8 = 0x01;  // A相有功功率
    pub const PB: u8 = 0x02;  // B相有功功率
    pub const PC: u8 = 0x03;  // C相有功功率
    pub const PT: u8 = 0x04;  // 合相有功功率
    pub const QA: u8 = 0x05;  // A相无功功率
    pub const QB: u8 = 0x06;  // B相无功功率
    pub const QC: u8 = 0x07;  // C相无功功率
    pub const QT: u8 = 0x08;  // 合相无功功率
    pub const SA: u8 = 0x09;  // A相视在功率
    pub const SB: u8 = 0x0A;  // B相视在功率
    pub const SC: u8 = 0x0B;  // C相视在功率
    pub const ST: u8 = 0x0C;  // 合相视在功率

    // ── 有效值 ──
    pub const UA_RMS: u8 = 0x0D;  // A相电压有效值
    pub const UB_RMS: u8 = 0x0E;  // B相电压有效值
    pub const UC_RMS: u8 = 0x0F;  // C相电压有效值
    pub const IA_RMS: u8 = 0x10;  // A相电流有效值
    pub const IB_RMS: u8 = 0x11;  // B相电流有效值
    pub const IC_RMS: u8 = 0x12;  // C相电流有效值
    pub const IT_RMS: u8 = 0x13;  // 三相电流矢量和有效值
    pub const I0_RMS: u8 = 0x29;  // 零线电流有效值
    pub const UT_RMS: u8 = 0x2B;  // 三相电压矢量和有效值

    // ── 功率因数 ──
    pub const PFA: u8 = 0x14;  // A相功率因数
    pub const PFB: u8 = 0x15;  // B相功率因数
    pub const PFC: u8 = 0x16;  // C相功率因数
    pub const PFT: u8 = 0x17;  // 合相功率因数

    // ── 相角/电压夹角 ──
    pub const PGA: u8 = 0x18;     // A相电流与电压相角
    pub const PGB: u8 = 0x19;     // B相电流与电压相角
    pub const PGC: u8 = 0x1A;     // C相电流与电压相角
    pub const Y_UA_UB: u8 = 0x26; // Ua与Ub电压夹角
    pub const Y_UA_UC: u8 = 0x27; // Ua与Uc电压夹角
    pub const Y_UB_UC: u8 = 0x28; // Ub与Uc电压夹角

    // ── 频率/状态 ──
    pub const FREQ: u8 = 0x1C;      // 线频率
    pub const INT_FLAG: u8 = 0x1B;  // 中断标志(读后清零)
    pub const E_FLAG: u8 = 0x1D;    // 电能工作状态(读后清零)
    pub const S_FLAG: u8 = 0x2C;    // 断相/相序/SIG标志

    // ── 有功电能 (可配置读后清零) ──
    pub const EPA: u8 = 0x1E;  // A相有功电能
    pub const EPB: u8 = 0x1F;  // B相有功电能
    pub const EPC: u8 = 0x20;  // C相有功电能
    pub const EPT: u8 = 0x21;  // 合相有功电能

    // ── 无功电能 ──
    pub const EQA: u8 = 0x22;  // A相无功电能
    pub const EQB: u8 = 0x23;  // B相无功电能
    pub const EQC: u8 = 0x24;  // C相无功电能
    pub const EQT: u8 = 0x25;  // 合相无功电能

    // ── 视在电能 ──
    pub const ESA: u8 = 0x35;  // A相视在电能
    pub const ESB: u8 = 0x36;  // B相视在电能
    pub const ESC: u8 = 0x37;  // C相视在电能
    pub const EST: u8 = 0x38;  // 合相视在电能

    // ── 基波参数 ──
    pub const LINE_PA: u8 = 0x40;  // A相基波有功功率
    pub const LINE_PB: u8 = 0x41;  // B相基波有功功率
    pub const LINE_PC: u8 = 0x42;  // C相基波有功功率
    pub const LINE_PT: u8 = 0x43;  // 合相基波有功功率
    pub const LINE_EPA: u8 = 0x44; // A相基波有功电能
    pub const LINE_EPB: u8 = 0x45; // B相基波有功电能
    pub const LINE_EPC: u8 = 0x46; // C相基波有功电能
    pub const LINE_EPT: u8 = 0x47; // 合相基波有功电能
    pub const LINE_UA: u8 = 0x48;  // 基波A相电压有效值
    pub const LINE_UB: u8 = 0x49;  // 基波B相电压有效值
    pub const LINE_UC: u8 = 0x4A;  // 基波C相电压有效值
    pub const LINE_IA: u8 = 0x4B;  // 基波A相电流有效值
    pub const LINE_IB: u8 = 0x4C;  // 基波B相电流有效值
    pub const LINE_IC: u8 = 0x4D;  // 基波C相电流有效值
    pub const LINE_EFLAG: u8 = 0x4E; // 基波电能状态

    // ── ADC 采样数据 ──
    pub const SAMPLE_IA: u8 = 0x2F; // A相电流ADC采样
    pub const SAMPLE_IB: u8 = 0x30; // B相电流ADC采样
    pub const SAMPLE_IC: u8 = 0x31; // C相电流ADC采样
    pub const SAMPLE_UA: u8 = 0x32; // A相电压ADC采样
    pub const SAMPLE_UB: u8 = 0x33; // B相电压ADC采样
    pub const SAMPLE_UC: u8 = 0x34; // C相电压ADC采样
    pub const SAMPLE_I0: u8 = 0x3F; // 零线电流ADC采样

    // ── 脉冲计数/方向/温度/校验 ──
    pub const FST_CNT_A: u8 = 0x39; // A相快速脉冲计数
    pub const FST_CNT_B: u8 = 0x3A; // B相快速脉冲计数
    pub const FST_CNT_C: u8 = 0x3B; // C相快速脉冲计数
    pub const FST_CNT_T: u8 = 0x3C; // 合相快速脉冲计数
    pub const P_FLAG: u8 = 0x3D;    // 功率方向标志
    pub const CHKSUM: u8 = 0x3E;    // 校表数据校验和
    pub const TPSD: u8 = 0x2A;      // 温度传感器输出
    pub const DEVICE_ID: u8 = 0x00; // Device ID (复位值 0x7122A0)

    // ── 缓冲 ──
    pub const PTR_WAVE: u8 = 0x7E;  // 缓冲数据指针
    pub const WAVE_BUFF: u8 = 0x7F; // 缓冲数据寄存器

    // ── 通讯 ──
    pub const BCK_REG: u8 = 0x2D;    // 通讯数据备份
    pub const COM_CHKSUM: u8 = 0x2E; // 通讯校验和
}

// ══════════════════════════════════════════════════════════════════
// 校表参数寄存器地址 (通过 0xC6 切换后读写)
// ══════════════════════════════════════════════════════════════════

pub mod cal_reg {
    pub const MODE_CFG: u8 = 0x01;     // 模式配置
    pub const ADC_GAIN: u8 = 0x02;     // ADC增益配置
    pub const EMU_CFG: u8 = 0x03;      // EMU单元配置
    // 功率增益补偿: 0x04~0x0C
    pub const PA_GAIN: u8 = 0x04;      // A相有功功率增益
    pub const PB_GAIN: u8 = 0x05;      // B相有功功率增益
    pub const PC_GAIN: u8 = 0x06;      // C相有功功率增益
    pub const QA_GAIN: u8 = 0x07;      // A相无功功率增益
    pub const QB_GAIN: u8 = 0x08;      // B相无功功率增益
    pub const QC_GAIN: u8 = 0x09;      // C相无功功率增益
    pub const SA_GAIN: u8 = 0x0A;      // A相视在功率增益
    pub const SB_GAIN: u8 = 0x0B;      // B相视在功率增益
    pub const SC_GAIN: u8 = 0x0C;      // C相视在功率增益
    // 相位校正: 0x10~0x12
    pub const PHA_CAL: u8 = 0x10;      // A相相位校正
    pub const PHB_CAL: u8 = 0x11;      // B相相位校正
    pub const PHC_CAL: u8 = 0x12;      // C相相位校正
    // 功率offset校正: 0x13~0x15, 0x21~0x23
    pub const PA_OFFSET: u8 = 0x13;    // A相有功offset
    pub const PB_OFFSET: u8 = 0x14;    // B相有功offset
    pub const PC_OFFSET: u8 = 0x15;    // C相有功offset
    pub const QA_OFFSET: u8 = 0x21;    // A相无功offset
    pub const QB_OFFSET: u8 = 0x22;    // B相无功offset
    pub const QC_OFFSET: u8 = 0x23;    // C相无功offset
    // 无功相位校正
    pub const Q_PH_CAL: u8 = 0x16;     // 无功相位校正
    // 电压增益校正: 0x17~0x19
    pub const UA_GAIN: u8 = 0x17;      // A相电压增益
    pub const UB_GAIN: u8 = 0x18;      // B相电压增益
    pub const UC_GAIN: u8 = 0x19;      // C相电压增益
    // 电流增益校正: 0x1A~0x1C, 0x20
    pub const IA_GAIN: u8 = 0x1A;      // A相电流增益
    pub const IB_GAIN: u8 = 0x1B;      // B相电流增益
    pub const IC_GAIN: u8 = 0x1C;      // C相电流增益
    pub const I0_GAIN: u8 = 0x20;      // 零线电流增益
    // 阈值/配置
    pub const START_I: u8 = 0x1D;      // 起动电流设置
    pub const HF_CONST: u8 = 0x1E;     // 高频脉冲常数
    pub const FAIL_VOLT: u8 = 0x1F;    // 失压阈值
    // 有效值offset校正: 0x24~0x29
    pub const UA_RMS_OFF: u8 = 0x24;   // A相电压RMS offset
    pub const UB_RMS_OFF: u8 = 0x25;   // B相电压RMS offset
    pub const UC_RMS_OFF: u8 = 0x26;   // C相电压RMS offset
    pub const IA_RMS_OFF: u8 = 0x27;   // A相电流RMS offset
    pub const IB_RMS_OFF: u8 = 0x28;   // B相电流RMS offset
    pub const IC_RMS_OFF: u8 = 0x29;   // C相电流RMS offset
    // ADC offset: 0x2A~0x2F
    pub const ADC_OFF_0: u8 = 0x2A;
    pub const ADC_OFF_1: u8 = 0x2B;
    pub const ADC_OFF_2: u8 = 0x2C;
    pub const ADC_OFF_3: u8 = 0x2D;
    pub const ADC_OFF_4: u8 = 0x2E;
    pub const ADC_OFF_5: u8 = 0x2F;
    // 其他
    pub const INT_EN: u8 = 0x30;       // 中断使能
    pub const ANA_EN: u8 = 0x31;       // 模拟模块使能
    pub const ALL_GAIN: u8 = 0x32;     // 全通道增益
    pub const PULSE_DOUBLE: u8 = 0x33; // 脉冲加倍
    pub const LINE_GAIN: u8 = 0x34;    // 基波增益
    pub const IO_CFG: u8 = 0x35;       // IO状态配置
    pub const START_P: u8 = 0x36;      // 起动功率
}

// ══════════════════════════════════════════════════════════════════
// 数据转换常量和公式
// ══════════════════════════════════════════════════════════════════

/// 24-bit 补码 → i32
fn signed24(raw: u32) -> i32 {
    let v = raw & 0x00FF_FFFF;
    if v >= 0x0080_0000 {
        (v as i32) - 0x0100_0000
    } else {
        v as i32
    }
}

/// 功率寄存器 → 实际功率值 (W or var)
/// 分相: actual = signed24(raw) * K
/// 合相: actual = signed24(raw) * 2 * K
/// K = 2.592e10 / (HFconst * EC * 2^23)
#[inline]
pub fn power_to_watts(raw: u32, hfconst: u32, ec: u32) -> i32 {
    let k_num = 25920_000_000u64; // 2.592e10 * 1000 (定点)
    let k_den = (hfconst as u64) * (ec as u64) * (1 << 23);
    let k_fixed = (k_num * 1000 / k_den) as i64; // K * 1e6
    let signed = signed24(raw) as i64;
    (signed * k_fixed / 1_000_000) as i32
}

/// 有效值寄存器 → 实际电压 (V)
/// actual = raw / 2^13
#[inline]
pub fn rms_to_voltage(raw: u32) -> u32 {
    raw / (1 << 13)
}

/// 有效值寄存器 → 实际电流 (A)
/// actual = (raw / 2^13) / N  (N = 60/Ib 当 Ib 取样 50mV)
#[inline]
pub fn rms_to_current(raw: u32, ratio_n: u32) -> u32 {
    let mv = raw / (1 << 13); // 毫伏值
    mv / ratio_n
}

/// 功率因数寄存器 → 实际 PF
/// actual = signed24(raw) / 2^23, 范围 -1.0 ~ +1.0
/// 返回 PF * 1000 (定点)
#[inline]
pub fn pf_to_fixed(raw: u32) -> i32 {
    signed24(raw) * 1000 / (1 << 23)
}

/// 相角寄存器 → 实际角度 (度)
/// actual = signed24(raw) / 2^20 * 180
/// 返回 角度 * 100 (定点, 0.01度分辨率)
#[inline]
pub fn angle_to_fixed(raw: u32) -> i32 {
    signed24(raw) * 18000 / (1 << 20)
}

/// 频率寄存器 → 实际频率 (Hz)
/// actual = raw / 2^13
/// 返回 Hz * 100 (定点)
#[inline]
pub fn freq_to_fixed(raw: u32) -> u32 {
    raw * 100 / (1 << 13)
}

/// 温度寄存器 → 温度值
/// 低8位有效, 补码; actual = TC - 0.726 * signed8
/// tc_calibration 为校正值 (通常室温25°C时校准)
#[inline]
pub fn temp_to_celsius(raw: u32, tc_calibration: i32) -> i32 {
    let tm = (raw & 0xFF) as i8 as i32;
    tc_calibration - (726 * tm / 1000) // 0.726 * tm
}

// ══════════════════════════════════════════════════════════════════
// ATT7022E 驱动结构
// ══════════════════════════════════════════════════════════════════

/// CS 引脚控制回调
pub trait CsPin {
    fn set_low(&self);
    fn set_high(&self);
}

/// ATT7022E 驱动
pub struct Att7022e<SPI: SpiOps, CS: CsPin> {
    spi: SPI,
    cs: CS,
}

/// SPI 操作 trait (抽象硬件 SPI)
pub trait SpiOps {
    /// 发送一字节并同时接收一字节
    fn transfer(&mut self, tx: u8) -> u8;
}

impl<SPI: SpiOps, CS: CsPin> Att7022e<SPI, CS> {
    pub fn new(spi: SPI, cs: CS) -> Self {
        Self { spi, cs }
    }

    /// 释放底层 SPI 和 CS
    pub fn release(self) -> (SPI, CS) {
        (self.spi, self.cs)
    }

    // ── SPI 事务 ──

    /// 读 24-bit 寄存器
    pub fn read_reg(&mut self, addr: u8) -> u32 {
        self.cs.set_low();
        let cmd = cmd_read(addr);
        self.spi.transfer(cmd);
        let b2 = self.spi.transfer(0) as u32;
        let b1 = self.spi.transfer(0) as u32;
        let b0 = self.spi.transfer(0) as u32;
        self.cs.set_high();
        (b2 << 16) | (b1 << 8) | b0
    }

    /// 写 24-bit 寄存器
    pub fn write_reg(&mut self, addr: u8, data: u32) {
        self.cs.set_low();
        let cmd = cmd_write(addr);
        self.spi.transfer(cmd);
        self.spi.transfer(((data >> 16) & 0xFF) as u8);
        self.spi.transfer(((data >> 8) & 0xFF) as u8);
        self.spi.transfer((data & 0xFF) as u8);
        self.cs.set_high();
    }

    /// 发送特殊命令
    pub fn send_special(&mut self, subcmd: u8, data: u32) {
        self.cs.set_low();
        self.spi.transfer(cmd_special(subcmd));
        self.spi.transfer(((data >> 16) & 0xFF) as u8);
        self.spi.transfer(((data >> 8) & 0xFF) as u8);
        self.spi.transfer((data & 0xFF) as u8);
        self.cs.set_high();
    }

    // ── 初始化流程 ──

    /// 软件复位
    pub fn soft_reset(&mut self) {
        self.send_special(0xD3 & 0x3F, 0x000000);
    }

    /// 读取 Device ID (应为 0x7122A0)
    pub fn read_device_id(&mut self) -> u32 {
        self.read_reg(reg::DEVICE_ID)
    }

    /// 使能校表数据写入
    pub fn enable_calibration_write(&mut self) {
        self.send_special(special_cmd::WRITE_EN & 0x3F, 0x00005A);
    }

    /// 禁止校表数据写入
    pub fn disable_calibration_write(&mut self) {
        self.send_special(special_cmd::WRITE_EN & 0x3F, 0x000000);
    }

    /// 切换到读校表数据模式
    pub fn switch_to_calib_read(&mut self) {
        self.send_special(special_cmd::READ_CALIB & 0x3F, 0x00005A);
    }

    /// 切换到读计量数据模式 (默认)
    pub fn switch_to_meter_read(&mut self) {
        self.send_special(special_cmd::READ_CALIB & 0x3F, 0x000000);
    }

    /// 清除校表数据 (恢复默认)
    pub fn clear_calibration(&mut self) {
        self.send_special(special_cmd::CLEAR_CALIB & 0x3F, 0x000000);
    }

    // ── 计量数据读取 ──

    /// 读取所有三相功率数据
    pub fn read_power(&mut self) -> PowerData {
        PowerData {
            pa: signed24(self.read_reg(reg::PA)),
            pb: signed24(self.read_reg(reg::PB)),
            pc: signed24(self.read_reg(reg::PC)),
            pt: signed24(self.read_reg(reg::PT)),
            qa: signed24(self.read_reg(reg::QA)),
            qb: signed24(self.read_reg(reg::QB)),
            qc: signed24(self.read_reg(reg::QC)),
            qt: signed24(self.read_reg(reg::QT)),
            sa: self.read_reg(reg::SA),
            sb: self.read_reg(reg::SB),
            sc: self.read_reg(reg::SC),
            st: self.read_reg(reg::ST),
        }
    }

    /// 读取三相电压电流有效值
    pub fn read_rms(&mut self) -> RmsData {
        RmsData {
            ua: self.read_reg(reg::UA_RMS),
            ub: self.read_reg(reg::UB_RMS),
            uc: self.read_reg(reg::UC_RMS),
            ia: self.read_reg(reg::IA_RMS),
            ib: self.read_reg(reg::IB_RMS),
            ic: self.read_reg(reg::IC_RMS),
            i0: self.read_reg(reg::I0_RMS),
            ut: self.read_reg(reg::UT_RMS),
            it: self.read_reg(reg::IT_RMS),
        }
    }

    /// 读取三相电能数据
    pub fn read_energy(&mut self) -> EnergyData {
        EnergyData {
            epa: self.read_reg(reg::EPA),
            epb: self.read_reg(reg::EPB),
            epc: self.read_reg(reg::EPC),
            ept: self.read_reg(reg::EPT),
            eqa: self.read_reg(reg::EQA),
            eqb: self.read_reg(reg::EQB),
            eqc: self.read_reg(reg::EQC),
            eqt: self.read_reg(reg::EQT),
            esa: self.read_reg(reg::ESA),
            esb: self.read_reg(reg::ESB),
            esc: self.read_reg(reg::ESC),
            est: self.read_reg(reg::EST),
        }
    }

    /// 读取线频率 (Hz * 100)
    pub fn read_frequency(&mut self) -> u32 {
        freq_to_fixed(self.read_reg(reg::FREQ))
    }

    /// 读取三相功率因数
    pub fn read_pf(&mut self) -> PfData {
        PfData {
            pfa: pf_to_fixed(self.read_reg(reg::PFA)),
            pfb: pf_to_fixed(self.read_reg(reg::PFB)),
            pfc: pf_to_fixed(self.read_reg(reg::PFC)),
            pft: pf_to_fixed(self.read_reg(reg::PFT)),
        }
    }

    /// 读取三相相角 (度 * 100)
    pub fn read_angles(&mut self) -> AngleData {
        AngleData {
            pga: angle_to_fixed(self.read_reg(reg::PGA)),
            pgb: angle_to_fixed(self.read_reg(reg::PGB)),
            pgc: angle_to_fixed(self.read_reg(reg::PGC)),
            y_ua_ub: self.read_reg(reg::Y_UA_UB),
            y_ua_uc: self.read_reg(reg::Y_UA_UC),
            y_ub_uc: self.read_reg(reg::Y_UB_UC),
        }
    }

    /// 读取温度 (需先校准)
    pub fn read_temperature(&mut self, tc_cal: i32) -> i32 {
        temp_to_celsius(self.read_reg(reg::TPSD), tc_cal)
    }

    /// 读取状态标志
    pub fn read_status(&mut self) -> StatusFlags {
        let sflag = self.read_reg(reg::S_FLAG);
        let eflag = self.read_reg(reg::E_FLAG);
        let pflag = self.read_reg(reg::P_FLAG);
        StatusFlags {
            raw_sflag: sflag,
            raw_eflag: eflag,
            raw_pflag: pflag,
        }
    }

    /// 读取基波功率和电能
    pub fn read_line_power(&mut self) -> LinePowerData {
        LinePowerData {
            line_pa: signed24(self.read_reg(reg::LINE_PA)),
            line_pb: signed24(self.read_reg(reg::LINE_PB)),
            line_pc: signed24(self.read_reg(reg::LINE_PC)),
            line_pt: signed24(self.read_reg(reg::LINE_PT)),
        }
    }

    // ── 校表操作 ──

    /// 写入校表参数 (需要先 enable_calibration_write)
    pub fn write_calib(&mut self, addr: u8, data: u32) {
        // 先切换到校表数据空间
        self.switch_to_calib_read();
        self.write_reg(addr, data);
        self.switch_to_meter_read();
    }

    /// 读取校表参数
    pub fn read_calib(&mut self, addr: u8) -> u32 {
        self.switch_to_calib_read();
        let val = self.read_reg(addr);
        self.switch_to_meter_read();
        val
    }

    /// 配置高频脉冲常数
    pub fn set_hfconst(&mut self, value: u32) {
        self.enable_calibration_write();
        self.write_calib(cal_reg::HF_CONST, value);
        self.disable_calibration_write();
    }

    // ── ADC 缓冲 ──

    /// 启动波形缓冲 (通道: 0=Ua,1=Ia,2=Ub,3=Ib,4=Uc,5=Ic,6=In,...)
    pub fn start_wave_buffer(&mut self, channel: u8) {
        let data = 0x00CC00 | (channel as u32 & 0x0F);
        self.send_special(special_cmd::WAVE_START & 0x3F, data);
    }

    /// 设置波形缓冲读指针 (0~1023)
    pub fn set_wave_ptr(&mut self, ptr: u32) {
        self.send_special(special_cmd::WAVE_PTR & 0x3F, ptr & 0x3FF);
    }

    /// 读取波形缓冲数据指针 (已写入的数据量)
    pub fn read_wave_ptr(&mut self) -> u32 {
        self.read_reg(reg::PTR_WAVE)
    }

    /// 读取波形缓冲数据 (连续读直到读完)
    pub fn read_wave_data(&mut self) -> u32 {
        self.read_reg(reg::WAVE_BUFF)
    }
}

// ══════════════════════════════════════════════════════════════════
// 数据结构
// ══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy)]
pub struct PowerData {
    pub pa: i32, pub pb: i32, pub pc: i32, pub pt: i32,
    pub qa: i32, pub qb: i32, pub qc: i32, pub qt: i32,
    pub sa: u32, pub sb: u32, pub sc: u32, pub st: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct RmsData {
    pub ua: u32, pub ub: u32, pub uc: u32,  // 电压有效值
    pub ia: u32, pub ib: u32, pub ic: u32,  // 电流有效值
    pub i0: u32,  // 零线电流
    pub ut: u32,  // 三相电压矢量和
    pub it: u32,  // 三相电流矢量和
}

#[derive(Debug, Clone, Copy)]
pub struct EnergyData {
    pub epa: u32, pub epb: u32, pub epc: u32, pub ept: u32,
    pub eqa: u32, pub eqb: u32, pub eqc: u32, pub eqt: u32,
    pub esa: u32, pub esb: u32, pub esc: u32, pub est: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct PfData {
    pub pfa: i32, pub pfb: i32, pub pfc: i32, pub pft: i32, // PF * 1000
}

#[derive(Debug, Clone, Copy)]
pub struct AngleData {
    pub pga: i32, pub pgb: i32, pub pgc: i32, // 相角 (度*100)
    pub y_ua_ub: u32, pub y_ua_uc: u32, pub y_ub_uc: u32, // 电压夹角
}

#[derive(Debug, Clone, Copy)]
pub struct LinePowerData {
    pub line_pa: i32, pub line_pb: i32, pub line_pc: i32, pub line_pt: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct StatusFlags {
    pub raw_sflag: u32,  // 断相/相序/SIG
    pub raw_eflag: u32,  // 电能状态
    pub raw_pflag: u32,  // 功率方向
}

// ══════════════════════════════════════════════════════════════════
// Device ID 常量
// ══════════════════════════════════════════════════════════════════

pub const DEVICE_ID_ATT7022E: u32 = 0x7122A0;

/// 检查读回的 Device ID 是否正确
pub fn is_valid_device_id(id: u32) -> bool {
    id == DEVICE_ID_ATT7022E
}
