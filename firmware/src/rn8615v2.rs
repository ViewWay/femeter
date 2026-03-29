/* ================================================================== */
/*                                                                    */
/*  rn8615v2.rs — RN8615V2 三相高精度电能计量芯片驱动                  */
/*                                                                    */
/*  锐能微 (RN) RN8615V2 — 三相高精度防窃电计量芯片                     */
/*  - 7 路 24-bit sigma-delta ADC (3×电压 + 3×电流 + 1×零线电流)      */
/*  - 有功/无功/视在功率和电能计量, **0.05 级精度**                     */
/*  - 动态范围 10000:1                                                 */
/*  - 谐波分析 ≤63 次 + **间谐波**                                     */
/*  - 电压闪变 (短/长) 硬件支持                                        */
/*  - 三相不平衡度硬件计算                                             */
/*  - 电压暂降/暂升事件检测                                            */
/*  - 增强防窃电 (开盖/短路/强磁/旁路/直流)                            */
/*  - SPI 从模式, 8bit 命令 + 24/32bit 数据, 最高 3.5 Mbps            */
/*  - 晶振 8.192MHz, 3.3V 供电, LQFP48 封装                           */
/*                                                                    */
/*  参考: RN8615V2 用户手册 V1.0                                       */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::hal::*;
use core::ops::Deref;

// ══════════════════════════════════════════════════════════════════
// SPI 协议 (与 RN8302B 兼容, 增加了扩展寄存器)
// ══════════════════════════════════════════════════════════════════

const CHIP_ID_VALUE: u32 = 0x8615_5000;

/* ================================================================== */
/*  寄存器地址定义                                                      */
/* ================================================================== */

pub mod reg {
    // ── 系统寄存器 ──
    pub const CHIP_ID:      u8 = 0x00;
    pub const SOFT_RESET:   u8 = 0x01;
    pub const SYSCON:       u8 = 0x02;
    pub const EMU:          u8 = 0x03;
    pub const STARTCURRENT: u8 = 0x04;
    pub const PQ_CTRL:      u8 = 0x05;  // 电网质量控制寄存器 (V2新增)

    // ── 有功功率 ──
    pub const PA:           u8 = 0x10;
    pub const PB:           u8 = 0x11;
    pub const PC:           u8 = 0x12;
    pub const PT:           u8 = 0x13;

    // ── 无功功率 ──
    pub const QA:           u8 = 0x14;
    pub const QB:           u8 = 0x15;
    pub const QC:           u8 = 0x16;
    pub const QT:           u8 = 0x17;

    // ── 视在功率 ──
    pub const SA:           u8 = 0x18;
    pub const SB:           u8 = 0x19;
    pub const SC:           u8 = 0x1A;
    pub const ST:           u8 = 0x1B;

    // ── 基波有功功率 (V2新增) ──
    pub const PA_FUND:      u8 = 0x1C;
    pub const PB_FUND:      u8 = 0x1D;
    pub const PC_FUND:      u8 = 0x1E;
    pub const PT_FUND:      u8 = 0x1F;

    // ── 电压有效值 ──
    pub const UA_RMS:       u8 = 0x20;
    pub const UB_RMS:       u8 = 0x21;
    pub const UC_RMS:       u8 = 0x22;

    // ── 电流有效值 ──
    pub const IA_RMS:       u8 = 0x23;
    pub const IB_RMS:       u8 = 0x24;
    pub const IC_RMS:       u8 = 0x25;
    pub const IN_RMS:       u8 = 0x26;

    // ── 频率 / 功率因数 / 相角 ──
    pub const FREQ:         u8 = 0x30;
    pub const PFA:          u8 = 0x31;
    pub const PFB:          u8 = 0x32;
    pub const PFC:          u8 = 0x33;
    pub const PFT:          u8 = 0x34;
    pub const PHA_UA_IA:    u8 = 0x35;
    pub const PHA_UB_IB:    u8 = 0x36;
    pub const PHA_UC_IC:    u8 = 0x37;

    // ── 电能寄存器 ──
    pub const ACTIVE_IMPORT:    u8 = 0x40;
    pub const ACTIVE_EXPORT:    u8 = 0x41;
    pub const REACTIVE_IMPORT:  u8 = 0x42;
    pub const REACTIVE_EXPORT:  u8 = 0x43;
    pub const ACTIVE_IMPORT_A:  u8 = 0x44;  // V2: 分相电能
    pub const ACTIVE_IMPORT_B:  u8 = 0x45;
    pub const ACTIVE_IMPORT_C:  u8 = 0x46;

    // ── 谐波分析 (扩展到 63 次) ──
    pub const HARMONIC_UA:      u8 = 0x50;
    pub const HARMONIC_UB:      u8 = 0x51;
    pub const HARMONIC_UC:      u8 = 0x52;
    pub const HARMONIC_IA:      u8 = 0x53;
    pub const HARMONIC_IB:      u8 = 0x54;
    pub const HARMONIC_IC:      u8 = 0x55;

    // ── 间谐波 (V2新增) ──
    pub const INTERHARMONIC_UA: u8 = 0x58;
    pub const INTERHARMONIC_UB: u8 = 0x59;
    pub const INTERHARMONIC_UC: u8 = 0x5A;
    pub const INTERHARMONIC_IA: u8 = 0x5B;
    pub const INTERHARMONIC_IB: u8 = 0x5C;
    pub const INTERHARMONIC_IC: u8 = 0x5D;

    // ── THD ──
    pub const THD_UA:     u8 = 0x60;
    pub const THD_UB:     u8 = 0x61;
    pub const THD_UC:     u8 = 0x62;
    pub const THD_IA:     u8 = 0x63;
    pub const THD_IB:     u8 = 0x64;
    pub const THD_IC:     u8 = 0x65;

    // ── 电网质量 (V2新增) ──
    pub const VOLTAGE_UNBALANCE:  u8 = 0x70;
    pub const CURRENT_UNBALANCE:  u8 = 0x71;
    pub const FLICKER_PST:        u8 = 0x72;  // 短时闪变
    pub const FLICKER_PLT:        u8 = 0x73;  // 长时闪变
    pub const DC_COMPONENT_UA:    u8 = 0x74;  // A相电压直流分量
    pub const DC_COMPONENT_UB:    u8 = 0x75;
    pub const DC_COMPONENT_UC:    u8 = 0x76;

    // ── 电压暂降/暂升事件 (V2新增) ──
    pub const SAG_SWELL_STATUS:    u8 = 0x78;
    pub const SAG_SWELL_CFG:       u8 = 0x79;
    pub const SAG_UA_THRESHOLD:    u8 = 0x7A;
    pub const SWELL_UA_THRESHOLD:  u8 = 0x7B;
    pub const SAG_UB_THRESHOLD:    u8 = 0x7C;
    pub const SWELL_UB_THRESHOLD:  u8 = 0x7D;
    pub const SAG_UC_THRESHOLD:    u8 = 0x7E;
    pub const SWELL_UC_THRESHOLD:  u8 = 0x7F;

    // ── 防窃电 (增强版) ──
    pub const ANTI_TAMPER_STAT:    u8 = 0x80;
    pub const ANTI_TAMPER_CFG:     u8 = 0x81;
    pub const DC_TAMPER_CFG:       u8 = 0x82;  // V2: 直流窃电检测

    // ── 校准寄存器 ──
    pub const UGAIN_A:    u8 = 0x90;
    pub const UGAIN_B:    u8 = 0x91;
    pub const UGAIN_C:    u8 = 0x92;
    pub const IGAIN_A:    u8 = 0x93;
    pub const IGAIN_B:    u8 = 0x94;
    pub const IGAIN_C:    u8 = 0x95;
    pub const PGAIN_A:    u8 = 0x96;
    pub const PGAIN_B:    u8 = 0x97;
    pub const PGAIN_C:    u8 = 0x98;
    pub const QGAIN_A:    u8 = 0x99;
    pub const QGAIN_B:    u8 = 0x9A;
    pub const QGAIN_C:    u8 = 0x9B;
    pub const PHCAL_A:    u8 = 0x9C;
    pub const PHCAL_B:    u8 = 0x9D;
    pub const PHCAL_C:    u8 = 0x9E;
    pub const POFFSET_A:  u8 = 0x9F;
    pub const POFFSET_B:  u8 = 0xA0;
    pub const POFFSET_C:  u8 = 0xA1;
    pub const QOFFSET_A:  u8 = 0xA2;
    pub const QOFFSET_B:  u8 = 0xA3;
    pub const QOFFSET_C:  u8 = 0xA4;
}

/* ================================================================== */
/*  CRC-8 (与 RN8302B 相同)                                            */
/* ================================================================== */

const CRC8_POLY: u8 = 0x07;

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
/*  SPI 读写 (与 RN8302B 兼容)                                         */
/* ================================================================== */

fn spi_write(spi: &mut dyn SpiTransfer, addr: u8, data: u32) -> Result<(), MeteringError> {
    let addr_byte = (addr & 0x7F) | 0x80;
    let dh = ((data >> 16) & 0xFF) as u8;
    let dm = ((data >> 8) & 0xFF) as u8;
    let dl = (data & 0xFF) as u8;
    let crc = crc8(&[addr_byte, dh, dm, dl]);
    let tx = [addr_byte, dh, dm, dl, crc];
    spi.transfer(&tx, &mut [0; 5]).map_err(|_| MeteringError::SpiError)
}

fn spi_read(spi: &mut dyn SpiTransfer, addr: u8) -> Result<u32, MeteringError> {
    let addr_byte = addr & 0x7F;
    let tx = [addr_byte, 0x00];
    let mut rx = [0u8; 6];

    spi.transfer(&tx, &mut rx[..2]).map_err(|_| MeteringError::SpiError)?;
    spi.transfer(&[0xFF; 4], &mut rx[2..]).map_err(|_| MeteringError::SpiError)?;

    let dh = rx[2];
    let dm = rx[3];
    let dl = rx[4];
    let crc_received = rx[5];

    let crc_calc = crc8(&[addr_byte, dh, dm, dl]);
    if crc_calc != crc_received {
        return Err(MeteringError::ChecksumError);
    }

    Ok(((dh as u32) << 16) | ((dm as u32) << 8) | (dl as u32))
}

fn signed24(val: u32) -> i32 {
    if val & 0x0080_0000 != 0 {
        (val | 0xFF00_0000) as i32
    } else {
        val as i32
    }
}

/* ================================================================== */
/*  RN8615V2 驱动结构体                                                 */
/* ================================================================== */

pub struct RN8615V2 {
    spi: &'static mut dyn SpiTransfer,
    u_coeff: [f32; 3],
    i_coeff: [f32; 3],
    p_coeff: [f32; 3],
    q_coeff: [f32; 3],
    /// 暂降/暂升事件缓冲
    pq_event_pending: Option<PowerQualityEvent>,
}

impl RN8615V2 {
    pub fn new(spi: &'static mut dyn SpiTransfer) -> Self {
        Self {
            spi,
            u_coeff: [1.0; 3],
            i_coeff: [1.0; 3],
            p_coeff: [1.0; 3],
            q_coeff: [1.0; 3],
            pq_event_pending: None,
        }
    }

    /// 读取防窃电状态 (增强版: 含直流窃电)
    pub fn read_tamper_status(&mut self) -> Result<u32, MeteringError> {
        spi_read(self.spi, reg::ANTI_TAMPER_STAT)
    }

    /// 配置电压暂降/暂升检测阈值
    ///
    /// # Arguments
    /// * `sag_threshold` - 暂降阈值 (0.01V), 低于此值触发
    /// * `swell_threshold` - 暂升阈值 (0.01V), 高于此值触发
    pub fn configure_sag_swell(
        &mut self,
        sag_threshold: [u16; 3],
        swell_threshold: [u16; 3],
    ) -> Result<(), MeteringError> {
        // 使能检测
        spi_write(self.spi, reg::SAG_SWELL_CFG, 0x00007F)?;

        // 设置各相阈值
        let regs = [
            reg::SAG_UA_THRESHOLD, reg::SAG_UB_THRESHOLD, reg::SAG_UC_THRESHOLD,
        ];
        for (i, &reg_addr) in regs.iter().enumerate() {
            spi_write(self.spi, reg_addr, sag_threshold[i] as u32)?;
        }

        let regs = [
            reg::SWELL_UA_THRESHOLD, reg::SWELL_UB_THRESHOLD, reg::SWELL_UC_THRESHOLD,
        ];
        for (i, &reg_addr) in regs.iter().enumerate() {
            spi_write(self.spi, reg_addr, swell_threshold[i] as u32)?;
        }

        Ok(())
    }

    /// 读取完整谐波数据 (63次)
    pub fn read_full_harmonics(&mut self, phase: Phase) -> Result<HarmonicData, MeteringError> {
        let base_reg = match phase {
            Phase::A => reg::HARMONIC_UA,
            Phase::B => reg::HARMONIC_UB,
            Phase::C => reg::HARMONIC_UC,
        };

        let mut data = HarmonicData::default();

        // THD
        let thd_reg = match phase {
            Phase::A => reg::THD_UA,
            Phase::B => reg::THD_UB,
            Phase::C => reg::THD_UC,
        };
        let thd_raw = spi_read(self.spi, thd_reg)?;
        data.thd = (thd_raw & 0xFFFF) as u16;

        // 63 次谐波 (RN8615V2 支持)
        for i in 0..HarmonicData::MAX_HARMONICS {
            let reg_addr = base_reg.wrapping_add(i as u8);
            if let Ok(val) = spi_read(self.spi, reg_addr) {
                data.harmonics[i] = (val & 0xFFFF) as u16;
            }
        }

        Ok(data)
    }

    /// 读取间谐波 (指定相, 指定次数)
    pub fn read_interharmonic_raw(&mut self, phase: Phase, order: u8) -> Result<u16, MeteringError> {
        if order > 63 {
            return Err(MeteringError::InvalidRegister);
        }
        let base_reg = match phase {
            Phase::A => reg::INTERHARMONIC_UA,
            Phase::B => reg::INTERHARMONIC_UB,
            Phase::C => reg::INTERHARMONIC_UC,
        };
        let raw = spi_read(self.spi, base_reg + order)?;
        Ok((raw & 0xFFFF) as u16)
    }

    /// 检查电压暂降/暂升事件
    fn check_sag_swell(&mut self) -> Option<PowerQualityEvent> {
        if let Ok(status) = spi_read(self.spi, reg::SAG_SWELL_STATUS) {
            // Bit[2:0] = A/B/C 相暂降, Bit[5:3] = A/B/C 相暂升
            let sag_a = status & 0x01 != 0;
            let sag_b = status & 0x02 != 0;
            let sag_c = status & 0x04 != 0;
            let swell_a = status & 0x08 != 0;
            let swell_b = status & 0x10 != 0;
            let swell_c = status & 0x20 != 0;

            if sag_a || sag_b || sag_c {
                let phase = if sag_a { 0 } else if sag_b { 1 } else { 2 };
                // 清除事件
                let _ = spi_write(self.spi, reg::SAG_SWELL_STATUS, 0);
                return Some(PowerQualityEvent::VoltageSag {
                    phase,
                    duration_ms: 0, // 需从寄存器读取实际持续时间
                    min_voltage: 0,
                });
            }
            if swell_a || swell_b || swell_c {
                let phase = if swell_a { 0 } else if swell_b { 1 } else { 2 };
                let _ = spi_write(self.spi, reg::SAG_SWELL_STATUS, 0);
                return Some(PowerQualityEvent::VoltageSwell {
                    phase,
                    duration_ms: 0,
                    max_voltage: 0,
                });
            }
        }
        None
    }
}

/* ================================================================== */
/*  实现 MeteringChip trait                                             */
/* ================================================================== */

impl MeteringChip for RN8615V2 {
    fn init(&mut self, params: &CalibrationParams) -> Result<(), MeteringError> {
        self.reset()?;

        // 等待上电稳定
        let mut delay = 0;
        while delay < 100_000 {
            delay += 1;
            core::hint::spin_loop();
        }

        // 验证 Chip ID
        let id = self.chip_id()?;
        if (id & 0xFFFF_F000) != (CHIP_ID_VALUE & 0xFFFF_F000) {
            return Err(MeteringError::SpiError);
        }

        // 写入校准参数
        for (i, &gain) in params.voltage_gain.iter().enumerate() {
            spi_write(self.spi, reg::UGAIN_A + i as u8, (gain * 65536.0) as u32)?;
        }
        for (i, &gain) in params.current_gain.iter().enumerate() {
            spi_write(self.spi, reg::IGAIN_A + i as u8, (gain * 65536.0) as u32)?;
        }
        for (i, &gain) in params.power_gain.iter().enumerate() {
            spi_write(self.spi, reg::PGAIN_A + i as u8, (gain * 65536.0) as u32)?;
        }
        for (i, &gain) in params.reactive_gain.iter().enumerate() {
            spi_write(self.spi, reg::QGAIN_A + i as u8, (gain * 65536.0) as u32)?;
        }
        for (i, &angle) in params.phase_angle.iter().enumerate() {
            spi_write(self.spi, reg::PHCAL_A + i as u8, (angle * 100.0) as u32)?;
        }

        // 使能电网质量监测
        spi_write(self.spi, reg::PQ_CTRL, 0x0000_007F)?;

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
        let ua_raw = spi_read(self.spi, reg::UA_RMS)?;
        let ub_raw = spi_read(self.spi, reg::UB_RMS)?;
        let uc_raw = spi_read(self.spi, reg::UC_RMS)?;
        let ia_raw = spi_read(self.spi, reg::IA_RMS)?;
        let ib_raw = spi_read(self.spi, reg::IB_RMS)?;
        let ic_raw = spi_read(self.spi, reg::IC_RMS)?;

        let pa_raw = spi_read(self.spi, reg::PA)?;
        let pb_raw = spi_read(self.spi, reg::PB)?;
        let pc_raw = spi_read(self.spi, reg::PC)?;
        let pt_raw = spi_read(self.spi, reg::PT)?;

        let qa_raw = spi_read(self.spi, reg::QA)?;
        let qb_raw = spi_read(self.spi, reg::QB)?;
        let qc_raw = spi_read(self.spi, reg::QC)?;
        let qt_raw = spi_read(self.spi, reg::QT)?;

        let freq_raw = spi_read(self.spi, reg::FREQ)?;

        let pfa_raw = spi_read(self.spi, reg::PFA)?;
        let pfb_raw = spi_read(self.spi, reg::PFB)?;
        let pfc_raw = spi_read(self.spi, reg::PFC)?;
        let pft_raw = spi_read(self.spi, reg::PFT)?;

        let pf_convert = |raw: u32| -> u16 {
            let val = signed24(raw) as f32;
            ((val.abs() / 32768.0 * 1000.0) as u16).min(1000)
        };

        // 检查电网质量事件
        self.pq_event_pending = self.check_sag_swell();

        Ok(PhaseData {
            voltage_a: (ua_raw as f32 * self.u_coeff[0] * 100.0) as u16,
            voltage_b: (ub_raw as f32 * self.u_coeff[1] * 100.0) as u16,
            voltage_c: (uc_raw as f32 * self.u_coeff[2] * 100.0) as u16,
            current_a: (ia_raw as f32 * self.i_coeff[0] * 1000.0) as u16,
            current_b: (ib_raw as f32 * self.i_coeff[1] * 1000.0) as u16,
            current_c: (ic_raw as f32 * self.i_coeff[2] * 1000.0) as u16,
            active_power_a: signed24(pa_raw),
            active_power_b: signed24(pb_raw),
            active_power_c: signed24(pc_raw),
            active_power_total: signed24(pt_raw),
            reactive_power_a: signed24(qa_raw),
            reactive_power_b: signed24(qb_raw),
            reactive_power_c: signed24(qc_raw),
            reactive_power_total: signed24(qt_raw),
            frequency: if freq_raw > 0 {
                (8192000.0 / (2.0 * freq_raw as f32) * 100.0) as u16
            } else { 5000 },
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

        // V2 支持分相电能!
        let active_import_a = spi_read(self.spi, reg::ACTIVE_IMPORT_A)?;
        let active_import_b = spi_read(self.spi, reg::ACTIVE_IMPORT_B)?;
        let active_import_c = spi_read(self.spi, reg::ACTIVE_IMPORT_C)?;

        let e_coeff = 0.001; // RN8615V2 更高精度

        Ok(EnergyData {
            active_import: (active_import as f64 * e_coeff * 100.0) as u64,
            active_export: (active_export as f64 * e_coeff * 100.0) as u64,
            reactive_import: (reactive_import as f64 * e_coeff * 100.0) as u64,
            reactive_export: (reactive_export as f64 * e_coeff * 100.0) as u64,
            active_import_a: (active_import_a as f64 * e_coeff * 100.0) as u64,
            active_import_b: (active_import_b as f64 * e_coeff * 100.0) as u64,
            active_import_c: (active_import_c as f64 * e_coeff * 100.0) as u64,
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
        "RN8615V2"
    }

    fn supports_fundamental(&self) -> bool {
        true
    }

    fn read_fundamental_power(&mut self) -> Result<[i32; 3], MeteringError> {
        let pa = signed24(spi_read(self.spi, reg::PA_FUND)?);
        let pb = signed24(spi_read(self.spi, reg::PB_FUND)?);
        let pc = signed24(spi_read(self.spi, reg::PC_FUND)?);
        Ok([pa, pb, pc])
    }
}

/* ================================================================== */
/*  实现 HarmonicAnalysis trait                                         */
/* ================================================================== */

impl HarmonicAnalysis for RN8615V2 {
    fn read_harmonics(&mut self, phase: Phase) -> Result<HarmonicData, MeteringError> {
        self.read_full_harmonics(phase)
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
        63
    }
}

/* ================================================================== */
/*  实现 PowerQuality trait (V2独有)                                    */
/* ================================================================== */

impl PowerQuality for RN8615V2 {
    fn read_voltage_unbalance(&mut self) -> Result<u16, MeteringError> {
        let raw = spi_read(self.spi, reg::VOLTAGE_UNBALANCE)?;
        Ok((raw & 0xFFFF) as u16) // 0.01%
    }

    fn read_current_unbalance(&mut self) -> Result<u16, MeteringError> {
        let raw = spi_read(self.spi, reg::CURRENT_UNBALANCE)?;
        Ok((raw & 0xFFFF) as u16)
    }

    fn read_short_flicker(&mut self) -> Result<u16, MeteringError> {
        let raw = spi_read(self.spi, reg::FLICKER_PST)?;
        Ok((raw & 0xFFFF) as u16)
    }

    fn read_long_flicker(&mut self) -> Result<u16, MeteringError> {
        let raw = spi_read(self.spi, reg::FLICKER_PLT)?;
        Ok((raw & 0xFFFF) as u16)
    }

    fn read_interharmonic(&mut self, phase: Phase, order: u8) -> Result<u16, MeteringError> {
        self.read_interharmonic_raw(phase, order)
    }

    fn read_dc_component(&mut self, phase: Phase) -> Result<u16, MeteringError> {
        let reg = match phase {
            Phase::A => reg::DC_COMPONENT_UA,
            Phase::B => reg::DC_COMPONENT_UB,
            Phase::C => reg::DC_COMPONENT_UC,
        };
        let raw = spi_read(self.spi, reg)?;
        Ok((raw & 0xFFFF) as u16)
    }

    fn check_pq_event(&mut self) -> Option<PowerQualityEvent> {
        self.pq_event_pending.take()
    }
}

/* ================================================================== */
/*  单元测试                                                           */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc8_v2() {
        assert_eq!(crc8(&[0x00]), 0x00);
    }

    #[test]
    fn test_signed24_v2() {
        assert_eq!(signed24(0x000000), 0);
        assert_eq!(signed24(0x7FFFFF), 8388607);
        assert_eq!(signed24(0x800000), -8388608);
        assert_eq!(signed24(0xFFFFFF), -1);
    }

    #[test]
    fn test_reg_v2_addresses() {
        assert_eq!(reg::CHIP_ID, 0x00);
        assert_eq!(reg::PA_FUND, 0x1C);
        assert_eq!(reg::ACTIVE_IMPORT_A, 0x44);
        assert_eq!(reg::INTERHARMONIC_UA, 0x58);
        assert_eq!(reg::VOLTAGE_UNBALANCE, 0x70);
        assert_eq!(reg::FLICKER_PST, 0x72);
        assert_eq!(reg::DC_COMPONENT_UA, 0x74);
        assert_eq!(reg::SAG_SWELL_STATUS, 0x78);
        assert_eq!(reg::ANTI_TAMPER_STAT, 0x80);
        assert_eq!(reg::UGAIN_A, 0x90);
    }
}
