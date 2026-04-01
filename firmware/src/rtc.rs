//! RTC 实时时钟驱动 — FM33A068EV
//!
//! 基于 32.768kHz 外部晶振，BCD 格式寄存器。
//! 提供日期时间读写、闹钟、亚秒、频率校准等功能。

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

// ══════════════════════════════════════════════════════════════════
// 数据结构
// ══════════════════════════════════════════════════════════════════

/// RTC 日期时间
#[derive(Clone, Copy, Debug)]
pub struct RtcTime {
    pub year:    u16,   // 2000~2099
    pub month:   u8,    // 1~12
    pub day:     u8,    // 1~31
    pub weekday: u8,    // 1=周一 ~ 7=周日
    pub hour:    u8,    // 0~23
    pub minute:  u8,    // 0~59
    pub second:  u8,    // 0~59
}

/// RTC 闹钟配置
#[derive(Clone, Copy, Debug)]
pub struct RtcAlarm {
    pub hour:    u8,
    pub minute:  u8,
    pub second:  u8,
    /// 星期匹配，0 = 不匹配星期
    pub weekday: u8,
    /// 亚秒匹配值（可选，0xFFFF 表示不匹配亚秒）
    pub subsecond: u16,
}

/// 闹钟回调类型
pub type AlarmCallback = fn();

// ══════════════════════════════════════════════════════════════════
// 全局状态
// ══════════════════════════════════════════════════════════════════

/// 闹钟回调函数指针
static mut ALARM_CB: Option<AlarmCallback> = None;

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

#[inline]
fn dec2bcd(v: u8) -> u8 {
    ((v / 10) << 4) | (v % 10)
}

#[inline]
fn bcd2dec(v: u8) -> u8 {
    (v >> 4) * 10 + (v & 0x0F)
}

#[inline]
fn bcd2dec_u16(v: u32) -> u16 {
    bcd2dec(((v >> 4) & 0x0F) as u8) as u16 * 10
        + bcd2dec((v & 0x0F) as u8) as u16
}

#[inline]
fn dec2bcd_u16(v: u16) -> u32 {
    let hi = dec2bcd(((v / 100) % 100) as u8) as u32;
    let lo = dec2bcd((v % 100) as u8) as u32;
    (hi << 8) | lo
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

        // 清中断标志
        reg_write(rtc, 0x08, ISR_ALMF | ISR_SCF);

        // 上锁写保护
        reg_write(rtc, 0x00, 0);
    }

    defmt::info!("RTC 初始化完成");
}

/// 获取当前时间
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

        RtcTime {
            year:    2000 + year,
            month:   bcd2dec(mon),
            day:     bcd2dec(day),
            weekday: bcd2dec(week),
            hour:    bcd2dec(hour),
            minute:  bcd2dec(min),
            second:  bcd2dec(sec),
        }
    }
}

/// 设置时间
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
    // TODO: 确认 SSR 的位宽和含义
    let ssr = unsafe { reg_read(rtc, 0x48) & 0x7FFF };
    ((ssr * 1000) / 32768) as u16
}

/// 设置闹钟回调
pub fn set_alarm_callback(cb: Option<AlarmCallback>) {
    cortex_m::interrupt::free(|_| {
        unsafe { ALARM_CB = cb; }
    });
}

/// 使能闹钟
pub fn enable_alarm(alarm: &RtcAlarm) {
    let rtc = fm33lg0::rtc();
    unsafe {
        reg_write(rtc, 0x00, WER_WEN);

        // TODO: 确认 ALARM 寄存器 BCD 格式
        let alarm_val = (dec2bcd(alarm.hour) as u32) << 16
                      | (dec2bcd(alarm.minute) as u32) << 8
                      | (dec2bcd(alarm.second) as u32);
        reg_write(rtc, 0x28, alarm_val);

        // 亚秒闹钟
        if alarm.subsecond != 0xFFFF {
            // TODO: 确认 SSA 寄存器格式
            reg_write(rtc, 0x4C, (alarm.subsecond as u32 * 32768) / 1000);
        }

        reg_write(rtc, 0x08, ISR_ALMF); // 清标志
        reg_modify(rtc, 0x04, |r| r | IER_ALMIE); // 使能中断

        reg_write(rtc, 0x00, 0);
    }

    // TODO: 使能 NVIC RTC 中断
    // unsafe { cortex_m::peripheral::NVIC::unmask(2); }
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
/// `ppm`: 校准值，范围约 -127 ~ +127 ppm
/// 正值加快时钟，负值减慢时钟
pub fn trim_ppm(ppm: i16) {
    let rtc = fm33lg0::rtc();
    unsafe {
        reg_write(rtc, 0x00, WER_WEN);

        // 校准方向
        if ppm >= 0 {
            reg_write(rtc, 0x34, 0); // 正方向（加快）
        } else {
            reg_write(rtc, 0x34, 1); // 负方向（减慢）
        }

        // TODO: 确认 adjust 和 calstep 寄存器精确含义
        reg_write(rtc, 0x30, ppm.unsigned_abs() as u32);

        reg_modify(rtc, 0x7C, |r| r | CR_CALEN);
        reg_write(rtc, 0x00, 0);
    }
}
