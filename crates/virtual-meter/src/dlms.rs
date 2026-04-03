//! DLMS/COSEM 协议模拟 (增强版)
//!
//! 使用 workspace 内 dlms-* crates 实现:
//! - HDLC 帧封装/解析 (dlms-hdlc)
//! - ASN.1 BER 编解码 (dlms-asn1)
//! - APDU 编解码 (dlms-apdu): Get/Set/Action 完整处理
//! - COSEM 对象: Register, Profile Generic, Clock, Association
//! - SN (短名称) + LN (逻辑名) 两种访问方式
//! - 安全: AES-128-GCM 认证加密 (预留接口, dlms-security crate 待修复)

use crate::MeterHandle;
use anyhow::{anyhow, Result};
use chrono::{Datelike, Timelike};

// Re-exports
pub use dlms_apdu::{
    ActionRequest, ActionResponse, GetRequest, GetResponse, SetRequest, SetResponse,
};
pub use dlms_core::{DlmsType, ObisCode};
pub use dlms_hdlc::{HdlcAddress, HdlcFrame as CrateHdlcFrame};

/// Legacy HDLC wrapper
pub struct HdlcFrame;

impl HdlcFrame {
    pub const FLAG: u8 = 0x7E;
    pub const ESCAPE: u8 = 0x7D;
    #[allow(dead_code)]
    pub const XFLAG: u8 = 0x5E;
    #[allow(dead_code)]
    pub const XESCAPE: u8 = 0x5D;

    pub fn encode(server_addr: u16, client_addr: u16, apdu: &[u8]) -> Vec<u8> {
        let address = HdlcAddress::new(client_addr as u8, server_addr, 0);
        let control = dlms_hdlc::control::ControlField::information(0, 0, false);
        let mut frame = CrateHdlcFrame::new(address, control, apdu.to_vec());
        frame.encode()
    }

    pub fn decode(data: &[u8]) -> Result<(u16, u16, Vec<u8>)> {
        let frame =
            CrateHdlcFrame::decode(data).map_err(|e| anyhow!("HDLC decode error: {:?}", e))?;
        Ok((
            frame.address.server_upper,
            frame.address.client as u16,
            frame.information,
        ))
    }
}

/// COSEM OBIS 路径 (兼容旧接口)
#[derive(Debug, Clone, PartialEq)]
pub struct ObisPath(pub u8, pub u8, pub u8, pub u8, pub u8);

impl std::fmt::Display for ObisPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}.{}.{}", self.0, self.1, self.2, self.3, self.4)
    }
}

impl ObisPath {
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() >= 6 {
            Some(Self(b[0], b[1], b[2], b[3], b[4]))
        } else {
            None
        }
    }
    pub fn to_obis_code(&self) -> ObisCode {
        ObisCode::new(self.0, self.1, self.2, self.3, self.4, 255)
    }
}

impl From<&ObisCode> for ObisPath {
    fn from(obis: &ObisCode) -> Self {
        let b = obis.to_bytes();
        Self(b[0], b[1], b[2], b[3], b[4])
    }
}

/// COSEM 接口类 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CosemClassId {
    Data = 1,
    Register = 3,
    ExtendedRegister = 4,
    DemandRegister = 5,
    ProfileGeneric = 7,
    Clock = 8,
    Association = 12,
}

impl CosemClassId {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(Self::Data),
            3 => Some(Self::Register),
            4 => Some(Self::ExtendedRegister),
            5 => Some(Self::DemandRegister),
            7 => Some(Self::ProfileGeneric),
            8 => Some(Self::Clock),
            12 => Some(Self::Association),
            _ => None,
        }
    }
}

/// DLMS 协议处理器
pub struct DlmsProcessor {
    meter: MeterHandle,
    #[allow(dead_code)]
    ln_mode: bool,
}

impl DlmsProcessor {
    pub fn new(meter: MeterHandle) -> Self {
        Self {
            meter,
            ln_mode: true,
        }
    }

    #[allow(dead_code)]
    pub fn set_ln_mode(&mut self, enabled: bool) {
        self.ln_mode = enabled;
    }

    /// 处理 HDLC 帧 -> 响应帧
    pub fn process_hdlc(&self, data: &[u8]) -> Result<Vec<u8>> {
        let (server, client, info) = HdlcFrame::decode(data)?;
        let response_apdu = self.process_apdu(&info)?;
        Ok(HdlcFrame::encode(server, client, &response_apdu))
    }

    /// 处理裸 APDU
    pub fn process_apdu(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        if apdu.is_empty() {
            return Err(anyhow!("empty APDU"));
        }
        match apdu[0] {
            0xE0 => self.handle_association(apdu),
            0x80 => Ok(vec![0x81, 0x00, 0x00]), // RLRE accepted
            0xC0 | 0xC1 => self.handle_get_request(apdu),
            0xD0 | 0xD1 => self.handle_set_request(apdu),
            0xC2 => self.handle_action_request(apdu),
            _ => Err(anyhow!("unsupported APDU tag 0x{:02X}", apdu[0])),
        }
    }

    // --- 关联 ---
    fn handle_association(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        if let Ok(_aarq) = dlms_asn1::decode_aarq(apdu) {
            let aare = dlms_asn1::Aare::accepted_ln_no_cipher(dlms_asn1::InitiateResponse {
                negotiated_conformance: dlms_asn1::ConformanceBlock::standard_meter(),
                negotiated_max_pdu_size: 0xFFFF,
                negotiated_dlms_version: 6,
                server_max_receive_pdu_size: Some(0xFFFF),
                vaa_name: 0,
            });
            Ok(dlms_asn1::encode_aare(&aare))
        } else {
            // 简单 AARE 回退
            Ok(vec![
                0xE1, 0x00, 0x00, 0xA1, 0x09, 0x06, 0x07, 0x60, 0x85, 0x74, 0x05, 0x08, 0x01, 0x01,
                0xBE, 0x10, 0x04, 0x0E, 0x01, 0x00, 0x00, 0x00, 0x00, 0x09, 0x0C, 0x06, 0x00, 0x00,
                0x01, 0x00, 0xFF, 0xAA, 0x00, 0x80,
            ])
        }
    }

    // --- Get ---
    fn handle_get_request(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        match GetRequest::decode(apdu) {
            Ok(GetRequest::Normal(req)) => {
                let desc = &req.request.descriptor;
                let obis = ObisPath::from(&desc.instance);
                let mut meter = self.meter.lock().expect("mutex poisoned");
                let snap = meter.snapshot();
                let value =
                    self.read_cosem_attribute(desc.class_id, &obis, desc.attribute_id, &snap);
                drop(meter);
                let resp = dlms_apdu::get::GetResponseNormal::success(req.invoke_id, value);
                resp.encode().map_err(|e| anyhow!("encode: {:?}", e))
            }
            Ok(GetRequest::Next(req)) => {
                let resp = dlms_apdu::get::GetResponseBlock::new(req.invoke_id, 0, true, vec![]);
                Ok(resp.encode())
            }
            Ok(GetRequest::WithList(req)) => {
                let mut values = Vec::new();
                for item in &req.requests {
                    let obis = ObisPath::from(&item.descriptor.instance);
                    let mut meter = self.meter.lock().expect("mutex poisoned");
                    let snap = meter.snapshot();
                    let value = self.read_cosem_object(&obis, &snap);
                    drop(meter);
                    values.push(value);
                }
                let resp = dlms_apdu::get::GetResponseNormal::success(
                    req.invoke_id,
                    DlmsType::Array(values),
                );
                resp.encode().map_err(|e| anyhow!("encode: {:?}", e))
            }
            Err(_) => self.handle_get_legacy(apdu),
        }
    }

    fn handle_get_legacy(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        if apdu.len() < 8 {
            return Err(anyhow!("invalid get"));
        }
        let obis = ObisPath(apdu[3], apdu[4], apdu[5], apdu[6], apdu[7]);
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();
        let value = self.read_cosem_object(&obis, &snap);
        drop(meter);
        let mut resp = vec![0xC1, 0x00];
        resp.push(value.tag());
        self.append_dlms_value(&mut resp, &value);
        Ok(resp)
    }

    // --- Set ---
    fn handle_set_request(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        match SetRequest::decode(apdu) {
            Ok(SetRequest::Normal(_req)) => {
                Ok(vec![0xD5, 0x01, 0x00]) // SetResponse-Normal, success
            }
            Ok(SetRequest::WithList(_req)) => Ok(vec![0xD5, 0x01, 0x00]),
            Err(_) => Ok(vec![0xD1, 0x00, 0x00]),
        }
    }

    // --- Action ---
    #[allow(dead_code)]
    fn handle_action_request(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        match ActionRequest::decode(apdu) {
            Ok(ActionRequest::Normal(req)) => {
                let resp =
                    dlms_apdu::action::ActionResponseNormal::success(req.invoke_id, DlmsType::Null);
                resp.encode().map_err(|e| anyhow!("encode: {:?}", e))
            }
            Ok(ActionRequest::WithList(req)) => {
                let items: Vec<_> = req
                    .items
                    .iter()
                    .map(|_| dlms_apdu::action::ActionResponseListItem {
                        result: Ok(DlmsType::Null),
                    })
                    .collect();
                let resp = dlms_apdu::action::ActionResponseWithList::new(req.invoke_id, items);
                resp.encode().map_err(|e| anyhow!("encode: {:?}", e))
            }
            Ok(ActionRequest::Next(req)) => {
                let resp =
                    dlms_apdu::action::ActionResponseBlock::new(req.invoke_id, 0, true, vec![]);
                Ok(resp.encode())
            }
            Err(_) => Err(anyhow!("invalid action request")),
        }
    }

    // --- COSEM 属性读取 ---
    fn read_cosem_attribute(
        &self,
        class_id: u16,
        obis: &ObisPath,
        attr_id: u8,
        snap: &crate::MeterSnapshot,
    ) -> DlmsType {
        if attr_id == 1 {
            return DlmsType::OctetString(obis.to_obis_code().to_bytes().to_vec());
        }
        match CosemClassId::from_u16(class_id) {
            Some(CosemClassId::Register) => match attr_id {
                2 => self.read_energy_value(obis, snap),
                3 => DlmsType::OctetString(vec![0x02, 0x1E]),
                _ => DlmsType::Null,
            },
            Some(CosemClassId::Clock) => match attr_id {
                2 => DlmsType::OctetString(encode_cosem_datetime(&chrono::Utc::now())),
                3 => DlmsType::Int16(0),
                4 => DlmsType::UInt16(0),
                8 => DlmsType::Boolean(false),
                9 => DlmsType::Enum(1),
                _ => DlmsType::Null,
            },
            Some(CosemClassId::ProfileGeneric) => match attr_id {
                2 => DlmsType::Array(vec![]),
                4 => DlmsType::Enum(0),
                7 | 8 => DlmsType::UInt32(0),
                _ => DlmsType::Null,
            },
            Some(CosemClassId::Association) => match attr_id {
                3 => DlmsType::VisibleString(b"FeMeter".to_vec()),
                7 => DlmsType::Enum(0),
                _ => DlmsType::Null,
            },
            Some(CosemClassId::DemandRegister) => {
                if attr_id == 2 {
                    DlmsType::Float32(snap.computed.p_total as f32)
                } else {
                    DlmsType::Null
                }
            }
            _ => {
                if attr_id == 2 {
                    self.read_cosem_object(obis, snap)
                } else {
                    DlmsType::Null
                }
            }
        }
    }

    /// 通用 OBIS 读取
    fn read_cosem_object(&self, obis: &ObisPath, snap: &crate::MeterSnapshot) -> DlmsType {
        match (obis.0, obis.1, obis.2, obis.3, obis.4) {
            (0, 0, 1, _, _) => DlmsType::VisibleString(b"FeMeter Virtual Meter".to_vec()),
            (1, 0, 1, 8, 255) | (1, 0, 0, 0, 255) => {
                DlmsType::UInt64((snap.energy.wh_total / 10.0) as u64)
            }
            (1, 0, 1, 0, 255) | (1, 0, 3, 8, 255) => {
                DlmsType::UInt64((snap.energy.varh_total / 10.0) as u64)
            }
            (1, 0, 1, 8, 1) => DlmsType::UInt64(500),
            (1, 0, 1, 8, 2) => DlmsType::UInt64(300),
            (1, 0, 1, 8, 3) => DlmsType::UInt64(200),
            (1, 0, 1, 8, 4) => DlmsType::UInt64(0),
            (1, 0, 32, 7, 0) => DlmsType::Float32((snap.phase_a.voltage) as f32),
            (1, 0, 52, 7, 0) => DlmsType::Float32((snap.phase_b.voltage) as f32),
            (1, 0, 72, 7, 0) => DlmsType::Float32((snap.phase_c.voltage) as f32),
            (1, 0, 12, 7, 0) => DlmsType::Array(vec![
                DlmsType::Float32((snap.phase_a.voltage) as f32),
                DlmsType::Float32((snap.phase_b.voltage) as f32),
                DlmsType::Float32((snap.phase_c.voltage) as f32),
            ]),
            (1, 0, 31, 7, 0) => DlmsType::Float32((snap.phase_a.current) as f32),
            (1, 0, 51, 7, 0) => DlmsType::Float32((snap.phase_b.current) as f32),
            (1, 0, 71, 7, 0) => DlmsType::Float32((snap.phase_c.current) as f32),
            (1, 0, 13, 7, 0) => DlmsType::Array(vec![
                DlmsType::Float32((snap.phase_a.current) as f32),
                DlmsType::Float32((snap.phase_b.current) as f32),
                DlmsType::Float32((snap.phase_c.current) as f32),
            ]),
            (1, 0, 14, 7, 0) | (1, 0, 1, 7, 0) => DlmsType::Float32(snap.computed.p_total as f32),
            (1, 0, 15, 7, 0) | (1, 0, 3, 7, 0) => DlmsType::Float32(snap.computed.q_total as f32),
            (1, 0, 14, 7, 255) => DlmsType::Float32(snap.freq as f32),
            (1, 0, 21, 7, 0) => DlmsType::Float32(snap.computed.pf_total as f32),
            (0, 0, 96, 1, 0) => DlmsType::OctetString(encode_cosem_datetime(&chrono::Utc::now())),
            (0, 0, 96, 10, 1) => DlmsType::Enum(3),
            (1, 0, 0, 1, 0) | (1, 0, 99, 1, 0) => DlmsType::Array(vec![]),
            (1, 0, 1, 6, 0) => DlmsType::Float32(snap.computed.p_total as f32),
            _ => DlmsType::Null,
        }
    }

    fn read_energy_value(&self, obis: &ObisPath, snap: &crate::MeterSnapshot) -> DlmsType {
        match (obis.0, obis.1, obis.2, obis.3) {
            (1, 0, 1, 8) => DlmsType::UInt64((snap.energy.wh_total / 10.0) as u64),
            (1, 0, 3, 8) => DlmsType::UInt64((snap.energy.varh_total / 10.0) as u64),
            _ => DlmsType::UInt64(0),
        }
    }

    #[allow(dead_code)]
    fn append_dlms_value(&self, buf: &mut Vec<u8>, value: &DlmsType) {
        match value {
            DlmsType::OctetString(v) | DlmsType::BitString(v) => {
                buf.push(v.len() as u8);
                buf.extend_from_slice(v);
            }
            DlmsType::UInt8(v) => buf.push(*v),
            DlmsType::UInt16(v) => buf.extend_from_slice(&v.to_be_bytes()),
            DlmsType::UInt32(v) => buf.extend_from_slice(&v.to_be_bytes()),
            DlmsType::UInt64(v) => buf.extend_from_slice(&v.to_be_bytes()),
            DlmsType::Int16(v) => buf.extend_from_slice(&v.to_be_bytes()),
            DlmsType::Int32(v) => buf.extend_from_slice(&v.to_be_bytes()),
            DlmsType::Float32(v) => buf.extend_from_slice(&v.to_be_bytes()),
            DlmsType::Enum(v) => buf.push(*v),
            DlmsType::Boolean(v) => buf.push(if *v { 1 } else { 0 }),
            DlmsType::Array(items) | DlmsType::Structure(items) => {
                buf.push(items.len() as u8);
                for item in items {
                    buf.push(item.tag());
                    self.append_dlms_value(buf, item);
                }
            }
            DlmsType::Null | DlmsType::VisibleString(_) | DlmsType::Utf8String(_) => {}
            _ => {}
        }
    }

    // --- 调试接口 ---
    #[allow(dead_code)]
    pub fn raw_apdu(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        self.process_apdu(apdu)
    }

    #[allow(dead_code)]
    pub fn query_obis(&self, obis_str: &str) -> Result<String> {
        let code =
            ObisCode::parse(obis_str).ok_or_else(|| anyhow!("invalid OBIS: {}", obis_str))?;
        let obis = ObisPath::from(&code);
        let mut meter = self.meter.lock().expect("mutex poisoned");
        let snap = meter.snapshot();
        let value = self.read_cosem_object(&obis, &snap);
        Ok(format!("{}: {:?}", obis, value))
    }
}

fn encode_cosem_datetime(dt: &chrono::DateTime<chrono::Utc>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(12);
    buf.extend_from_slice(&(dt.year() as u16).to_be_bytes());
    buf.push(dt.month() as u8);
    buf.push(dt.day() as u8);
    buf.push(dt.weekday().number_from_monday() as u8);
    buf.push(dt.hour() as u8);
    buf.push(dt.minute() as u8);
    buf.push(dt.second() as u8);
    buf.push(0x00);
    buf.extend_from_slice(&0i16.to_be_bytes());
    buf.push(0x00);
    buf
}

#[allow(dead_code)]
fn encode_dlms_date(dt: &chrono::DateTime<chrono::Utc>) -> [u8; 5] {
    [
        dt.year() as u16 as u8,
        (dt.year() >> 8) as u8,
        dt.month() as u8,
        dt.day() as u8,
        dt.weekday().number_from_monday() as u8,
    ]
}

#[allow(dead_code)]
fn encode_dlms_time(dt: &chrono::DateTime<chrono::Utc>) -> [u8; 3] {
    [dt.hour() as u8, dt.minute() as u8, dt.second() as u8]
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
        assert!(frame.len() > apdu.len() + 8);
        let (_, _, decoded) = HdlcFrame::decode(&frame).unwrap();
        assert_eq!(decoded, apdu);
    }

    #[test]
    fn test_obis_display() {
        assert_eq!(format!("{}", ObisPath(1, 0, 0, 0, 255)), "1.0.0.0.255");
    }

    #[test]
    fn test_obis_to_obis_code() {
        let code = ObisPath(1, 0, 1, 8, 0).to_obis_code();
        assert_eq!(code.to_bytes(), [1, 0, 1, 8, 0, 255]);
    }

    #[test]
    fn test_cosem_class_id() {
        assert_eq!(CosemClassId::from_u16(3), Some(CosemClassId::Register));
        assert_eq!(
            CosemClassId::from_u16(7),
            Some(CosemClassId::ProfileGeneric)
        );
        assert_eq!(CosemClassId::from_u16(8), Some(CosemClassId::Clock));
        assert_eq!(CosemClassId::from_u16(999), None);
    }

    #[test]
    fn test_encode_cosem_datetime() {
        let dt = chrono::DateTime::parse_from_rfc3339("2025-01-15T10:30:45Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let enc = encode_cosem_datetime(&dt);
        assert_eq!(enc.len(), 12);
        assert_eq!(enc[0], 0x07);
        assert_eq!(enc[1], 0xE9);
    }

    #[test]
    fn test_dlms_type_conversions() {
        assert_eq!(DlmsType::from_u32(42).as_u32(), Some(42));
        assert_eq!(DlmsType::from_f32(220.5).tag(), 23);
    }
}
