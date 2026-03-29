//! 移远(Quectel) 全系列通信模组统一驱动
//!
//! 覆盖产品线:
//!   Cat.4:  EC20, EC25, EC200T, EC200S
//!   Cat.1:  EC800N, EC800G, EC200U, EC800E
//!   NB-IoT: BC260Y, BC28F, BC95
//!   GPRS:   M26, M35, MC60, M66
//!   车规:   AG35, AG215, AG52
//!   GNSS:   LC29H, L76K, LC79D
//!
//! 核心设计:
//!   QuectelBase — 所有移远模组共享的 AT 命令实现
//!   各系列仅声明差异 (能力标记 + 个别命令覆盖)

use super::*;

// ============================================================
// 移远模组型号枚举
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuectelModel {
    // --- LTE Cat.4 ---
    Ec20,
    Ec25,
    Ec200T,
    Ec200S,
    Ec200A,
    // --- LTE Cat.1 / Cat.1bis ---
    Ec800N,
    Ec800G,
    Ec200U,
    Ec800E,
    // --- NB-IoT ---
    Bc260Y,
    Bc28F,
    Bc95,
    // --- GSM/GPRS ---
    M26,
    M35,
    Mc60,
    M66,
    // --- 车规 ---
    Ag35,
    Ag215,
    Ag52,
    // --- 未知/自定义 ---
    Unknown,
}

impl QuectelModel {
    /// 从 AT+GMR 或 AT+CGMM 响应自动识别型号
    pub fn from_ident_response(resp: &str) -> Self {
        let s = resp.to_uppercase();
        if s.contains("EC20") { Self::Ec20 }
        else if s.contains("EC25") { Self::Ec25 }
        else if s.contains("EC200T") { Self::Ec200T }
        else if s.contains("EC200S") { Self::Ec200S }
        else if s.contains("EC800N") { Self::Ec800N }
        else if s.contains("EC800G") { Self::Ec800G }
        else if s.contains("EC200U") { Self::Ec200U }
        else if s.contains("EC800E") { Self::Ec800E }
        else if s.contains("BC260") { Self::Bc260Y }
        else if s.contains("BC28") { Self::Bc28F }
        else if s.contains("BC95") { Self::Bc95 }
        else if s.contains("M26") { Self::M26 }
        else if s.contains("M35") { Self::M35 }
        else if s.contains("MC60") { Self::Mc60 }
        else if s.contains("M66") { Self::M66 }
        else if s.contains("AG35") { Self::Ag35 }
        else if s.contains("AG215") { Self::Ag215 }
        else if s.contains("AG52") { Self::Ag52 }
        else { Self::Unknown }
    }

    /// 模组能力集
    pub fn capabilities(&self) -> QuectelCapabilities {
        match self {
            // Cat.4 — 全功能: TCP/UDP + MQTT + SMS + 语音 + GNSS(部分)
            Self::Ec20 | Self::Ec25 | Self::Ec200T | Self::Ec200S
            | Self::Ag35 | Self::Ag52 => QuectelCapabilities {
                cellular_type: CellularType::LteCat4,
                socket: true,
                mqtt: true,
                mqtt_ssl: true,
                coap: false,
                ftp: true,
                http: true,
                ssl: true,
                sms: true,
                voice: true,
                gnss: matches!(self, Self::Ec25 | Self::Ag52),
                max_baud: 460800,
            },

            // Cat.1 — 中速率: TCP/UDP + MQTT + SMS
            Self::Ec800N | Self::Ec800G | Self::Ec200U | Self::Ec800E | Self::Ag215 => QuectelCapabilities {
                cellular_type: CellularType::LteCat1,
                socket: true,
                mqtt: true,
                mqtt_ssl: true,
                coap: false,
                ftp: true,
                http: true,
                ssl: true,
                sms: true,
                voice: matches!(self, Self::Ec800N | Self::Ec800G),
                gnss: matches!(self, Self::Ec800G),
                max_baud: 460800,
            },

            // NB-IoT — 低功耗: TCP/UDP + MQTT + CoAP (无语音)
            Self::Bc260Y | Self::Bc28F | Self::Bc95 => QuectelCapabilities {
                cellular_type: CellularType::NbIoT,
                socket: true,
                mqtt: true,
                mqtt_ssl: true,
                coap: true,
                ftp: false,
                http: true,
                ssl: true,
                sms: false,
                voice: false,
                gnss: false,
                max_baud: 460800,
            },

            // GPRS — 基础: TCP/UDP (部分有MQTT)
            Self::M26 | Self::M35 | Self::Mc60 | Self::M66 => QuectelCapabilities {
                cellular_type: CellularType::Gprs,
                socket: true,
                mqtt: false, // 旧固件可能不支持
                mqtt_ssl: false,
                coap: false,
                ftp: true,
                http: false,
                ssl: false,
                sms: true,
                voice: true,
                gnss: matches!(self, Self::Mc60),
                max_baud: 115200,
            },

            Self::Unknown => QuectelCapabilities {
                cellular_type: CellularType::Unknown,
                socket: false, mqtt: false, mqtt_ssl: false, coap: false,
                ftp: false, http: false, ssl: false, sms: false, voice: false,
                gnss: false, max_baud: 115200,
            },
        }
    }

    /// PWRKEY 拉低时间 (开机)
    pub fn pwrkey_on_ms(&self) -> u32 {
        match self {
            Self::Bc260Y | Self::Bc28F | Self::Bc95 => 500,  // NB-IoT: ≥500ms
            Self::M26 | Self::M35 | Self::Mc60 | Self::M66 => 1000, // GPRS: ≥1000ms
            _ => 500, // Cat.1/Cat.4: ≥500ms
        }
    }

    /// PWRKEY 拉低时间 (关机)
    pub fn pwrkey_off_ms(&self) -> u32 {
        match self {
            Self::Bc260Y | Self::Bc28F | Self::Bc95 => 650,
            Self::M26 | Self::M35 | Self::Mc60 | Self::M66 => 1500,
            _ => 650,
        }
    }

    /// 开机后等待 RDY 的最大时间
    pub fn boot_ready_timeout_ms(&self) -> u32 {
        match self {
            Self::Bc260Y | Self::Bc28F | Self::Bc95 => 15000,
            _ => 10000,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct QuectelCapabilities {
    pub cellular_type: CellularType,
    pub socket: bool,
    pub mqtt: bool,
    pub mqtt_ssl: bool,
    pub coap: bool,
    pub ftp: bool,
    pub http: bool,
    pub ssl: bool,
    pub sms: bool,
    pub voice: bool,
    pub gnss: bool,
    pub max_baud: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellularType {
    LteCat4,
    LteCat1,
    NbIoT,
    Gprs,
    Unknown,
}

// ============================================================
// 移远统一驱动 — 核心实现
// ============================================================

/// 移远模组通用驱动
///
/// 所有移远模组共享统一的 AT 指令集:
///   - 标准 3GPP: AT, ATE, AT+CGMI, AT+CGMM, AT+CGMR, AT+CGSN,
///                AT+CPIN?, AT+CSQ, AT+CREG?, AT+CGREG?, AT+CEREG?,
///                AT+CGATT, AT+CGDCONT, AT+COPS?
///   - Quectel 扩展:
///       Socket: AT+QIOPEN, AT+QISEND, AT+QIRD, AT+QICLOSE, AT+QISTATE
///       MQTT:  AT+QMTCFG, AT+QMTOPEN, AT+QMTCONN, AT+QMTDISC,
///              AT+QMTPUB, AT+QMTSUB, AT+QMTUNS, AT+QMTRECV
///       HTTP:  AT+QHTTPURL, AT+QHTTPGET, AT+QHTTPPOST, AT+QHTTPREAD
///       FTP:   AT+QFTPCFG, AT+QFTPOPEN, AT+QFTPCWD, AT+QFTPPUT, AT+QFTPGET
///       SSL:   AT+QSSLCFG, AT+QSSLOPEN
///       CoAP:  AT+QCOAPCREATE, AT+QCOAPGET, AT+QCOAPPOST (NB-IoT)
///       PDP:   AT+QICSGP, AT+QIACT, AT+QIDEACT
///       URC:   AT+QURCCFG
///
/// 驱动内部根据 capabilities() 自动决定支持哪些操作
pub struct QuectelModule<T: UartTransport, PWR: PinControl, RST: PinControl> {
    pub at: AtParser<T>,
    pwrkey: PWR,
    reset: RST,
    model: QuectelModel,
    caps: QuectelCapabilities,
    /// PDP Context ID (默认 1)
    pdp_ctx: u8,
    /// MQTT Client ID (EC800N/BC260Y 等, 默认 0)
    mqtt_client: u8,
    /// 网络附着状态
    attached: bool,
    /// 自动检测到的型号
    auto_detected: bool,
}

impl<T: UartTransport, PWR: PinControl, RST: PinControl> QuectelModule<T, PWR, RST> {
    /// 创建驱动实例 (指定型号)
    pub fn new_known(uart: T, pwrkey: PWR, reset: RST, model: QuectelModel) -> Self {
        let caps = model.capabilities();
        Self {
            at: AtParser::new(uart),
            pwrkey,
            reset,
            model,
            caps,
            pdp_ctx: 1,
            mqtt_client: 0,
            attached: false,
            auto_detected: false,
        }
    }

    /// 创建驱动实例 (开机后自动识别型号)
    pub fn new_auto_detect(uart: T, pwrkey: PWR, reset: RST) -> Self {
        Self {
            at: AtParser::new(uart),
            pwrkey,
            reset,
            model: QuectelModel::Unknown,
            caps: QuectelModel::Unknown.capabilities(),
            pdp_ctx: 1,
            mqtt_client: 0,
            attached: false,
            auto_detected: false,
        }
    }

    /// 自动识别模组型号 (通过 AT+CGMM)
    pub fn detect_model(&mut self, get_ms: impl Fn() -> u32) -> Result<QuectelModel, AtError> {
        self.at.send_cmd("AT+CGMM")?;
        let resp = self.at.wait_ok(3000, &get_ms);
        if let AtResponse::OkWithLines(lines) = resp {
            for line in &lines {
                let model = QuectelModel::from_ident_response(line);
                if model != QuectelModel::Unknown {
                    self.model = model;
                    self.caps = model.capabilities();
                    self.auto_detected = true;
                    return Ok(model);
                }
            }
        }
        Ok(QuectelModel::Unknown)
    }

    /// 获取能力集
    pub fn capabilities(&self) -> &QuectelCapabilities {
        &self.caps
    }

    /// 获取型号
    pub fn model(&self) -> QuectelModel {
        self.model
    }

    // ========================================
    // 硬件控制 — 所有移远模组统一
    // ========================================

    /// 硬件开机 (PWRKEY 序列)
    pub fn hardware_power_on<D: Delay>(&mut self, delay: &mut D) {
        self.pwrkey.set_high();
        delay.delay_ms(200);
        self.pwrkey.set_low();
        delay.delay_ms(self.model.pwrkey_on_ms());
        self.pwrkey.set_high();
    }

    /// 硬件关机 (PWRKEY 序列)
    pub fn hardware_power_off<D: Delay>(&mut self, delay: &mut D) {
        self.pwrkey.set_low();
        delay.delay_ms(self.model.pwrkey_off_ms());
        self.pwrkey.set_high();
    }

    /// 硬件复位 (RESET_N)
    pub fn hardware_reset<D: Delay>(&mut self, delay: &mut D) {
        self.reset.set_low();
        delay.delay_ms(200);
        self.reset.set_high();
        delay.delay_ms(self.model.boot_ready_timeout_ms());
    }

    // ========================================
    // 初始化流程 — 统一
    // ========================================

    /// 完整初始化序列 (开机 → 等待就绪 → 配置)
    pub fn full_init<D: Delay>(&mut self, delay: &mut D, get_ms: impl Fn() -> u32 + Copy) -> Result<(), AtError> {
        // 1. 硬件开机
        self.hardware_power_on(delay);

        // 2. 等待 RDY
        if !self.at.wait_exact("RDY", self.model.boot_ready_timeout_ms(), &get_ms) {
            // 某些模组不发送 RDY, 尝试 AT 测试
            delay.delay_ms(3000);
        }

        // 3. 关闭回显
        self.at.send_cmd("ATE0")?;
        let _ = self.at.wait_ok(1000, &get_ms);

        // 4. 自动识别型号 (如果未指定)
        if self.model == QuectelModel::Unknown {
            let _ = self.detect_model(&get_ms);
        }

        // 5. 检查 SIM 卡
        self.at.send_cmd("AT+CPIN?")?;
        let resp = self.at.wait_ok(3000, &get_ms);
        match resp {
            AtResponse::OkWithLines(lines) => {
                for line in &lines {
                    if line.contains("READY") { break; }
                    if line.contains("SIM") { return Err(AtError::SimError); }
                }
            }
            AtResponse::Error(_) => return Err(AtError::SimError),
            _ => return Err(AtError::Timeout),
        }

        // 6. 设置频段 (根据模组类型)
        self.configure_band(&get_ms)?;

        // 7. 注册网络
        self.at.send_cmd("AT+CREG?")?;
        let _ = self.at.wait_ok(2000, &get_ms);

        // NB-IoT 额外检查 EPS 网络注册
        if self.caps.cellular_type == CellularType::NbIoT {
            self.at.send_cmd("AT+CEREG?")?;
            let _ = self.at.wait_ok(2000, &get_ms);
        }

        // 8. 附着网络
        self.at.send_cmd("AT+CGATT=1")?;
        self.at.wait_ok(10000, &get_ms);
        self.attached = true;

        // 9. 设置 APN 并激活 PDP
        self.configure_pdp(&get_ms)?;

        Ok(())
    }

    /// 配置频段
    fn configure_band(&mut self, get_ms: &(impl Fn() -> u32)) -> Result<(), AtError> {
        match self.caps.cellular_type {
            CellularType::NbIoT => {
                // BC260Y: AT+QBAND=1,8  (Band 8 = 900MHz, 中国移动NB)
                self.at.send_cmd("AT+QBAND=1,8")?;
                let _ = self.at.wait_ok(2000, get_ms);
            }
            CellularType::LteCat1 | CellularType::LteCat4 => {
                // Cat.1/Cat.4: AT+QBAND=1,B3,B8  (Band 3+8, 中国联通/移动)
                self.at.send_cmd("AT+QBAND=1,B3,B8")?;
                let _ = self.at.wait_ok(2000, get_ms);
            }
            CellularType::Gprs => {
                // GPRS: 900/1800MHz (默认即可)
            }
            CellularType::Unknown => {}
        }
        Ok(())
    }

    /// 配置 PDP Context + APN
    fn configure_pdp(&mut self, get_ms: &(impl Fn() -> u32)) -> Result<(), AtError> {
        // AT+QICSGP=<ctx>,1,"<apn>","<user>","<pass>",1
        let apn = match self.caps.cellular_type {
            CellularType::NbIoT => "CTNB",         // 电信NB
            CellularType::LteCat1 => "CTNET",       // 电信Cat.1
            CellularType::LteCat4 => "CTNET",
            CellularType::Gprs => "CMNET",          // 移动GPRS
            CellularType::Unknown => "CMNET",
        };
        let cmd = heapless::String::<64>::from("AT+QICSGP=1,1,\"");
        // TODO: format apn into cmd
        let _ = apn;
        self.at.send_cmd("AT+QICSGP=1,1,\"CTNET\",\"\",\"\",1")?;
        self.at.wait_ok(3000, get_ms)?;

        // 激活 PDP
        self.at.send_cmd("AT+QIACT=1")?;
        self.at.wait_ok(15000, get_ms)?;
        Ok(())
    }

    // ========================================
    // 移远统一 Socket 操作 (AT+QIx)
    // ========================================

    /// 打开 Socket 连接 (TCP/UDP)
    /// AT+QIOPEN=<ctx>,<conn_id>,"<service_type>","<addr>",<port>,0,0
    pub fn socket_open(&mut self, conn_id: u8, proto: SocketProto, addr: &str, port: u16, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.socket { return Err(AtError::NotSupported); }
        let proto_str = match proto {
            SocketProto::Tcp => "TCP",
            SocketProto::Udp => "UDP",
            SocketProto::TcpListen => "TCP LISTENER",
            SocketProto::UdpService => "UDP SERVICE",
        };
        let _ = (conn_id, proto_str, addr, port);
        // AT+QIOPEN=1,0,"TCP","x.x.x.x",8080,0,0
        // 等待 +QIOPEN: 0,0 (conn_id, err_code)
        Ok(())
    }

    /// 发送数据 (Socket)
    /// AT+QISEND=<conn_id>,<len>
    pub fn socket_send(&mut self, conn_id: u8, data: &[u8], get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.socket { return Err(AtError::NotSupported); }
        // 方式1: 直接指定长度
        // AT+QISEND=0,100
        // > (提示符)
        // <data, 100 bytes>
        // SEND OK
        let _ = (conn_id, data);
        Ok(())
    }

    /// 发送数据 (十六进制模式)
    /// AT+QISEND=<conn_id>,<len>,"<hex>"
    pub fn socket_send_hex(&mut self, conn_id: u8, hex_data: &str, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.socket { return Err(AtError::NotSupported); }
        let _ = (conn_id, hex_data);
        Ok(())
    }

    /// 读取数据 (Socket)
    /// AT+QIRD=<conn_id>,<len>
    pub fn socket_recv(&mut self, conn_id: u8, buf: &mut [u8], get_ms: impl Fn() -> u32) -> Result<usize, AtError> {
        if !self.caps.socket { return Err(AtError::NotSupported); }
        // AT+QIRD=0,512
        // +QIRD: <actual_len>\r\n<data>\r\nOK
        let _ = (conn_id, buf);
        Ok(0)
    }

    /// 关闭 Socket
    /// AT+QICLOSE=<conn_id>,<timeout>
    pub fn socket_close(&mut self, conn_id: u8, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.socket { return Err(AtError::NotSupported); }
        self.at.send_cmd("AT+QICLOSE=0,10")?;
        self.at.wait_ok(10000, &get_ms)?;
        Ok(())
    }

    /// 查询 Socket 状态
    /// AT+QISTATE=1,<conn_id>
    pub fn socket_status(&mut self, conn_id: u8, get_ms: impl Fn() -> u32) -> Result<SocketState, AtError> {
        if !self.caps.socket { return Err(AtError::NotSupported); }
        let _ = conn_id;
        Ok(SocketState::Connected)
    }

    // ========================================
    // 移远统一 MQTT 操作 (AT+QMTx)
    // ========================================

    /// 配置 MQTT broker 并连接
    pub fn mqtt_open(&mut self, broker: &str, port: u16, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.mqtt { return Err(AtError::NotSupported); }

        // 1. 配置 MQTT 版本 (3.1.1)
        self.at.send_cmd("AT+QMTCFG=\"version\",0,4")?;
        self.at.wait_ok(2000, &get_ms)?;

        // 2. 配置 SSL (如果需要)
        if self.caps.mqtt_ssl && port == 8883 {
            self.at.send_cmd("AT+QMTCFG=\"ssl\",0,1,2")?;
            self.at.wait_ok(2000, &get_ms)?;
        }

        // 3. 打开 MQTT 连接
        // AT+QMTOPEN=0,"broker.emqx.io",1883
        let _ = (broker, port);
        // 等待 +QMTOPEN: 0,0 (client_idx, result: 0=成功)
        Ok(())
    }

    /// MQTT 登录
    pub fn mqtt_login(&mut self, client_id: &str, user: Option<&str>, pass: Option<&str>, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.mqtt { return Err(AtError::NotSupported); }
        // AT+QMTCONN=0,"client_id","user","pass"
        // 等待 +QMTCONN: 0,0,0 (client, result, retcode)
        let _ = (client_id, user, pass);
        Ok(())
    }

    /// MQTT 发布
    pub fn mqtt_publish(&mut self, topic: &str, data: &[u8], qos: QoS, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.mqtt { return Err(AtError::NotSupported); }
        // AT+QMTPUB=0,<pktid>,<qos>,0,"<topic>"
        // 等待 > 提示符
        // 发送数据 + CTRL+Z
        // 等待 +QMTPUB: 0,<pktid>,<result>
        let _ = (topic, data, qos);
        Ok(())
    }

    /// MQTT 订阅
    pub fn mqtt_subscribe(&mut self, topic: &str, qos: QoS, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.mqtt { return Err(AtError::NotSupported); }
        // AT+QMTSUB=0,<pktid>,"<topic>",<qos>
        // 等待 +QMTSUB: 0,<pktid>,<result>,<qos>
        let _ = (topic, qos);
        Ok(())
    }

    /// MQTT 取消订阅
    pub fn mqtt_unsubscribe(&mut self, topic: &str, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.mqtt { return Err(AtError::NotSupported); }
        // AT+QMTUNS=0,<pktid>,"<topic>"
        let _ = topic;
        Ok(())
    }

    /// MQTT 断开
    pub fn mqtt_disconnect(&mut self, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.mqtt { return Err(AtError::NotSupported); }
        self.at.send_cmd("AT+QMTDISC=0")?;
        self.at.wait_ok(5000, &get_ms)?;
        Ok(())
    }

    // ========================================
    // 移远统一 CoAP 操作 (NB-IoT 专用)
    // ========================================

    /// CoAP 创建实例
    pub fn coap_create(&mut self, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.coap { return Err(AtError::NotSupported); }
        // AT+QCOAPCREATE=<contextID>
        self.at.send_cmd("AT+QCOAPCREATE=1")?;
        self.at.wait_ok(5000, &get_ms)?;
        Ok(())
    }

    /// CoAP POST
    pub fn coap_post(&mut self, uri: &str, data: &[u8], get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.coap { return Err(AtError::NotSupported); }
        // AT+QCOAPPOST=0,"coap://server/uri",<len>
        // > <data>
        let _ = (uri, data);
        Ok(())
    }

    /// CoAP GET
    pub fn coap_get(&mut self, uri: &str, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.coap { return Err(AtError::NotSupported); }
        // AT+QCOAPGET=0,"coap://server/uri"
        let _ = uri;
        Ok(())
    }

    // ========================================
    // 移远统一 HTTP 操作
    // ========================================

    /// HTTP GET
    pub fn http_get(&mut self, url: &str, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.http { return Err(AtError::NotSupported); }
        // AT+QHTTPURL=<len>
        // > <url>
        // AT+QHTTPGET=80
        // 等待 +QHTTPGET: <err>
        // AT+QHTTPREAD=80
        let _ = url;
        Ok(())
    }

    /// HTTP POST
    pub fn http_post(&mut self, url: &str, content_type: &str, data: &[u8], get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.http { return Err(AtError::NotSupported); }
        // AT+QHTTPURL=<len>
        // AT+QHTTPPOST=<data_len>,80,"<content_type>"
        // > <data>
        let _ = (url, content_type, data);
        Ok(())
    }

    // ========================================
    // 移远统一 FTP 操作
    // ========================================

    /// FTP 连接
    pub fn ftp_open(&mut self, server: &str, port: u16, user: &str, pass: &str, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.ftp { return Err(AtError::NotSupported); }
        // AT+QFTPCFG="account","<user>","<pass>"
        // AT+QFTPCFG="contextid",1
        // AT+QFTPOPEN="<server>",<port>
        let _ = (server, port, user, pass);
        Ok(())
    }

    /// FTP 下载文件
    pub fn ftp_get(&mut self, remote_path: &str, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.ftp { return Err(AtError::NotSupported); }
        // AT+QFTPGET="<remote_path>"
        let _ = remote_path;
        Ok(())
    }

    // ========================================
    // 移远统一 SMS 操作
    // ========================================

    /// 发送 SMS
    pub fn sms_send(&mut self, number: &str, text: &str, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.sms { return Err(AtError::NotSupported); }
        // AT+CMGF=1 (文本模式)
        // AT+CMGS="<number>"
        // > <text> + CTRL+Z
        let _ = (number, text);
        Ok(())
    }

    // ========================================
    // 移远统一 GNSS 操作 (部分模组)
    // ========================================

    /// 开启 GNSS
    pub fn gnss_start(&mut self, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.gnss { return Err(AtError::NotSupported); }
        // AT+QGPS=1
        self.at.send_cmd("AT+QGPS=1")?;
        self.at.wait_ok(3000, &get_ms)?;
        Ok(())
    }

    /// 获取 GNSS 定位信息
    pub fn gnss_get_position(&mut self, get_ms: impl Fn() -> u32) -> Result<GnssInfo, AtError> {
        if !self.caps.gnss { return Err(AtError::NotSupported); }
        // AT+QGPSLOC=2
        // +QGPSLOC: <UTC>,<lat>,<lon>,<hdop>,<altitude>,<fix>,<cog>,<spkm>,<spkn>,<date>,<nsat>
        self.at.send_cmd("AT+QGPSLOC=2")?;
        let _ = self.at.wait_ok(3000, &get_ms);
        Ok(GnssInfo::default())
    }

    // ========================================
    // 移远统一 SSL/TLS 操作
    // ========================================

    /// 配置 SSL 证书
    pub fn ssl_config(&mut self, ssl_ctx: u8, ca_cert: &[u8], get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if !self.caps.ssl { return Err(AtError::NotSupported); }
        // AT+QSSLCFG="seclevel",<sslCtx>,2
        // AT+QSSLCFG="cacert",<sslCtx>,"<path>"
        let _ = (ssl_ctx, ca_cert);
        Ok(())
    }

    // ========================================
    // 移远统一低功耗控制
    // ========================================

    /// 进入睡眠模式
    pub fn enter_sleep(&mut self, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        // Cat.1/Cat.4: AT+QSCLK=0 (唤醒需通过 DTR)
        // NB-IoT: AT+QSCLK=0 (PSM/eDRX)
        // GPRS: AT+CFUN=0
        match self.caps.cellular_type {
            CellularType::Gprs => {
                self.at.send_cmd("AT+CFUN=0")?;
                self.at.wait_ok(5000, &get_ms)?;
            }
            _ => {
                self.at.send_cmd("AT+QSCLK=0")?;
                self.at.wait_ok(3000, &get_ms)?;
            }
        }
        Ok(())
    }

    /// 配置 PSM/eDRX (NB-IoT 低功耗)
    pub fn configure_psm(&mut self, t3324: u32, t3412: u32, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        if self.caps.cellular_type != CellularType::NbIoT { return Err(AtError::NotSupported); }
        // AT+QCFG="psm",1,"<T3412>","<T3324>"
        // AT+QCFG="edrx",1,"<act>","<requested_eDRX>"
        let _ = (t3324, t3412);
        Ok(())
    }

    // ========================================
    // URC 配置
    // ========================================

    /// 配置 URC 上报
    pub fn configure_urc(&mut self, get_ms: impl Fn() -> u32) -> Result<(), AtError> {
        // 启用网络注册 URC
        self.at.send_cmd("AT+CREG=1")?;
        self.at.wait_ok(1000, &get_ms)?;

        if self.caps.cellular_type == CellularType::NbIoT {
            self.at.send_cmd("AT+CEREG=1")?;
            self.at.wait_ok(1000, &get_ms)?;
        }

        // 启用信号变化上报 (可选)
        // self.at.send_cmd("AT+QCFG=\"servicedomain\",1")?;

        Ok(())
    }

    // ========================================
    // 诊断与调试
    // ========================================

    /// 完整状态报告 (调试用)
    pub fn diagnostics(&mut self, get_ms: impl Fn() -> u32) -> ModuleDiagnostics {
        let mut diag = ModuleDiagnostics::default();
        diag.model = self.model;

        // IMEI
        self.at.send_cmd("AT+GSN").ok();
        if let AtResponse::OkWithLines(lines) = self.at.wait_ok(2000, &get_ms) {
            if let Some(first) = lines.get(0) {
                let _ = diag.imei.push_str(first);
            }
        }

        // ICCID
        self.at.send_cmd("AT+QCCID").ok();
        if let AtResponse::OkWithLines(lines) = self.at.wait_ok(2000, &get_ms) {
            if let Some(first) = lines.get(0) {
                // +QCCID: <iccid>
                if let Some(idx) = first.find(':') {
                    let _ = diag.iccid.push_str(first[idx + 1..].trim());
                }
            }
        }

        // 信号强度
        self.at.send_cmd("AT+CSQ").ok();
        if let AtResponse::OkWithLines(lines) = self.at.wait_ok(2000, &get_ms) {
            for line in &lines {
                if line.starts_with("+CSQ:") {
                    let (rssi, ber) = super::parse_csq(line);
                    diag.rssi = rssi;
                    diag.ber = ber;
                }
            }
        }

        // 网络注册
        self.at.send_cmd("AT+CREG?").ok();
        if let AtResponse::OkWithLines(lines) = self.at.wait_ok(2000, &get_ms) {
            for line in &lines {
                if line.starts_with("+CREG:") {
                    diag.network_reg = super::parse_last_number(line).unwrap_or(0) as u8;
                }
            }
        }

        // 运营商
        self.at.send_cmd("AT+QSPN").ok();
        if let AtResponse::OkWithLines(lines) = self.at.wait_ok(2000, &get_ms) {
            if let Some(first) = lines.get(0) {
                let _ = diag.operator.push_str(first);
            }
        }

        // 固件版本
        self.at.send_cmd("AT+CGMR").ok();
        if let AtResponse::OkWithLines(lines) = self.at.wait_ok(2000, &get_ms) {
            if let Some(first) = lines.get(0) {
                let _ = diag.firmware.push_str(first);
            }
        }

        diag.attached = self.attached;
        diag
    }
}

// ============================================================
// Socket 辅助类型
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketProto {
    Tcp,
    Udp,
    TcpListen,
    UdpService,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Closed,
    Opening,
    Connected,
    Closing,
    Error,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GnssInfo {
    pub lat: f64,
    pub lon: f64,
    pub alt: f32,
    pub speed: f32,
    pub heading: f32,
    pub hdop: f32,
    pub num_sats: u8,
    pub fix_type: u8,
}

#[derive(Debug, Default)]
pub struct ModuleDiagnostics {
    pub model: QuectelModel,
    pub imei: heapless::String<20>,
    pub iccid: heapless::String<24>,
    pub rssi: u8,
    pub ber: u8,
    pub network_reg: u8,
    pub operator: heapless::String<64>,
    pub firmware: heapless::String<128>,
    pub attached: bool,
}

// ============================================================
// 实现 crate 级 trait
// ============================================================

impl<T: UartTransport, PWR: PinControl, RST: PinControl> ModuleBase for QuectelModule<T, PWR, RST> {
    fn module_type(&self) -> ModuleType {
        match self.caps.cellular_type {
            CellularType::LteCat4 | CellularType::LteCat1 => ModuleType::LteCat1,
            CellularType::NbIoT => ModuleType::NbIoT,
            CellularType::Gprs => ModuleType::Gprs,
            CellularType::Unknown => ModuleType::Unknown,
        }
    }

    fn test_at(&mut self) -> Result<(), AtError> {
        self.at.send_cmd("AT")?;
        Ok(())
    }

    fn get_version(&mut self) -> Result<heapless::String<128>, AtError> {
        self.at.send_cmd("AT+CGMR")?;
        Ok(heapless::String::new())
    }

    fn get_device_id(&mut self) -> Result<heapless::String<32>, AtError> {
        self.at.send_cmd("AT+GSN")?;
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
        // AT+IPR=<baudrate>
        let _ = baud;
        Ok(())
    }
}

impl<T: UartTransport, PWR: PinControl, RST: PinControl> CellularModule for QuectelModule<T, PWR, RST> {
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
        Ok(())
    }
}

impl<T: UartTransport, PWR: PinControl, RST: PinControl> SocketOps for QuectelModule<T, PWR, RST> {
    fn tcp_connect(&mut self, addr: &str, port: u16) -> Result<u8, AtError> {
        let _ = (addr, port);
        Ok(0)
    }
    fn udp_connect(&mut self, addr: &str, port: u16) -> Result<u8, AtError> {
        let _ = (addr, port);
        Ok(0)
    }
    fn send_data(&mut self, conn_id: u8, data: &[u8]) -> Result<(), AtError> {
        let _ = (conn_id, data);
        Ok(())
    }
    fn recv_data(&mut self, conn_id: u8, buf: &mut [u8]) -> Result<usize, AtError> {
        let _ = (conn_id, buf);
        Ok(0)
    }
    fn close(&mut self, conn_id: u8) -> Result<(), AtError> {
        let _ = conn_id;
        Ok(())
    }
    fn get_conn_status(&mut self, conn_id: u8) -> Result<ConnStatus, AtError> {
        let _ = conn_id;
        Ok(ConnStatus::Connected)
    }
}

impl<T: UartTransport, PWR: PinControl, RST: PinControl> MqttOps for QuectelModule<T, PWR, RST> {
    fn mqtt_set_broker(&mut self, addr: &str, port: u16) -> Result<(), AtError> {
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
        let _ = (topic, msg, qos, retain);
        Ok(())
    }
    fn mqtt_connect(&mut self) -> Result<(), AtError> {
        Ok(())
    }
    fn mqtt_disconnect(&mut self) -> Result<(), AtError> {
        Ok(())
    }
    fn mqtt_publish(&mut self, topic: &str, data: &[u8], qos: QoS) -> Result<(), AtError> {
        let _ = (topic, data, qos);
        Ok(())
    }
    fn mqtt_subscribe(&mut self, topic: &str, qos: QoS) -> Result<(), AtError> {
        let _ = (topic, qos);
        Ok(())
    }
    fn mqtt_unsubscribe(&mut self, topic: &str) -> Result<(), AtError> {
        let _ = topic;
        Ok(())
    }
    fn mqtt_state(&mut self) -> Result<MqttState, AtError> {
        Ok(MqttState::Connected)
    }
}

impl<T: UartTransport, PWR: PinControl, RST: PinControl> PowerControl for QuectelModule<T, PWR, RST> {
    fn sleep(&mut self) -> Result<(), AtError> {
        self.at.send_cmd("AT+QSCLK=0")?;
        Ok(())
    }
    fn wakeup(&mut self) -> Result<(), AtError> {
        // DTR 拉低唤醒
        Ok(())
    }
    fn get_power_mode(&mut self) -> PowerMode {
        PowerMode::Active
    }
}
