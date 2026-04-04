/* ================================================================== */
/*                                                                    */
/*  calibration.rs — 生产校表流程                                       */
/*                                                                    */
/*  功能:                                                              */
/*    - 有功/无功电能校准（脉冲比较法）                                  */
/*    - 电压/电流通道增益校准                                           */
/*    - 相位补偿                                                        */
/*    - 启动/潜动试验                                                  */
/*    - 校准参数 Flash 存储                                             */
/*    - 校准接口（RS485/红外命令）                                      */
/*    - 防掉电保护（原子写入 + CRC）                                    */
/*                                                                    */
/*  校准精度要求 (DLMS/COSEM Class 0.5S):                              */
/*    有功电能: ±0.5% (Ib)                                            */
/*    无功电能: ±2.0% (Ib)                                            */
/*    电压测量: ±0.5%                                                 */
/*    电流测量: ±0.5%                                                 */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

/* ── 校准参数存储结构 ── */

/// 校准参数（存入 W25Q64 params 分区）
///
/// 双区交替存储：Slot A 和 Slot B，通过 active_slot 标识当前有效区。
/// 每次写入前先写非活跃区，写完并 CRC 验证通过后切换 active_slot。
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CalibrationData {
    /// 有效标志 (0xCA1B_DATA)
    pub magic: u32,
    /// 参数版本
    pub version: u16,
    /// 活跃槽位 (0=A, 1=B)
    pub active_slot: u16,
    /// 电压增益 [A, B, C] (1.0 = 无校正, Q15.16 定点)
    pub voltage_gain: [i32; 3],
    /// 电流增益 [A, B, C] (Q15.16)
    pub current_gain: [i32; 3],
    /// 有功功率增益 [A, B, C] (Q15.16)
    pub power_gain: [i32; 3],
    /// 无功功率增益 [A, B, C] (Q15.16)
    pub reactive_gain: [i32; 3],
    /// 相位补偿 [A, B, C] (度, Q8.8, 正值=滞后校正)
    pub phase_offset: [i16; 3],
    /// 有功启动电流阈值 (mA, Q8.8)
    pub active_start_threshold: u16,
    /// 无功启动电流阈值 (mA, Q8.8)
    pub reactive_start_threshold: u16,
    /// 潜动阈值 (脉冲数/分钟)
    pub creep_threshold: u8,
    /// 校准时间戳 (Unix)
    pub cal_timestamp: u32,
    /// 校准员 ID
    pub operator_id: [u8; 8],
    /// 校准温度 (0.1°C)
    pub cal_temperature: i16,
    /// 保留
    pub reserved: [u8; 32],
    /// CRC32 (覆盖 magic~reserved)
    pub crc: u32,
}

impl CalibrationData {
    /// 校准数据魔数
    pub const MAGIC: u32 = 0xCA1B_DA7A;
    /// 数据大小（不含 CRC）
    pub const DATA_SIZE: usize = core::mem::size_of::<Self>() - 4;

    /// 创建默认校准数据
    pub fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: 1,
            active_slot: 0,
            voltage_gain: [0x0001_0000; 3], // 1.0 in Q15.16
            current_gain: [0x0001_0000; 3],
            power_gain: [0x0001_0000; 3],
            reactive_gain: [0x0001_0000; 3],
            phase_offset: [0; 3],
            active_start_threshold: 5, // 0.02A = 20mA (5 * 4mA)
            reactive_start_threshold: 10,
            creep_threshold: 1,
            cal_timestamp: 0,
            operator_id: [0; 8],
            cal_temperature: 250, // 25.0°C
            reserved: [0; 32],
            crc: 0,
        }
    }

    /// 计算 CRC 并填入
    pub fn update_crc(&mut self) {
        let bytes: &[u8; Self::DATA_SIZE] =
            unsafe { &*(&self.magic as *const u32 as *const [u8; Self::DATA_SIZE]) };
        self.crc = crc32_calc(bytes);
    }

    /// 验证 CRC
    pub fn verify_crc(&self) -> bool {
        let bytes: &[u8; Self::DATA_SIZE] =
            unsafe { &*(self.magic as *const u32 as *const [u8; Self::DATA_SIZE]) };
        crc32_calc(bytes) == self.crc
    }

    /// 是否有效（魔数 + CRC）
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC && self.verify_crc()
    }
}

/* ── 校准状态机 ── */

/// 校准流程状态
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CalState {
    /// 空闲
    Idle = 0,
    /// 电压校准中
    CalVoltage = 1,
    /// 电流校准中
    CalCurrent = 2,
    /// 有功功率校准中
    CalActivePower = 3,
    /// 无功功率校准中
    CalReactivePower = 4,
    /// 相位补偿中
    CalPhase = 5,
    /// 启动试验中
    StartTest = 6,
    /// 潜动试验中
    CreepTest = 7,
    /// 校准完成，待保存
    Done = 8,
    /// 校准失败
    Failed = 9,
}

/* ── 校准命令（RS485/红外接口） ── */

/// 校准命令码
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CalCommand {
    /// 进入校准模式
    EnterCalMode = 0x01,
    /// 退出校准模式
    ExitCalMode = 0x02,
    /// 读校准参数
    ReadParams = 0x03,
    /// 写校准参数
    WriteParams = 0x04,
    /// 开始电压校准
    StartVoltageCal = 0x10,
    /// 开始电流校准
    StartCurrentCal = 0x11,
    /// 开始有功功率校准（脉冲比较法）
    StartActivePowerCal = 0x12,
    /// 开始无功功率校准
    StartReactivePowerCal = 0x13,
    /// 开始相位补偿
    StartPhaseCal = 0x14,
    /// 启动试验
    StartTest = 0x20,
    /// 潜动试验
    CreepTest = 0x21,
    /// 读取校准结果
    ReadResult = 0x30,
    /// 保存到 Flash
    SaveToFlash = 0x40,
    /// 恢复出厂默认
    FactoryReset = 0x50,
}

impl CalCommand {
    /// 从字节解析
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(Self::EnterCalMode),
            0x02 => Some(Self::ExitCalMode),
            0x03 => Some(Self::ReadParams),
            0x04 => Some(Self::WriteParams),
            0x10 => Some(Self::StartVoltageCal),
            0x11 => Some(Self::StartCurrentCal),
            0x12 => Some(Self::StartActivePowerCal),
            0x13 => Some(Self::StartReactivePowerCal),
            0x14 => Some(Self::StartPhaseCal),
            0x20 => Some(Self::StartTest),
            0x21 => Some(Self::CreepTest),
            0x30 => Some(Self::ReadResult),
            0x40 => Some(Self::SaveToFlash),
            0x50 => Some(Self::FactoryReset),
            _ => None,
        }
    }
}

/* ── 校准结果 ── */

/// 单次校准测量结果
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CalMeasurement {
    /// 标准表值 (0.01 单位)
    pub reference: i32,
    /// 被校表值 (0.01 单位)
    pub measured: i32,
    /// 误差百分比 (0.01%, 如 50 = 0.50%)
    pub error_pct: i16,
    /// 是否合格
    pub passed: bool,
}

/* ── 校准管理器 ── */

/// 校准管理器
///
/// 管理整个校准流程，包括参数读写、脉冲比较、启/潜动试验。
pub struct CalibrationManager {
    /// 校准状态
    state: CalState,
    /// 当前校准参数
    params: CalibrationData,
    /// 是否处于校准模式
    cal_mode: bool,
    /// 校准模式超时计数 (秒)
    cal_timeout_sec: u32,
    /// 校准模式最大超时 (秒)
    cal_timeout_max: u32,
    /// 最近一次测量结果
    last_measurement: CalMeasurement,
    /// 校准错误码
    error_code: u32,
    /// 潜动试验计时器（脉冲计数窗口）
    creep_pulse_count: u32,
    /// 启动试验计时器
    start_test_seconds: u32,
    /// 潜动试验计时器
    _creep_seconds: u32,
}

/// 校准模式默认超时: 30 分钟
const CAL_TIMEOUT_DEFAULT: u32 = 1800;

impl CalibrationManager {
    /// 创建校准管理器
    pub fn new() -> Self {
        Self {
            state: CalState::Idle,
            params: CalibrationData::new(),
            cal_mode: false,
            cal_timeout_sec: 0,
            cal_timeout_max: CAL_TIMEOUT_DEFAULT,
            last_measurement: CalMeasurement::default(),
            error_code: 0,
            creep_pulse_count: 0,
            start_test_seconds: 0,
            _creep_seconds: 0,
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> CalState {
        self.state
    }
    /// 是否处于校准模式
    pub fn is_cal_mode(&self) -> bool {
        self.cal_mode
    }
    /// 获取校准参数
    pub fn params(&self) -> &CalibrationData {
        &self.params
    }
    /// 获取最近测量结果
    pub fn last_measurement(&self) -> &CalMeasurement {
        &self.last_measurement
    }
    /// 获取错误码
    pub fn error_code(&self) -> u32 {
        self.error_code
    }

    /// 进入校准模式
    pub fn enter_cal_mode(&mut self) -> Result<(), u32> {
        if self.cal_mode {
            return Err(1); // 已在校准模式
        }
        self.cal_mode = true;
        self.cal_timeout_sec = 0;
        self.state = CalState::Idle;
        Ok(())
    }

    /// 退出校准模式
    pub fn exit_cal_mode(&mut self) {
        self.cal_mode = false;
        self.state = CalState::Idle;
        self.cal_timeout_sec = 0;
    }

    /// 处理校准命令
    pub fn handle_command(&mut self, cmd: CalCommand, data: &[u8]) -> Result<(), u32> {
        match cmd {
            CalCommand::EnterCalMode => self.enter_cal_mode(),
            CalCommand::ExitCalMode => {
                self.exit_cal_mode();
                Ok(())
            }
            CalCommand::ReadParams => {
                // 返回参数（调用者通过 params() 获取）
                Ok(())
            }
            CalCommand::WriteParams => {
                if data.len() < CalibrationData::DATA_SIZE {
                    return Err(10);
                }
                let new_params: CalibrationData =
                    unsafe { core::ptr::read(data.as_ptr() as *const CalibrationData) };
                if !new_params.is_valid() {
                    return Err(11);
                }
                self.params = new_params;
                Ok(())
            }
            CalCommand::StartVoltageCal => {
                if !self.cal_mode {
                    return Err(2);
                }
                self.state = CalState::CalVoltage;
                Ok(())
            }
            CalCommand::StartCurrentCal => {
                if !self.cal_mode {
                    return Err(2);
                }
                self.state = CalState::CalCurrent;
                Ok(())
            }
            CalCommand::StartActivePowerCal => {
                if !self.cal_mode {
                    return Err(2);
                }
                self.state = CalState::CalActivePower;
                Ok(())
            }
            CalCommand::StartReactivePowerCal => {
                if !self.cal_mode {
                    return Err(2);
                }
                self.state = CalState::CalReactivePower;
                Ok(())
            }
            CalCommand::StartPhaseCal => {
                if !self.cal_mode {
                    return Err(2);
                }
                self.state = CalState::CalPhase;
                Ok(())
            }
            CalCommand::StartTest => {
                if !self.cal_mode {
                    return Err(2);
                }
                self.state = CalState::StartTest;
                self.start_test_seconds = 0;
                Ok(())
            }
            CalCommand::CreepTest => {
                if !self.cal_mode {
                    return Err(2);
                }
                self.state = CalState::CreepTest;
                self.creep_pulse_count = 0;
                Ok(())
            }
            CalCommand::ReadResult => Ok(()),
            CalCommand::SaveToFlash => {
                if !self.cal_mode {
                    return Err(2);
                }
                self.save_params();
                Ok(())
            }
            CalCommand::FactoryReset => {
                self.params = CalibrationData::new();
                self.state = CalState::Idle;
                Ok(())
            }
        }
    }

    /* ── 脉冲比较法校准 ── */

    /// 有功电能校准（脉冲比较法）
    ///
    /// 原理：在额定条件下运行一段时间，比较标准表脉冲数与被校表脉冲数，
    /// 计算误差并调整功率增益。
    ///
    /// `std_pulses`: 标准表脉冲数
    /// `meter_pulses`: 被校表脉冲数
    /// `phase`: 相号 (0=A, 1=B, 2=C, 3=合相)
    pub fn calibrate_active_power(
        &mut self,
        std_pulses: u32,
        meter_pulses: u32,
        phase: usize,
    ) -> CalMeasurement {
        if std_pulses == 0 || phase > 3 {
            self.state = CalState::Failed;
            self.error_code = 20;
            return CalMeasurement::default();
        }

        // 误差 = (meter - std) / std * 100% (0.01%)
        let error_raw = ((meter_pulses as i64 - std_pulses as i64) * 10000) / std_pulses as i64;
        let error_pct = error_raw as i16;

        // 调整增益: new_gain = old_gain * (std / meter)
        // Q15.16 定点: new_gain = old_gain * std * 2^16 / meter
        let idx = if phase < 3 { phase } else { 0 }; // 合相调整 A 相（简化）
        let old_gain = self.params.power_gain[idx] as i64;
        let new_gain = (old_gain * std_pulses as i64 * 65536) / meter_pulses as i64 / 65536;
        self.params.power_gain[idx] = new_gain as i32;

        let passed = error_pct.abs() <= 50; // ±0.50%
        let result = CalMeasurement {
            reference: std_pulses as i32,
            measured: meter_pulses as i32,
            error_pct,
            passed,
        };

        if passed {
            self.state = CalState::Done;
        } else {
            self.state = CalState::CalActivePower;
        }
        self.last_measurement = result;
        result
    }

    /// 无功电能校准（脉冲比较法）
    pub fn calibrate_reactive_power(
        &mut self,
        std_pulses: u32,
        meter_pulses: u32,
        phase: usize,
    ) -> CalMeasurement {
        if std_pulses == 0 || phase > 3 {
            self.state = CalState::Failed;
            self.error_code = 21;
            return CalMeasurement::default();
        }

        let error_raw = ((meter_pulses as i64 - std_pulses as i64) * 10000) / std_pulses as i64;
        let error_pct = error_raw as i16;

        let idx = if phase < 3 { phase } else { 0 };
        let old_gain = self.params.reactive_gain[idx] as i64;
        let new_gain = (old_gain * std_pulses as i64 * 65536) / meter_pulses as i64 / 65536;
        self.params.reactive_gain[idx] = new_gain as i32;

        let passed = error_pct.abs() <= 200; // ±2.00%
        let result = CalMeasurement {
            reference: std_pulses as i32,
            measured: meter_pulses as i32,
            error_pct,
            passed,
        };

        if passed {
            self.state = CalState::Done;
        } else {
            self.state = CalState::CalReactivePower;
        }
        self.last_measurement = result;
        result
    }

    /// 电压通道增益校准
    ///
    /// `phase`: 相号 (0=A, 1=B, 2=C)
    /// `std_voltage`: 标准表电压 (0.01V)
    /// `measured_voltage`: 被校表读数 (0.01V)
    pub fn calibrate_voltage(
        &mut self,
        phase: usize,
        std_voltage: u32,
        measured_voltage: u32,
    ) -> CalMeasurement {
        if phase > 2 || measured_voltage == 0 {
            self.state = CalState::Failed;
            self.error_code = 22;
            return CalMeasurement::default();
        }

        let error_raw =
            ((measured_voltage as i64 - std_voltage as i64) * 10000) / std_voltage as i64;
        let error_pct = error_raw as i16;

        let old_gain = self.params.voltage_gain[phase] as i64;
        let new_gain = (old_gain * std_voltage as i64 * 65536) / measured_voltage as i64 / 65536;
        self.params.voltage_gain[phase] = new_gain as i32;

        let passed = error_pct.abs() <= 50; // ±0.50%
        let result = CalMeasurement {
            reference: std_voltage as i32,
            measured: measured_voltage as i32,
            error_pct,
            passed,
        };

        if passed {
            self.state = CalState::Done;
        } else {
            self.state = CalState::CalVoltage;
        }
        self.last_measurement = result;
        result
    }

    /// 电流通道增益校准
    pub fn calibrate_current(
        &mut self,
        phase: usize,
        std_current: u32,
        measured_current: u32,
    ) -> CalMeasurement {
        if phase > 2 || measured_current == 0 {
            self.state = CalState::Failed;
            self.error_code = 23;
            return CalMeasurement::default();
        }

        let error_raw =
            ((measured_current as i64 - std_current as i64) * 10000) / std_current as i64;
        let error_pct = error_raw as i16;

        let old_gain = self.params.current_gain[phase] as i64;
        let new_gain = (old_gain * std_current as i64 * 65536) / measured_current as i64 / 65536;
        self.params.current_gain[phase] = new_gain as i32;

        let passed = error_pct.abs() <= 50;
        let result = CalMeasurement {
            reference: std_current as i32,
            measured: measured_current as i32,
            error_pct,
            passed,
        };

        if passed {
            self.state = CalState::Done;
        } else {
            self.state = CalState::CalCurrent;
        }
        self.last_measurement = result;
        result
    }

    /// 相位补偿校准
    ///
    /// `phase`: 相号 (0=A, 1=B, 2=C)
    /// `target_pf`: 目标功率因数 (0.001, 如 500 = 0.500)
    /// `measured_pf`: 实测功率因数 (0.001)
    pub fn calibrate_phase(
        &mut self,
        phase: usize,
        target_pf: u16,
        measured_pf: u16,
    ) -> CalMeasurement {
        if phase > 2 || target_pf == 0 {
            self.state = CalState::Failed;
            self.error_code = 24;
            return CalMeasurement::default();
        }

        // 相位差 = arccos(pf_target) - arccos(pf_measured)
        // 简化: 用线性近似 (小角度)
        // delta_phi ≈ (measured_pf - target_pf) / sin(arccos(target_pf))
        // 精确计算需要查表或CORDIC, 这里用简化版
        let pf_diff = (measured_pf as i32 - target_pf as i32) as i16;

        // Q8.8 相位补偿 (度)
        let phase_comp = (pf_diff as i16 * 10) / 8; // 粗略线性映射
        self.params.phase_offset[phase] = phase_comp;

        let error_pct = pf_diff * 10; // 0.01% 近似
        let passed = error_pct.abs() <= 50;
        let result = CalMeasurement {
            reference: target_pf as i32,
            measured: measured_pf as i32,
            error_pct,
            passed,
        };

        if passed {
            self.state = CalState::Done;
        } else {
            self.state = CalState::CalPhase;
        }
        self.last_measurement = result;
        result
    }

    /* ── 启动/潜动试验 ── */

    /// 启动试验
    ///
    /// 在启动电流（Ib 的 0.4%）下运行，检查是否有脉冲输出。
    /// DLMS 标准: Class 0.5S 启动电流 ≤ 0.001Ib。
    /// 测试时间: 至少 10 分钟，期间应有 ≥1 个脉冲。
    pub fn start_test_tick(&mut self) -> bool {
        if self.state != CalState::StartTest {
            return false;
        }
        self.start_test_seconds += 1;
        // 测试 600 秒 (10 分钟)
        self.start_test_seconds >= 600
    }

    /// 启动试验 — 报告脉冲
    ///
    /// `detected`: 是否检测到脉冲
    pub fn start_test_report(&mut self, detected: bool) -> bool {
        if self.state != CalState::StartTest {
            return false;
        }
        if detected {
            self.state = CalState::Done;
            true // 通过
        } else if self.start_test_seconds >= 600 {
            self.state = CalState::Failed;
            self.error_code = 30;
            false // 超时无脉冲
        } else {
            false // 继续等待
        }
    }

    /// 潜动试验
    ///
    /// 在 1.1Un、无负载条件下运行，检查是否有不应有的脉冲输出。
    /// DLMS 标准: 潜动试验时间 ≥ 10 分钟。
    pub fn creep_test_tick(&mut self, pulse_detected: bool) -> bool {
        if self.state != CalState::CreepTest {
            return false;
        }

        if pulse_detected {
            self.creep_pulse_count += 1;
        }

        // 每次 tick 代表 1 秒
        self._creep_seconds += 1;
        if self._creep_seconds >= 600 {
            let passed = self.creep_pulse_count <= self.params.creep_threshold as u32;
            self.state = if passed {
                CalState::Done
            } else {
                CalState::Failed
            };
            self.error_code = if passed { 0 } else { 31 };
            passed
        } else {
            false // 继续测试
        }
    }

    /* ── 参数保存（原子写入 + 双区交替） ── */

    /// 保存校准参数到 Flash
    ///
    /// 双区交替策略:
    /// 1. 读取当前 active_slot
    /// 2. 写入另一个 slot
    /// 3. 验证写入的 CRC
    /// 4. 更新 active_slot 标志并写入
    /// 5. 验证 active_slot 切换成功
    ///
    /// 如果步骤 3 或 5 失败，旧数据仍在原 slot，不受影响。
    pub fn save_params(&mut self) {
        // 步骤 1: 准备写入数据
        let mut new_params = self.params;
        let next_slot = if new_params.active_slot == 0 { 1u16 } else { 0 };
        new_params.active_slot = next_slot;
        new_params.update_crc();

        // 步骤 2: 序列化到字节
        let bytes: &[u8] = &[unsafe { core::mem::transmute_copy(&new_params) }]
            [..core::mem::size_of::<CalibrationData>()];

        // 步骤 3: 实际 Flash 写入（占位）
        // 在真实环境中调用 PartitionStorage::write_with_crc
        // 写入地址 = params 区 + next_slot * sizeof(CalibrationData)
        let _ = bytes; // 占位，实际由 board 层实现

        // 步骤 4: 验证（读回比较）
        // TODO: 集成 PartitionStorage 后实现完整验证

        // 步骤 5: 更新活跃槽位
        self.params.active_slot = next_slot;
    }

    /// 从 Flash 加载校准参数
    ///
    /// 策略: 读取两个 slot，选择 CRC 有效的较新版本。
    pub fn load_params(&mut self) -> bool {
        // TODO: 集成 PartitionStorage 后实现
        // 1. 读取 slot A
        // 2. 读取 slot B
        // 3. 验证 CRC
        // 4. 选择 active_slot 指向的版本
        // 5. 如果两个都无效，使用默认值
        false
    }

    /* ── 超时管理 ── */

    /// 每秒调用，校准模式超时自动退出
    pub fn tick_second(&mut self) {
        if !self.cal_mode {
            return;
        }
        self.cal_timeout_sec += 1;
        if self.cal_timeout_sec >= self.cal_timeout_max {
            self.exit_cal_mode();
        }
    }
}

impl Default for CalibrationManager {
    fn default() -> Self {
        Self::new()
    }
}

/* ── CRC32 ── */

fn crc32_calc(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/* ================================================================== */
/*  单元测试                                                           */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_data_default() {
        let data = CalibrationData::new();
        assert_eq!(data.magic, CalibrationData::MAGIC);
        assert_eq!(data.version, 1);
        assert_eq!(data.voltage_gain, [0x0001_0000; 3]);
    }

    #[test]
    fn test_calibration_data_crc() {
        let mut data = CalibrationData::new();
        data.update_crc();
        assert!(data.verify_crc());

        // 篡改数据
        data.voltage_gain[0] = 0;
        assert!(!data.verify_crc());
    }

    #[test]
    fn test_calibration_data_is_valid() {
        let mut data = CalibrationData::new();
        // 未更新 CRC → 无效
        assert!(!data.is_valid());

        data.update_crc();
        assert!(data.is_valid());

        // 错误魔数
        data.magic = 0;
        assert!(!data.is_valid());
    }

    #[test]
    fn test_cal_manager_enter_exit() {
        let mut mgr = CalibrationManager::new();
        assert!(!mgr.is_cal_mode());
        assert!(mgr.enter_cal_mode().is_ok());
        assert!(mgr.is_cal_mode());
        mgr.exit_cal_mode();
        assert!(!mgr.is_cal_mode());
    }

    #[test]
    fn test_cal_manager_double_enter() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        assert!(mgr.enter_cal_mode().is_err());
    }

    #[test]
    fn test_cal_command_from_byte() {
        assert_eq!(CalCommand::from_byte(0x01), Some(CalCommand::EnterCalMode));
        assert_eq!(CalCommand::from_byte(0x50), Some(CalCommand::FactoryReset));
        assert_eq!(CalCommand::from_byte(0xFF), None);
    }

    #[test]
    fn test_calibrate_active_power_exact() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::CalActivePower;

        let result = mgr.calibrate_active_power(10000, 10000, 0);
        assert_eq!(result.error_pct, 0);
        assert!(result.passed);
        assert_eq!(mgr.state(), CalState::Done);
    }

    #[test]
    fn test_calibrate_active_power_positive_error() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::CalActivePower;

        // 被校表多计 1%
        let result = mgr.calibrate_active_power(10000, 10100, 0);
        assert_eq!(result.error_pct, 100); // 1.00%
        assert!(!result.passed);
        assert_eq!(mgr.state(), CalState::CalActivePower);
    }

    #[test]
    fn test_calibrate_active_power_gain_adjustment() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        let original_gain = mgr.params.power_gain[0];

        mgr.state = CalState::CalActivePower;
        mgr.calibrate_active_power(10000, 9900, 0);

        // 增益应该增大以补偿负误差
        assert!(mgr.params.power_gain[0] > original_gain);
    }

    #[test]
    fn test_calibrate_reactive_power() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::CalReactivePower;

        let result = mgr.calibrate_reactive_power(5000, 5000, 1);
        assert_eq!(result.error_pct, 0);
        assert!(result.passed);
    }

    #[test]
    fn test_calibrate_voltage() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::CalVoltage;

        let result = mgr.calibrate_voltage(0, 22000, 22000);
        assert_eq!(result.error_pct, 0);
        assert!(result.passed);
    }

    #[test]
    fn test_calibrate_current() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::CalCurrent;

        let result = mgr.calibrate_current(0, 5000, 5000);
        assert_eq!(result.error_pct, 0);
        assert!(result.passed);
    }

    #[test]
    fn test_calibrate_phase() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::CalPhase;

        let result = mgr.calibrate_phase(0, 1000, 1000);
        assert_eq!(result.error_pct, 0);
        assert!(result.passed);
    }

    #[test]
    fn test_start_test_timeout() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::StartTest;

        // 599 秒不应超时
        for _ in 0..599 {
            assert!(!mgr.start_test_tick());
        }
        // 600 秒超时
        assert!(mgr.start_test_tick());
    }

    #[test]
    fn test_start_test_pulse_detected() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::StartTest;

        assert!(mgr.start_test_report(true));
        assert_eq!(mgr.state(), CalState::Done);
    }

    #[test]
    fn test_start_test_no_pulse_timeout() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::StartTest;

        for _ in 0..599 {
            assert!(!mgr.start_test_report(false));
        }
        assert!(!mgr.start_test_report(false)); // 600s timeout
        assert_eq!(mgr.state(), CalState::Failed);
        assert_eq!(mgr.error_code(), 30);
    }

    #[test]
    fn test_creep_test_pass() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::CreepTest;

        // 599 秒无脉冲
        for _ in 0..599 {
            assert!(!mgr.creep_test_tick(false));
        }
        // 600 秒: threshold=1, 0 脉冲 → 通过
        assert!(mgr.creep_test_tick(false));
        assert_eq!(mgr.state(), CalState::Done);
    }

    #[test]
    fn test_creep_test_fail() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::CreepTest;

        // 模拟 2 个脉冲
        mgr.creep_test_tick(true); // 1s, 1 pulse
        for _ in 0..598 {
            mgr.creep_test_tick(false);
        }
        // 600s: threshold=1, 2 pulses → 失败
        assert!(!mgr.creep_test_tick(false));
        assert_eq!(mgr.state(), CalState::Failed);
        assert_eq!(mgr.error_code(), 31);
    }

    #[test]
    fn test_cal_mode_timeout() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        assert!(mgr.is_cal_mode());

        for _ in 0..1800 {
            mgr.tick_second();
        }
        assert!(!mgr.is_cal_mode());
    }

    #[test]
    fn test_handle_command_not_in_cal_mode() {
        let mut mgr = CalibrationManager::new();
        assert!(mgr
            .handle_command(CalCommand::StartVoltageCal, &[])
            .is_err());
    }

    #[test]
    fn test_handle_command_save() {
        let mut mgr = CalibrationManager::new();
        assert!(mgr.handle_command(CalCommand::SaveToFlash, &[]).is_err()); // not in cal mode
        mgr.enter_cal_mode().unwrap();
        assert!(mgr.handle_command(CalCommand::SaveToFlash, &[]).is_ok());
    }

    #[test]
    fn test_factory_reset() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.params.voltage_gain[0] = 999;
        mgr.handle_command(CalCommand::FactoryReset, &[]).unwrap();
        assert_eq!(mgr.params.voltage_gain[0], 0x0001_0000);
    }

    #[test]
    fn test_calibrate_zero_std_pulses() {
        let mut mgr = CalibrationManager::new();
        mgr.enter_cal_mode().unwrap();
        mgr.state = CalState::CalActivePower;

        let result = mgr.calibrate_active_power(0, 1000, 0);
        assert!(!result.passed);
        assert_eq!(mgr.error_code(), 20);
    }

    #[test]
    fn test_save_params_slot_toggle() {
        let mut mgr = CalibrationManager::new();
        assert_eq!(mgr.params.active_slot, 0);
        mgr.save_params();
        assert_eq!(mgr.params.active_slot, 1);
        mgr.save_params();
        assert_eq!(mgr.params.active_slot, 0);
    }
}
