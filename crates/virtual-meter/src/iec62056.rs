//! IEC 62056-21 红外协议模拟
//!
//! 标识转换, 波特率协商, 数据读取

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
            300 => Some(Self::B300), 600 => Some(Self::B600),
            1200 => Some(Self::B1200), 2400 => Some(Self::B2400),
            4800 => Some(Self::B4800), 9600 => Some(Self::B9600),
            _ => None,
        }
    }
    pub fn value(&self) -> u32 { *self as u32 }
}

/// 电表标识报文
#[derive(Debug, Clone)]
pub struct MeterIdentification {
    pub manufacturer: String,    // 3 chars
    pub baud_rate: BaudRate,
    pub identification: String,  // up to 16 chars
}

impl Default for MeterIdentification {
    fn default() -> Self {
        Self {
            manufacturer: "FEM".to_string(),
            baud_rate: BaudRate::B9600,
            identification: "FeMeter Virtual 1.0".to_string(),
        }
    }
}

impl MeterIdentification {
    /// 编码标识报文 (IEC 62056-21 format)
    pub fn encode(&self) -> String {
        format!("/{}{}\r\n", self.manufacturer, self.baud_rate.value() / 100)
    }
}

/// IEC 62056-21 协议处理器
pub struct Iec62056Processor {
    id: MeterIdentification,
    baud_rate: BaudRate,
}

impl Default for Iec62056Processor {
    fn default() -> Self { Self::new() }
}

impl Iec62056Processor {
    pub fn new() -> Self {
        Self { id: MeterIdentification::default(), baud_rate: BaudRate::B300 }
    }

    pub fn set_baud_rate(&mut self, baud: BaudRate) { self.baud_rate = baud; }
    pub fn current_baud_rate(&self) -> BaudRate { self.baud_rate }

    /// 处理标识阶段 (接收标识请求, 返回标识报文)
    pub fn handle_identification(&self) -> String {
        self.id.encode()
    }

    /// 处理波特率协商 (ACK + 选择波特率)
    pub fn handle_baud_rate_ack(&mut self, requested: BaudRate) -> String {
        self.baud_rate = requested;
        "\x06".to_string() // ACK
    }

    /// 处理数据读取命令 (格式: 命令代码)
    /// 返回数据行
    pub fn handle_read_command(&self, command: &str) -> Result<String> {
        // Simplified: return stub data for common commands
        match command {
            "0.0.0" => Ok(format!("F.F({:.2}*kWh)\r\n", 0.0)),
            "1.8.0" => Ok(format!("({:.2}*kWh)\r\n", 100.0)),
            "1.8.1" => Ok(format!("({:.2}*kWh)\r\n", 50.0)),
            "1.8.2" => Ok(format!("({:.2}*kWh)\r\n", 50.0)),
            "32.7.0" => Ok(format!("({:.1}*V)\r\n", 220.0)),
            "31.7.0" => Ok(format!("({:.2}*A)\r\n", 5.0)),
            "!" => Ok("\r\n".to_string()), // end of data
            _ => Err(anyhow::anyhow!("unknown command: {}", command)),
        }
    }

    /// 完整协议流程: 标识 -> 协商 -> 数据读取
    pub fn process_input(&mut self, input: &str) -> Vec<String> {
        let mut responses = Vec::new();
        let input = input.trim();

        if input.is_empty() {
            // 标识请求 (任意字符触发)
            responses.push(self.handle_identification());
        } else if input == "\x06" {
            // Second ACK, enter data mode
            responses.push("OK\r\n".to_string());
        } else if let Ok(baud_val) = input.parse::<u32>() {
            if let Some(baud) = BaudRate::from_value(baud_val) {
                responses.push(self.handle_baud_rate_ack(baud));
            }
        } else {
            // Data read command
            match self.handle_read_command(input) {
                Ok(resp) => responses.push(resp),
                Err(_) => responses.push("ERR\r\n".to_string()),
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
    }

    #[test]
    fn test_baud_rate_negotiation() {
        let mut proc = Iec62056Processor::new();
        let resp = proc.handle_baud_rate_ack(BaudRate::B9600);
        assert_eq!(resp, "\x06");
        assert_eq!(proc.current_baud_rate(), BaudRate::B9600);
    }

    #[test]
    fn test_read_command() {
        let proc = Iec62056Processor::new();
        let resp = proc.handle_read_command("1.8.0").unwrap();
        assert!(resp.contains("kWh"));
    }

    #[test]
    fn test_end_command() {
        let proc = Iec62056Processor::new();
        let resp = proc.handle_read_command("!").unwrap();
        assert_eq!(resp, "\r\n");
    }
}
