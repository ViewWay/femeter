//! IEC 62056-21 红外协议模拟 (增强版)
//!
//! 完整协议数据单元格式, 波特率切换握手, 标准数据标识读取

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaudRate {
    B300 = 300,
    B600 = 600,
    B1200 = 1200,
    B2400 = 2400,
    B4800 = 4800,
    B9600 = 9600,
}

impl BaudRate {
    pub fn from_value(v: u32) -> Option<Self> {
        match v {
            300 => Some(Self::B300),
            600 => Some(Self::B600),
            1200 => Some(Self::B1200),
            2400 => Some(Self::B2400),
            4800 => Some(Self::B4800),
            9600 => Some(Self::B9600),
            _ => None,
        }
    }
    pub fn value(&self) -> u32 {
        *self as u32
    }

    /// 所有支持的波特率
    #[allow(dead_code)]
    pub fn all() -> &'static [BaudRate] {
        &[
            Self::B300,
            Self::B600,
            Self::B1200,
            Self::B2400,
            Self::B4800,
            Self::B9600,
        ]
    }

    /// 波特率识别码 (标识报文中使用的)
    #[allow(dead_code)]
    pub fn id_char(&self) -> char {
        match self {
            Self::B300 => '0',
            Self::B600 => '1',
            Self::B1200 => '2',
            Self::B2400 => '3',
            Self::B4800 => '4',
            Self::B9600 => '5',
        }
    }

    #[allow(dead_code)]
    pub fn from_id_char(c: char) -> Option<Self> {
        match c {
            '0' => Some(Self::B300),
            '1' => Some(Self::B600),
            '2' => Some(Self::B1200),
            '3' => Some(Self::B2400),
            '4' => Some(Self::B4800),
            '5' => Some(Self::B9600),
            _ => None,
        }
    }
}

/// 电表标识报文
#[derive(Debug, Clone)]
pub struct MeterIdentification {
    pub manufacturer: String,   // 3 chars (max)
    pub baud_rate: BaudRate,    // 协商后使用的波特率
    pub identification: String, // up to 16 chars
    pub firmware_version: String,
}

impl Default for MeterIdentification {
    fn default() -> Self {
        Self {
            manufacturer: "FEM".to_string(),
            baud_rate: BaudRate::B9600,
            identification: "FeMeter Virtual 1.0".to_string(),
            firmware_version: "1.0.0".to_string(),
        }
    }
}

impl MeterIdentification {
    /// 编码标识报文 (IEC 62056-21 格式 A)
    /// 格式: /XXXZZ\r\n
    /// XXX = manufacturer (3 uppercase), ZZ = baud rate ID
    pub fn encode(&self) -> String {
        format!("/{}{}\r\n", self.manufacturer, self.baud_rate.id_char())
    }

    /// 编码标识报文 (格式 B - 完整)
    /// 格式: /XXXZZ VV.VV\r\n
    #[allow(dead_code)]
    pub fn encode_full(&self) -> String {
        format!(
            "/{}{} {}\r\n",
            self.manufacturer,
            self.baud_rate.id_char(),
            self.identification
        )
    }
}

/// 协议状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ProtocolState {
    Idle,
    IdentificationSent,
    WaitingForAck,
    DataMode,
}

/// IEC 62056-21 协议处理器 (增强版)
pub struct Iec62056Processor {
    id: MeterIdentification,
    baud_rate: BaudRate,
    state: ProtocolState,
}

impl Default for Iec62056Processor {
    fn default() -> Self {
        Self::new()
    }
}

impl Iec62056Processor {
    pub fn new() -> Self {
        Self {
            id: MeterIdentification::default(),
            baud_rate: BaudRate::B300,
            state: ProtocolState::Idle,
        }
    }

    pub fn set_baud_rate(&mut self, baud: BaudRate) {
        self.baud_rate = baud;
    }
    pub fn current_baud_rate(&self) -> BaudRate {
        self.baud_rate
    }
    #[allow(dead_code)]
    pub fn state(&self) -> ProtocolState {
        self.state
    }

    /// 处理标识阶段 (接收标识请求, 返回标识报文)
    pub fn handle_identification(&mut self) -> String {
        self.state = ProtocolState::IdentificationSent;
        self.id.encode()
    }

    /// 处理波特率协商 (ACK + 选择波特率)
    /// 返回 (ACK 字符, 建议的新波特率)
    pub fn handle_baud_rate_ack(&mut self, requested: BaudRate) -> (String, Option<BaudRate>) {
        // 协议: 电表回复 ACK 表示接受, 然后切换到协商后的波特率
        // 如果请求的波特率不支持, 使用默认 9600
        let new_baud = if BaudRate::from_value(requested.value()).is_some() {
            requested
        } else {
            BaudRate::B9600
        };
        self.baud_rate = new_baud;
        self.state = ProtocolState::WaitingForAck;
        ("\x06".to_string(), Some(new_baud))
    }

    /// 进入数据模式
    #[allow(dead_code)]
    fn enter_data_mode(&mut self) {
        self.state = ProtocolState::DataMode;
    }

    /// 处理数据读取命令
    /// 支持 IEC 62056-21 标准数据标识 (D代码)
    pub fn handle_read_command(&self, command: &str) -> Result<String> {
        let cmd = command.trim();

        // 结束符
        if cmd == "!" {
            return Ok("\r\n!\r\n".to_string());
        }

        // 标准数据标识读取 (C代码格式: XXXXX)
        // 电能量
        match cmd {
            // 总有功电能 (kWh)
            "0.0.0" => Ok(format!("F.F({:.2}*kWh)\r\n", 0.0)),
            "1.8.0" => Ok(format!("({:.2}*kWh)\r\n", 100.0)),
            // 分费率有功电能
            "1.8.1" => Ok(format!("({:.2}*kWh)\r\n", 50.0)),
            "1.8.2" => Ok(format!("({:.2}*kWh)\r\n", 30.0)),
            "1.8.3" => Ok(format!("({:.2}*kWh)\r\n", 20.0)),
            "1.8.4" => Ok(format!("({:.2}*kWh)\r\n", 0.0)),
            // 总无功电能 (kvarh)
            "2.8.0" => Ok(format!("({:.2}*kvarh)\r\n", 10.0)),
            // 电压 (V)
            "32.7.0" => Ok(format!("({:.1}*V)\r\n", 220.0)),
            "52.7.0" => Ok(format!("({:.1}*V)\r\n", 221.0)),
            "72.7.0" => Ok(format!("({:.1}*V)\r\n", 219.5)),
            "34.7.0" => Ok(format!("({:.1}*V)\r\n", 380.5)), // 线电压 AB
            // 电流 (A)
            "31.7.0" => Ok(format!("({:.2}*A)\r\n", 5.0)),
            "51.7.0" => Ok(format!("({:.2}*A)\r\n", 4.8)),
            "71.7.0" => Ok(format!("({:.2}*A)\r\n", 5.2)),
            // 功率 (W/kW)
            "1.7.0" => Ok(format!("({:.2}*kW)\r\n", 1.1)),
            "2.7.0" => Ok(format!("({:.2}*kvar)\r\n", 0.1)),
            // 功率因数
            "13.7.0" => Ok(format!("({:.3})\r\n", 0.956)),
            // 频率 (Hz)
            "14.7.0" => Ok(format!("({:.2}*Hz)\r\n", 50.0)),
            // 需量 (kW)
            "1.6.0" => Ok(format!("({:.2}*kW)\r\n", 1.5)),
            // 最大需量
            "1.6.1" => Ok(format!("({:.2}*kW)\r\n", 2.3)),
            // 费率号
            "96.10.1" => Ok("(03)\r\n".to_string()),
            // 时钟
            "0.0.1" => Ok(format!(
                "({} {})\r\n",
                chrono::Local::now().format("%y%m%d"),
                chrono::Local::now().format("%H%M%S")
            )),
            // 表号
            "0.0.96.1.255" => Ok("(FEM001234567890)\r\n".to_string()),
            // CRC 校验请求
            "0.0.96.5.5" => Ok("(0000)\r\n".to_string()),
            _ => Err(anyhow::anyhow!("unknown command: {}", cmd)),
        }
    }

    /// 批量读取 (发送请求报文模式)
    #[allow(dead_code)]
    pub fn handle_read_request(&self, commands: &[&str]) -> Vec<String> {
        let mut results = Vec::new();
        for cmd in commands {
            match self.handle_read_command(cmd) {
                Ok(resp) => results.push(resp),
                Err(_) => results.push("ERR\r\n".to_string()),
            }
        }
        results.push("\r\n!\r\n".to_string());
        results
    }

    /// 完整协议流程
    pub fn process_input(&mut self, input: &str) -> Vec<String> {
        let mut responses = Vec::new();
        let input = input.trim();

        if input.is_empty() || input == "\x06" && self.state == ProtocolState::WaitingForAck {
            // 进入数据模式
            self.enter_data_mode();
            responses.push("OK\r\n".to_string());
        } else if input == "\x06" && self.state == ProtocolState::Idle {
            // 标识请求
            responses.push(self.handle_identification());
        } else if let Some(c) = input.chars().next() {
            if let Some(baud) = BaudRate::from_id_char(c) {
                // 波特率协商 (用 ID 字符)
                let (ack, new_baud) = self.handle_baud_rate_ack(baud);
                responses.push(ack);
                if let Some(b) = new_baud {
                    responses.push(format!("[BAUD:{}]\r\n", b.value()));
                }
            } else if let Ok(baud_val) = input.parse::<u32>() {
                if let Some(baud) = BaudRate::from_value(baud_val) {
                    let (ack, new_baud) = self.handle_baud_rate_ack(baud);
                    responses.push(ack);
                    if let Some(b) = new_baud {
                        responses.push(format!("[BAUD:{}]\r\n", b.value()));
                    }
                }
            } else {
                // Data read command
                match self.handle_read_command(input) {
                    Ok(resp) => responses.push(resp),
                    Err(_) => responses.push("ERR\r\n".to_string()),
                }
            }
        }

        responses
    }
}

pub fn create_iec_processor() -> Iec62056Processor {
    Iec62056Processor::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_identification() {
        let id = MeterIdentification::default();
        let encoded = id.encode();
        assert!(encoded.starts_with("/FEM"));
        assert!(encoded.ends_with("\r\n"));
        // 应包含波特率 ID
        assert!(encoded.contains('5')); // 9600 -> '5'
    }

    #[test]
    fn test_baud_rate_negotiation() {
        let mut proc = Iec62056Processor::new();
        let (resp, new_baud) = proc.handle_baud_rate_ack(BaudRate::B9600);
        assert_eq!(resp, "\x06");
        assert_eq!(new_baud, Some(BaudRate::B9600));
        assert_eq!(proc.current_baud_rate(), BaudRate::B9600);
    }

    #[test]
    fn test_baud_rate_from_char() {
        assert_eq!(BaudRate::from_id_char('0'), Some(BaudRate::B300));
        assert_eq!(BaudRate::from_id_char('5'), Some(BaudRate::B9600));
        assert_eq!(BaudRate::from_id_char('9'), None);
    }

    #[test]
    fn test_read_energy() {
        let proc = Iec62056Processor::new();
        let resp = proc.handle_read_command("1.8.0").unwrap();
        assert!(resp.contains("kWh"));
    }

    #[test]
    fn test_read_voltage() {
        let proc = Iec62056Processor::new();
        let resp = proc.handle_read_command("32.7.0").unwrap();
        assert!(resp.contains("V"));
    }

    #[test]
    fn test_read_current() {
        let proc = Iec62056Processor::new();
        let resp = proc.handle_read_command("31.7.0").unwrap();
        assert!(resp.contains("A"));
    }

    #[test]
    fn test_read_power() {
        let proc = Iec62056Processor::new();
        let resp = proc.handle_read_command("1.7.0").unwrap();
        assert!(resp.contains("kW"));
    }

    #[test]
    fn test_read_frequency() {
        let proc = Iec62056Processor::new();
        let resp = proc.handle_read_command("14.7.0").unwrap();
        assert!(resp.contains("Hz"));
    }

    #[test]
    fn test_read_tariff() {
        let proc = Iec62056Processor::new();
        let resp = proc.handle_read_command("96.10.1").unwrap();
        assert!(resp.contains("03"));
    }

    #[test]
    fn test_end_command() {
        let proc = Iec62056Processor::new();
        let resp = proc.handle_read_command("!").unwrap();
        assert!(resp.contains("!"));
    }

    #[test]
    fn test_unknown_command() {
        let proc = Iec62056Processor::new();
        assert!(proc.handle_read_command("999.999").is_err());
    }

    #[test]
    fn test_batch_read() {
        let proc = Iec62056Processor::new();
        let results = proc.handle_read_request(&["1.8.0", "32.7.0", "31.7.0"]);
        assert_eq!(results.len(), 4); // 3 data + 1 end marker
    }

    #[test]
    fn test_full_identification() {
        let id = MeterIdentification::default();
        let encoded = id.encode_full();
        assert!(encoded.contains("FeMeter Virtual 1.0"));
    }
}
