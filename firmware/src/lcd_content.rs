/* ================================================================== */
/*                                                                    */
/*  LcdContent — LCD 显示内容数据结构                                  */
/*                                                                    */
/*  从 hal.rs 移到独立文件, 避免 hal.rs 臃肿                           */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

/// LCD 显示内容 (由应用层填充, display.rs 消费)
#[derive(Clone, Copy, Default, Debug)]
pub struct LcdContent {
    /// 电压显示值 (0.01V)
    pub voltage: u16,
    /// 电流显示值 (mA)
    pub current: u16,
    /// 有功功率 (W)
    pub active_power: i32,
    /// 无功功率 (var)
    pub reactive_power: i32,
    /// 功率因数 (0~1000)
    pub power_factor: u16,
    /// 频率 (0.01Hz)
    pub frequency: u16,
    /// 正向有功总电能 (0.01 kWh)
    pub active_import_energy: u64,
    /// 当前费率 (0~3)
    pub tariff: u8,
    /// 通信状态 (bit0=RS485, bit1=红外, bit2=LoRa, bit3=蜂窝)
    pub comm_status: u8,
    /// 告警标志 (bit0=过压, bit1=欠压, bit2=过流, bit3=功率反向)
    pub alarm_flags: u8,
}

/// LCD 显示模式
#[derive(Clone, Copy, Debug)]
pub enum LcdDisplayMode {
    /// 自动轮显
    AutoRotate { interval_sec: u8 },
    /// 手动翻页
    Manual,
    /// 掉电保持
    PowerOffHold,
    /// 测试 (全显)
    TestAllOn,
    /// 关闭
    Off,
}

impl Default for LcdDisplayMode {
    fn default() -> Self {
        Self::Off
    }
}

/// LCD bias
#[derive(Clone, Copy, Debug)]
pub enum LcdBias {
    Third,
    Quarter,
}

impl Default for LcdBias {
    fn default() -> Self {
        Self::Third
    }
}
