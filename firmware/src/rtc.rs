//! RTC 实时时钟驱动 — FM33A068EV
//!
//! 基于 32.768kHz 外部晶振，BCD 格式寄存器。
//! 提供日期时间读写、闹钟、亚秒、频率校准、NTP/PPS 对时接口等功能。

use crate::fm33lg0;

// ══════════════════════════════════════════════════════════════════
// 寄存器位定义（基于 FM33A0XXEV 参考手册）
// ══════════════════════════════════════════════════════════════════

/// WER (写使能)
const WER_WEN: u32 = 1 << 0;

/// IER (中断使能)
const IER_ALMIE: u32 = 1 << 0;   // 闹钟中断使能
const IER_SCIE:  u32 = 1 << 1;   // 秒中断使能

/// ISR (中断标志)
const ISR_ALMF: u32 = 1 << 0;    // 闹钟标志
const ISR_SCF:  u32 = 1 << 1;    // 秒标志

/// CR (控制)
const CR_RTCEN:  u32 = 1 << 0;   // RTC 使能
const CR_CALEN:  u32 = 1 << 1;   // 校准使能
const CR_SELCK:  u32 = 1 << 2;   // 时钟选择: 0=外部32.768k, 1=内部低速
const CR_CNTMD:  u32 = 1 << 3;   // 计数器模式: 1=日历模式, 0=纯计数

/// 闹钟匹配控制位
const ALM_EN_HOURLY: u32 = 1 << 24;  // 每小时匹配
const ALM_EN_DAILY:   u32 = 1 << 25;  // 每天匹配
const ALM_EN_WEEKDAY: u32 = 1 << 26;  // 星期匹配使能
const ALM_EN_SUBSEC:  u32 = 1 << 27;  // 亚秒匹配使能

// ══════════════════════════════════════════════════════════════════
// 数据结构
// ══════════════════════════════════════════════════════════════════

/// RTC 日期时间
#[derive(Clone, Copy, Debug)]
pub struct RtcTime {
    /// 年 (2000~2099)
    pub year:    u16,
    /// 月 (1~12)
    pub month:   u8,
    /// 日 (1~31)
    pub day:     u8,
    /// 星期 (1=周一 ~ 7=周日)
    pub weekday: u8,
    /// 时 (0~23)
    pub hour:    u8,
    /// 分 (0~59)
    pub minute:  u8,
    /// 秒 (0~59)
    pub second:  u8,
}

/// RTC 闹钟配置
#[derive(Clone, Copy, Debug)]
pub struct RtcAlarm {
    /// 时 (0~23)
    pub hour:    u8,
    /// 分 (0~59)
    pub minute:  u8,
    /// 秒 (0~59)
    pub second:  u8,
    /// 星期匹配，0 = 不匹配星期
    pub weekday: u8,
    /// 亚秒匹配值（可选，0xFFFF 表示不匹配亚秒）
    pub subsecond: u16,
    /// 闹钟模式
    pub mode: AlarmMode,
}

/// 闹钟模式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlarmMode {
    /// 单次触发
    Once,
    /// 每天重复
    Daily,
    /// 每周重复（指定 weekday）
    Weekly,
    /// 每小时重复
    Hourly,
}

/// RTC 时间同步源
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncSource {
    /// 未同步
    None,
    /// 通过 NTP/蜂窝网络同步
    Ntp,
    /// 通过 PPS (秒脉冲) 同步
    Pps,
    /// 从计量芯片内部时钟同步
    MeteringChip,
    /// 本地红外/RS485 对时
    Local,
}

/// 时间同步状态
#[derive(Clone, Copy, Debug)]
pub struct SyncStatus {
    /// 同步源
    pub source: SyncSource,
    /// 上次同步时间戳（秒）
    pub last_sync_timestamp: u32,
    /// 同步偏差（ms，上次同步时的时间差）
    pub last_offset_ms: i32,
    /// 是否已同步过
    pub synced: bool,
}

/// 闹钟回调类型
pub type AlarmCallback = fn();

// ══════════════════════════════════════════════════════════════════
// 全局状态
// ══════════════════════════════════════════════════════════════════

/// 闹钟回调函数指针
static mut ALARM_CB: Option<AlarmCallback> = None;

/// 时间同步状态
static mut SYNC_STATUS: SyncStatus = SyncStatus {
    source: SyncSource::None,
    last_sync_timestamp: 0,
    last_offset_ms: 0,
    synced: false,
};

// ══════════════════════════════════════════════════════════════════
// 寄存器读写辅助（volatile，因为 &'static RtcRegs 是不可变引用）
// ══════════════════════════════════════════════════════════════════

/// 写寄存器字段
#[inline]
unsafe fn reg_write(rtc: &fm33lg0::RtcRegs, offset: usize, val: u32) {
    let p = (rtc as *const _ as *const u8).add(offset) as *mut u32;
    core::ptr::write_volatile(p, val);
}

/// 读寄存器字段
#[inline]
unsafe fn reg_read(rtc: &fm33lg0::RtcRegs, offset: usize) -> u32 {
    let p = (rtc as *const _ as *const u8).add(offset) as *const u32;
    core::ptr::read_volatile(p)
}

/// 读改写寄存器字段
#[inline]
unsafe fn reg_modify(rtc: &fm33lg0::RtcRegs, offset: usize, f: impl Fn(u32) -> u32) {
    let old = reg_read(rtc, offset);
    reg_write(rtc, offset, f(old));
}

// ══════════════════════════════════════════════════════════════════
// BCD 工具函数
// ══════════════════════════════════════════════════════════════════

/// 十进制转 BCD
#[inline]
pub fn dec2bcd(v: u8) -> u8 {
    ((v / 10) << 4) | (v % 10)
}

/// BCD 转十进制
#[inline]
pub fn bcd2dec(v: u8) -> u8 {
    (v >> 4) * 10 + (v & 0x0F)
}

/// BCD 16-bit 转十进制（年寄存器）
#[inline]
fn bcd2dec_u16(v: u32) -> u16 {
    bcd2dec(((v >> 4) & 0x0F) as u8) as u16 * 10
        + bcd2dec((v & 0x0F) as u8) as u16
}

/// 16-bit 十进制转 BCD（年寄存器）
#[inline]
fn dec2bcd_u16(v: u16) -> u32 {
    let hi = dec2bcd(((v / 100) % 100) as u8) as u32;
    let lo = dec2bcd((v % 100) as u8) as u32;
    (hi << 8) | lo
}

/// BCD 值合法性检查
#[inline]
fn bcd_is_valid(v: u8) -> bool {
    (v & 0x0F) <= 9 && (v >> 4) <= 9
}

// ══════════════════════════════════════════════════════════════════
// defmt::Format 实现
// ══════════════════════════════════════════════════════════════════

impl defmt::Format for RtcTime {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} (w{})",
            self.year, self.month, self.day,
            self.hour, self.minute, self.second,
            self.weekday,
        );
    }
}

// ══════════════════════════════════════════════════════════════════
// 核心 API
// ══════════════════════════════════════════════════════════════════

/// 初始化 RTC
///
/// - 使能 32.768kHz 外部晶振
/// - 进入日历计数模式
/// - 默认时间 2000-01-01 00:00:00
/// - 配置亚秒计数器
pub fn init() {
    let rtc = fm33lg0::rtc();

    unsafe {
        // 解锁写保护
        reg_write(rtc, 0x00, WER_WEN);

        // 停止 RTC 以配置
        reg_write(rtc, 0x7C, 0);

        // 选择外部 32.768kHz 晶振，日历模式
        reg_write(rtc, 0x7C, CR_RTCEN | CR_CNTMD);

        // 设置默认时间
        let default_time = RtcTime {
            year: 2000, month: 1, day: 1,
            weekday: 6, // 2000-01-01 是周六
            hour: 0, minute: 0, second: 0,
        };
        set_time_inner(rtc, &default_time);

        // 配置亚秒寄存器清零
        reg_write(rtc, 0x48, 0);

        // 清中断标志
        reg_write(rtc, 0x08, ISR_ALMF | ISR_SCF);

        // 上锁写保护
        reg_write(rtc, 0x00, 0);
    }

    defmt::info!("RTC 初始化完成");
}

/// 获取当前时间
///
/// 读取 RTC BCD 寄存器并转换为十进制。
/// 返回包含年月日时分秒和星期的 `RtcTime`。
pub fn get_time() -> RtcTime {
    let rtc = fm33lg0::rtc();

    unsafe {
        let sec  = (reg_read(rtc, 0x0C) & 0x7F) as u8;
        let min  = (reg_read(rtc, 0x10) & 0x7F) as u8;
        let hour = (reg_read(rtc, 0x14) & 0x3F) as u8;
        let day  = (reg_read(rtc, 0x18) & 0x3F) as u8;
        let week = (reg_read(rtc, 0x1C) & 0x07) as u8;
        let mon  = (reg_read(rtc, 0x20) & 0x1F) as u8;
        let year = bcd2dec_u16(reg_read(rtc, 0x24) & 0xFF);

        // BCD 合法性检查
        let (sec, min, hour, day, week, mon) = (
            if bcd_is_valid(sec) { bcd2dec(sec) } else { 0 },
            if bcd_is_valid(min) { bcd2dec(min) } else { 0 },
            if bcd_is_valid(hour) { bcd2dec(hour) } else { 0 },
            if bcd_is_valid(day) { bcd2dec(day) } else { 1 },
            if bcd_is_valid(week) { bcd2dec(week) } else { 0 },
            if bcd_is_valid(mon) { bcd2dec(mon) } else { 1 },
        );

        RtcTime {
            year:    2000 + year,
            month:   mon,
            day:     day,
            weekday: week,
            hour:    hour,
            minute:  min,
            second:  sec,
        }
    }
}

/// 设置时间
///
/// `t`: 要设置的日期时间，自动转换为 BCD 格式写入寄存器。
pub fn set_time(t: &RtcTime) {
    let rtc = fm33lg0::rtc();
    unsafe { set_time_inner(rtc, t) };
}

/// 内部设置时间（已假设写保护已解锁）
unsafe fn set_time_inner(rtc: &fm33lg0::RtcRegs, t: &RtcTime) {
    reg_write(rtc, 0x00, WER_WEN);

    let y = t.year.saturating_sub(2000).min(99);

    reg_write(rtc, 0x0C, dec2bcd(t.second) as u32);
    reg_write(rtc, 0x10, dec2bcd(t.minute) as u32);
    reg_write(rtc, 0x14, dec2bcd(t.hour) as u32);
    reg_write(rtc, 0x18, dec2bcd(t.day) as u32);
    reg_write(rtc, 0x1C, dec2bcd(t.weekday) as u32);
    reg_write(rtc, 0x20, dec2bcd(t.month) as u32);
    reg_write(rtc, 0x24, dec2bcd_u16(y));

    reg_write(rtc, 0x00, 0);
}

/// 获取从 2000-01-01 起的秒数（简化 Unix 时间戳）
///
/// 用于电表记录的时间戳，不处理闰秒。
pub fn get_timestamp() -> u32 {
    let t = get_time();

    /// 每月前累计天数（平年）
    const DAYS_BEFORE: [u16; 13] = [0, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

    let y = t.year.saturating_sub(2000);
    let mut days: u32 = y as u32 * 365;
    // 2000年起每4年一个闰年
    days += ((y / 4) + 1) as u32;
    // 当年月份累计
    let m = t.month.min(12) as usize;
    days += DAYS_BEFORE[m] as u32;
    // 闰年且过2月
    if t.year % 4 == 0 && t.month > 2 {
        days += 1;
    }
    days += t.day as u32;

    days * 86400 + t.hour as u32 * 3600 + t.minute as u32 * 60 + t.second as u32
}

/// 获取亚秒毫秒值 (0~999)
///
/// 基于 SSR 亚秒寄存器，32.768kHz → 1秒 = 32768 tick
pub fn get_subsecond_ms() -> u16 {
    let rtc = fm33lg0::rtc();
    let ssr = unsafe { reg_read(rtc, 0x48) & 0x7FFF };
    ((ssr * 1000) / 32768) as u16
}

/// 获取亚秒 tick 值 (0~32767)
///
/// 原始 32.768kHz 计数值，可用于高精度时间测量。
pub fn get_subsecond_raw() -> u16 {
    let rtc = fm33lg0::rtc();
    (unsafe { reg_read(rtc, 0x48) & 0x7FFF }) as u16
}

/// 设置闹钟回调
///
/// `cb`: 闹钟触发时调用的函数指针，`None` 禁用回调。
pub fn set_alarm_callback(cb: Option<AlarmCallback>) {
    cortex_m::interrupt::free(|_| {
        unsafe { ALARM_CB = cb; }
    });
}

/// 使能闹钟
///
/// `alarm`: 闹钟配置，包含时/分/秒/星期/亚秒和触发模式。
pub fn enable_alarm(alarm: &RtcAlarm) {
    let rtc = fm33lg0::rtc();
    unsafe {
        reg_write(rtc, 0x00, WER_WEN);

        // 写入闹钟时间 (BCD)
        let alarm_val = (dec2bcd(alarm.hour) as u32) << 16
                      | (dec2bcd(alarm.minute) as u32) << 8
                      | (dec2bcd(alarm.second) as u32);

        // 添加匹配控制位
        let mut ctrl = 0u32;
        match alarm.mode {
            AlarmMode::Once => {}
            AlarmMode::Daily => { ctrl |= ALM_EN_DAILY; }
            AlarmMode::Weekly => { ctrl |= ALM_EN_DAILY | ALM_EN_WEEKDAY; }
            AlarmMode::Hourly => { ctrl |= ALM_EN_HOURLY; }
        }

        // 亚秒匹配
        if alarm.subsecond != 0xFFFF {
            ctrl |= ALM_EN_SUBSEC;
            reg_write(rtc, 0x4C, (alarm.subsecond as u32 * 32768) / 1000);
        }

        reg_write(rtc, 0x28, alarm_val | ctrl);

        // 清标志 + 使能中断
        reg_write(rtc, 0x08, ISR_ALMF);
        reg_modify(rtc, 0x04, |r| r | IER_ALMIE);

        reg_write(rtc, 0x00, 0);
    }
}

/// 禁用闹钟
pub fn disable_alarm() {
    let rtc = fm33lg0::rtc();
    unsafe {
        reg_write(rtc, 0x00, WER_WEN);
        reg_modify(rtc, 0x04, |r| r & !IER_ALMIE);
        reg_write(rtc, 0x08, ISR_ALMF);
        reg_write(rtc, 0x00, 0);
    }
}

/// 检查闹钟是否已触发
///
/// 非阻塞检查闹钟标志位。
pub fn is_alarm_triggered() -> bool {
    let rtc = fm33lg0::rtc();
    unsafe { reg_read(rtc, 0x08) & ISR_ALMF != 0 }
}

/// RTC 闹钟中断处理（在 RTC ISR 中调用）
pub fn irq_handler() {
    let rtc = fm33lg0::rtc();
    unsafe {
        if reg_read(rtc, 0x08) & ISR_ALMF != 0 {
            reg_write(rtc, 0x08, ISR_ALMF);
        }
    }

    cortex_m::interrupt::free(|_| {
        unsafe {
            if let Some(cb) = ALARM_CB {
                cb();
            }
        }
    });
}

/// RTC 频率校准
///
/// `ppm`: 校准值，范围约 -127 ~ +127 ppm。
/// 正值加快时钟，负值减慢时钟。
pub fn trim_ppm(ppm: i16) {
    let rtc = fm33lg0::rtc();
    unsafe {
        reg_write(rtc, 0x00, WER_WEN);

        if ppm >= 0 {
            reg_write(rtc, 0x34, 0);
        } else {
            reg_write(rtc, 0x34, 1);
        }

        reg_write(rtc, 0x30, ppm.unsigned_abs() as u32);

        reg_modify(rtc, 0x7C, |r| r | CR_CALEN);
        reg_write(rtc, 0x00, 0);
    }
}

/// NTP / 网络对时接口
///
/// `unix_ts`: Unix 时间戳（秒，从 1970-01-01 起）。
/// `offset_ms`: NTP 偏差（毫秒），用于校准 RTC 频率。
///
/// 将 Unix 时间戳转换为 RTC 时间并设置，同时记录同步状态。
pub fn sync_from_ntp(unix_ts: u32, offset_ms: i32) {
    // Unix epoch (1970-01-01) → RTC epoch (2000-01-01) 偏移
    const UNIX_TO_RTC_OFFSET: u32 = 946684800; // 30 years

    if unix_ts < UNIX_TO_RTC_OFFSET {
        return;
    }

    let rtc_ts = unix_ts - UNIX_TO_RTC_OFFSET;

    // 转换为日期时间
    let days = rtc_ts / 86400;
    let time_of_day = rtc_ts % 86400;
    let hour = (time_of_day / 3600) as u8;
    let minute = ((time_of_day % 3600) / 60) as u8;
    let second = (time_of_day % 60) as u8;

    let (year, month, day, weekday) = days_to_date(days);

    let t = RtcTime {
        year: 2000 + year as u16,
        month,
        day,
        weekday,
        hour,
        minute,
        second,
    };

    set_time(&t);

    // 根据 NTP 偏差微调 RTC 频率
    // 简单的线性校准：偏差 > 100ms 时调整 ppm
    if offset_ms.abs() > 100 {
        let ppm = (offset_ms as i32 / 100).clamp(-127, 127) as i16;
        trim_ppm(ppm);
    }

    // 更新同步状态
    cortex_m::interrupt::free(|_| {
        unsafe {
            SYNC_STATUS = SyncStatus {
                source: SyncSource::Ntp,
                last_sync_timestamp: get_timestamp(),
                last_offset_ms: offset_ms,
                synced: true,
            };
        }
    });

    defmt::info!("RTC NTP 同步完成, offset={}ms", offset_ms);
}

/// PPS (秒脉冲) 对时接口
///
/// `pps_ts`: PPS 信号对应的 Unix 时间戳。
/// 在 PPS 上升沿中断中调用，实现微秒级对时。
pub fn sync_from_pps(pps_ts: u32) {
    // PPS 对时：在整秒时刻设置 RTC 秒寄存器
    // 亚秒清零，因为 PPS 信号本身是精确的 1Hz
    let rtc = fm33lg0::rtc();

    const UNIX_TO_RTC_OFFSET: u32 = 946684800;
    if pps_ts < UNIX_TO_RTC_OFFSET { return; }

    let rtc_ts = pps_ts - UNIX_TO_RTC_OFFSET;
    let time_of_day = rtc_ts % 86400;
    let hour = (time_of_day / 3600) as u8;
    let minute = ((time_of_day % 3600) / 60) as u8;
    let second = (time_of_day % 60) as u8;

    unsafe {
        reg_write(rtc, 0x00, WER_WEN);
        reg_write(rtc, 0x0C, dec2bcd(second) as u32);
        reg_write(rtc, 0x10, dec2bcd(minute) as u32);
        reg_write(rtc, 0x14, dec2bcd(hour) as u32);
        reg_write(rtc, 0x48, 0); // 亚秒清零
        reg_write(rtc, 0x00, 0);
    }

    cortex_m::interrupt::free(|_| {
        unsafe {
            SYNC_STATUS = SyncStatus {
                source: SyncSource::Pps,
                last_sync_timestamp: get_timestamp(),
                last_offset_ms: 0,
                synced: true,
            };
        }
    });

    defmt::info!("RTC PPS 同步完成");
}

/// 从计量芯片同步 RTC
///
/// ATT7022E 有内部 32.768kHz 时钟，可提供精确时间。
/// `chip_time`: 从计量芯片读取的 BCD 时间数据 (秒/分/时寄存器值)。
pub fn sync_from_metering_chip(chip_time: &RtcTime) {
    // 验证时间合法性
    if chip_time.month < 1 || chip_time.month > 12 { return; }
    if chip_time.day < 1 || chip_time.day > 31 { return; }
    if chip_time.hour > 23 { return; }
    if chip_time.minute > 59 { return; }
    if chip_time.second > 59 { return; }

    set_time(chip_time);

    cortex_m::interrupt::free(|_| {
        unsafe {
            SYNC_STATUS = SyncStatus {
                source: SyncSource::MeteringChip,
                last_sync_timestamp: get_timestamp(),
                last_offset_ms: 0,
                synced: true,
            };
        }
    });

    defmt::info!("RTC 从计量芯片同步完成");
}

/// 获取时间同步状态
pub fn sync_status() -> SyncStatus {
    cortex_m::interrupt::free(|_| {
        unsafe { SYNC_STATUS }
    })
}

/// 检查 RTC 是否已同步过
pub fn is_synced() -> bool {
    sync_status().synced
}

/// 将天数转换为年月日和星期
///
/// `days`: 从 2000-01-01 起的天数。
/// 返回 `(year, month, day, weekday)`。
fn days_to_date(days: u32) -> (u32, u8, u8, u8) {
    /// 每月天数（平年）
    const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut remaining = days;
    let mut year: u32 = 0;

    // 计算年
    loop {
        let days_in_year = if is_leap_year(2000 + year) { 366 } else { 365 };
        if remaining < days_in_year { break; }
        remaining -= days_in_year;
        year += 1;
    }

    // 计算月
    let mut month: u8 = 1;
    for &dim in DAYS_IN_MONTH.iter() {
        let mut days = dim as u32;
        if month == 2 && is_leap_year(2000 + year) {
            days = 29;
        }
        if remaining < days { break; }
        remaining -= days;
        month += 1;
    }

    let day = (remaining + 1) as u8;

    // 计算星期 (2000-01-01 = 周六 = 6)
    // Zeller 公式简化
    let weekday = ((days + 5) % 7 + 1) as u8; // 1=Mon

    (year, month, day, weekday)
}

/// 判断闰年
fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
