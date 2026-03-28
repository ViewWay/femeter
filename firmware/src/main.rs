//! FeMeter Smart Meter Firmware
//!
//! STM32F407VGT6 Cortex-M4F | RS-485 HDLC | RN8209C | LCD | Relay

#![no_main]
#![no_std]

extern crate alloc;

use cortex_m::asm;
use defmt_rtt as _;
use panic_halt as _;

mod board;
mod metering;
mod comm;
mod display;
mod task_scheduler;
mod power_manager;

use board::Board;
use task_scheduler::TaskScheduler;

// ── Global allocator (bump, 32KB) ──────────────────────

const HEAP_SIZE: usize = 32 * 1024;
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

// ── Vector table (cortex-m-rt compatible) ──────────────

use cortex_m_rt::{entry, exception};

#[entry]
fn main() -> ! {
    let mut board = Board::init();
    let mut scheduler = TaskScheduler::new();

    scheduler.register(0, 1);
    scheduler.register(1, 200);
    scheduler.register(2, 500);
    scheduler.register(3, 10);
    scheduler.register(4, 900_000);
    scheduler.register(5, 60_000);
    scheduler.register(6, 200);
    scheduler.register(7, 100);

    loop {
        let now = board.systick_ms();

        for task_id in scheduler.poll(now) {
            match task_id {
                0 => board.sample_energy(),
                1 => board.calculate_power(),
                2 => board.update_display(),
                3 => board.process_hdlc(),
                4 => board.capture_profile(),
                5 => board.check_tariff(),
                6 => board.check_alarms(),
                7 => board.feed_watchdog(),
                _ => {}
            }
        }

        asm::wfi();
    }
}
