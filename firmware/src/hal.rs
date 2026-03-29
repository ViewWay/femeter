/* ================================================================== */
/*                                                                    */
/*  hal.rs — FeMeter 硬件抽象层 (Hardware Abstraction Layer)          */
/*                                                                    */
/*  所有硬件接口通过 trait 抽象，应用层只依赖 trait，不依赖具体实现。    */
/*  编译时通过 cargo feature 选择物料组合，零运行时开销。               */
/*                                                                    */
/*  支持的物料组合:                                                    */
/*    计量: ATT7022E / RN8302B / RN8615V2                             */
/*    蜂窝: EC800N (Cat.1) / BC260Y (NB-IoT)                         */
/*    红外: 对管 / 模块                                               */
/*    隔离: 数字隔离 / 光耦                                            */
/*    Flash: 内部 / W25Q64                                            */
/*    电池: ER26500 / ER17335                                         */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

/// 计量芯片三相数据 (工程单位)
#[derive(Clone, Copy, Default, Debug)]
pub struct PhaseData {
    /// A 相电压 RMS (0.01V)
    pub voltage_a: u16,
    /// B 相电压 RMS (0.01V)
    pub voltage_b: u16,
    /// C 相电压 RMS (0.01V)
    pub voltage_c: u16,
    /// A 相电流 RMS (mA)
    pub current_a: u16,
    /// B 相电流 RMS (mA)
    pub current_b: u16,
    /// C 相电流 RMS (mA)
    pub current_c: u16,
    /// A 相有功功率 (W, signed)
    pub active_power_a: i32,
    /// B 相有功功率 (W, signed)
    pub active_power_b: i32,
    /// C 相有功功率 (W, signed)
    pub active_power_c: i32,
    /// 合相有功功率 (W, signed)
    pub active_power_total: i32,
    /// A 相无功功率 (var, signed)
    pub reactive_power_a: i32,
    /// B 相无功功率 (var, signed)
    pub reactive_power_b: i32,
    /// C 相无功功率 (var, signed)
    pub reactive_power_c: i32,
    /// 合相无功功率 (var, signed)
    pub reactive_power_total: i32,
    /// 电网频率 (0.01Hz)
    pub frequency: u16,
    /// A 相功率因数 (0~1000, 1000=1.000)
    pub power_factor_a: u16,
    /// B 相功率因数 (0~1000)
    pub power_factor_b: u16,
    /// C 相功率因数 (0~1000)
    pub power_factor_c: u16,
    /// 合相功率因数 (0~1000)
    pub power_factor_total: u16,
}

/// 电能累计值 (0.01 kWh / 0.01 kvarh)
#[derive(Clone, Copy, Default, Debug)]
pub struct EnergyData {
    /// 正向有功电能 (import, 0.01 kWh)
    pub active_import: u64,
    /// 反向有功电能 (export, 0.01 kWh)
    pub active_export: u64,
    /// 正向无功电能 (import, 0.01 kvarh)
    pub reactive_import: u64,
    /// 反向无功电能 (export, 0.01 kvarh)
    pub reactive_export: u64,
    /// A 相正向有功电能 (0.01 kWh)
    pub active_import_a: u64,
    /// B 相正向有功电能 (0.01 kWh)
    pub active_import_b: u64,
    /// C 相正向有功电能 (0.01 kWh)
    pub active_import_c: u64,
}

/// 谐波数据 (单相)
#[derive(Clone, Copy, Debug)]
pub struct HarmonicData {
    /// THD 百分比 (0.01%, 即 1234 = 12.34%)
    pub thd: u16,
    /// 各次谐波含量 (1~63次, 百分比 0.01%)
    /// 索引 0 = 基波(=10000), 索引 n = 第(n+1)次谐波
    pub harmonics: [u16; Self::MAX_HARMONICS],
}

impl Default for HarmonicData {
    fn default() -> Self {
        Self {
            thd: 0,
            harmonics: [0; Self::MAX_HARMONICS],
        }
    }
}

impl HarmonicData {
    pub const MAX_HARMONICS: usize = 63;
}

/// 电网质量事件
#[derive(Clone, Copy, Debug)]
pub enum PowerQualityEvent {
    /// 电压暂降 (相号, 持续ms, 最低0.01V)
    VoltageSag { phase: u8, duration_ms: u16, min_voltage: u16 },
    /// 电压暂升 (相号, 持续ms, 最高0.01V)
    VoltageSwell { phase: u8, duration_ms: u16, max_voltage: u16 },
    /// 频率越限 (当前0.01Hz)
    FrequencyDeviation { frequency: u16 },
    /// 三相不平衡度越限 (百分比 0.01%)
    PhaseUnbalance { percentage: u16 },
    /// 电压闪变 (短闪变值, 长闪变值)
    Flicker { short_term: u16, long_term: u16 },
}

/// 校准参数 (三相)
#[derive(Clone, Copy, Debug)]
pub struct CalibrationParams {
    /// 电压增益 [A, B, C]
    pub voltage_gain: [f32; 3],
    /// 电流增益 [A, B, C]
    pub current_gain: [f32; 3],
    /// 有功功率增益 [A, B, C]
    pub power_gain: [f32; 3],
    /// 无功功率增益 [A, B, C]
    pub reactive_gain: [f32; 3],
    /// 相角校正 [A, B, C] (度)
    pub phase_angle: [f32; 3],
}

impl Default for CalibrationParams {
    fn default() -> Self {
        Self {
            voltage_gain: [1.0; 3],
            current_gain: [1.0; 3],
            power_gain: [1.0; 3],
            reactive_gain: [1.0; 3],
            phase_angle: [0.0; 3],
        }
    }
}

/// 计量芯片错误
#[derive(Clone, Copy, Debug)]
pub enum MeteringError {
    /// SPI 通信错误
    SpiError,
    /// 校验和错误
    ChecksumError,
    /// 芯片未就绪 / 上电未完成
    NotReady,
    /// 寄存器地址无效
    InvalidRegister,
    /// 数据溢出
    Overflow,
    /// 防窃电检测触发
    TamperDetected,
}

/* ================================================================== */
/*                                                                    */
/*  trait MeteringChip — 计量芯片统一接口                              */
/*                                                                    */
/*  所有计量芯片 (ATT7022E / RN8302B / RN8615V2) 实现此 trait         */
/*  应用层通过此 trait 操作，不关心具体芯片型号                         */
/*                                                                    */
/* ================================================================== */

/// 计量芯片统一抽象接口
pub trait MeteringChip {
    /// 初始化芯片, 写入校准参数
    fn init(&mut self, params: &CalibrationParams) -> Result<(), MeteringError>;

    /// 芯片软件复位
    fn reset(&mut self) -> Result<(), MeteringError>;

    /// 读取实时三相数据 (电压/电流/功率/频率/PF)
    fn read_instant_data(&mut self) -> Result<PhaseData, MeteringError>;

    /// 读取电能累计值 (正向/反向 有功/无功)
    fn read_energy(&mut self) -> Result<EnergyData, MeteringError>;

    /// 读取零线电流 (mA)
    fn read_neutral_current(&mut self) -> Result<u16, MeteringError>;

    /// 获取芯片 ID / 版本
    fn chip_id(&mut self) -> Result<u32, MeteringError>;

    /// 芯片名称 (编译时已知)
    fn name() -> &'static str where Self: Sized;

    /// 是否支持基波/谐波分离测量
    fn supports_fundamental(&self) -> bool {
        false
    }

    /// 读取基波有功功率 (若支持)
    fn read_fundamental_power(&mut self) -> Result<[i32; 3], MeteringError> {
        Err(MeteringError::InvalidRegister)
    }
}

/* ================================================================== */
/*  trait HarmonicAnalysis — 谐波分析 (仅 RN8302B / RN8615V2)         */
/* ================================================================== */

/// 谐波分析扩展接口
pub trait HarmonicAnalysis: MeteringChip {
    /// 读取指定相的谐波数据
    fn read_harmonics(&mut self, phase: Phase) -> Result<HarmonicData, MeteringError>;

    /// 读取 THD (总谐波畸变率, 0.01%)
    fn read_thd(&mut self, phase: Phase) -> Result<u16, MeteringError>;

    /// 支持的最大谐波次数
    fn max_harmonic_order(&self) -> u8;
}

/// 相选择
#[derive(Clone, Copy, Debug)]
pub enum Phase {
    A = 0,
    B = 1,
    C = 2,
}

/* ================================================================== */
/*  trait PowerQuality — 电网质量分析 (仅 RN8615V2)                   */
/* ================================================================== */

/// 电网质量分析扩展接口
pub trait PowerQuality: HarmonicAnalysis {
    /// 读取三相电压不平衡度 (0.01%)
    fn read_voltage_unbalance(&mut self) -> Result<u16, MeteringError>;

    /// 读取三相电流不平衡度 (0.01%)
    fn read_current_unbalance(&mut self) -> Result<u16, MeteringError>;

    /// 读取短时闪变值 (Pst)
    fn read_short_flicker(&mut self) -> Result<u16, MeteringError>;

    /// 读取长时闪变值 (Plt)
    fn read_long_flicker(&mut self) -> Result<u16, MeteringError>;

    /// 读取间谐波 (指定相, 指定次数)
    fn read_interharmonic(&mut self, phase: Phase, order: u8) -> Result<u16, MeteringError>;

    /// 读取直流分量 (指定相电压, 0.01V)
    fn read_dc_component(&mut self, phase: Phase) -> Result<u16, MeteringError>;

    /// 检查电网质量事件 (非阻塞)
    fn check_pq_event(&mut self) -> Option<PowerQualityEvent>;
}

/* ================================================================== */
/*  SPI 传输抽象                                                       */
/* ================================================================== */

/// SPI 传输接口 (由 board.rs 实现)
pub trait SpiTransfer {
    /// SPI 全双工传输, CS 自动拉低/拉高
    fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), MeteringError>;
}

/* ================================================================== */
/*  UART 通道抽象                                                      */
/* ================================================================== */

/// UART 通道编号
#[derive(Clone, Copy, Debug)]
pub enum UartChannel {
    /// UART0 → RS485 (DLMS/COSEM)
    Uart0,
    /// UART1 → 红外 (IEC 62056-21)
    Uart1,
    /// UART2 → LoRaWAN (ASR6601)
    Uart2,
    /// UART3 → 蜂窝模组 (EC800N / BC260Y)
    Uart3,
    /// UART4 → 备用
    Uart4,
    /// UART5 → 备用
    Uart5,
    /// LPUART0 → 调试/低功耗唤醒
    LpUart0,
    /// LPUART1 → 备用
    LpUart1,
}

/// UART 配置
#[derive(Clone, Copy, Debug)]
pub struct UartConfig {
    pub baudrate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: Parity,
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            baudrate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: Parity::None,
        }
    }
}

/// 校验位
#[derive(Clone, Copy, Debug)]
pub enum Parity {
    None,
    Even,
    Odd,
}

/// UART 错误
#[derive(Clone, Copy, Debug)]
pub enum UartError {
    /// 发送超时
    TxTimeout,
    /// 接收超时
    RxTimeout,
    /// 帧错误 (停止位错误)
    FramingError,
    /// 校验错误
    ParityError,
    /// 溢出错误 (数据未及时读取)
    OverrunError,
    /// 缓冲区满
    BufferFull,
}

/// UART 接口抽象
pub trait UartDriver {
    /// 初始化 UART
    fn init(&mut self, config: &UartConfig) -> Result<(), UartError>;

    /// 发送数据 (阻塞, 带超时)
    fn write(&mut self, data: &[u8]) -> Result<(), UartError>;

    /// 接收数据 (阻塞, 带超时, 返回实际接收长度)
    fn read(&mut self, buf: &mut [u8], timeout_ms: u32) -> Result<usize, UartError>;

    /// 检查是否有数据可读 (非阻塞)
    fn readable(&self) -> bool;

    /// 获取 UART 通道编号
    fn channel(&self) -> UartChannel;
}

/* ================================================================== */
/*  GPIO 抽象                                                          */
/* ================================================================== */

/// GPIO 引脚
#[derive(Clone, Copy, Debug)]
pub struct GpioPin {
    pub port: u8,
    pub pin: u8,
}

impl GpioPin {
    pub const fn new(port: u8, pin: u8) -> Self {
        Self { port, pin }
    }
}

/// GPIO 方向
#[derive(Clone, Copy, Debug)]
pub enum GpioDirection {
    Input,
    Output,
}

/// GPIO 电平
#[derive(Clone, Copy, Debug)]
pub enum GpioLevel {
    Low,
    High,
}

/// GPIO 驱动
pub trait GpioDriver {
    /// 配置引脚方向
    fn set_direction(&mut self, pin: GpioPin, dir: GpioDirection);

    /// 写电平
    fn write(&mut self, pin: GpioPin, level: GpioLevel);

    /// 读电平
    fn read(&self, pin: GpioPin) -> GpioLevel;

    /// 翻转电平
    fn toggle(&mut self, pin: GpioPin);
}

/* ================================================================== */
/*  LCD 段码显示抽象                                                    */
/* ================================================================== */

/// LCD 段码内容 (由应用层填充)
#[derive(Clone, Copy, Default, Debug)]
pub struct LcdContent {
    /// 电压显示值 (0.01V)
    pub voltage: u16,
    /// 电流显示值 (mA)
    pub current: u16,
    /// 有功功率 (W)
    pub active_power: i32,
    /// 无功功率 (var)
    pub reactive_power: i32,
    /// 功率因数 (0~1000)
    pub power_factor: u16,
    /// 频率 (0.01Hz)
    pub frequency: u16,
    /// 正向有功总电能 (0.01 kWh)
    pub active_import_energy: u64,
    /// 当前费率 (0~3)
    pub tariff: u8,
    /// 通信状态 (bit0=RS485, bit1=红外, bit2=LoRa, bit3=蜂窝)
    pub comm_status: u8,
    /// 告警标志 (bit0=过压, bit1=欠压, bit2=过流, bit3=功率反向)
    pub alarm_flags: u8,
}

/// LCD 显示模式
#[derive(Clone, Copy, Debug)]
pub enum LcdDisplayMode {
    /// 自动轮显 (每隔 N 秒切换)
    AutoRotate { interval_sec: u8 },
    /// 按键翻页
    Manual,
    /// 掉电保持显示 (仅显示电能)
    PowerOffHold,
    /// 测试模式 (全显)
    TestAllOn,
    /// 关闭显示
    Off,
}

/// LCD 驱动接口
pub trait LcdDriver {
    /// 初始化 LCD 控制器
    fn init(&mut self);

    /// 更新显示内容
    fn update(&mut self, content: &LcdContent);

    /// 设置显示模式
    fn set_mode(&mut self, mode: LcdDisplayMode);

    /// 开启/关闭显示 (掉电后可关闭省电)
    fn enable(&mut self, on: bool);

    /// LCD bias 方式
    fn set_bias(&mut self, bias: LcdBias);
}

/// LCD Bias 方式
#[derive(Clone, Copy, Debug)]
pub enum LcdBias {
    /// 1/3 bias
    Third,
    /// 1/4 bias
    Quarter,
}

/* ================================================================== */
/*  蜂窝模组抽象                                                       */
/* ================================================================== */

/// 蜂窝模组类型
#[derive(Clone, Copy, Debug)]
pub enum CellularModule {
    /// 移远 EC800N (LTE Cat.1 bis)
    Ec800n,
    /// 移远 BC260Y (NB-IoT)
    Bc260y,
}

/// 蜂窝网络状态
#[derive(Clone, Copy, Debug)]
pub enum NetworkStatus {
    /// 未注册, 未搜索
    NotRegistered,
    /// 已注册, 本地网
    RegisteredHome,
    /// 搜索中
    Searching,
    /// 注册被拒
    Denied,
    /// 未知
    Unknown,
    /// 已注册, 漫游
    RegisteredRoaming,
}

/// MQTT 连接配置
#[derive(Clone, Copy, Debug)]
pub struct MqttConfig {
    pub broker: [u8; 64],  // URL (以 \0 结尾的 C 字符串风格)
    pub port: u16,
    pub client_id: [u8; 32],
    pub username: [u8; 32],
    pub password: [u8; 32],
    pub keepalive_sec: u16,
}

/// GPS 定位数据
#[derive(Clone, Copy, Default, Debug)]
pub struct GpsData {
    /// 纬度 (1e-7 度, 正=北)
    pub latitude: i32,
    /// 经度 (1e-7 度, 正=东)
    pub longitude: i32,
    /// 海拔 (cm)
    pub altitude: i32,
    /// 速度 (cm/s)
    pub speed: u32,
    /// 可见卫星数
    pub satellites: u8,
    /// HDOP (0.1)
    pub hdop: u16,
    /// UTC 时间戳 (Unix)
    pub utc_timestamp: u64,
}

/// 蜂窝模组抽象接口
pub trait CellularDriver {
    /// 初始化模组
    fn init(&mut self) -> Result<(), CellularError>;

    /// 获取模组类型
    fn module_type(&self) -> CellularModule;

    /// 查询网络注册状态
    fn network_status(&mut self) -> Result<NetworkStatus, CellularError>;

    /// 查询信号强度 (dBm, 负数)
    fn signal_strength(&mut self) -> Result<i16, CellularError>;

    /// MQTT 连接
    fn mqtt_connect(&mut self, config: &MqttConfig) -> Result<(), CellularError>;

    /// MQTT 发布
    fn mqtt_publish(&mut self, topic: &str, data: &[u8], qos: u8) -> Result<(), CellularError>;

    /// MQTT 订阅
    fn mqtt_subscribe(&mut self, topic: &str, qos: u8) -> Result<(), CellularError>;

    /// MQTT 断开
    fn mqtt_disconnect(&mut self) -> Result<(), CellularError>;

    /// HTTP GET
    fn http_get(&mut self, url: &str) -> Result<HttpResult, CellularError>;

    /// HTTP POST
    fn http_post(&mut self, url: &str, content_type: &str, body: &[u8]) -> Result<HttpResult, CellularError>;

    /// 发送 SMS
    fn sms_send(&mut self, number: &str, text: &str) -> Result<(), CellularError>;

    /// 读取 SMS
    fn sms_read(&mut self, index: u8) -> Result<SmsMessage, CellularError>;

    /// 删除 SMS
    fn sms_delete(&mut self, index: u8) -> Result<(), CellularError>;

    /// GPS 定位 (仅 EC800N)
    fn gps_position(&mut self) -> Result<GpsData, CellularError>;

    /// 网络时间同步 (NITZ/NTP)
    fn sync_time(&mut self) -> Result<u64, CellularError>;

    /// FOTA 固件升级 (模组自身)
    fn fota_update(&mut self, url: &str) -> Result<(), CellularError>;

    /// 进入省电模式 (PSM/eDRX)
    fn enter_psm(&mut self, tau_sec: u32, active_sec: u32) -> Result<(), CellularError>;

    /// 唤醒
    fn wakeup(&mut self) -> Result<(), CellularError>;

    /// 关机
    fn power_off(&mut self) -> Result<(), CellularError>;
}

/// HTTP 响应
#[derive(Clone, Copy, Debug)]
pub struct HttpResult {
    pub status_code: u16,
    pub content_length: u32,
    /// 数据偏移 (实际数据通过模组缓冲区读取)
    pub data_offset: u32,
}

/// SMS 消息
#[derive(Clone, Copy, Debug)]
pub struct SmsMessage {
    pub index: u8,
    pub sender: [u8; 20],
    pub timestamp: [u8; 20],
    pub text: [u8; 160],
    pub text_len: u8,
}

/// 蜂窝错误
#[derive(Clone, Copy, Debug)]
pub enum CellularError {
    /// AT 指令超时
    AtTimeout,
    /// AT 指令错误
    AtError,
    /// 模组未响应
    NoResponse,
    /// SIM 卡未插入
    NoSim,
    /// 网络未注册
    NotRegistered,
    /// MQTT 未连接
    MqttNotConnected,
    /// GPS 未定位
    GpsNotFixed,
    /// 内存不足
    OutOfMemory,
    /// 不支持的操作 (如 BC260Y 不支持 GPS)
    NotSupported,
    /// HTTP 错误
    HttpError(u16),
}

/* ================================================================== */
/*  LoRaWAN 抽象                                                       */
/* ================================================================== */

/// LoRaWAN 入网方式
#[derive(Clone, Copy, Debug)]
pub enum LorawanJoinMode {
    /// OTAA (Over-The-Air Activation)
    Otaa,
    /// ABP (Activation By Personalization)
    Abp,
}

/// LoRaWAN 配置
#[derive(Clone, Copy, Debug)]
pub struct LorawanConfig {
    /// DevEUI (8字节)
    pub dev_eui: [u8; 8],
    /// AppEUI (8字节)
    pub app_eui: [u8; 8],
    /// AppKey (16字节)
    pub app_key: [u8; 16],
    /// 入网方式
    pub join_mode: LorawanJoinMode,
}

/// LoRaWAN 连接状态
#[derive(Clone, Copy, Debug)]
pub enum LorawanStatus {
    /// 未初始化
    Idle,
    /// 入网中
    Joining,
    /// 已入网
    Joined,
    /// 发送中
    Sending,
    /// 错误
    Error,
}

/// LoRaWAN 驱动接口
pub trait LorawanDriver {
    /// 初始化 ASR6601 模组
    fn init(&mut self) -> Result<(), LorawanError>;

    /// 配置入网参数
    fn configure(&mut self, config: &LorawanConfig) -> Result<(), LorawanError>;

    /// 入网
    fn join(&mut self) -> Result<(), LorawanError>;

    /// 发送数据 (指定端口)
    fn send(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), LorawanError>;

    /// 查询状态
    fn status(&mut self) -> LorawanStatus;

    /// 获取信号强度 (dBm)
    fn rssi(&mut self) -> Result<i16, LorawanError>;
}

/// LoRaWAN 错误
#[derive(Clone, Copy, Debug)]
pub enum LorawanError {
    AtTimeout,
    AtError,
    NoResponse,
    JoinFailed,
    SendFailed,
    NotJoined,
    Busy,
}

/* ================================================================== */
/*  存储抽象                                                            */
/* ================================================================== */

/// 存储区域
#[derive(Clone, Copy, Debug)]
pub enum StorageRegion {
    /// MCU 内部 Flash 参数区
    InternalParam,
    /// 外部 W25Q64 Flash (若存在)
    ExternalFlash,
}

/// 存储驱动接口
pub trait StorageDriver {
    /// 读取数据
    fn read(&mut self, region: StorageRegion, offset: u32, buf: &mut [u8]) -> Result<(), StorageError>;

    /// 写入数据 (需先擦除)
    fn write(&mut self, region: StorageRegion, offset: u32, data: &[u8]) -> Result<(), StorageError>;

    /// 擦除扇区 (内部: 2KB, 外部: 4KB)
    fn erase_sector(&mut self, region: StorageRegion, sector_index: u32) -> Result<(), StorageError>;

    /// 获取区域总大小 (字节)
    fn capacity(&self, region: StorageRegion) -> u32;

    /// 获取扇区大小 (字节)
    fn sector_size(&self, region: StorageRegion) -> u32;
}

/// 存储错误
#[derive(Clone, Copy, Debug)]
pub enum StorageError {
    ReadError,
    WriteError,
    EraseError,
    OutOfBounds,
    NotAvailable,
}

/* ================================================================== */
/*  脉冲输出抽象                                                       */
/* ================================================================== */

/// 脉冲类型
#[derive(Clone, Copy, Debug)]
pub enum PulseType {
    /// 有功电能脉冲
    Active,
    /// 无功电能脉冲
    Reactive,
}

/// 脉冲输出配置
#[derive(Clone, Copy, Debug)]
pub struct PulseConfig {
    /// 脉冲常数 (imp/kWh 或 imp/kvarh), 例如 6400
    pub constant_imp: u32,
    /// 脉冲宽度 (ms), 例如 80
    pub pulse_width_ms: u16,
}

/// 脉冲输出驱动接口
pub trait PulseDriver {
    /// 配置脉冲参数
    fn configure(&mut self, config: &PulseConfig);

    /// 更新电能增量, 内部自动计算是否需要输出脉冲
    fn update_energy(&mut self, pulse_type: PulseType, delta_wh: u32);
}

/* ================================================================== */
/*  电池 / 电源管理抽象                                                 */
/* ================================================================== */

/// 电源状态
#[derive(Clone, Copy, Debug)]
pub enum PowerState {
    /// 市电正常
    MainsNormal,
    /// 市电掉电, 电池供电
    BatteryBackup,
    /// 电池电量低
    BatteryLow,
}

/// 电源管理驱动接口
pub trait PowerDriver {
    /// 查询当前电源状态
    fn state(&self) -> PowerState;

    /// 读取电池电压 (mV)
    fn battery_voltage(&mut self) -> u16;

    /// 读取 MCU 温度 (0.1°C)
    fn temperature(&mut self) -> i16;

    /// 进入低功耗模式
    fn enter_low_power(&mut self);

    /// 唤醒
    fn wakeup(&mut self);
}

/* ================================================================== */
/*  按键 / 输入抽象                                                    */
/* ================================================================== */

/// 按键事件
#[derive(Clone, Copy, Debug)]
pub enum KeyEvent {
    /// 短按
    ShortPress(Button),
    /// 长按 (>1s)
    LongPress(Button),
    /// 双击
    DoublePress(Button),
}

/// 按键编号
#[derive(Clone, Copy, Debug)]
pub enum Button {
    /// 翻页键
    Page,
    /// 编程键
    Program,
}

/* ================================================================== */
/*  告警 / 指示抽象                                                    */
/* ================================================================== */

/// LED 编号
#[derive(Clone, Copy, Debug)]
pub enum Led {
    /// 电源指示 (绿)
    Power,
    /// 通信指示 (黄)
    Communication,
    /// 告警指示 (红)
    Alarm,
    /// 有功脉冲 (红)
    PulseActive,
    /// 无功脉冲 (绿)
    PulseReactive,
}

/// 告警驱动接口
pub trait IndicatorDriver {
    /// 控制 LED
    fn set_led(&mut self, led: Led, on: bool);

    /// 翻转 LED
    fn toggle_led(&mut self, led: Led);

    /// 蜂鸣器告警
    fn buzzer_alarm(&mut self, duration_ms: u16);

    /// 关闭蜂鸣器
    fn buzzer_off(&mut self);
}

/* ================================================================== */
/*  防窃电检测抽象                                                     */
/* ================================================================== */

/// 窃电事件类型
#[derive(Clone, Copy, Debug)]
pub enum TamperEvent {
    /// 上盖打开
    CoverOpen,
    /// 端子盖打开
    TerminalCoverOpen,
    /// 强磁场检测
    MagneticFieldDetected,
    /// 计量芯片防窃电 (RN8302B/RN8615V2)
    ChipTamperDetected,
}

/// 防窃电驱动接口
pub trait TamperDriver {
    /// 检查所有窃电事件 (非阻塞)
    fn check_events(&mut self) -> Option<TamperEvent>;

    /// 读取磁场强度 (若霍尔传感器存在, 0.1μT)
    fn magnetic_field_strength(&mut self) -> Option<u16>;
}
