//! AT 指令通用框架 — 适配所有 UART 通信模组
//!
//! 支持的模组类型:
//! - LTE Cat.1: 移远 EC800N, 广和通 L610, 合宙 Air724UG
//! - NB-IoT: 移远 BC260Y/BC28F, 广和通 L620
//! - LoRaWAN: 亿佰特 E78-470LN22S (ASR6601)
//! - Wi-Fi: ESP8266/ESP32 (AT firmware)
//!
//! 设计原则:
//! 1. 传输层与命令集解耦
//! 2. URC (主动上报) 统一处理
//! 3. 超时/重试/错误恢复
//! 4. 异步非阻塞 (no heap, no alloc)

#![no_std]

use core::fmt;
use core::time::Duration;

// ============================================================
// 错误类型
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtError {
    /// 串口错误
    UartError,
    /// 超时
    Timeout,
    /// 模组返回 ERROR
    Error { code: u16 },
    /// 模组返回 COMMAND NOT SUPPORT
    NotSupported,
    /// 模组未就绪
    NotReady,
    /// 缓冲区溢出
    BufferOverflow,
    /// 参数错误
    InvalidParam,
    /// SIM 卡错误
    SimError,
    /// 网络未注册
    NetworkNotRegistered,
    /// 连接失败
    ConnectionFailed,
    /// 发送失败
    SendFailed,
    /// 响应解析失败
    ParseError,
}

// ============================================================
// UART 抽象 — 由硬件层实现
// ============================================================

/// UART 驱动 trait — 底层串口抽象
pub trait UartTransport {
    /// 读取一个字节 (非阻塞), None = 无数据
    fn read_byte(&mut self) -> Option<u8>;
    /// 写入字节切片 (阻塞直到写完)
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), AtError>;
    /// 清空接收缓冲区
    fn flush_rx(&mut self);
    /// 检查串口是否可用
    fn is_available(&self) -> bool;
}

/// GPIO 控制引脚 trait
pub trait PinControl {
    fn set_high(&mut self);
    fn set_low(&mut self);
    fn is_high(&self) -> bool;
}

/// 延时 trait
pub trait Delay {
    fn delay_ms(&mut self, ms: u32);
}

// ============================================================
// AT 响应类型
// ============================================================

/// AT 指令响应
#[derive(Debug)]
pub enum AtResponse {
    /// 成功 (OK)
    Ok,
    /// 成功并附带数据行
    OkWithLines(heapless::Vec<heapless::String<256>, 16>),
    /// 错误
    Error(u16),
    /// 无响应 (超时前没收到任何数据)
    NoResponse,
}

/// URC (主动上报) 事件
#[derive(Debug, Clone)]
pub enum UrcEvent {
    /// 模组就绪
    Ready,
    /// 关机
    PowerDown,
    /// 网络注册状态变化
    NetworkRegChanged { stat: u8 },
    /// 信号变化
    SignalChanged { rssi: u8, ber: u8 },
    /// 收到 IP 数据
    DataReceived { conn_id: u8, size: usize },
    /// 连接关闭
    ConnectionClosed { conn_id: u8 },
    /// 收到 SMS
    SmsReceived { index: u16 },
    /// LoRaWAN: 加入成功
    LorawanJoined,
    /// LoRaWAN: 收到下行数据
    LorawanRxReceived { port: u8, rssi: i16, snr: i8, data: heapless::Vec<u8, 256> },
    /// LoRaWAN: 发送完成
    LorawanTxDone { status: u8 },
    /// Ring 指示 (来电/数据)
    Ring,
    /// 自定义 URC (模组特有)
    Custom(heapless::String<128>),
}

// ============================================================
// AT 解析器核心
// ============================================================

/// AT 指令解析器 — 通用、无堆、非阻塞
///
/// 负责所有 AT 指令模组共有的:
/// - 行缓冲与分割
/// - OK/ERROR 判定
/// - URC 检测与回调
/// - 超时处理
pub struct AtParser<T: UartTransport, const LINE_BUF: usize = 256, const MAX_LINES: usize = 16> {
    transport: T,
    /// 当前行缓冲
    line_buf: heapless::String<LINE_BUF>,
    /// 多行响应收集
    response_lines: heapless::Vec<heapless::String<LINE_BUF>, MAX_LINES>,
    /// 毫秒计时 (由外部 tick 驱动)
    tick_ms: u32,
}

impl<T: UartTransport, const LINE_BUF: usize, const MAX_LINES: usize> AtParser<T, LINE_BUF, MAX_LINES> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            line_buf: heapless::String::new(),
            response_lines: heapless::Vec::new(),
            tick_ms: 0,
        }
    }

    /// 释放内部 transport
    pub fn into_inner(self) -> T {
        self.transport
    }

    // ---- 低层操作 ----

    /// 发送原始字节
    pub fn send_raw(&mut self, data: &[u8]) -> Result<(), AtError> {
        self.transport.write_bytes(data)
    }

    /// 发送 AT 指令 (自动追加 \r\n)
    pub fn send_cmd(&mut self, cmd: &str) -> Result<(), AtError> {
        self.transport.write_bytes(cmd.as_bytes())?;
        self.transport.write_bytes(b"\r\n")
    }

    /// 发送带十六进制数据的 AT 指令
    pub fn send_cmd_hex(&mut self, cmd: &str, hex_data: &[u8]) -> Result<(), AtError> {
        self.transport.write_bytes(cmd.as_bytes())?;
        // 逐字节转 hex
        for &b in hex_data {
            let hex = hex_digit(b >> 4);
            let hex2 = hex_digit(b & 0x0F);
            self.transport.write_bytes(&[hex, hex2])?;
        }
        self.transport.write_bytes(b"\r\n")
    }

    /// 轮询串口, 收集完整行
    /// 返回 Some(line) 当收到一行完整数据 (以 \n 结尾)
    pub fn poll_line(&mut self) -> Option<heapless::String<LINE_BUF>> {
        while let Some(byte) = self.transport.read_byte() {
            if byte == b'\r' {
                continue; // 跳过 \r
            }
            if byte == b'\n' {
                if self.line_buf.is_empty() {
                    continue; // 空行跳过
                }
                let line = self.line_buf.clone();
                self.line_buf.clear();
                return Some(line);
            }
            // 追加到行缓冲
            if self.line_buf.push(byte as char).is_err() {
                self.line_buf.clear(); // 溢出则丢弃
            }
        }
        None
    }

    // ---- 标准响应等待 ----

    /// 等待 OK 或 ERROR (标准 AT 响应)
    /// 返回响应之间的所有数据行
    pub fn wait_ok(&mut self, timeout_ms: u32, get_ms: impl Fn() -> u32) -> AtResponse {
        self.response_lines.clear();
        let start = get_ms();

        loop {
            // 超时检查
            if get_ms().wrapping_sub(start) >= timeout_ms {
                return AtResponse::NoResponse;
            }

            if let Some(line) = self.poll_line() {
                if line == "OK" {
                    if self.response_lines.is_empty() {
                        return AtResponse::Ok;
                    } else {
                        return AtResponse::OkWithLines(self.response_lines.clone());
                    }
                }
                if line == "ERROR" || line.starts_with("+CME ERROR:") {
                    let code = if let Some(idx) = line.find(':') {
                        line[idx + 1..].trim().parse().unwrap_or(0)
                    } else {
                        0
                    };
                    return AtResponse::Error(code);
                }
                // 不是终止符, 存入响应行
                let _ = self.response_lines.push(line);
            }
        }
    }

    /// 等待特定前缀的响应 (如 "+CSQ:", "+CREG:")
    pub fn wait_prefix(
        &mut self,
        prefix: &str,
        timeout_ms: u32,
        get_ms: impl Fn() -> u32,
    ) -> Option<heapless::String<LINE_BUF>> {
        let start = get_ms();
        loop {
            if get_ms().wrapping_sub(start) >= timeout_ms {
                return None;
            }
            if let Some(line) = self.poll_line() {
                if line.starts_with(prefix) {
                    return Some(line);
                }
                if line == "OK" || line == "ERROR" {
                    return None;
                }
            }
        }
    }

    /// 等待精确字符串匹配 (如 "RDY", "> ", "CONNECT OK")
    pub fn wait_exact(
        &mut self,
        expected: &str,
        timeout_ms: u32,
        get_ms: impl Fn() -> u32,
    ) -> bool {
        let start = get_ms();
        loop {
            if get_ms().wrapping_sub(start) >= timeout_ms {
                return false;
            }
            if let Some(line) = self.poll_line() {
                if line == expected {
                    return true;
                }
            }
        }
    }

    /// 等待发送提示符 ">" (用于 AT+QISEND 等数据发送命令)
    pub fn wait_prompt(&mut self, timeout_ms: u32, get_ms: impl Fn() -> u32) -> bool {
        // ">" 通常不带换行, 需要字节级检测
        let start = get_ms();
        loop {
            if get_ms().wrapping_sub(start) >= timeout_ms {
                return false;
            }
            if let Some(byte) = self.transport.read_byte() {
                if byte == b'>' {
                    return true;
                }
            }
        }
    }

    // ---- URC 检测 ----

    /// 检查一行是否是 URC (主动上报)
    /// URC 格式: 以 "+" 开头且不以 "AT+" 开头
    pub fn detect_urc(line: &str) -> Option<UrcEvent> {
        // 标准 URC
        match line {
            "RDY" => return Some(UrcEvent::Ready),
            "NORMAL POWER DOWN" | "POWER DOWN" => return Some(UrcEvent::PowerDown),
            "RING" => return Some(UrcEvent::Ring),
            _ => {}
        }

        // 前缀匹配 URC
        if line.starts_with("+CREG:") || line.starts_with("+CGREG:") || line.starts_with("+CEREG:") {
            // +CREG: <n>,<stat> 或 +CREG: <stat>
            let stat = parse_last_number(line).unwrap_or(0) as u8;
            return Some(UrcEvent::NetworkRegChanged { stat });
        }

        if line.starts_with("+CSQ:") {
            let (rssi, ber) = parse_csq(line);
            return Some(UrcEvent::SignalChanged { rssi, ber });
        }

        if line.starts_with("+QIURC:") {
            if line.contains("recv") || line.contains("\"recv\"") {
                let parts: heapless::Vec<&str, 4> = line.split(',').collect();
                let conn_id = parts.get(1).and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(0);
                let size = parts.get(2).and_then(|s| s.trim().parse::<usize>().ok()).unwrap_or(0);
                return Some(UrcEvent::DataReceived { conn_id, size });
            }
            if line.contains("closed") {
                let conn_id = parse_last_number(line).unwrap_or(0) as u8;
                return Some(UrcEvent::ConnectionClosed { conn_id });
            }
        }

        if line.starts_with("+CMT:") {
            let index = parse_last_number(line).unwrap_or(0) as u16;
            return Some(UrcEvent::SmsReceived { index });
        }

        // LoRaWAN URC (ASR6601 / E78-470LN22S)
        if line.starts_with("+LORARX") || line.starts_with("+RECV") {
            return Some(UrcEvent::LorawanRxReceived {
                port: 0,
                rssi: 0,
                snr: 0,
                data: heapless::Vec::new(),
            });
        }

        if line.starts_with("+LORATX") || line.starts_with("+SEND") {
            return Some(UrcEvent::LorawanTxDone { status: 0 });
        }

        if line.contains("JOIN") && (line.contains("OK") || line.contains("Success")) {
            return Some(UrcEvent::LorawanJoined);
        }

        // 无法识别的 URC
        if line.starts_with('+') && !line.starts_with("AT+") {
            let mut s = heapless::String::new();
            let _ = s.push_str(&line[..line.len().min(128)]);
            return Some(UrcEvent::Custom(s));
        }

        None
    }

    /// 非阻塞轮询 URC
    /// 应在主循环中持续调用
    pub fn poll_urc(&mut self) -> Option<UrcEvent> {
        if let Some(line) = self.poll_line() {
            Self::detect_urc(&line)
        } else {
            None
        }
    }
}

// ============================================================
// 模组通信 trait 体系 — 按能力分层
// ============================================================

/// 基础模组能力 — 所有模组必须实现
pub trait ModuleBase {
    /// 模组类型标识
    fn module_type(&self) -> ModuleType;
    /// 发送 AT 测试指令
    fn test_at(&mut self) -> Result<(), AtError>;
    /// 查询模组版本信息
    fn get_version(&mut self) -> Result<heapless::String<128>, AtError>;
    /// 获取 IMEI (蜂窝模组) 或 DevEUI (LoRaWAN)
    fn get_device_id(&mut self) -> Result<heapless::String<32>, AtError>;
    /// 软件复位模组
    fn reset(&mut self) -> Result<(), AtError>;
    /// 检查模组是否就绪
    fn is_ready(&mut self) -> bool;
    /// 设置波特率
    fn set_baudrate(&mut self, baud: u32) -> Result<(), AtError>;
}

/// 蜂窝模组能力 — Cat.1 / NB-IoT / GPRS
pub trait CellularModule: ModuleBase {
    /// 查询 SIM 卡状态
    fn get_sim_status(&mut self) -> Result<SimStatus, AtError>;
    /// 查询 ICCID
    fn get_iccid(&mut self) -> Result<heapless::String<24>, AtError>;
    /// 查询信号强度
    fn get_signal(&mut self) -> Result<SignalInfo, AtError>;
    /// 查询网络注册状态
    fn get_network_reg(&mut self) -> Result<NetworkStatus, AtError>;
    /// 查询运营商信息
    fn get_operator(&mut self) -> Result<heapless::String<32>, AtError>;
    /// 手动附着网络
    fn attach_network(&mut self) -> Result<(), AtError>;
    /// 查询 IP 地址
    fn get_ip_address(&mut self) -> Result<heapless::String<64>, AtError>;
    /// 设置 APN
    fn set_apn(&mut self, apn: &str) -> Result<(), AtError>;
}

/// Socket 网络能力 — TCP/UDP
pub trait SocketOps: CellularModule {
    /// 建立 TCP 连接
    fn tcp_connect(&mut self, addr: &str, port: u16) -> Result<u8, AtError>;
    /// 建立 UDP 连接
    fn udp_connect(&mut self, addr: &str, port: u16) -> Result<u8, AtError>;
    /// 发送数据
    fn send_data(&mut self, conn_id: u8, data: &[u8]) -> Result<(), AtError>;
    /// 接收数据 (非阻塞)
    fn recv_data(&mut self, conn_id: u8, buf: &mut [u8]) -> Result<usize, AtError>;
    /// 关闭连接
    fn close(&mut self, conn_id: u8) -> Result<(), AtError>;
    /// 查询连接状态
    fn get_conn_status(&mut self, conn_id: u8) -> Result<ConnStatus, AtError>;
}

/// MQTT 能力 — Cat.1 内置 MQTT
pub trait MqttOps: CellularModule {
    /// 配置 MQTT broker
    fn mqtt_set_broker(&mut self, addr: &str, port: u16) -> Result<(), AtError>;
    /// 配置 MQTT 客户端 ID
    fn mqtt_set_client_id(&mut self, id: &str) -> Result<(), AtError>;
    /// 配置 MQTT 用户名/密码
    fn mqtt_set_auth(&mut self, user: &str, pass: &str) -> Result<(), AtError>;
    /// 配置遗嘱消息
    fn mqtt_set_will(&mut self, topic: &str, msg: &[u8], qos: QoS, retain: bool) -> Result<(), AtError>;
    /// 连接 MQTT broker
    fn mqtt_connect(&mut self) -> Result<(), AtError>;
    /// 断开 MQTT
    fn mqtt_disconnect(&mut self) -> Result<(), AtError>;
    /// 发布消息
    fn mqtt_publish(&mut self, topic: &str, data: &[u8], qos: QoS) -> Result<(), AtError>;
    /// 订阅主题
    fn mqtt_subscribe(&mut self, topic: &str, qos: QoS) -> Result<(), AtError>;
    /// 取消订阅
    fn mqtt_unsubscribe(&mut self, topic: &str) -> Result<(), AtError>;
    /// 查询 MQTT 状态
    fn mqtt_state(&mut self) -> Result<MqttState, AtError>;
}

/// LoRaWAN 能力 — ASR6601
pub trait LorawanOps: ModuleBase {
    /// 设置 AppEUI
    fn set_app_eui(&mut self, eui: &[u8; 8]) -> Result<(), AtError>;
    /// 设置 AppKey
    fn set_app_key(&mut self, key: &[u8; 16]) -> Result<(), AtError>;
    /// 设置 DevEUI
    fn set_dev_eui(&mut self, eui: &[u8; 8]) -> Result<(), AtError>;
    /// OTAA 入网
    fn join_otaa(&mut self) -> Result<(), AtError>;
    /// ABP 入网 (设置 NwkSKey/AppSKey/DevAddr)
    fn join_abp(&mut self, dev_addr: &[u8; 4], nwk_skey: &[u8; 16], app_s_key: &[u8; 16]) -> Result<(), AtError>;
    /// 发送数据 (确认)
    fn send_confirmed(&mut self, port: u8, data: &[u8]) -> Result<(), AtError>;
    /// 发送数据 (非确认)
    fn send_unconfirmed(&mut self, port: u8, data: &[u8]) -> Result<(), AtError>;
    /// 查询入网状态
    fn is_joined(&mut self) -> Result<bool, AtError>;
    /// 设置数据速率 (DR0~DR5)
    fn set_dr(&mut self, dr: u8) -> Result<(), AtError>;
    /// 设置发射功率
    fn set_tx_power(&mut self, power: i8) -> Result<(), AtError>;
    /// 设置信道 (CN470)
    fn set_channel_mask(&mut self, mask: &[u8; 8]) -> Result<(), AtError>;
    /// 设置 Class (A/B/C)
    fn set_class(&mut self, class: LorawanClass) -> Result<(), AtError>;
    /// 查询 RSSI/SNR
    fn get_link_quality(&mut self) -> Result<(i16, i8), AtError>;
}

/// 低功耗控制
pub trait PowerControl {
    /// 进入睡眠模式
    fn sleep(&mut self) -> Result<(), AtError>;
    /// 唤醒
    fn wakeup(&mut self) -> Result<(), AtError>;
    /// 查询当前功耗模式
    fn get_power_mode(&mut self) -> PowerMode;
}

// ============================================================
// 数据类型
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleType {
    LteCat1,
    NbIoT,
    Gprs,
    Lorawan,
    Wifi,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimStatus {
    Ready,
    NotInserted,
    PinRequired,
    PukRequired,
    PinLocked,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct SignalInfo {
    pub rssi: i16,   // dBm, 负值
    pub ber: u8,      // 0~7
    pub rsrp: i16,    // dBm (LTE)
    pub rsrq: i16,    // dB (LTE)
    pub snr: i16,     // dB (LTE)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkStatus {
    NotRegistered,
    HomeNetwork,
    Searching,
    RegistrationDenied,
    Unknown,
    Roaming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnStatus {
    Disconnected,
    Connecting,
    Connected,
    Closing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqttState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QoS {
    QoS0 = 0,
    QoS1 = 1,
    QoS2 = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LorawanClass {
    ClassA,
    ClassB,
    ClassC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerMode {
    Active,
    Sleep,
    DeepSleep,
    PowerOff,
}

// ============================================================
// 辅助函数
// ============================================================

fn hex_digit(n: u8) -> u8 {
    if n < 10 { b'0' + n } else { b'A' + n - 10 }
}

fn parse_last_number(s: &str) -> Option<u32> {
    s.split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .last()
        .and_then(|p| p.parse().ok())
}

fn parse_csq(s: &str) -> (u8, u8) {
    // +CSQ: <rssi>,<ber>
    let parts: heapless::Vec<&str, 4> = s.split(|c: char| c == ':' || c == ',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    let rssi = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(99);
    let ber = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(99);
    (rssi, ber)
}

// ============================================================
// 具体模组实现示例 (骨架)
// ============================================================

pub mod ec800n {
    //! 移远 EC800N LTE Cat.1 模组驱动

    use super::*;

    pub struct Ec800n<T: UartTransport, PWR: PinControl, RST: PinControl> {
        pub at: AtParser<T>,
        pwrkey: PWR,
        reset: RST,
        mqtt_state: MqttState,
    }

    impl<T: UartTransport, PWR: PinControl, RST: PinControl> Ec800n<T, PWR, RST> {
        pub fn new(uart: T, pwrkey: PWR, reset: RST) -> Self {
            Self {
                at: AtParser::new(uart),
                pwrkey,
                reset,
                mqtt_state: MqttState::Disconnected,
            }
        }

        /// 硬件开机序列
        pub fn hardware_power_on<D: Delay>(&mut self, delay: &mut D) -> Result<(), AtError> {
            self.pwrkey.set_high();
            delay.delay_ms(200);
            self.pwrkey.set_low();
            delay.delay_ms(500); // PWRKEY 拉低 ≥500ms
            self.pwrkey.set_high();
            Ok(())
        }

        /// 硬件关机序列
        pub fn hardware_power_off<D: Delay>(&mut self, delay: &mut D) {
            self.pwrkey.set_low();
            delay.delay_ms(650); // PWRKEY 拉低 ≥650ms 关机
            self.pwrkey.set_high();
        }

        /// 硬件复位
        pub fn hardware_reset<D: Delay>(&mut self, delay: &mut D) {
            self.reset.set_low();
            delay.delay_ms(200);
            self.reset.set_high();
            delay.delay_ms(3000); // 等待重启
        }
    }

    // 实现 ModuleBase
    impl<T: UartTransport, PWR: PinControl, RST: PinControl> ModuleBase for Ec800n<T, PWR, RST> {
        fn module_type(&self) -> ModuleType { ModuleType::LteCat1 }

        fn test_at(&mut self) -> Result<(), AtError> {
            self.at.send_cmd("AT")?;
            Ok(())
        }

        fn get_version(&mut self) -> Result<heapless::String<128>, AtError> {
            self.at.send_cmd("AT+GMR")?;
            // TODO: 解析响应
            Ok(heapless::String::new())
        }

        fn get_device_id(&mut self) -> Result<heapless::String<32>, AtError> {
            self.at.send_cmd("AT+GSN")?;
            // TODO: 解析 IMEI
            Ok(heapless::String::new())
        }

        fn reset(&mut self) -> Result<(), AtError> {
            self.at.send_cmd("AT+CFUN=1,1")?;
            Ok(())
        }

        fn is_ready(&mut self) -> bool {
            self.test_at().is_ok()
        }

        fn set_baudrate(&mut self, baud: u32) -> Result<(), AtError> {
            let cmd = heapless::String::<32>::from("AT+IPR=");
            // 格式化波特率并追加
            // self.at.send_cmd(&format!("AT+IPR={}", baud))?;
            let _ = baud;
            Ok(())
        }
    }

    // 实现 CellularModule
    impl<T: UartTransport, PWR: PinControl, RST: PinControl> CellularModule for Ec800n<T, PWR, RST> {
        fn get_sim_status(&mut self) -> Result<SimStatus, AtError> {
            self.at.send_cmd("AT+CPIN?")?;
            Ok(SimStatus::Ready)
        }

        fn get_iccid(&mut self) -> Result<heapless::String<24>, AtError> {
            self.at.send_cmd("AT+QCCID")?;
            Ok(heapless::String::new())
        }

        fn get_signal(&mut self) -> Result<SignalInfo, AtError> {
            self.at.send_cmd("AT+CSQ")?;
            // TODO: 解析 +CSQ: <rssi>,<ber>
            // LTE 模组还支持 AT+QENG="servingcell" 获取 RSRP/RSRQ/SINR
            Ok(SignalInfo { rssi: -99, ber: 0, rsrp: 0, rsrq: 0, snr: 0 })
        }

        fn get_network_reg(&mut self) -> Result<NetworkStatus, AtError> {
            self.at.send_cmd("AT+CREG?")?;
            Ok(NetworkStatus::HomeNetwork)
        }

        fn get_operator(&mut self) -> Result<heapless::String<32>, AtError> {
            self.at.send_cmd("AT+QSPN")?;
            Ok(heapless::String::new())
        }

        fn attach_network(&mut self) -> Result<(), AtError> {
            self.at.send_cmd("AT+CGATT=1")?;
            Ok(())
        }

        fn get_ip_address(&mut self) -> Result<heapless::String<64>, AtError> {
            self.at.send_cmd("AT+QIACT?")?;
            Ok(heapless::String::new())
        }

        fn set_apn(&mut self, apn: &str) -> Result<(), AtError> {
            let _ = apn;
            // AT+QICSGP=1,1,"CMNET","","",1
            Ok(())
        }
    }

    // 实现 SocketOps
    impl<T: UartTransport, PWR: PinControl, RST: PinControl> SocketOps for Ec800n<T, PWR, RST> {
        fn tcp_connect(&mut self, addr: &str, port: u16) -> Result<u8, AtError> {
            let _ = (addr, port);
            // AT+QIOPEN=1,0,"TCP","x.x.x.x",port,0,0
            Ok(0)
        }

        fn udp_connect(&mut self, addr: &str, port: u16) -> Result<u8, AtError> {
            let _ = (addr, port);
            Ok(0)
        }

        fn send_data(&mut self, conn_id: u8, data: &[u8]) -> Result<(), AtError> {
            let _ = (conn_id, data);
            // AT+QISEND=0,<len>
            // 等待 ">" 提示符
            // 发送数据
            // 等待 "SEND OK"
            Ok(())
        }

        fn recv_data(&mut self, conn_id: u8, buf: &mut [u8]) -> Result<usize, AtError> {
            let _ = (conn_id, buf);
            // AT+QIRD=0,<len>
            Ok(0)
        }

        fn close(&mut self, conn_id: u8) -> Result<(), AtError> {
            let _ = conn_id;
            // AT+QICLOSE=0
            Ok(())
        }

        fn get_conn_status(&mut self, conn_id: u8) -> Result<ConnStatus, AtError> {
            let _ = conn_id;
            // AT+QISTATE=1,0
            Ok(ConnStatus::Connected)
        }
    }

    // 实现 MqttOps (EC800N 内置 MQTT)
    impl<T: UartTransport, PWR: PinControl, RST: PinControl> MqttOps for Ec800n<T, PWR, RST> {
        fn mqtt_set_broker(&mut self, addr: &str, port: u16) -> Result<(), AtError> {
            // AT+QMTCFG="version",0,4    (MQTT 3.1.1)
            // AT+QMTOPEN=0,"broker.emqx.io",1883
            let _ = (addr, port);
            Ok(())
        }

        fn mqtt_set_client_id(&mut self, id: &str) -> Result<(), AtError> {
            let _ = id;
            Ok(())
        }

        fn mqtt_set_auth(&mut self, user: &str, pass: &str) -> Result<(), AtError> {
            let _ = (user, pass);
            Ok(())
        }

        fn mqtt_set_will(&mut self, topic: &str, msg: &[u8], qos: QoS, retain: bool) -> Result<(), AtError> {
            // AT+QMTCFG="will",0,<qos>,<retain>,"<topic>","<msg>"
            let _ = (topic, msg, qos, retain);
            Ok(())
        }

        fn mqtt_connect(&mut self) -> Result<(), AtError> {
            // AT+QMTCONN=0,"client_id"
            self.mqtt_state = MqttState::Connected;
            Ok(())
        }

        fn mqtt_disconnect(&mut self) -> Result<(), AtError> {
            // AT+QMTDISC=0
            self.mqtt_state = MqttState::Disconnected;
            Ok(())
        }

        fn mqtt_publish(&mut self, topic: &str, data: &[u8], qos: QoS) -> Result<(), AtError> {
            // AT+QMTPUB=0,0,<qos>,0,"<topic>","<data>"
            let _ = (topic, data, qos);
            Ok(())
        }

        fn mqtt_subscribe(&mut self, topic: &str, qos: QoS) -> Result<(), AtError> {
            // AT+QMTSUB=0,1,"<topic>",<qos>
            let _ = (topic, qos);
            Ok(())
        }

        fn mqtt_unsubscribe(&mut self, topic: &str) -> Result<(), AtError> {
            // AT+QMTUNS=0,1,"<topic>"
            let _ = topic;
            Ok(())
        }

        fn mqtt_state(&mut self) -> Result<MqttState, AtError> {
            Ok(self.mqtt_state)
        }
    }
}

pub mod bc260y {
    //! 移远 BC260Y NB-IoT 模组驱动
    //! AT 指令集与 EC800N 高度相似 (都是移远 Quectel 系列)
    //! 主要差异: CoAP 代替 TCP、无语音、更低功耗

    use super::*;

    pub struct Bc260y<T: UartTransport, PWR: PinControl> {
        pub at: AtParser<T>,
        pwrkey: PWR,
    }

    // 实现 ModuleBase + CellularModule
    // AT 指令大部分复用 EC800N 的逻辑
    // BC260Y 额外支持 CoAP:
    //   AT+QCOAPCREATE=<contextID>          创建 CoAP 实例
    //   AT+QCOAPGET=<token>,"<uri>",<len>   CoAP GET
    //   AT+QCOAPPUT=<token>,"<uri>",<len>   CoAP PUT
    //   AT+QCOAPPOST=<token>,"<uri>",<len>  CoAP POST

    /// CoAP 操作 (NB-IoT 特有)
    pub trait CoapOps: CellularModule {
        fn coap_create(&mut self) -> Result<u8, AtError>;
        fn coap_get(&mut self, uri: &str) -> Result<heapless::Vec<u8, 512>, AtError>;
        fn coap_post(&mut self, uri: &str, data: &[u8]) -> Result<(), AtError>;
        fn coap_close(&mut self) -> Result<(), AtError>;
    }
}

pub mod asr6601 {
    //! 亿佰特 E78-470LN22S (ASR6601) LoRaWAN 模组驱动
    //!
    //! AT 指令集 (亿佰特自定义):
    //!   AT+LORACFG          配置 LoRa 参数
    //!   AT+LORAKEY          配置密钥
    //!   AT+LORAJOIN         入网
    //!   AT+LORASEND         发送
    //!   AT+LORARECV         接收
    //!   AT+LORASTATE        查询状态

    use super::*;

    pub struct Asr6601<T: UartTransport> {
        pub at: AtParser<T>,
        joined: bool,
    }

    impl<T: UartTransport> Asr6601<T> {
        pub fn new(uart: T) -> Self {
            Self {
                at: AtParser::new(uart),
                joined: false,
            }
        }
    }

    impl<T: UartTransport> ModuleBase for Asr6601<T> {
        fn module_type(&self) -> ModuleType { ModuleType::Lorawan }

        fn test_at(&mut self) -> Result<(), AtError> {
            self.at.send_cmd("AT")?;
            Ok(())
        }

        fn get_version(&mut self) -> Result<heapless::String<128>, AtError> {
            self.at.send_cmd("AT+CGMI")?; // 厂商
            self.at.send_cmd("AT+CGMM")?; // 型号
            self.at.send_cmd("AT+CGMR")?; // 版本
            Ok(heapless::String::new())
        }

        fn get_device_id(&mut self) -> Result<heapless::String<32>, AtError> {
            self.at.send_cmd("AT+LORADEVEUI?")?;
            Ok(heapless::String::new())
        }

        fn reset(&mut self) -> Result<(), AtError> {
            self.at.send_cmd("AT+RESET")?;
            Ok(())
        }

        fn is_ready(&mut self) -> bool {
            self.test_at().is_ok()
        }

        fn set_baudrate(&mut self, baud: u32) -> Result<(), AtError> {
            let _ = baud;
            // AT+UART=38400 (E78 固定几个波特率)
            Ok(())
        }
    }

    // 实现 LorawanOps
    impl<T: UartTransport> LorawanOps for Asr6601<T> {
        fn set_app_eui(&mut self, eui: &[u8; 8]) -> Result<(), AtError> {
            // AT+LORAKEY=APPEUI,<hex16>
            let _ = eui;
            Ok(())
        }

        fn set_app_key(&mut self, key: &[u8; 16]) -> Result<(), AtError> {
            // AT+LORAKEY=APPKEY,<hex32>
            let _ = key;
            Ok(())
        }

        fn set_dev_eui(&mut self, eui: &[u8; 8]) -> Result<(), AtError> {
            // AT+LORAKEY=DEVEUI,<hex16>
            let _ = eui;
            Ok(())
        }

        fn join_otaa(&mut self) -> Result<(), AtError> {
            // AT+LORAJOIN=OTAA
            // 等待 +LORAEVENT:JOIN,OK 或超时
            self.at.send_cmd("AT+LORAJOIN=OTAA")?;
            self.joined = true;
            Ok(())
        }

        fn join_abp(&mut self, dev_addr: &[u8; 4], nwk_skey: &[u8; 16], app_s_key: &[u8; 16]) -> Result<(), AtError> {
            // AT+LORAJOIN=ABP
            // AT+LORAKEY=NWKSKEY,<hex32>
            // AT+LORAKEY=APPSKEY,<hex32>
            // AT+LORAKEY=DEVADDR,<hex8>
            let _ = (dev_addr, nwk_skey, app_s_key);
            Ok(())
        }

        fn send_confirmed(&mut self, port: u8, data: &[u8]) -> Result<(), AtError> {
            // AT+LORASEND=<port>,1,<hex_data>
            let _ = (port, data);
            Ok(())
        }

        fn send_unconfirmed(&mut self, port: u8, data: &[u8]) -> Result<(), AtError> {
            // AT+LORASEND=<port>,0,<hex_data>
            let _ = (port, data);
            Ok(())
        }

        fn is_joined(&mut self) -> Result<bool, AtError> {
            // AT+LORASTATE?
            Ok(self.joined)
        }

        fn set_dr(&mut self, dr: u8) -> Result<(), AtError> {
            // AT+LORACFG=DR,<dr>
            let _ = dr;
            Ok(())
        }

        fn set_tx_power(&mut self, power: i8) -> Result<(), AtError> {
            // AT+LORACFG=TXPOWER,<power>
            let _ = power;
            Ok(())
        }

        fn set_channel_mask(&mut self, mask: &[u8; 8]) -> Result<(), AtError> {
            // AT+LORACFG=CHMASK,<hex16>
            let _ = mask;
            Ok(())
        }

        fn set_class(&mut self, class: LorawanClass) -> Result<(), AtError> {
            // AT+LORACFG=CLASS,<A|B|C>
            let _ = class;
            Ok(())
        }

        fn get_link_quality(&mut self) -> Result<(i16, i8), AtError> {
            // AT+LORASTATE? 解析 RSSI/SNR
            Ok((-50, 8))
        }
    }
}

// ============================================================
// 通道管理 — 统一四通道
// ============================================================

/// 通信通道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelId {
    Rs485 = 0,
    Irda = 1,
    Cat1 = 2,
    Lorawan = 3,
}

/// 通道统一消息
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub source: ChannelId,
    pub obis: [u8; 6],       // OBIS 短码
    pub value: ChannelValue,
    pub timestamp: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum ChannelValue {
    Float(f64),
    U32(u32),
    I32(i32),
    Bytes(heapless::Vec<u8, 64>),
    String(heapless::String<64>),
}

/// 通道管理器 — 统一调度四个通信通道
pub struct ChannelManager<R, I, C, L>
where
    R: Rs485Channel,
    I: IrdaChannel,
    C: CloudChannel,
    L: LorawanChannel,
{
    rs485: R,
    irda: I,
    cat1: C,
    lora: L,
}

/// RS485 通道 (DLMS/COSEM)
pub trait Rs485Channel {
    fn send_dlms(&mut self, frame: &[u8]) -> Result<(), AtError>;
    fn recv_dlms(&mut self, buf: &mut [u8]) -> Result<usize, AtError>;
    fn set_baudrate(&mut self, baud: u32);
}

/// 红外通道 (IEC 62056-21)
pub trait IrdaChannel {
    fn send_iec62056(&mut self, data: &str) -> Result<(), AtError>;
    fn recv_iec62056(&mut self, buf: &mut [u8]) -> Result<usize, AtError>;
    fn set_mode(&mut self, mode: IecMode);
}

#[derive(Debug, Clone, Copy)]
pub enum IecMode {
    /// 协议模式 A — 本地读表
    ModeA,
    /// 协议模式 B — 本地读表 (HDLC)
    ModeB,
    /// 协议模式 C — 双向握手
    ModeC,
    /// 协议模式 D — 被动响应
    ModeD,
}

/// 云通道 (Cat.1 MQTT)
pub trait CloudChannel {
    fn is_connected(&self) -> bool;
    fn publish(&mut self, topic: &str, data: &[u8]) -> Result<(), AtError>;
    fn subscribe(&mut self, topic: &str) -> Result<(), AtError>;
    fn poll_incoming(&mut self) -> Option<CloudMessage>;
}

#[derive(Debug)]
pub struct CloudMessage {
    pub topic: heapless::String<128>,
    pub data: heapless::Vec<u8, 512>,
}

/// LoRaWAN 通道
pub trait LorawanChannel {
    fn is_joined(&self) -> bool;
    fn send(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), AtError>;
    fn poll_rx(&mut self) -> Option<LorawanRxMessage>;
}

#[derive(Debug)]
pub struct LorawanRxMessage {
    pub port: u8,
    pub rssi: i16,
    pub snr: i8,
    pub data: heapless::Vec<u8, 256>,
}

// ============================================================
// 通道管理器路由逻辑
// ============================================================

impl<R, I, C, L> ChannelManager<R, I, C, L>
where
    R: Rs485Channel,
    I: IrdaChannel,
    C: CloudChannel,
    L: LorawanChannel,
{
    pub fn new(rs485: R, irda: I, cat1: C, lora: L) -> Self {
        Self { rs485, irda, cat1, lora }
    }

    /// 上行数据路由 — 优先级: Cat.1 > LoRaWAN > RS485 > 红外
    pub fn route_uplink(&mut self, topic: &str, data: &[u8]) -> Result<ChannelId, AtError> {
        // 优先 Cat.1 MQTT
        if self.cat1.is_connected() {
            self.cat1.publish(topic, data)?;
            return Ok(ChannelId::Cat1);
        }
        // 降级 LoRaWAN
        if self.lora.is_joined() {
            self.lora.send(1, data, false)?;
            return Ok(ChannelId::Lorawan);
        }
        // 无法上行
        Err(AtError::NotReady)
    }

    /// 轮询所有通道 URC/下行
    pub fn poll_all(&mut self) -> Option<(ChannelId, CloudMessage)> {
        // Cat.1 MQTT 下行
        if let Some(msg) = self.cat1.poll_incoming() {
            return Some((ChannelId::Cat1, msg));
        }
        None
    }

    /// 逐通道非阻塞轮询
    pub fn poll_cat1_urc(&mut self) -> Option<UrcEvent> {
        // 由各模组的 AtParser 内部实现
        None
    }
}
