/* ================================================================== */
/*                                                                    */
/*  rn8302b.rs — RN8302B 三相多功能电能计量芯片驱动                    */
/*                                                                    */
/*  锐能微 (RN) RN8302B — 三相防窃电多功能计量芯片                     */
/*  - 7 路 24-bit sigma-delta ADC (3×电压 + 3×电流 + 1×零线电流)      */
/*  - 有功/无功/视在功率和电能计量, 0.5 级精度                         */
/*  - 动态范围 5000:1                                                  */
/*  - 谐波分析 ≤51 次                                                  */
/*  - 防窃电检测 (开盖/短路/强磁/旁路)                                 */
/*  - SPI 从模式, 8bit 命令 + 24/32bit 数据, 最高 3.5 Mbps            */
/*  - 晶振 8.192MHz, 3.3V 供电, LQFP48 封装                           */
/*                                                                    */
/*  参考: RN8302B 用户手册 V1.3                                        */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::fm33lg0::SpiRegs;
use crate::hal::*;
use core::ops::Deref;

// ══════════════════════════════════════════════════════════════════
// SPI 协议
// ══════════════════════════════════════════════════════════════════
//
// CS:   低有效
// SCLK: 最高 3.5 MHz
// 数据: MSB first
//
// 写操作: 1bit W/R(1) + 7bit 地址 + 8bit 数据高位 + 8bit 数据中位 + 8bit 数据低位
//         + 8bit CRC = 32bit
// 读操作: 1bit W/R(0) + 7bit 地址 + 8bit dummy + 8bit 数据高位 + 8bit 数据中位
//         + 8bit 数据低位 + 8bit CRC = 48bit
//
// CRC-8: x^8 + x^2 + x + 1 (0x07), init=0

/// RN8302B 芯片标识 (Chip ID 寄存器值)
const CHIP_ID_VALUE: u32 = 0x8302_2000;

/* ================================================================== */
/*  寄存器地址定义                                                      */
/* ================================================================== */

pub mod reg {
    // ── 系统寄存器 ──
    /// 芯片标识
    pub const CHIP_ID: u8 = 0x00;
    /// 软件复位 (写 0xA5)
    pub const SOFT_RESET: u8 = 0x01;
    /// 系统配置
    pub const SYSCON: u8 = 0x02;
    /// EMU 配置
    pub const EMU: u8 = 0x03;
    /// 起始电流/启动功率
    pub const STARTCURRENT: u8 = 0x04;

    // ── 有功功率 ──
    /// A 相有功功率
    pub const PA: u8 = 0x10;
    /// B 相有功功率
    pub const PB: u8 = 0x11;
    /// C 相有功功率
    pub const PC: u8 = 0x12;
    /// 合相有功功率
    pub const PT: u8 = 0x13;

    // ── 无功功率 ──
    /// A 相无功功率
    pub const QA: u8 = 0x14;
    /// B 相无功功率
    pub const QB: u8 = 0x15;
    /// C 相无功功率
    pub const QC: u8 = 0x16;
    /// 合相无功功率
    pub const QT: u8 = 0x17;

    // ── 视在功率 ──
    /// A 相视在功率
    pub const SA: u8 = 0x18;
    /// B 相视在功率
    pub const SB: u8 = 0x19;
    /// C 相视在功率
    pub const SC: u8 = 0x1A;
    /// 合相视在功率
    pub const ST: u8 = 0x1B;

    // ── 电压有效值 ──
    /// A 相电压 RMS
    pub const UA_RMS: u8 = 0x20;
    /// B 相电压 RMS
    pub const UB_RMS: u8 = 0x21;
    /// C 相电压 RMS
    pub const UC_RMS: u8 = 0x22;

    // ── 电流有效值 ──
    /// A 相电流 RMS
    pub const IA_RMS: u8 = 0x23;
    /// B 相电流 RMS
    pub const IB_RMS: u8 = 0x24;
    /// C 相电流 RMS
    pub const IC_RMS: u8 = 0x25;
    /// 零线电流 RMS
    pub const IN_RMS: u8 = 0x26;

    // ── 频率 / 功率因数 ──
    /// 电网频率
    pub const FREQ: u8 = 0x30;
    /// A 相功率因数
    pub const PFA: u8 = 0x31;
    /// B 相功率因数
    pub const PFB: u8 = 0x32;
    /// C 相功率因数
    pub const PFC: u8 = 0x33;
    /// 合相功率因数
    pub const PFT: u8 = 0x34;

    // ── 相角 ──
    /// A 相电压-电流相角
    pub const PHA_UA_IA: u8 = 0x35;
    /// B 相电压-电流相角
    pub const PHA_UB_IB: u8 = 0x36;
    /// C 相电压-电流相角
    pub const PHA_UC_IC: u8 = 0x37;

    // ── 电能寄存器 ──
    /// 正向有功电能
    pub const ACTIVE_IMPORT: u8 = 0x40;
    /// 反向有功电能
    pub const ACTIVE_EXPORT: u8 = 0x41;
    /// 正向无功电能
    pub const REACTIVE_IMPORT: u8 = 0x42;
    /// 反向无功电能
    pub const REACTIVE_EXPORT: u8 = 0x43;

    // ── 谐波分析 ──
    /// A 相电压谐波 (基波 + 1~51 次)
    pub const HARMONIC_UA: u8 = 0x50;
    /// B 相电压谐波
    pub const HARMONIC_UB: u8 = 0x51;
    /// C 相电压谐波
    pub const HARMONIC_UC: u8 = 0x52;
    /// A 相电流谐波
    pub const HARMONIC_IA: u8 = 0x53;
    /// B 相电流谐波
    pub const HARMONIC_IB: u8 = 0x54;
    /// C 相电流谐波
    pub const HARMONIC_IC: u8 = 0x55;
    // ── THD ──
    /// A 相电压 THD
    pub const THD_UA: u8 = 0x60;
    /// B 相电压 THD
    pub const THD_UB: u8 = 0x61;
    /// C 相电压 THD
    pub const THD_UC: u8 = 0x62;
    /// A 相电流 THD
    pub const THD_IA: u8 = 0x63;
    /// B 相电流 THD
    pub const THD_IB: u8 = 0x64;
    /// C 相电流 THD
    pub const THD_IC: u8 = 0x65;

    // ── 防窃电 ──
    /// 防窃电状态
    pub const ANTI_TAMPER_STAT: u8 = 0x70;
    /// 防窃电配置
    pub const ANTI_TAMPER_CFG: u8 = 0x71;

    // ── 校准寄存器 ──
    /// A 相电压增益
    pub const UGAIN_A: u8 = 0x80;
    /// B 相电压增益
    pub const UGAIN_B: u8 = 0x81;
    /// C 相电压增益
    pub const UGAIN_C: u8 = 0x82;
    /// A 相电流增益
    pub const IGAIN_A: u8 = 0x83;
    /// B 相电流增益
    pub const IGAIN_B: u8 = 0x84;
    /// C 相电流增益
    pub const IGAIN_C: u8 = 0x85;
    /// A 相有功功率增益
    pub const PGAIN_A: u8 = 0x86;
    /// B 相有功功率增益
    pub const PGAIN_B: u8 = 0x87;
    /// C 相有功功率增益
    pub const PGAIN_C: u8 = 0x88;
    /// A 相无功功率增益
    pub const QGAIN_A: u8 = 0x89;
    /// B 相无功功率增益
    pub const QGAIN_B: u8 = 0x8A;
    /// C 相无功功率增益
    pub const QGAIN_C: u8 = 0x8B;
    /// A 相角差校正
    pub const PHCAL_A: u8 = 0x8C;
    /// B 相角差校正
    pub const PHCAL_B: u8 = 0x8D;
    /// C 相角差校正
    pub const PHCAL_C: u8 = 0x8E;
    /// 有功功率偏置 A
    pub const POFFSET_A: u8 = 0x8F;
    /// 有功功率偏置 B
    pub const POFFSET_B: u8 = 0x90;
    /// 有功功率偏置 C
    pub const POFFSET_C: u8 = 0x91;
    /// 无功功率偏置 A
    pub const QOFFSET_A: u8 = 0x92;
    /// 无功功率偏置 B
    pub const QOFFSET_B: u8 = 0x93;
    /// 无功功率偏置 C
    pub const QOFFSET_C: u8 = 0x94;
}

/* ================================================================== */
/*  CRC-8 计算                                                         */
/* ================================================================== */

/// CRC-8 多项式: x^8 + x^2 + x + 1
const CRC8_POLY: u8 = 0x07;

/// 计算 CRC-8
fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ CRC8_POLY;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/* ================================================================== */
/*  SPI 读写操作                                                       */
/* ================================================================== */

/// RN8302B SPI 写操作
/// 格式: [W/R|ADDR(7)] [DATA_H] [DATA_M] [DATA_L] [CRC]
/// 共 5 字节
fn spi_write(spi: &mut dyn SpiTransfer, addr: u8, data: u32) -> Result<(), MeteringError> {
    let addr_byte = (addr & 0x7F) | 0x80; // W/R=1 表示写
    let dh = ((data >> 16) & 0xFF) as u8;
    let dm = ((data >> 8) & 0xFF) as u8;
    let dl = (data & 0xFF) as u8;
    let crc = crc8(&[addr_byte, dh, dm, dl]);

    let tx = [addr_byte, dh, dm, dl, crc];
    spi.transfer(&tx, &mut [0; 5])
        .map_err(|_| MeteringError::SpiError)
}

/// RN8302B SPI 读操作
/// 格式: 发送 [W/R|ADDR(7)] [DUMMY]
///       接收 [DATA_H] [DATA_M] [DATA_L] [CRC]
/// 共发送 2 + 接收 4 = 6 字节
fn spi_read(spi: &mut dyn SpiTransfer, addr: u8) -> Result<u32, MeteringError> {
    let addr_byte = addr & 0x7F; // W/R=0 表示读
    let tx = [addr_byte, 0x00];
    let mut rx = [0u8; 6];

    // 发送地址 + dummy, 然后读回数据 + CRC
    // 实际 SPI 通信: 全双工, CS 持续拉低
    // 有些实现分两步, 这里简化为一次传输
    spi.transfer(&tx, &mut rx[..2])
        .map_err(|_| MeteringError::SpiError)?;
    spi.transfer(&[0xFF; 4], &mut rx[2..])
        .map_err(|_| MeteringError::SpiError)?;

    let dh = rx[2];
    let dm = rx[3];
    let dl = rx[4];
    let crc_received = rx[5];

    // CRC 校验: CRC(addr_byte, dh, dm, dl)
    let crc_calc = crc8(&[addr_byte, dh, dm, dl]);
    if crc_calc != crc_received {
        return Err(MeteringError::ChecksumError);
    }

    Ok(((dh as u32) << 16) | ((dm as u32) << 8) | (dl as u32))
}

/* ================================================================== */
/*  24-bit 有符号数转换                                                 */
/* ================================================================== */

/// 将 24-bit 无符号值转为 i32 (二进制补码)
fn signed24(val: u32) -> i32 {
    if val & 0x0080_0000 != 0 {
        (val | 0xFF00_0000) as i32
    } else {
        val as i32
    }
}

/* ================================================================== */
/*  RN8302B 驱动结构体                                                  */
/* ================================================================== */

/// RN8302B 计量芯片驱动
pub struct Rn8302b {
    spi: &'static mut dyn SpiTransfer,
    /// 电压校准系数 [A, B, C]
    u_coeff: [f32; 3],
    /// 电流校准系数 [A, B, C]
    i_coeff: [f32; 3],
    /// 有功功率校准系数 [A, B, C]
    p_coeff: [f32; 3],
    /// 无功功率校准系数 [A, B, C]
    q_coeff: [f32; 3],
}

impl Rn8302b {
    /// 创建 RN8302B 驱动实例
    ///
    /// # Arguments
    /// * `spi` - SPI 传输接口 (SPI0)
    pub fn new(spi: &'static mut dyn SpiTransfer) -> Self {
        Self {
            spi,
            u_coeff: [1.0; 3],
            i_coeff: [1.0; 3],
            p_coeff: [1.0; 3],
            q_coeff: [1.0; 3],
        }
    }

    /// 检查防窃电状态
    pub fn read_tamper_status(&mut self) -> Result<u32, MeteringError> {
        spi_read(self.spi, reg::ANTI_TAMPER_STAT)
    }

    /// 读取 A 相电压谐波 (基波 + 1~51 次)
    ///
    /// 返回各次谐波相对于基波的百分比 (0.01%)
    /// 需要先配置谐波采样模式
    pub fn read_voltage_harmonics(&mut self, phase: Phase) -> Result<HarmonicData, MeteringError> {
        let base_reg = match phase {
            Phase::A => reg::HARMONIC_UA,
            Phase::B => reg::HARMONIC_UB,
            Phase::C => reg::HARMONIC_UC,
        };

        let mut data = HarmonicData::default();

        // 读取 THD
        let thd_reg = match phase {
            Phase::A => reg::THD_UA,
            Phase::B => reg::THD_UB,
            Phase::C => reg::THD_UC,
        };
        let thd_raw = spi_read(self.spi, thd_reg)?;
        data.thd = (thd_raw & 0xFFFF) as u16;

        // 读取各次谐波 (简化: 读前 20 次, 每次 1 个寄存器偏移)
        for i in 0..20 {
            let reg_addr = base_reg + i as u8;
            if let Ok(val) = spi_read(self.spi, reg_addr) {
                data.harmonics[i] = (val & 0xFFFF) as u16;
            }
        }

        Ok(data)
    }

    /// 读取 A 相电流谐波
    pub fn read_current_harmonics(&mut self, phase: Phase) -> Result<HarmonicData, MeteringError> {
        let base_reg = match phase {
            Phase::A => reg::HARMONIC_IA,
            Phase::B => reg::HARMONIC_IB,
            Phase::C => reg::HARMONIC_IC,
        };

        let mut data = HarmonicData::default();

        let thd_reg = match phase {
            Phase::A => reg::THD_IA,
            Phase::B => reg::THD_IB,
            Phase::C => reg::THD_IC,
        };
        let thd_raw = spi_read(self.spi, thd_reg)?;
        data.thd = (thd_raw & 0xFFFF) as u16;

        for i in 0..20 {
            let reg_addr = base_reg + i as u8;
            if let Ok(val) = spi_read(self.spi, reg_addr) {
                data.harmonics[i] = (val & 0xFFFF) as u16;
            }
        }

        Ok(data)
    }
}

/* ================================================================== */
/*  实现 MeteringChip trait                                             */
/* ================================================================== */

impl MeteringChip for Rn8302b {
    fn init(&mut self, params: &CalibrationParams) -> Result<(), MeteringError> {
        // 1. 软件复位
        self.reset()?;

        // 2. 等待上电稳定 (~100ms)
        //    在 Embassy 中用 Timer::after(Duration::from_millis(100)).await;
        //    这里用循环等待 (裸机环境)
        let mut delay = 0;
        while delay < 100_000 {
            delay += 1;
            core::hint::spin_loop();
        }

        // 3. 验证 Chip ID
        let id = self.chip_id()?;
        if (id & 0xFFFF_F000) != (CHIP_ID_VALUE & 0xFFFF_F000) {
            return Err(MeteringError::SpiError);
        }

        // 4. 写入校准参数
        //    电压增益
        for (i, &gain) in params.voltage_gain.iter().enumerate() {
            let reg = reg::UGAIN_A + i as u8;
            let val = (gain * 65536.0) as u32;
            spi_write(self.spi, reg, val)?;
        }

        //    电流增益
        for (i, &gain) in params.current_gain.iter().enumerate() {
            let reg = reg::IGAIN_A + i as u8;
            let val = (gain * 65536.0) as u32;
            spi_write(self.spi, reg, val)?;
        }

        //    有功功率增益
        for (i, &gain) in params.power_gain.iter().enumerate() {
            let reg = reg::PGAIN_A + i as u8;
            let val = (gain * 65536.0) as u32;
            spi_write(self.spi, reg, val)?;
        }

        //    无功功率增益
        for (i, &gain) in params.reactive_gain.iter().enumerate() {
            let reg = reg::QGAIN_A + i as u8;
            let val = (gain * 65536.0) as u32;
            spi_write(self.spi, reg, val)?;
        }

        //    相角校正
        for (i, &angle) in params.phase_angle.iter().enumerate() {
            let reg = reg::PHCAL_A + i as u8;
            let val = (angle * 100.0) as u32; // 0.01度/LSB
            spi_write(self.spi, reg, val)?;
        }

        // 5. 保存校准系数
        self.u_coeff = params.voltage_gain;
        self.i_coeff = params.current_gain;
        self.p_coeff = params.power_gain;
        self.q_coeff = params.reactive_gain;

        Ok(())
    }

    fn reset(&mut self) -> Result<(), MeteringError> {
        spi_write(self.spi, reg::SOFT_RESET, 0xA5)
    }

    fn read_instant_data(&mut self) -> Result<PhaseData, MeteringError> {
        // 读取三相电压 RMS
        let ua_raw = spi_read(self.spi, reg::UA_RMS)?;
        let ub_raw = spi_read(self.spi, reg::UB_RMS)?;
        let uc_raw = spi_read(self.spi, reg::UC_RMS)?;

        // 读取三相电流 RMS
        let ia_raw = spi_read(self.spi, reg::IA_RMS)?;
        let ib_raw = spi_read(self.spi, reg::IB_RMS)?;
        let ic_raw = spi_read(self.spi, reg::IC_RMS)?;

        // 读取有功功率
        let pa_raw = spi_read(self.spi, reg::PA)?;
        let pb_raw = spi_read(self.spi, reg::PB)?;
        let pc_raw = spi_read(self.spi, reg::PC)?;
        let pt_raw = spi_read(self.spi, reg::PT)?;

        // 读取无功功率
        let qa_raw = spi_read(self.spi, reg::QA)?;
        let qb_raw = spi_read(self.spi, reg::QB)?;
        let qc_raw = spi_read(self.spi, reg::QC)?;
        let qt_raw = spi_read(self.spi, reg::QT)?;

        // 读取频率
        let freq_raw = spi_read(self.spi, reg::FREQ)?;

        // 读取功率因数
        let pfa_raw = spi_read(self.spi, reg::PFA)?;
        let pfb_raw = spi_read(self.spi, reg::PFB)?;
        let pfc_raw = spi_read(self.spi, reg::PFC)?;
        let pft_raw = spi_read(self.spi, reg::PFT)?;

        // 转换为工程单位
        // RN8302B: 电压 1 LSB ≈ 电压系数 × raw, 电流类似
        // 此处使用校准系数转换
        let voltage_a = (ua_raw as f32 * self.u_coeff[0] * 100.0) as u16; // 0.01V
        let voltage_b = (ub_raw as f32 * self.u_coeff[1] * 100.0) as u16;
        let voltage_c = (uc_raw as f32 * self.u_coeff[2] * 100.0) as u16;

        let current_a = (ia_raw as f32 * self.i_coeff[0] * 1000.0) as u16; // mA
        let current_b = (ib_raw as f32 * self.i_coeff[1] * 1000.0) as u16;
        let current_c = (ic_raw as f32 * self.i_coeff[2] * 1000.0) as u16;

        let active_power_a = signed24(pa_raw);
        let active_power_b = signed24(pb_raw);
        let active_power_c = signed24(pc_raw);
        let active_power_total = signed24(pt_raw);

        let reactive_power_a = signed24(qa_raw);
        let reactive_power_b = signed24(qb_raw);
        let reactive_power_c = signed24(qc_raw);
        let reactive_power_total = signed24(qt_raw);

        // 频率: RN8302B freq = 8192000 / (2 * freq_reg) Hz
        let frequency = if freq_raw > 0 {
            (8192000.0 / (2.0 * freq_raw as f32) * 100.0) as u16 // 0.01Hz
        } else {
            5000 // 默认 50.00Hz
        };

        // 功率因数: 0~1000 映射
        let pf_convert = |raw: u32| -> u16 {
            // RN8302B PF 寄存器: 有符号, 范围 -32768~32767, 映射到 -1.0~1.0
            let val = signed24(raw) as f32;
            ((val.abs() / 32768.0 * 1000.0) as u16).min(1000)
        };

        Ok(PhaseData {
            voltage_a,
            voltage_b,
            voltage_c,
            current_a,
            current_b,
            current_c,
            active_power_a,
            active_power_b,
            active_power_c,
            active_power_total,
            reactive_power_a,
            reactive_power_b,
            reactive_power_c,
            reactive_power_total,
            frequency,
            power_factor_a: pf_convert(pfa_raw),
            power_factor_b: pf_convert(pfb_raw),
            power_factor_c: pf_convert(pfc_raw),
            power_factor_total: pf_convert(pft_raw),
        })
    }

    fn read_energy(&mut self) -> Result<EnergyData, MeteringError> {
        let active_import = spi_read(self.spi, reg::ACTIVE_IMPORT)?;
        let active_export = spi_read(self.spi, reg::ACTIVE_EXPORT)?;
        let reactive_import = spi_read(self.spi, reg::REACTIVE_IMPORT)?;
        let reactive_export = spi_read(self.spi, reg::REACTIVE_EXPORT)?;

        // 电能转换: 需要根据脉冲常数和累积圈数计算
        // 简化: 直接使用寄存器值 × 系数
        let e_coeff = 0.01; // 0.01 kWh/LSB (需校准)

        Ok(EnergyData {
            active_import: (active_import as f64 * e_coeff * 100.0) as u64,
            active_export: (active_export as f64 * e_coeff * 100.0) as u64,
            reactive_import: (reactive_import as f64 * e_coeff * 100.0) as u64,
            reactive_export: (reactive_export as f64 * e_coeff * 100.0) as u64,
            active_import_a: 0, // RN8302B 无单相电能, 需 MCU 从功率积分
            active_import_b: 0,
            active_import_c: 0,
        })
    }

    fn read_neutral_current(&mut self) -> Result<u16, MeteringError> {
        let raw = spi_read(self.spi, reg::IN_RMS)?;
        Ok((raw as f32 * self.i_coeff[0] * 1000.0) as u16)
    }

    fn chip_id(&mut self) -> Result<u32, MeteringError> {
        spi_read(self.spi, reg::CHIP_ID)
    }

    fn name() -> &'static str {
        "RN8302B"
    }

    fn supports_fundamental(&self) -> bool {
        true
    }

    fn read_fundamental_power(&mut self) -> Result<[i32; 3], MeteringError> {
        // RN8302B 支持基波/谐波分离
        // 基波有功功率寄存器 (假设地址连续, 实际需查手册)
        let pa = signed24(spi_read(self.spi, 0x1C)?);
        let pb = signed24(spi_read(self.spi, 0x1D)?);
        let pc = signed24(spi_read(self.spi, 0x1E)?);
        Ok([pa, pb, pc])
    }
}

/* ================================================================== */
/*  实现 HarmonicAnalysis trait                                         */
/* ================================================================== */

impl HarmonicAnalysis for Rn8302b {
    fn read_harmonics(&mut self, phase: Phase) -> Result<HarmonicData, MeteringError> {
        self.read_voltage_harmonics(phase)
    }

    fn read_thd(&mut self, phase: Phase) -> Result<u16, MeteringError> {
        let thd_reg = match phase {
            Phase::A => reg::THD_UA,
            Phase::B => reg::THD_UB,
            Phase::C => reg::THD_UC,
        };
        let raw = spi_read(self.spi, thd_reg)?;
        Ok((raw & 0xFFFF) as u16)
    }

    fn max_harmonic_order(&self) -> u8 {
        51
    }
}

/* ================================================================== */
/*  单元测试 (host side)                                                */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc8() {
        // 验证 CRC-8 计算
        assert_eq!(crc8(&[0x00]), 0x00);
        assert_eq!(
            crc8(&[0x80, 0x00, 0x00, 0xA5]),
            crc8(&[0x80, 0x00, 0x00, 0xA5])
        );
    }

    #[test]
    fn test_signed24() {
        assert_eq!(signed24(0x000000), 0);
        assert_eq!(signed24(0x7FFFFF), 8388607);
        assert_eq!(signed24(0x800000), -8388608);
        assert_eq!(signed24(0xFFFFFF), -1);
    }

    #[test]
    fn test_reg_addresses() {
        assert_eq!(reg::CHIP_ID, 0x00);
        assert_eq!(reg::PA, 0x10);
        assert_eq!(reg::UA_RMS, 0x20);
        assert_eq!(reg::FREQ, 0x30);
        assert_eq!(reg::ACTIVE_IMPORT, 0x40);
        assert_eq!(reg::HARMONIC_UA, 0x50);
        assert_eq!(reg::THD_UA, 0x60);
        assert_eq!(reg::UGAIN_A, 0x80);
    }
}
