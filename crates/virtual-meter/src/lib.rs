//! 虚拟电表 - 跨平台桌面虚拟电表 (v2.0)
//!
//! 模拟 ATT7022E / RN8302B 计量芯片，支持：
//! - 三相电压/电流/相角设置
//! - 有功/无功/视在功率计算
//! - 电能累计 + 时间加速
//! - 事件检测 (过压/欠压/断相/过流/反向功率)
//! - 场景预设 (正常/满载/空载/故障)
//! - 日志开关
//! - 虚拟串口服务
//! - 交互式 Shell
//! - 分时费率 (TOU)
//! - 负荷曲线
//! - 需量测量
//! - DLMS/COSEM 协议
//! - LCD 段码显示模拟
//! - 统计记录
//! - 校准参数
//! - 状态持久化
//! - TCP 服务器
//! - IEC 62056-21 红外协议
//!
//! v2.0 新增:
//! - ADC 采样仿真 (adc.rs)
//! - 脉冲累计 (pulse.rs)
//! - ATT7022E 寄存器模型 (registers.rs)
//! - 真实计量引擎 (meter.rs 重写)

pub mod adc;
pub mod calibration;
pub mod demand;
pub mod demand_new;
pub mod display;
pub mod dlms;
pub mod freeze;
pub mod html_report;
pub mod iec62056;
pub mod load_profile;
mod meter;
pub mod persistence;
pub mod profile;
mod protocol;
mod pulse;
pub mod registers;
mod serial;
mod shell;
pub mod statistics;
pub mod tariff;
pub mod tcp_server;
pub mod tou;

pub use dlms::{create_dlms_processor, DlmsProcessor};
pub use meter::*;
pub use protocol::*;
pub use serial::*;
pub use shell::*;
pub use tcp_server::TcpServer;
