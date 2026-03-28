//! 虚拟电表 - 跨平台桌面虚拟电表
//!
//! 模拟 ATT7022E / RN8302B 计量芯片，支持：
//! - 三相电压/电流/相角设置
//! - 有功/无功/视在功率计算
//! - 电能累计
//! - 噪声模拟
//! - 虚拟串口服务
//! - 交互式 Shell

mod meter;
mod protocol;
mod serial;
mod shell;

pub use meter::*;
pub use protocol::*;
pub use serial::*;
pub use shell::*;
