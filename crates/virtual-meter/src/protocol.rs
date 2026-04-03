//! 串口文本协议解析/响应
//!
//! 简单文本协议:
//! - READ <addr_hex>          读寄存器 -> 返回 24bit hex
//! - WRITE <addr_hex> <data>  写寄存器
//! - SNAPSHOT                 读取全部计量数据 (JSON)
//! - ID                       读 Device ID
//! - RESET                    软件复位
//!
//! 响应:
//! - OK <data_hex>            成功
//! - ERR <msg>                失败
//! - DATA {...}               JSON 快照

use crate::{ChipType, MeterHandle};

/// 协议处理器
pub struct ProtocolHandler {
    meter: MeterHandle,
}

impl ProtocolHandler {
    /// 创建协议处理器
    pub fn new(meter: MeterHandle) -> Self {
        Self { meter }
    }

    /// 处理一行命令
    pub fn handle_line(&self, line: &str) -> String {
        let line = line.trim();
        if line.is_empty() {
            return String::new();
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return "ERR empty command".to_string();
        }

        match parts[0].to_uppercase().as_str() {
            "READ" => self.handle_read(&parts[1..]),
            "WRITE" => self.handle_write(&parts[1..]),
            "SNAPSHOT" => self.handle_snapshot(),
            "ID" => self.handle_id(),
            "RESET" => self.handle_reset(),
            "HELP" => self.handle_help(),
            _ => format!("ERR unknown command: {}", parts[0]),
        }
    }

    /// 读寄存器
    fn handle_read(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "ERR READ requires address".to_string();
        }

        let addr_str = if args[0].starts_with("0x") || args[0].starts_with("0X") {
            &args[0][2..]
        } else {
            args[0]
        };
        let addr = match u16::from_str_radix(addr_str, 16) {
            Ok(v) => v,
            Err(_) => return format!("ERR invalid address: {}", args[0]),
        };

        let mut meter = self.meter.lock().unwrap();
        let value = meter.format_register(addr);
        format!("OK {}", value)
    }

    /// 写寄存器 (简化实现，仅支持部分寄存器)
    fn handle_write(&self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "ERR WRITE requires address and data".to_string();
        }

        let addr_str = if args[0].starts_with("0x") || args[0].starts_with("0X") {
            &args[0][2..]
        } else {
            args[0]
        };
        let addr = match u16::from_str_radix(addr_str, 16) {
            Ok(v) => v,
            Err(_) => return format!("ERR invalid address: {}", args[0]),
        };

        let data_str = if args[1].starts_with("0x") || args[1].starts_with("0X") {
            &args[1][2..]
        } else {
            args[1]
        };
        let _data = match u32::from_str_radix(data_str, 16) {
            Ok(v) => v,
            Err(_) => return format!("ERR invalid data: {}", args[1]),
        };

        let mut meter = self.meter.lock().unwrap();

        // 简化的写操作 - 实际应用中可扩展
        match addr {
            0x10 => {
                // 设置芯片类型
                meter.set_chip(ChipType::ATT7022E);
                "OK".to_string()
            }
            0x11 => {
                meter.set_chip(ChipType::RN8302B);
                "OK".to_string()
            }
            0xF0 => {
                meter.reset_energy();
                "OK RESET".to_string()
            }
            _ => format!("ERR write not supported for address {:02X}", addr),
        }
    }

    /// 获取 JSON 快照
    fn handle_snapshot(&self) -> String {
        let mut meter = self.meter.lock().unwrap();
        let snapshot = meter.snapshot();

        match serde_json::to_string(&snapshot) {
            Ok(json) => format!("DATA {}", json),
            Err(e) => format!("ERR serialization failed: {}", e),
        }
    }

    /// 获取设备 ID
    fn handle_id(&self) -> String {
        let mut meter = self.meter.lock().unwrap();
        let chip = meter.config().chip;
        let id = meter.format_register(0xFF);
        drop(meter);

        let chip_name = match chip {
            ChipType::ATT7022E => "ATT7022E",
            ChipType::RN8302B => "RN8302B",
        };

        format!("OK {} {}", chip_name, id)
    }

    /// 软件复位
    fn handle_reset(&self) -> String {
        let mut meter = self.meter.lock().unwrap();
        meter.reset_energy();
        "OK RESET".to_string()
    }

    /// 帮助信息
    fn handle_help(&self) -> String {
        let help = r#"Virtual Meter Protocol v0.1
Commands:
  READ <addr_hex>          - Read register (returns 24-bit hex)
  WRITE <addr_hex> <data>  - Write register
  SNAPSHOT                 - Get full JSON snapshot
  ID                       - Get device ID
  RESET                    - Reset energy counters
  HELP                     - This message

Register Map:
  0x00-0x02: Voltage A/B/C (mV)
  0x03-0x05: Current A/B/C (mA)
  0x06-0x08: Power A/B/C (cW)
  0x09:      Total Power (cW)
  0x0A:      Frequency (cHz)
  0x0B-0x0D: Energy A/B/C (cWh)
  0x0E:      Total Energy (cWh)
  0xFF:      Chip ID
"#;
        format!("OK\n{}", help)
    }
}

/// 创建协议处理器
pub fn create_protocol_handler(meter: MeterHandle) -> ProtocolHandler {
    ProtocolHandler::new(meter)
}
