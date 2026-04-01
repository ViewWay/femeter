/* ================================================================== */
/*                                                                    */
/*  key_scan.rs — 按键扫描驱动                                         */
/*                                                                    */
/*  2 个按键:                                                         */
/*    - 翻页键 (PB0): 短按翻页，长按 3s 进入编程模式                    */
/*    - 编程键 (PB1): 切换编程模式（需密码确认）                        */
/*                                                                    */
/*  去抖: GPIO 中断触发 + 定时器轮询 (20ms)                           */
/*  状态机: Idle → Debounce → Pressed → Released / LongPress          */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use crate::board::pins;
use crate::hal::GpioPin;

/* ================================================================== */
/*  按键事件                                                           */
/* ================================================================== */

/// 按键 ID
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyId {
    /// 翻页键 (PB0)
    Page,
    /// 编程键 (PB1)
    Prog,
}

/// 按键事件
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyEvent {
    /// 按键按下（去抖确认后）
    Press(KeyId),
    /// 按键释放
    Release(KeyId),
    /// 长按（翻页键 3s）
    LongPress(KeyId),
}

/* ================================================================== */
/*  LCD 显示页                                                         */
/* ================================================================== */

/// LCD 显示页（共 4 页）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DisplayPage {
    /// 第 1 页: 实时数据（电压/电流/功率/功率因数/频率）
    Realtime = 0,
    /// 第 2 页: 电能（正向有功/反向有功/正向无功/反向无功）
    Energy = 1,
    /// 第 3 页: 事件记录（开盖/失压/断相/编程等）
    Events = 2,
    /// 第 4 页: 表计信息（地址/版本/通信参数）
    Info = 3,
}

impl DisplayPage {
    pub const COUNT: u8 = 4;

    /// 切换到下一页（循环）
    pub fn next(self) -> Self {
        match self {
            DisplayPage::Realtime => DisplayPage::Energy,
            DisplayPage::Energy => DisplayPage::Events,
            DisplayPage::Events => DisplayPage::Info,
            DisplayPage::Info => DisplayPage::Realtime,
        }
    }
}

/* ================================================================== */
/*  按键状态机                                                         */
/* ================================================================== */

/// 按键扫描状态
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KeyState {
    /// 空闲，等待按下
    Idle,
    /// 去抖中，等待 20ms 稳定
    Debounce,
    /// 已按下，等待释放或超时（长按检测）
    Pressed,
}

/// 单个按键的运行时状态
#[derive(Clone, Copy)]
struct KeyChannel {
    /// 状态机当前状态
    state: KeyState,
    /// 按键 ID
    id: KeyId,
    /// GPIO 引脚
    pin: GpioPin,
    /// 去抖定时器计数（ms）
    debounce_ms: u16,
    /// 按下持续时间（ms），用于长按检测
    press_duration_ms: u32,
    /// 上次读取的电平（true=高, false=低, 低=按下）
    last_level: bool,
}

/* ================================================================== */
/*  长按阈值常量                                                       */
/* ================================================================== */

/// 去抖时间（毫秒）
const DEBOUNCE_MS: u16 = 20;
/// 长按判定时间（毫秒）
const LONG_PRESS_MS: u32 = 3000;

/* ================================================================== */
/*  KeyDriver trait — 硬件抽象                                         */
/* ================================================================== */

/// 按键硬件驱动 trait
pub trait KeyDriver {
    /// 读取指定按键的电平（true=未按下, false=按下）
    fn read_key(&self, key: KeyId) -> bool;
    /// 获取当前 tick 数（毫秒）
    fn get_tick_ms(&self) -> u32;
}

/* ================================================================== */
/*  KeyHandler trait — 事件回调                                        */
/* ================================================================== */

/// 按键事件回调 trait
pub trait KeyHandler {
    /// 处理按键事件
    fn on_key_event(&mut self, event: KeyEvent);
}

/* ================================================================== */
/*  KeyScanner — 按键扫描器                                            */
/* ================================================================== */

/// 按键扫描器（非中断模式，在任务中周期调用）
///
/// 用法:
///   1. 在 10ms 定时任务中调用 `tick()`
///   2. 扫描器自动去抖和检测长按
///   3. 事件通过 KeyHandler 回调
pub struct KeyScanner<D: KeyDriver> {
    /// 硬件驱动
    driver: D,
    /// 翻页键通道
    page_key: KeyChannel,
    /// 编程键通道
    prog_key: KeyChannel,
    /// 当前 LCD 显示页
    current_page: DisplayPage,
    /// 是否处于编程模式
    programming_mode: bool,
    /// 编程模式超时计数（分钟, 超时自动退出）
    prog_timeout_min: u16,
    // 事件通过 tick() 返回 KeyEvent 让调用者处理
}

impl<D: KeyDriver> KeyScanner<D> {
    /// 创建按键扫描器
    pub fn new(driver: D) -> Self {
        Self {
            driver,
            page_key: KeyChannel {
                state: KeyState::Idle,
                id: KeyId::Page,
                pin: pins::KEY_PAGE,
                debounce_ms: 0,
                press_duration_ms: 0,
                last_level: true, // 上拉，默认高电平
            },
            prog_key: KeyChannel {
                state: KeyState::Idle,
                id: KeyId::Prog,
                pin: pins::KEY_PROG,
                debounce_ms: 0,
                press_duration_ms: 0,
                last_level: true,
            },
            current_page: DisplayPage::Realtime,
            programming_mode: false,
            prog_timeout_min: 0,
        }
    }

    /// 周期性调用（建议每 1~5ms 调用一次）
    ///
    /// 返回本次产生的事件（最多 1 个），由调用者处理
    pub fn tick(&mut self) -> Option<KeyEvent> {
        let mut event = None;

        // 处理两个按键（先读 GPIO 状态，再更新状态机）
        let page_level = self.driver.read_key(KeyId::Page);
        let prog_level = self.driver.read_key(KeyId::Prog);
        let tick = self.driver.get_tick_ms();

        event = Self::tick_channel_inner(&mut self.page_key, page_level, tick);
        if event.is_none() {
            event = Self::tick_channel_inner(&mut self.prog_key, prog_level, tick);
        }

        // 处理事件（页切换 / 编程模式）
        if let Some(e) = event {
            self.handle_event(e);
        }

        event
    }

    /// 处理单个按键通道的状态机（纯函数，无 self 借用）
    fn tick_channel_inner(ch: &mut KeyChannel, level: bool, _tick: u32) -> Option<KeyEvent> {

        match ch.state {
            KeyState::Idle => {
                // 检测到按下（低电平）
                if !level {
                    ch.state = KeyState::Debounce;
                    ch.debounce_ms = 0;
                    ch.last_level = level;
                }
                None
            }
            KeyState::Debounce => {
                ch.debounce_ms += 1;
                // 去抖期间再次检测
                if !level {
                    // 仍然低电平，去抖完成
                    if ch.debounce_ms >= DEBOUNCE_MS {
                        ch.state = KeyState::Pressed;
                        ch.press_duration_ms = 0;
                        return Some(KeyEvent::Press(ch.id));
                    }
                } else {
                    // 电平恢复，抖动，回空闲
                    ch.state = KeyState::Idle;
                }
                ch.last_level = level;
                None
            }
            KeyState::Pressed => {
                ch.press_duration_ms += 1;

                if level {
                    // 释放
                    ch.state = KeyState::Idle;
                    return Some(KeyEvent::Release(ch.id));
                }

                // 长按检测（仅翻页键）
                if ch.id == KeyId::Page && ch.press_duration_ms >= LONG_PRESS_MS {
                    ch.state = KeyState::Idle;
                    return Some(KeyEvent::LongPress(ch.id));
                }

                ch.last_level = level;
                None
            }
        }
    }

    /// 处理按键事件（页切换逻辑）
    fn handle_event(&mut self, event: KeyEvent) {
        match event {
            KeyEvent::Release(KeyId::Page) => {
                // 短按翻页键 → 切换显示页
                if !self.programming_mode {
                    self.current_page = self.current_page.next();
                }
            }
            KeyEvent::LongPress(KeyId::Page) => {
                // 翻页键长按 3s → 进入编程模式（需密码）
                // TODO: 实际密码校验逻辑，暂用 placeholder
                self.enter_programming_mode();
            }
            KeyEvent::Release(KeyId::Prog) => {
                // 编程键短按 → 切换编程模式
                if self.programming_mode {
                    self.exit_programming_mode();
                } else {
                    self.enter_programming_mode();
                }
            }
            _ => {}
        }
    }

    /// 进入编程模式（placeholder: 密码校验）
    fn enter_programming_mode(&mut self) {
        // TODO: 实际密码校验:
        //   1. 显示密码输入界面
        //   2. 通过红外/RS485 接收密码
        //   3. 校验 DLMS/COSEM 密码 (默认 00000000)
        //   4. 校验失败则不进入编程模式
        //
        // 暂时直接进入
        self.programming_mode = true;
        self.prog_timeout_min = 0;
    }

    /// 退出编程模式
    fn exit_programming_mode(&mut self) {
        self.programming_mode = false;
        self.prog_timeout_min = 0;
    }

    /// 每分钟调用一次，用于编程模式超时检测
    pub fn tick_minute(&mut self) {
        if self.programming_mode {
            self.prog_timeout_min += 1;
            // 编程模式超时: 10 分钟无操作自动退出
            // DLMS/COSEM 标准要求编程模式有超时保护
            if self.prog_timeout_min >= 10 {
                self.exit_programming_mode();
            }
        }
    }

    /// 获取当前显示页
    pub fn current_page(&self) -> DisplayPage {
        self.current_page
    }

    /// 设置当前显示页
    pub fn set_page(&mut self, page: DisplayPage) {
        self.current_page = page;
    }

    /// 是否处于编程模式
    pub fn is_programming(&self) -> bool {
        self.programming_mode
    }

    /// 重置（唤醒后调用）
    pub fn reset(&mut self) {
        self.page_key.state = KeyState::Idle;
        self.prog_key.state = KeyState::Idle;
        self.page_key.debounce_ms = 0;
        self.prog_key.debounce_ms = 0;
        self.page_key.press_duration_ms = 0;
        self.prog_key.press_duration_ms = 0;
    }
}

/* ================================================================== */
/*  DefaultKeyDriver — 默认硬件驱动                                     */
/* ================================================================== */

/// 默认按键驱动，直接读取 GPIO
pub struct DefaultKeyDriver {
    /// 上次 tick（用于计算 ms 差值）
    last_tick: u32,
}

impl DefaultKeyDriver {
    pub const fn new() -> Self {
        Self { last_tick: 0 }
    }

    /// 初始化按键 GPIO（上拉输入）
    pub fn init(&self) {
        for pin in &[pins::KEY_PAGE, pins::KEY_PROG] {
            // GPIO 已在 board.rs::gpio_init() 中配置为上拉输入
        // 这里无需重复配置
        }
    }
}

impl KeyDriver for DefaultKeyDriver {
    fn read_key(&self, key: KeyId) -> bool {
        let pin = match key {
            KeyId::Page => pins::KEY_PAGE,
            KeyId::Prog => pins::KEY_PROG,
        };
        // 上拉输入: 低电平 = 按下, 高电平 = 释放
        crate::board::gpio_read_pin(pin)
    }

    fn get_tick_ms(&self) -> u32 {
        // TODO: 集成 FreeRTOS xTaskGetTickCount()
        // 当前返回 0，实际应使用 FreeRTOS tick (configTICK_RATE_HZ = 1000)
        // tick_ms = xTaskGetTickCount() * portTICK_PERIOD_MS
        0
    }
}
