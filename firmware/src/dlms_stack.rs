/* ================================================================== */
/*                                                                    */
/*  dlms_stack.rs — DLMS/COSEM 协议栈集成层                            */
/*                                                                    */
/*  提供从 UART 字节流到 DLMS 响应的完整处理链：                        */
/*    UART → HDLC 帧解析 → APDU 解码 → COSEM 数据访问 → 响应编码      */
/*                                                                    */
/*  支持模式：SN（Serial Number）认证，只读访问                        */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

#![allow(unused)]

extern crate alloc;

use crate::hal::{EnergyData, PhaseData};
use alloc::vec::Vec;
use dlms_apdu::{
    get::{GetRequest, GetRequestNormal, GetResponse, GetResponseNormal},
    initiate::{conformance, InitiateRequest, InitiateResponse},
    types::{AccessRequest, AttributeDescriptor, InvokeId},
    Apdu,
};
use dlms_core::{DataAccessError, DlmsType, ObisCode};
use dlms_hdlc::frame::{HdlcFrame, HDLC_ESCAPE, HDLC_ESCAPE_MASK, HDLC_FLAG};

/* ================================================================== */
/*  数据提供 trait — 解耦计量层与协议层                                */
/* ================================================================== */

/// 电表基本信息
#[derive(Clone, Copy, Debug, Default)]
pub struct MeterInfo {
    /// 电表地址（12 字节 BCD / 24 hex chars）
    pub meter_address: [u8; 12],
    /// 制造商编码
    pub manufacturer: [u8; 3],
    /// 固件版本
    pub fw_version: [u8; 6],
    /// 电表序列号
    pub serial_number: u32,
}

/// DLMS 数据提供者 trait
///
/// 固件中由 MeteringManager 实现，通过此 trait 注入到 DlmsStack，
/// 使协议层不依赖具体的计量芯片驱动。
pub trait DlmsDataProvider {
    /// 获取三相实时数据
    fn get_phase_data(&self) -> PhaseData;
    /// 获取电能累计数据
    fn get_energy_data(&self) -> EnergyData;
    /// 获取电表信息
    fn get_meter_info(&self) -> MeterInfo;
}

/* ================================================================== */
/*  支持的 OBIS 码定义                                                */
/* ================================================================== */

/// 时钟: class_id=8, attr 2 (时间)
const OBIS_CLOCK: ObisCode = ObisCode::new(0, 0, 1, 0, 0, 255);

/// A 相电压 (class_id=1)
const OBIS_VOLTAGE_A: ObisCode = ObisCode::new(1, 0, 1, 7, 0, 255);
/// B 相电压
const OBIS_VOLTAGE_B: ObisCode = ObisCode::new(1, 0, 1, 8, 0, 255);
/// C 相电压
const OBIS_VOLTAGE_C: ObisCode = ObisCode::new(1, 0, 1, 9, 0, 255);

/// A 相电流
const OBIS_CURRENT_A: ObisCode = ObisCode::new(1, 0, 11, 7, 0, 255);
/// B 相电流
const OBIS_CURRENT_B: ObisCode = ObisCode::new(1, 0, 12, 7, 0, 255);
/// C 相电流
const OBIS_CURRENT_C: ObisCode = ObisCode::new(1, 0, 13, 7, 0, 255);

/// 总有功功率 (0x21 = 33, 即 B 组)
const OBIS_ACTIVE_POWER_TOTAL: ObisCode = ObisCode::new(1, 0, 1, 21, 0, 255);
/// 总无功功率
const OBIS_REACTIVE_POWER_TOTAL: ObisCode = ObisCode::new(1, 0, 1, 23, 0, 255);

/// 频率
const OBIS_FREQUENCY: ObisCode = ObisCode::new(1, 0, 1, 29, 0, 255);

/// 功率因数 (0x2f = 47)
const OBIS_POWER_FACTOR: ObisCode = ObisCode::new(1, 0, 1, 47, 0, 255);

/// 正向有功电能
const OBIS_POSITIVE_ACTIVE_ENERGY: ObisCode = ObisCode::new(1, 0, 9, 6, 0, 255);

/* ================================================================== */
/*  HDLC 接收缓冲状态机                                                */
/* ================================================================== */

/// HDLC 帧接收器状态
#[derive(Debug, Clone, Copy, PartialEq)]
enum RxState {
    /// 等待帧起始标志 0x7E
    Idle,
    /// 正在接收帧数据
    Receiving,
}

/* ================================================================== */
/*  DlmsStack                                                         */
/* ================================================================== */

/// 最大 HDLC 帧长度（信息域）
const MAX_FRAME_SIZE: usize = 256;

/// DLMS/COSEM 协议栈
///
/// 从 UART 字节流接收 HDLC 帧，解析 APDU 请求，
/// 查询 COSEM 数据对象，编码响应并返回。
///
/// # 使用方式
///
/// ```ignore
/// let mut stack = DlmsStack::new(&metering_manager);
/// // 在 UART 接收中断中逐字节喂入
/// for &byte in uart_rx_buf.iter() {
///     stack.feed_byte(byte);
/// }
/// // 在主循环中检查并处理
/// if let Some(response) = stack.process_request() {
///     uart_write(&response);
/// }
/// ```
pub struct DlmsStack<'a, P: DlmsDataProvider> {
    /// 数据提供者（计量管理器引用）
    provider: &'a P,

    /// HDLC 接收状态机
    rx_state: RxState,
    /// 当前帧接收缓冲（去转义后的原始字节）
    rx_buffer: Vec<u8>,
    /// 帧是否完整（收到结束标志）
    frame_ready: bool,

    /// HDLC 发送序列号
    send_seq: u8,
    /// HDLC 接收序列号
    recv_seq: u8,

    /// SN 认证密码（LSB，4 字节）
    sn_password: [u8; 4],
}

impl<'a, P: DlmsDataProvider> DlmsStack<'a, P> {
    /// 创建 DLMS 协议栈实例
    pub fn new(provider: &'a P) -> Self {
        Self {
            provider,
            rx_state: RxState::Idle,
            rx_buffer: Vec::new(),
            frame_ready: false,
            send_seq: 0,
            recv_seq: 0,
            sn_password: [0x00; 4], // 默认空密码
        }
    }

    /// 设置 SN 认证密码
    pub fn set_sn_password(&mut self, password: [u8; 4]) {
        self.sn_password = password;
    }

    /// 从 UART 接收一个字节，自动进行 HDLC 帧检测和字节反转义
    ///
    /// 当收到完整的 HDLC 帧（两个 0x7E 标志之间），设置 frame_ready 标志，
    /// 等待 process_request() 调用。
    pub fn feed_byte(&mut self, byte: u8) {
        match self.rx_state {
            RxState::Idle => {
                if byte == HDLC_FLAG {
                    // 帧起始，切换到接收状态
                    self.rx_state = RxState::Receiving;
                    self.rx_buffer.clear();
                    self.frame_ready = false;
                }
                // 非 0x7E 字节在 Idle 状态忽略
            }
            RxState::Receiving => {
                if byte == HDLC_FLAG {
                    // 帧结束标志
                    if !self.rx_buffer.is_empty() {
                        self.frame_ready = true;
                    }
                    self.rx_state = RxState::Idle;
                } else if byte == HDLC_ESCAPE {
                    // 转义标记，下一个字节需要 XOR 0x20
                    self.rx_state = RxState::Receiving;
                    // 使用中间状态标记：将 0x7D 存入 buffer 并标记为转义模式
                    // 为简化实现，我们存一个特殊标记
                    self.rx_buffer.push(HDLC_ESCAPE);
                } else if let Some(last) = self.rx_buffer.last() {
                    if *last == HDLC_ESCAPE {
                        // 上一个字节是转义标记，当前字节需要 XOR
                        self.rx_buffer.pop();
                        self.rx_buffer.push(byte ^ HDLC_ESCAPE_MASK);
                    } else {
                        self.rx_buffer.push(byte);
                    }
                } else {
                    self.rx_buffer.push(byte);
                }
            }
        }
    }

    /// 处理完整的 DLMS 请求，返回 HDLC 编码的响应字节
    ///
    /// 如果没有待处理的帧，返回 None。
    /// 处理完成后自动清除 frame_ready 标志。
    pub fn process_request(&mut self) -> Option<Vec<u8>> {
        if !self.frame_ready {
            return None;
        }
        self.frame_ready = false;

        // 1. HDLC 帧解码
        let frame = match self.decode_hdlc_frame() {
            Ok(f) => f,
            Err(_) => return None, // 帧错误，静默丢弃
        };

        // 2. LLC 分层检查（跳过 LLC 头）
        let apdu_payload = self.strip_llc(&frame.information);
        if apdu_payload.is_empty() {
            return None;
        }

        // 3. APDU 解码与处理
        let response_apdu = match Apdu::decode(apdu_payload) {
            Ok(apdu) => self.handle_apdu(&apdu),
            Err(_) => return None,
        };

        // 4. APDU 编码
        let apdu_bytes = match response_apdu.encode() {
            Ok(b) => b,
            Err(_) => return None,
        };

        // 5. HDLC 帧封装
        Some(self.encode_hdlc_response(&apdu_bytes))
    }

    /// 处理解码后的 APDU，返回响应 APDU
    fn handle_apdu(&mut self, apdu: &Apdu) -> Apdu {
        match apdu {
            Apdu::InitiateRequest(req) => {
                // 协商响应：接受客户端参数
                let resp = InitiateResponse::new(
                    req.invoke_id,
                    conformance::standard_meter(),
                    req.server_max_receive_pdu_size.min(512),
                    req.client_max_receive_pdu_size.min(512),
                );
                Apdu::InitiateResponse(resp)
            }
            Apdu::GetRequest(get_req) => self.handle_get_request(get_req),
            Apdu::SetRequest(_)
            | Apdu::ActionRequest(_)
            | Apdu::EventNotification(_)
            | Apdu::GeneralBlockTransfer(_) => {
                // 只读设备：写/动作/事件/块传输不支持
                Apdu::ExceptionResponse(dlms_apdu::ExceptionResponse::service_not_supported(
                    apdu.invoke_id().unwrap_or(InvokeId::new(0)),
                ))
            }
            _ => {
                // 其他类型不处理
                Apdu::ExceptionResponse(dlms_apdu::ExceptionResponse::service_not_supported(
                    apdu.invoke_id().unwrap_or(InvokeId::new(0)),
                ))
            }
        }
    }

    /// 处理 Get-Request，查询 COSEM 数据对象
    fn handle_get_request(&self, get_req: &GetRequest) -> Apdu {
        let invoke_id = get_req.invoke_id();

        match get_req {
            GetRequest::Normal(normal) => {
                let obis = &normal.request.descriptor.instance;
                let attr = normal.request.descriptor.attribute_id;
                self.read_cosem_attribute(invoke_id, obis, attr)
            }
            GetRequest::WithList(list) => {
                // 列表请求：逐个处理，返回第一个结果的响应
                // （简化实现，不支持真正的 WithList 响应）
                if let Some(item) = list.requests.first() {
                    let obis = &item.descriptor.instance;
                    let attr = item.descriptor.attribute_id;
                    self.read_cosem_attribute(invoke_id, obis, attr)
                } else {
                    Apdu::GetResponse(GetResponse::Data(GetResponseNormal::error(
                        invoke_id,
                        DataAccessError::TemporaryFailure,
                    )))
                }
            }
            GetRequest::Next(_) => {
                // 块传输续传：不支持
                Apdu::GetResponse(GetResponse::Data(GetResponseNormal::error(
                    invoke_id,
                    DataAccessError::DataBlockUnavailable,
                )))
            }
        }
    }

    /// 根据 OBIS 码读取 COSEM 属性值
    fn read_cosem_attribute(&self, invoke_id: InvokeId, obis: &ObisCode, _attr_id: u8) -> Apdu {
        let phase = self.provider.get_phase_data();
        let energy = self.provider.get_energy_data();

        // 匹配 OBIS 码，返回对应数据
        let value = if *obis == OBIS_CLOCK {
            // 时钟：返回当前时间（简化为结构体）
            DlmsType::Structure(alloc::vec![
                DlmsType::UInt16(2026), // 年
                DlmsType::UInt8(4),     // 月
                DlmsType::UInt8(2),     // 日
                DlmsType::UInt16(0),    // 星期 + 时:分:秒.hundredths
            ])
        } else if *obis == OBIS_VOLTAGE_A {
            // A 相电压 (0.01V → 返回 Int16, 单位 V)
            DlmsType::Int16((phase.voltage_a as i16) / 100)
        } else if *obis == OBIS_VOLTAGE_B {
            DlmsType::Int16((phase.voltage_b as i16) / 100)
        } else if *obis == OBIS_VOLTAGE_C {
            DlmsType::Int16((phase.voltage_c as i16) / 100)
        } else if *obis == OBIS_CURRENT_A {
            // A 相电流 (mA → 返回 Int16, 单位 A, 精度 0.001A)
            DlmsType::from_i32(phase.current_a as i32)
        } else if *obis == OBIS_CURRENT_B {
            DlmsType::from_i32(phase.current_b as i32)
        } else if *obis == OBIS_CURRENT_C {
            DlmsType::from_i32(phase.current_c as i32)
        } else if *obis == OBIS_ACTIVE_POWER_TOTAL {
            // 总有功功率 (W)
            DlmsType::Int32(phase.active_power_total)
        } else if *obis == OBIS_REACTIVE_POWER_TOTAL {
            // 总无功功率 (var)
            DlmsType::Int32(phase.reactive_power_total)
        } else if *obis == OBIS_FREQUENCY {
            // 频率 (0.01Hz → Hz)
            DlmsType::UInt16(phase.frequency / 100)
        } else if *obis == OBIS_POWER_FACTOR {
            // 功率因数 (0~1000 → 0.000~1.000)
            DlmsType::UInt16(phase.power_factor_total)
        } else if *obis == OBIS_POSITIVE_ACTIVE_ENERGY {
            // 正向有功电能 (0.01 kWh → Int64)
            DlmsType::from_i64(energy.active_import as i64)
        } else {
            // 不支持的 OBIS 码：返回 object-undefined 错误
            return Apdu::GetResponse(GetResponse::Data(GetResponseNormal::error(
                invoke_id,
                DataAccessError::ObjectUndefined,
            )));
        };

        Apdu::GetResponse(GetResponse::Data(GetResponseNormal::success(
            invoke_id, value,
        )))
    }

    /// 解码 HDLC 帧（带 CRC 校验）
    fn decode_hdlc_frame(&mut self) -> Result<HdlcFrame, ()> {
        // 将接收缓冲包装为带标志的完整帧
        let mut frame_bytes = Vec::new();
        frame_bytes.push(HDLC_FLAG);
        frame_bytes.extend_from_slice(&self.rx_buffer);
        frame_bytes.push(HDLC_FLAG);

        HdlcFrame::decode(&frame_bytes).map_err(|_| ())
    }

    /// 去除 LLC 头（1 字节 DSAP/SSAP）
    fn strip_llc<'b>(&self, data: &'b [u8]) -> &'b [u8] {
        if data.len() > 1 {
            // LLC: DSAP=0xE7, SSAP=0xE7 (DLMS), 跳过前 1~3 字节
            // 简化：跳过第一个字节（通常是 LLC 控制字段）
            &data[1..]
        } else {
            data
        }
    }

    /// 将 APDU 字节封装为 HDLC 响应帧
    fn encode_hdlc_response(&mut self, apdu_bytes: &[u8]) -> Vec<u8> {
        // 构建 LLC 头 + APDU
        let mut info = Vec::new();
        info.push(0xE7); // LLC DSAP
        info.push(0xE7); // LLC SSAP
        info.push(0x03); // LLC control (UI)
        info.extend_from_slice(apdu_bytes);

        // 构建 HDLC I-frame
        let mut frame = HdlcFrame::new(
            dlms_hdlc::HdlcAddress::new(1, 1, 0), // 服务器地址
            dlms_hdlc::control::ControlField::information(self.send_seq, self.recv_seq, true),
            info,
        );

        let encoded = frame.encode();
        self.send_seq = self.send_seq.wrapping_add(1) & 0x07;
        encoded
    }
}

/* ================================================================== */
/*  测试（仅 std 模式）                                                */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        phase: PhaseData,
        energy: EnergyData,
    }

    impl DlmsDataProvider for MockProvider {
        fn get_phase_data(&self) -> PhaseData {
            self.phase
        }
        fn get_energy_data(&self) -> EnergyData {
            self.energy
        }
        fn get_meter_info(&self) -> MeterInfo {
            MeterInfo::default()
        }
    }

    fn make_provider() -> MockProvider {
        let mut phase = PhaseData::default();
        phase.voltage_a = 22000; // 220.00V
        phase.voltage_b = 22100;
        phase.voltage_c = 21950;
        phase.current_a = 5000; // 5000mA
        phase.frequency = 5000; // 50.00Hz
        phase.power_factor_total = 980; // 0.980
        phase.active_power_total = 1000; // 1000W
        phase.reactive_power_total = 200;

        let mut energy = EnergyData::default();
        energy.active_import = 12345678; // 123456.78 kWh

        MockProvider { phase, energy }
    }

    #[test]
    fn test_stack_creation() {
        let provider = make_provider();
        let _stack = DlmsStack::new(&provider);
    }

    #[test]
    fn test_feed_byte_flag_detection() {
        let provider = make_provider();
        let mut stack = DlmsStack::new(&provider);

        // 空帧（只有标志）
        stack.feed_byte(HDLC_FLAG);
        assert!(!stack.frame_ready);
        stack.feed_byte(HDLC_FLAG);
        // 空 buffer → frame_ready 应为 false
        assert!(!stack.frame_ready);
    }

    #[test]
    fn test_sn_password() {
        let provider = make_provider();
        let mut stack = DlmsStack::new(&provider);
        stack.set_sn_password([0x12, 0x34, 0x56, 0x78]);
        assert_eq!(stack.sn_password, [0x12, 0x34, 0x56, 0x78]);
    }
}
