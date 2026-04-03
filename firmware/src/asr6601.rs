/* ================================================================== */
/*                                                                    */
/*  asr6601.rs — ASR6601 LoRaWAN 模组 AT 指令驱动                     */
/*                                                                    */
/*  亿佰特 E78-470LN22S (ASR6601 内核)                                 */
/*  - CN470~510MHz LoRaWAN                                            */
/*  - UART AT 指令, 默认 38400bps                                     */
/*  - OTAA / ABP 入网                                                  */
/*  - Class A / B / C                                                 */
/*  - 内置天线匹配, 最大 22dBm 输出                                    */
/*  - 接收灵敏度 -137dBm @SF12                                        */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::hal::*;
use core::fmt::Write;

/* ================================================================== */
/*  CN470 频段支持                                                     */
/* ================================================================== */

/// CN470 频段配置参数
#[derive(Clone, Copy, Debug)]
pub struct Cn470Config {
    /// 数据速率 (DR0~DR5)
    pub data_rate: u8,
    /// 发射功率 (0~14, 对应 2~22dBm)
    pub tx_power: u8,
    /// ADR 自适应速率开关
    pub adr_enabled: bool,
    /// 工作模式 (Class A 或 Class C)
    pub device_class: LoraClass,
    /// 接收窗口1延迟 (ms)
    pub rx1_delay_ms: u16,
    /// 接收窗口2延迟 (ms)
    pub rx2_delay_ms: u16,
}

impl Default for Cn470Config {
    fn default() -> Self {
        Self {
            data_rate: 0,
            tx_power: 14,
            adr_enabled: true,
            device_class: LoraClass::A,
            rx1_delay_ms: 1000,
            rx2_delay_ms: 2000,
        }
    }
}

/// LoRa 设备工作模式
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoraClass {
    /// Class A — 最省电, 上行后开两个短接收窗口
    A,
    /// Class B — 预约接收时隙
    B,
    /// Class C — 持续接收 (除上行外)
    C,
}

impl LoraClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoraClass::A => "A",
            LoraClass::B => "B",
            LoraClass::C => "C",
        }
    }
}

/// CN470 数据速率对应的调制参数
#[derive(Clone, Copy, Debug)]
pub struct DrParams {
    /// 扩频因子
    pub spreading_factor: u8,
    /// 带宽 (kHz)
    pub bandwidth_khz: u32,
    /// 最大有效载荷 (bytes)
    pub max_payload: u8,
}

/// CN470 数据速率表
pub const CN470_DR_TABLE: [DrParams; 6] = [
    DrParams { spreading_factor: 12, bandwidth_khz: 125, max_payload: 59 },  // DR0
    DrParams { spreading_factor: 11, bandwidth_khz: 125, max_payload: 59 },  // DR1
    DrParams { spreading_factor: 10, bandwidth_khz: 125, max_payload: 59 },  // DR2
    DrParams { spreading_factor: 9,  bandwidth_khz: 125, max_payload: 123 }, // DR3
    DrParams { spreading_factor: 8,  bandwidth_khz: 125, max_payload: 230 }, // DR4
    DrParams { spreading_factor: 7,  bandwidth_khz: 125, max_payload: 230 }, // DR5
];

/// Get CN470 DR parameters
pub fn cn470_dr_params(dr: u8) -> Option<&'static DrParams> {
    CN470_DR_TABLE.get(dr as usize)
}

/* ================================================================== */
/*  DLMS over LoRaWAN 封装                                              */
/* ================================================================== */

/// DLMS over LoRaWAN 封装格式
///
/// 用于在 LoRaWAN 的有限载荷中传输 DLMS APDU:
/// [Header(1)][InvokeID(1)][APDU data...]
///
/// Header byte:
///   Bit 7:     Encrypted (1=yes)
///   Bit 6:     Compressed (1=yes)
///   Bit 5-4:   Fragment index (for multi-uplink)
///   Bit 3-0:   DLMS APDU type tag

/// DLMS over LoRaWAN header byte bit definitions
pub const DLMS_LORA_ENCRYPTED: u8 = 0x80;
pub const DLMS_LORA_COMPRESSED: u8 = 0x40;
pub const DLMS_LORA_FRAG_MASK: u8 = 0x30;
pub const DLMS_LORA_FRAG_SHIFT: u8 = 4;
pub const DLMS_LORA_TYPE_MASK: u8 = 0x0F;

/// DLMS APDU type tags for LoRaWAN header
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DlmsLoraType {
    /// Get Request
    GetRequest = 0x05,
    /// Get Response
    GetResponse = 0x0C,
    /// Set Request
    SetRequest = 0x06,
    /// Set Response
    SetResponse = 0x0D,
    /// Action Request
    ActionRequest = 0x07,
    /// Action Response
    ActionResponse = 0x0E,
    /// Event Notification
    EventNotification = 0x08,
    /// Initiate Request
    InitiateRequest = 0x09,
}

impl DlmsLoraType {
    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag & DLMS_LORA_TYPE_MASK {
            0x05 => Some(DlmsLoraType::GetRequest),
            0x0C => Some(DlmsLoraType::GetResponse),
            0x06 => Some(DlmsLoraType::SetRequest),
            0x0D => Some(DlmsLoraType::SetResponse),
            0x07 => Some(DlmsLoraType::ActionRequest),
            0x0E => Some(DlmsLoraType::ActionResponse),
            0x08 => Some(DlmsLoraType::EventNotification),
            0x09 => Some(DlmsLoraType::InitiateRequest),
            _ => None,
        }
    }
}

/// Build a DLMS over LoRaWAN frame
/// Returns the total bytes written to `out`
pub fn build_dlms_lora_frame(
    apdu_type: DlmsLoraType,
    invoke_id: u8,
    apdu_data: &[u8],
    encrypted: bool,
    frag_index: u8,
    out: &mut [u8],
) -> usize {
    if out.len() < 2 {
        return 0;
    }

    // Build header byte
    let mut header = apdu_type as u8 & DLMS_LORA_TYPE_MASK;
    if encrypted {
        header |= DLMS_LORA_ENCRYPTED;
    }
    header |= (frag_index & 0x03) << DLMS_LORA_FRAG_SHIFT;

    out[0] = header;
    out[1] = invoke_id;

    let payload_len = apdu_data.len().min(out.len().saturating_sub(2));
    out[2..2 + payload_len].copy_from_slice(&apdu_data[..payload_len]);

    2 + payload_len
}

/// Parse a DLMS over LoRaWAN frame header
/// Returns (apdu_type, invoke_id, payload, encrypted, frag_index)
pub fn parse_dlms_lora_frame(
    data: &[u8],
) -> Option<(DlmsLoraType, u8, &[u8], bool, u8)> {
    if data.len() < 2 {
        return None;
    }

    let header = data[0];
    let encrypted = (header & DLMS_LORA_ENCRYPTED) != 0;
    let frag_index = (header & DLMS_LORA_FRAG_MASK) >> DLMS_LORA_FRAG_SHIFT;
    let invoke_id = data[1];
    let apdu_type = DlmsLoraType::from_tag(header)?;
    let payload = &data[2..];

    Some((apdu_type, invoke_id, payload, encrypted, frag_index))
}

/// Calculate the maximum DLMS APDU payload size for a given DR
/// Accounts for LoRaWAN MAC header (13 bytes) and our DLMS-Lora header (2 bytes)
pub fn max_dlms_payload(dr: u8) -> usize {
    let dr_params = match cn470_dr_params(dr) {
        Some(p) => p,
        None => return 51, // conservative default
    };
    // LoRaWAN PHYPayload = MHDR(1) + FHDR(7) + FPort(1) + FRMPayload + MIC(4)
    // For CN470: MHDR(1) + FHDR(7) + FPort(1) + MIC(4) = 13 bytes overhead
    // Our header: 2 bytes
    (dr_params.max_payload as usize).saturating_sub(13 + 2)
}

/* ================================================================== */
/* AT 指令定义                                                        */
/* ================================================================== */

mod at {
    /// 测试 AT 连接
    pub const TEST: &str = "AT\r\n";
    /// 软件复位
    pub const RESET: &str = "ATZ\r\n";
    /// 恢复出厂设置
    pub const RESTORE: &str = "AT+FDEFAULT\r\n";
    /// 查询固件版本
    pub const VERSION: &str = "AT+VER?\r\n";
    /// 查询设备 EUI
    pub const DEVEUI: &str = "AT+ID=DevEui\r\n";
    /// 设置设备 EUI
    pub const SET_DEVEUI: &str = "AT+ID=DevEui,\"";
    /// 查询应用 EUI
    pub const APPEUI: &str = "AT+ID=AppEui\r\n";
    /// 设置应用 EUI
    pub const SET_APPEUI: &str = "AT+ID=AppEui,\"";
    /// 设置应用密钥
    pub const SET_APPKEY: &str = "AT+KEY=AppKey,\"";
    /// 设置入网模式: OTAA
    pub const SET_OTAA: &str = "AT+MODE=LWOTAA\r\n";
    /// 设置入网模式: ABP
    pub const SET_ABP: &str = "AT+MODE=LWABP\r\n";
    /// 加入网络
    pub const JOIN: &str = "AT+JOIN\r\n";
    /// 发送数据 (不确认)
    pub const SEND_UC: &str = "AT+MSG=";
    /// 发送数据 (确认)
    pub const SEND_CF: &str = "AT+CMSG=";
    /// 查询信号强度
    pub const RSSI: &str = "AT+RSSI?\r\n";
    /// 设置串口波特率
    pub const BAUDRATE: &str = "AT+UART=BAUDRATE,";

    // ABP 模式设置
    /// 设置设备地址
    pub const SET_DEVADDR: &str = "AT+ID=DevAddr,\"";
    /// 设置网络会话密钥
    pub const SET_NWKSKEY: &str = "AT+KEY=NwkSKey,\"";
    /// 设置应用会话密钥
    pub const SET_APPSKEY: &str = "AT+KEY=AppSKey,\"";

    // LoRa 配置
    /// 设置频段 (CN470)
    pub const SET_BAND: &str = "AT+BAND=CN470\r\n";
    /// 设置数据速率 (DR0~DR5)
    pub const SET_DR: &str = "AT+DR=";
    /// 设置发射功率 (0~14, 对应 2~22dBm)
    pub const SET_PWR: &str = "AT+POWER=";
    /// 设置 ADR (自适应速率)
    pub const SET_ADR: &str = "AT+ADR=";
    /// 设置 Class (A/B/C)
    pub const SET_CLASS: &str = "AT+CLASS=";

    // 省电
    /// 设置休眠模式
    pub const SLEEP: &str = "AT+SLEEP=ON\r\n";
    /// 唤醒
    pub const WAKEUP: &str = "AT\r\n";
}

/* ================================================================== */
/*  AT 响应解析                                                        */
/* ================================================================== */

/// AT 指令响应
#[derive(Clone, Copy, Debug)]
pub enum AtResponse {
    /// "OK"
    Ok,
    /// "ERROR"
    Error,
    /// "+JOIN: Network joined"
    JoinSuccess,
    /// "+JOIN: Join failed"
    JoinFailed,
    /// "+MSG: Done" / "+CMSG: Done"
    SendSuccess,
    /// "+MSG: ERROR" / "+CMSG: ERROR"
    SendFailed,
    /// "+MSG: PORT: x; data"
    ReceivedData { port: u8, len: u8 },
    /// "+RSSI: xxx"
    RssiValue(i16),
    /// "+VER: x.x.x"
    VersionInfo,
    /// "+ID: DevEui, xxx"
    DevEuiInfo,
    /// 其他未知响应
    Unknown,
}

/// AT 响应缓冲区大小
const AT_BUF_SIZE: usize = 256;

/* ================================================================== */
/*  ASR6601 驱动结构体                                                  */
/* ================================================================== */

/// ASR6601 LoRaWAN 模组驱动
pub struct Asr6601 {
    uart: &'static mut dyn UartDriver,
    /// AT 响应缓冲区
    rx_buf: [u8; AT_BUF_SIZE],
    /// 当前状态
    state: LorawanStatus,
    /// 当前 RSSI
    last_rssi: i16,
}

impl Asr6601 {
    /// 创建 ASR6601 驱动实例
    pub fn new(uart: &'static mut dyn UartDriver) -> Self {
        Self {
            uart,
            rx_buf: [0; AT_BUF_SIZE],
            state: LorawanStatus::Idle,
            last_rssi: -127,
        }
    }

    /// 发送 AT 指令并等待响应
    ///
    /// 返回响应文本的长度 (写入 rx_buf)
    fn send_at(&mut self, cmd: &str, timeout_ms: u32) -> Result<usize, LorawanError> {
        // 清空接收缓冲区
        self.rx_buf = [0; AT_BUF_SIZE];

        // 发送 AT 指令
        self.uart
            .write(cmd.as_bytes())
            .map_err(|_| LorawanError::AtError)?;

        // 等待响应 (读到 "\r\n" 结尾或超时)
        let mut total = 0;
        let deadline = timeout_ms;

        loop {
            match self.uart.read(&mut self.rx_buf[total..], 100) {
                Ok(n) => {
                    total += n;
                    // 检查是否收到完整响应 (包含 "OK\r\n" 或 "ERROR\r\n")
                    if total >= 4 {
                        let tail = &self.rx_buf[total - 4..total];
                        if tail == b"OK\r\n" || tail == b"ROR\r\n" {
                            break;
                        }
                    }
                    if total >= AT_BUF_SIZE - 1 {
                        break; // 缓冲区满
                    }
                }
                Err(UartError::RxTimeout) => {
                    // 超时检查
                    if total > 0 {
                        break; // 已收到部分数据
                    }
                    if deadline == 0 {
                        return Err(LorawanError::AtTimeout);
                    }
                }
                Err(_) => return Err(LorawanError::AtError),
            }
        }

        if total == 0 {
            return Err(LorawanError::NoResponse);
        }

        Ok(total)
    }

    /// 发送 AT 指令并检查是否返回 OK
    fn send_at_ok(&mut self, cmd: &str, timeout_ms: u32) -> Result<(), LorawanError> {
        let len = self.send_at(cmd, timeout_ms)?;
        if Self::contains_ok(&self.rx_buf[..len]) {
            Ok(())
        } else {
            Err(LorawanError::AtError)
        }
    }

    /// 检查缓冲区是否包含 "OK"
    fn contains_ok(buf: &[u8]) -> bool {
        for i in 0..buf.len().saturating_sub(1) {
            if buf[i] == b'O' && buf[i + 1] == b'K' {
                return true;
            }
        }
        false
    }

    /// 将十六进制字符串转为字节数组
    /// 输入: "0123456789ABCDEF" (不带 0x 前缀, 无分隔符)
    fn hex_to_bytes(hex: &str, out: &mut [u8]) -> Result<usize, LorawanError> {
        let hex_bytes = hex.as_bytes();
        if hex_bytes.len() / 2 > out.len() {
            return Err(LorawanError::AtError);
        }
        let mut i = 0;
        while i * 2 + 1 < hex_bytes.len() {
            let hi = Self::hex_digit(hex_bytes[i * 2])?;
            let lo = Self::hex_digit(hex_bytes[i * 2 + 1])?;
            out[i] = (hi << 4) | lo;
            i += 1;
        }
        Ok(i)
    }

    fn hex_digit(b: u8) -> Result<u8, LorawanError> {
        match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            b'A'..=b'F' => Ok(b - b'A' + 10),
            _ => Err(LorawanError::AtError),
        }
    }

    /// 字节数组转十六进制字符串 (写入缓冲区, 不含前缀)
    fn bytes_to_hex<'a>(bytes: &[u8], buf: &'a mut [u8]) -> &'a [u8] {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        for (i, &b) in bytes.iter().enumerate() {
            buf[i * 2] = HEX[(b >> 4) as usize];
            buf[i * 2 + 1] = HEX[(b & 0x0F) as usize];
        }
        &buf[..bytes.len() * 2]
    }
}

/* ================================================================== */
/*  实现 LorawanDriver trait                                            */
/* ================================================================== */

impl LorawanDriver for Asr6601 {
    fn init(&mut self) -> Result<(), LorawanError> {
        // 1. 配置 UART: 38400 8N1
        let config = UartConfig {
            baudrate: 38400,
            data_bits: 8,
            stop_bits: 1,
            parity: Parity::None,
        };
        self.uart.init(&config).map_err(|_| LorawanError::AtError)?;

        // 2. 测试 AT 连接
        self.send_at_ok(at::TEST, 1000)?;

        // 3. 设置频段 CN470
        self.send_at_ok(at::SET_BAND, 2000)?;

        // 4. 设置 Class A (最省电)
        self.send_at_ok(at::SET_CLASS, 1000)?;

        // 5. 开启 ADR
        self.send_at_ok("AT+ADR=ON\r\n", 1000)?;

        self.state = LorawanStatus::Idle;
        Ok(())
    }

    fn configure(&mut self, config: &LorawanConfig) -> Result<(), LorawanError> {
        match config.join_mode {
            LorawanJoinMode::Otaa => {
                // OTAA 模式
                self.send_at_ok(at::SET_OTAA, 1000)?;

                // 设置 DevEUI
                let mut hex_buf = [0u8; 16];
                let hex_str = Self::bytes_to_hex(&config.dev_eui, &mut hex_buf);
                let mut cmd_buf = [0u8; 64];
                let prefix = at::SET_DEVEUI.as_bytes();
                let mut pos = 0;
                for &b in prefix {
                    cmd_buf[pos] = b;
                    pos += 1;
                }
                for &b in hex_str {
                    cmd_buf[pos] = b;
                    pos += 1;
                }
                cmd_buf[pos] = b'"';
                pos += 1;
                cmd_buf[pos] = b'\r';
                pos += 1;
                cmd_buf[pos] = b'\n';
                pos += 1;
                self.send_at_ok(
                    core::str::from_utf8(&cmd_buf[..pos]).unwrap_or("AT\r\n"),
                    1000,
                )?;

                // 设置 AppEUI
                let hex_str = Self::bytes_to_hex(&config.app_eui, &mut hex_buf);
                let mut pos = 0;
                let prefix = at::SET_APPEUI.as_bytes();
                for &b in prefix {
                    cmd_buf[pos] = b;
                    pos += 1;
                }
                for &b in hex_str {
                    cmd_buf[pos] = b;
                    pos += 1;
                }
                cmd_buf[pos] = b'"';
                pos += 1;
                cmd_buf[pos] = b'\r';
                pos += 1;
                cmd_buf[pos] = b'\n';
                pos += 1;
                self.send_at_ok(
                    core::str::from_utf8(&cmd_buf[..pos]).unwrap_or("AT\r\n"),
                    1000,
                )?;

                // 设置 AppKey
                let mut key_hex = [0u8; 32];
                let hex_str = Self::bytes_to_hex(&config.app_key, &mut key_hex);
                let prefix = at::SET_APPKEY.as_bytes();
                let mut pos = 0;
                for &b in prefix {
                    cmd_buf[pos] = b;
                    pos += 1;
                }
                for &b in hex_str {
                    cmd_buf[pos] = b;
                    pos += 1;
                }
                cmd_buf[pos] = b'"';
                pos += 1;
                cmd_buf[pos] = b'\r';
                pos += 1;
                cmd_buf[pos] = b'\n';
                pos += 1;
                self.send_at_ok(
                    core::str::from_utf8(&cmd_buf[..pos]).unwrap_or("AT\r\n"),
                    1000,
                )?;
            }
            LorawanJoinMode::Abp => {
                // ABP 模式: 直接设置密钥
                self.send_at_ok(at::SET_ABP, 1000)?;
            }
        }

        Ok(())
    }

    fn join(&mut self) -> Result<(), LorawanError> {
        self.state = LorawanStatus::Joining;

        // OTAA 入网, 等待 10 秒
        let len = self.send_at(at::JOIN, 10_000)?;

        // 检查是否入网成功
        let response = &self.rx_buf[..len];
        let joined = response.windows(5).any(|w| w == b"oined");

        if joined {
            self.state = LorawanStatus::Joined;
            Ok(())
        } else {
            self.state = LorawanStatus::Error;
            Err(LorawanError::JoinFailed)
        }
    }

    fn send(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), LorawanError> {
        if !matches!(self.state, LorawanStatus::Joined) {
            return Err(LorawanError::NotJoined);
        }

        self.state = LorawanStatus::Sending;

        // 构建 AT+CMSG 或 AT+MSG 命令
        // AT+CMSG= "hex_data"  (确认)
        // AT+MSG= "hex_data"   (不确认)
        let mut cmd_buf = [0u8; 256];
        let mut pos = 0;

        let prefix = if confirmed {
            at::SEND_CF.as_bytes()
        } else {
            at::SEND_UC.as_bytes()
        };
        for &b in prefix {
            cmd_buf[pos] = b;
            pos += 1;
        }
        cmd_buf[pos] = b'"';
        pos += 1;

        // 数据转十六进制
        let mut hex_buf = [0u8; 128];
        let data_hex = Self::bytes_to_hex(data, &mut hex_buf);
        for &b in data_hex {
            cmd_buf[pos] = b;
            pos += 1;
        }

        cmd_buf[pos] = b'"';
        pos += 1;
        cmd_buf[pos] = b'\r';
        pos += 1;
        cmd_buf[pos] = b'\n';
        pos += 1;

        let len = self.send_at(
            core::str::from_utf8(&cmd_buf[..pos]).unwrap_or("AT\r\n"),
            10_000,
        )?;

        // 检查发送结果
        let response = &self.rx_buf[..len];
        let done = response.windows(4).any(|w| w == b"Done");

        if done {
            self.state = LorawanStatus::Joined;
            Ok(())
        } else {
            self.state = LorawanStatus::Joined; // 恢复状态
            Err(LorawanError::SendFailed)
        }
    }

    fn status(&mut self) -> LorawanStatus {
        self.state
    }

    fn rssi(&mut self) -> Result<i16, LorawanError> {
        let len = self.send_at(at::RSSI, 2000)?;
        let response = &self.rx_buf[..len];

        // 解析 "+RSSI: -xxx"
        let mut rssi = 0i16;
        let mut neg = false;
        let mut found = false;
        for i in 0..response.len() {
            if response[i] == b':' {
                found = true;
                let mut j = i + 1;
                while j < response.len() && (response[j] == b' ' || response[j] == b'\r') {
                    j += 1;
                }
                if j < response.len() && response[j] == b'-' {
                    neg = true;
                    j += 1;
                }
                while j < response.len() && response[j].is_ascii_digit() {
                    rssi = rssi * 10 + (response[j] - b'0') as i16;
                    j += 1;
                }
                break;
            }
        }

        if found {
            self.last_rssi = if neg { -rssi } else { rssi };
            Ok(self.last_rssi)
        } else {
            Ok(self.last_rssi)
        }
    }
}

/* ================================================================== */
/*  Tests                                                              */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cn470_dr_table() {
        let dr0 = cn470_dr_params(0).unwrap();
        assert_eq!(dr0.spreading_factor, 12);
        assert_eq!(dr0.max_payload, 59);

        let dr5 = cn470_dr_params(5).unwrap();
        assert_eq!(dr5.spreading_factor, 7);
        assert_eq!(dr5.max_payload, 230);

        assert!(cn470_dr_params(6).is_none());
    }

    #[test]
    fn test_lora_class() {
        assert_eq!(LoraClass::A.as_str(), "A");
        assert_eq!(LoraClass::C.as_str(), "C");
    }

    #[test]
    fn test_dlms_lora_frame_roundtrip() {
        let mut out = [0u8; 128];
        let apdu_data = [0x01, 0x02, 0x03, 0x04];
        let len = build_dlms_lora_frame(
            DlmsLoraType::GetRequest,
            0x42,
            &apdu_data,
            false,
            0,
            &mut out,
        );

        assert_eq!(len, 6);
        assert_eq!(out[1], 0x42);

        let parsed = parse_dlms_lora_frame(&out[..len]).unwrap();
        assert_eq!(parsed.0, DlmsLoraType::GetRequest);
        assert_eq!(parsed.1, 0x42);
        assert_eq!(parsed.2, &apdu_data);
        assert!(!parsed.3);
        assert_eq!(parsed.4, 0);
    }

    #[test]
    fn test_dlms_lora_encrypted_frame() {
        let mut out = [0u8; 128];
        let len = build_dlms_lora_frame(
            DlmsLoraType::SetRequest,
            0x01,
            &[0xAA, 0xBB],
            true,
            2,
            &mut out,
        );

        let parsed = parse_dlms_lora_frame(&out[..len]).unwrap();
        assert_eq!(parsed.0, DlmsLoraType::SetRequest);
        assert!(parsed.3); // encrypted
        assert_eq!(parsed.4, 2); // frag_index
    }

    #[test]
    fn test_dlms_lora_parse_too_short() {
        assert!(parse_dlms_lora_frame(&[]).is_none());
        assert!(parse_dlms_lora_frame(&[0x05]).is_none());
    }

    #[test]
    fn test_max_dlms_payload() {
        let max = max_dlms_payload(0);
        assert!(max > 0);
        assert!(max < 59); // DR0 max_payload=59, minus overhead

        let max_dr5 = max_dlms_payload(5);
        assert!(max_dr5 > max); // DR5 has larger max payload
    }

    #[test]
    fn test_cn470_config_default() {
        let config = Cn470Config::default();
        assert_eq!(config.data_rate, 0);
        assert_eq!(config.tx_power, 14);
        assert!(config.adr_enabled);
        assert_eq!(config.device_class, LoraClass::A);
    }

    #[test]
    fn test_dlms_lora_type_from_tag() {
        assert_eq!(DlmsLoraType::from_tag(0x05), Some(DlmsLoraType::GetRequest));
        assert_eq!(DlmsLoraType::from_tag(0x0C), Some(DlmsLoraType::GetResponse));
        assert_eq!(DlmsLoraType::from_tag(0xFF), None);
        assert_eq!(DlmsLoraType::from_tag(0x00), None);
    }

    #[test]
    fn test_build_dlms_lora_empty_payload() {
        let mut out = [0u8; 16];
        let len = build_dlms_lora_frame(DlmsLoraType::GetRequest, 0x01, &[], false, 0, &mut out);
        assert_eq!(len, 2);

        let parsed = parse_dlms_lora_frame(&out[..len]).unwrap();
        assert_eq!(parsed.2.len(), 0);
    }

    #[test]
    fn test_build_dlms_lora_buffer_too_small() {
        let mut out = [0u8; 1];
        let len = build_dlms_lora_frame(DlmsLoraType::GetRequest, 0x01, &[0x01], false, 0, &mut out);
        assert_eq!(len, 0);
    }

    #[test]
    fn test_hex_to_bytes() {
        // Note: can't call instance methods without a UartDriver mock
        // Test the utility function indirectly through the driver
        let result = Asr6601::hex_digit(b'A');
        assert_eq!(result.unwrap(), 10);
        let result = Asr6601::hex_digit(b'0');
        assert_eq!(result.unwrap(), 0);
        let result = Asr6601::hex_digit(b'G');
        assert!(result.is_err());
    }

    #[test]
    fn test_dlms_lora_all_types() {
        let types = [
            DlmsLoraType::GetRequest,
            DlmsLoraType::GetResponse,
            DlmsLoraType::SetRequest,
            DlmsLoraType::SetResponse,
            DlmsLoraType::ActionRequest,
            DlmsLoraType::ActionResponse,
            DlmsLoraType::EventNotification,
            DlmsLoraType::InitiateRequest,
        ];
        for t in &types {
            let mut out = [0u8; 32];
            let len = build_dlms_lora_frame(*t, 0x01, &[0xFF], false, 0, &mut out);
            let parsed = parse_dlms_lora_frame(&out[..len]).unwrap();
            assert_eq!(parsed.0, *t);
        }
    }
}
