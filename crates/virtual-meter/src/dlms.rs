//! DLMS/COSEM 协议模拟
//!
//! HDLC 帧封装/解析, APDU 编解码, 核心 COSEM 对象

use crate::MeterHandle;
use chrono::{Datelike, Timelike};
use anyhow::Result;

/// HDLC 帧封装
pub struct HdlcFrame;

impl HdlcFrame {
    const FLAG: u8 = 0x7E;
    const ESCAPE: u8 = 0x7D;
    const XFLAG: u8 = 0x5E;
    const XESCAPE: u8 = 0x5D;

    pub fn encode(server_addr: u16, client_addr: u16, apdu: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(Self::FLAG);

        // Address: client(1byte low) + server(2bytes low,high)
        frame.push(client_addr as u8);
        frame.push((server_addr & 0xFF) as u8);
        frame.push(((server_addr >> 8) & 0x0F) as u8);

        // Frame control: I-frame S-frame U-frame
        // Use I-frame: 0xA3 (no ack)
        frame.push(0xA3);
        frame.push(0x00); // HCS placeholder
        frame.push(0x00);

        // Information field
        for &b in apdu {
            if b == Self::FLAG || b == Self::ESCAPE {
                frame.push(Self::ESCAPE);
                frame.push(b ^ 0x20);
            } else {
                frame.push(b);
            }
        }

        // FCS (simplified: just zeros)
        frame.push(0x00);
        frame.push(0x00);
        frame.push(Self::FLAG);
        frame
    }

    pub fn decode(data: &[u8]) -> Result<(u16, u16, Vec<u8>)> {
        if data.len() < 8 { return Err(anyhow::anyhow!("frame too short")); }
        if data[0] != Self::FLAG || data[data.len()-1] != Self::FLAG {
            return Err(anyhow::anyhow!("invalid flags"));
        }

        let client_addr = data[1] as u16;
        let server_addr = data[2] as u16 | ((data[3] as u16) << 8);

        // Unescape information field (after addr+ctrl+hcs, before fcs+flag)
        let info_start = 7; // after FLAG(1) + addr(3) + ctrl(1) + hcs(2)
        let info_end = data.len() - 3; // before fcs(2) + FLAG(1)
        let mut info = Vec::new();
        let mut i = info_start;
        while i < info_end {
            if data[i] == Self::ESCAPE && i + 1 < info_end {
                info.push(data[i+1] ^ 0x20);
                i += 2;
            } else {
                info.push(data[i]);
                i += 1;
            }
        }

        Ok((server_addr, client_addr, info))
    }
}

/// COSEM OBIS 路径
#[derive(Debug, Clone, PartialEq)]
pub struct ObisPath(pub u8, pub u8, pub u8, pub u8, pub u8);

impl std::fmt::Display for ObisPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}.{}.{}", self.0, self.1, self.2, self.3, self.4)
    }
}

impl ObisPath {
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() >= 6 { Some(Self(b[0], b[1], b[2], b[3], b[4])) } else { None }
    }
}

/// DLMS 协议处理器
pub struct DlmsProcessor {
    meter: MeterHandle,
}

impl DlmsProcessor {
    pub fn new(meter: MeterHandle) -> Self { Self { meter } }

    /// 处理 HDLC 帧 -> 响应帧
    pub fn process_hdlc(&self, data: &[u8]) -> Result<Vec<u8>> {
        let (_server, _client, apdu) = HdlcFrame::decode(data)?;
        let response_apdu = self.process_apdu(&apdu)?;
        Ok(HdlcFrame::encode(0x0001, _client, &response_apdu))
    }

    /// 处理 APDU
    pub fn process_apdu(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        if apdu.is_empty() { return Err(anyhow::anyhow!("empty APDU")); }

        let tag = apdu[0];
        match tag {
            // AARQ (Association Request)
            0xE0 => self.handle_association(apdu),
            // GetRequest
            0xC0 | 0xC1 => self.handle_get_request(apdu),
            // SetRequest
            0xD0 | 0xD1 => self.handle_set_request(apdu),
            _ => Err(anyhow::anyhow!("unsupported APDU tag 0x{:02X}", tag)),
        }
    }

    fn handle_association(&self, _apdu: &[u8]) -> Result<Vec<u8>> {
        // AARE (Association Response) - accepted
        Ok(vec![
            0xE1, // AARE tag
            0x00, 0x00, 0x00, // result: accepted
            // Associate source
            0xA1, 0x09, 0x06, 0x07, 0x60, 0x85, 0x74, 0x05, 0x08, 0x01, 0x01,
            // ACSE
            0xBE, 0x10, 0x04, 0x0E, 0x01, 0x00, 0x00, 0x00, 0x00, 0x09, 0x0C, 0x06, 0x00, 0x00, 0x01, 0x00, 0xFF,
            // xDLMS
            0xAA, 0x00, 0x80,
        ])
    }

    fn handle_get_request(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        let mut meter = self.meter.lock().unwrap();
        let snap = meter.snapshot();

        if apdu.len() >= 8 {
            // Long name invoke-id-and-priority + OBIS
            let _invoke = apdu[1];
            let obis = ObisPath(apdu[3], apdu[4], apdu[5], apdu[6], apdu[7]);
            let response = self.read_cosem_object(&obis, &snap);
            drop(meter);
            return Ok(response);
        }

        drop(meter);
        Err(anyhow::anyhow!("invalid get request"))
    }

    fn handle_set_request(&self, _apdu: &[u8]) -> Result<Vec<u8>> {
        // SetResponse - success
        Ok(vec![0xD1, 0x00, 0x00])
    }

    fn read_cosem_object(&self, obis: &ObisPath, snap: &crate::MeterSnapshot) -> Vec<u8> {
        let mut resp = vec![0xC1, 0x00]; // GetResponse, success

        match (obis.0, obis.1, obis.2, obis.3, obis.4) {
            // 逻辑设备名
            (0, 0, 1, _, _) => {
                let name = b"FeMeter Virtual Meter";
                resp.push(0x01); // octet-string
                resp.push(name.len() as u8);
                resp.extend_from_slice(name);
            }
            // 累计有功电能 (正) - 1.0.0.0.255
            (1, 0, 0, 0, 255) => {
                // Return as octet-string with date + value
                let now = chrono::Utc::now();
                resp.push(0x01); resp.push(0x0C); // octet-string 12 bytes
                // date (5 bytes)
                resp.extend_from_slice(&encode_dlms_date(&now));
                // value (7 bytes) - 0.01 kWh
                let val = (snap.energy.wh_total / 10.0) as i64;
                resp.extend_from_slice(&val.to_be_bytes());
            }
            // 累计无功电能 - 1.0.1.0.255
            (1, 0, 1, 0, 255) => {
                resp.push(0x01); resp.push(0x0C);
                resp.extend_from_slice(&encode_dlms_date(&chrono::Utc::now()));
                let val = (snap.energy.varh_total / 10.0) as i64;
                resp.extend_from_slice(&val.to_be_bytes());
            }
            // 电压 - 1.0.12.7.0
            (1, 0, 12, 7, 0) => {
                // array of 3 scalars
                resp.push(0x02); resp.push(0x03); // array of 3
                for v in [snap.phase_a.voltage, snap.phase_b.voltage, snap.phase_c.voltage] {
                    resp.push(0x12); // long-unsigned
                    resp.extend_from_slice(&(v as u32).to_be_bytes());
                }
            }
            // 电流 - 1.0.13.7.0
            (1, 0, 13, 7, 0) => {
                resp.push(0x02); resp.push(0x03);
                for i in [snap.phase_a.current, snap.phase_b.current, snap.phase_c.current] {
                    resp.push(0x12);
                    resp.extend_from_slice(&(i as u32).to_be_bytes());
                }
            }
            // 有功功率总 - 1.0.14.7.0
            (1, 0, 14, 7, 0) => {
                resp.push(0x12);
                resp.extend_from_slice(&(snap.computed.p_total as u32).to_be_bytes());
            }
            // 无功功率总 - 1.0.15.7.0
            (1, 0, 15, 7, 0) => {
                resp.push(0x12);
                resp.extend_from_slice(&(snap.computed.q_total as u32).to_be_bytes());
            }
            // 频率 - 1.0.1.7.0
            (1, 0, 1, 7, 0) => {
                resp.push(0x12);
                resp.extend_from_slice(&((snap.freq * 100.0) as u32).to_be_bytes());
            }
            // 功率因数 - 1.0.21.7.0
            (1, 0, 21, 7, 0) => {
                resp.push(0x12);
                resp.extend_from_slice(&((snap.computed.pf_total * 1000.0) as u32).to_be_bytes());
            }
            // 时钟 - 0.0.96.1.0.255
            (0, 0, 96, 1, 0) => {
                resp.push(0x09); // octet-string date-time
                resp.push(0x0C);
                let now = chrono::Utc::now();
                resp.extend_from_slice(&encode_dlms_date(&now));
                // time part
                let time_bytes = encode_dlms_time(&now);
                resp.extend_from_slice(&time_bytes);
            }
            // 费率 - 0.0.96.10.1.255
            (0, 0, 96, 10, 1) => {
                resp.push(0x16); // enum
                resp.push(0x03); // Normal
            }
            // 负荷曲线 - 1.0.0.1.0.255
            (1, 0, 0, 1, 0) => {
                resp.push(0x01); resp.push(0x00); // empty block
            }
            // 事件日志 - 1.0.99.1.0.255
            (1, 0, 99, 1, 0) => {
                resp.push(0x01); resp.push(0x00);
            }
            _ => {
                resp = vec![0xC1, 0x01]; // object-undefined
            }
        }
        resp
    }
}

fn encode_dlms_date(dt: &chrono::DateTime<chrono::Utc>) -> [u8; 5] {
    [
        dt.year() as u16 as u8,
        (dt.year() >> 8) as u8,
        dt.month() as u8,
        dt.day() as u8,
        dt.weekday().number_from_monday() as u8,
    ]
}

fn encode_dlms_time(dt: &chrono::DateTime<chrono::Utc>) -> [u8; 3] {
    [
        dt.hour() as u8,
        dt.minute() as u8,
        dt.second() as u8,
    ]
}

pub fn create_dlms_processor(meter: MeterHandle) -> DlmsProcessor {
    DlmsProcessor::new(meter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hdlc_roundtrip() {
        let apdu = [0xC0, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xFF];
        let frame = HdlcFrame::encode(0x0001, 0x0010, &apdu);
        assert_eq!(frame[0], 0x7E);
        assert_eq!(*frame.last().unwrap(), 0x7E);
        let (server, client, decoded) = HdlcFrame::decode(&frame).unwrap();
        assert_eq!(server, 0x0001);
        assert_eq!(client, 0x0010);
        assert_eq!(decoded, apdu);
    }

    #[test]
    fn test_hdlc_escape() {
        let apdu = [0x7E, 0x7D, 0x00];
        let frame = HdlcFrame::encode(0x0001, 0x0001, &apdu);
        // 0x7E should be escaped in the info field
        assert!(frame.len() > apdu.len() + 8);
        let (_, _, decoded) = HdlcFrame::decode(&frame).unwrap();
        assert_eq!(decoded, apdu);
    }

    #[test]
    fn test_obis_display() {
        let obis = ObisPath(1, 0, 0, 0, 255);
        assert_eq!(format!("{}", obis), "1.0.0.0.255");
    }
}
