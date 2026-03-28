//! FeMeter 智能电表固件 — FM33LG0xx (复旦微电子)
//!
//! 硬件平台:
//!   MCU:  FM33LG0xx Cortex-M0+ @ 64MHz
//!   Flash: 256KB  RAM: 32KB (24K main + 8K battery-backed)
//!   内置外设: LCD控制器(4x40段码), UARTx4, SPIx2, I2Cx2, ADC 12-bit, RTC, AES-128
//!
//! 通信:
//!   RS-485: UART0, HDLC/DLMS, 9600/19200/115200 bps, 8N1
//!   红外:   UART1, IEC 62056-21 (mode C/D), 300/2400/9600 bps, 8N1
//!   模块通信: UART2, 38400 bps, 8N1 (for metering module communication)
//!
//! 计量: 外部计量芯片 (ATT7022/BL6523/BL0937) via SPI0
//!
//! 显示: 内置 LCD 控制器 (4COM x 40SEG, 段码/米字/8字)

#![no_main]
#![no_std]

extern crate alloc;

use cortex_m::asm;
use cortex_m_rt::entry;
use defmt_rtt as _;
use panic_halt as _;

mod board;
mod metering;
mod comm;
mod display;
mod task_scheduler;
mod fm33lg0;
mod lcd;

use board::Board;
use task_scheduler::TaskScheduler;

// ── Bump allocator (16KB heap, leave 8KB for stack + globals) ──────

const HEAP_SIZE: usize = 16 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];
static mut HEAP_PTR: usize = 0;

struct BumpAlloc;

unsafe impl core::alloc::GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();
        unsafe {
            let base = HEAP.as_ptr() as usize;
            let ptr = HEAP_PTR;
            let aligned = (base + ptr + align - 1) & !(align - 1);
            let offset = aligned - base;
            if offset + size > HEAP_SIZE {
                return core::ptr::null_mut();
            }
            HEAP_PTR = offset + size;
            aligned as *mut u8
        }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[global_allocator]
static ALLOCATOR: BumpAlloc = BumpAlloc;

// ── Main entry ─────────────────────────────────────────────────────

#[entry]
fn main() -> ! {
    // Phase 1: 硬件初始化
    let mut board = Board::init();

    // Phase 2: 从 Flash 读取校准参数和计量配置
    board.load_calibration();

    // Phase 3: 初始化任务调度器
    let mut scheduler = TaskScheduler::new();

    // ┌──────────────────────────────────────────────────────┐
    // │  任务ID  │ 周期    │ 说明                              │
    // ├──────────┼─────────┼───────────────────────────────────┤
    // │  0       │ 1ms     │ SPI读取计量芯片 (ATT7022/BL6523) │
    // │  1       │ 200ms   │ 功率计算 + 能量累加               │
    // │  2       │ 500ms   │ LCD显示刷新                       │
    // │  3       │ 10ms    │ RS-485 HDLC 收发                  │
    // │  4       │ 50ms    │ 红外通信处理                      │
    // │  5       │ 1000ms  │ 模块UART通信 (38400 bps)          │
    // │  6       │ 900000ms│ 负荷曲线捕获 (15分钟)              │
    // │  7       │ 60000ms │ 费率时段切换检查                   │
    // │  8       │ 200ms   │ 越限告警检查                       │
    // │  9       │ 100ms   │ 看门狗喂狗                        │
    // └──────────────────────────────────────────────────────┘
    scheduler.register(0,  1);         // 计量采样
    scheduler.register(1,  200);       // 功率计算
    scheduler.register(2,  500);       // 显示刷新
    scheduler.register(3,  10);        // RS-485 HDLC
    scheduler.register(4,  50);        // 红外通信
    scheduler.register(5,  1000);      // 模块通信
    scheduler.register(6,  900_000);   // 负荷曲线
    scheduler.register(7,  60_000);    // 费率检查
    scheduler.register(8,  200);       // 告警
    scheduler.register(9,  100);       // 看门狗

    // Phase 4: 主循环 (super-loop)
    loop {
        let now = board.systick_ms();

        for task_id in scheduler.poll(now) {
            match task_id {
                0 => board.sample_metering(),
                1 => board.calculate_power_energy(),
                2 => board.refresh_display(),
                3 => board.process_rs485_hdlc(),
                4 => board.process_infrared(),
                5 => board.process_module_uart(),
                6 => board.capture_load_profile(),
                7 => board.check_tariff_schedule(),
                8 => board.check_alarm_thresholds(),
                9 => board.feed_watchdog(),
                _ => {}
            }
        }

        // Wait For Interrupt — 低功耗
        asm::wfi();
    }
}
