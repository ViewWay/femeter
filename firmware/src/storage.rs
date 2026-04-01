/* ================================================================== */
/*                                                                    */
/*  storage.rs — 外挂 Flash 存储管理 (W25Q64)                          */
/*                                                                    */
/*  8MB Flash, 分区管理:                                                */
/*    0x000000 - 0x00FFFF: 参数区 (64KB, 系统参数/校准数据)             */
/*    0x010000 - 0x07FFFF: 电能数据区 (448KB, 每日一条记录)             */
/*    0x080000 - 0x0FFFFF: 事件日志区 (512KB)                          */
/*    0x100000 - 0x1FFFFF: 负荷曲线区 (1MB, 间隔记录)                   */
/*    0x200000 - 0x7FFFFF: 保留 (6MB)                                  */
/*                                                                    */
/*  SPI 模式: 0 (CPOL=0, CPHA=0), 最高 80MHz                          */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

use core::ops::Range;

/* ── Flash 分区定义 ── */

pub struct FlashPartition {
    pub name: &'static str,
    pub range: Range<u32>,
    pub sector_size: u32,
}

/// W25Q64 分区表
pub const PARTITIONS: &[FlashPartition] = &[
    FlashPartition { name: "params",   range: 0x000000..0x010000, sector_size: 4096 },   // 64KB
    FlashPartition { name: "energy",   range: 0x010000..0x080000, sector_size: 4096 },   // 448KB
    FlashPartition { name: "events",   range: 0x080000..0x100000, sector_size: 4096 },   // 512KB
    FlashPartition { name: "load",     range: 0x100000..0x200000, sector_size: 4096 },   // 1MB
    FlashPartition { name: "reserved", range: 0x200000..0x800000, sector_size: 4096 },   // 6MB
];

/// 按 name 查找分区
pub fn find_partition(name: &str) -> Option<&'static FlashPartition> {
    PARTITIONS.iter().find(|p| p.name == name)
}

/* ── W25Q64 SPI 指令 ── */

pub mod cmd {
    pub const WRITE_ENABLE:   u8 = 0x06;
    pub const WRITE_DISABLE:  u8 = 0x04;
    pub const READ_STATUS:    u8 = 0x05;
    pub const WRITE_STATUS:   u8 = 0x01;
    pub const READ_DATA:      u8 = 0x03;
    pub const FAST_READ:      u8 = 0x0B;
    pub const PAGE_PROGRAM:   u8 = 0x02;
    pub const SECTOR_ERASE:   u8 = 0x20;
    pub const BLOCK_ERASE_32: u8 = 0x52;
    pub const BLOCK_ERASE_64: u8 = 0xD8;
    pub const CHIP_ERASE:     u8 = 0xC7;
    pub const READ_ID:        u8 = 0x9F;
    pub const READ_UID:       u8 = 0x4B;
    pub const POWER_DOWN:     u8 = 0xB9;
    pub const RELEASE_PD:     u8 = 0xAB;
}

/* ── SPI 传输 trait ── */

/// Flash SPI 传输接口 (由 board.rs 实现)
pub trait FlashSpi {
    /// SPI 读写 (发送 tx, 同时接收 rx)
    fn spi_transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), ()>;
    /// 选中 Flash (CS 拉低)
    fn cs_low(&mut self);
    /// 释放 Flash (CS 拉高)
    fn cs_high(&mut self);
    /// 延时 (ms)
    fn delay_ms(&mut self, ms: u32);
}

/* ── 状态寄存器位 ── */

const STATUS_BUSY: u8 = 0x01;
const STATUS_WEL:  u8 = 0x02;

/* ── W25Q64 驱动 ── */

pub struct W25Q64<SPI: FlashSpi> {
    spi: SPI,
}

impl<SPI: FlashSpi> W25Q64<SPI> {
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    /// 初始化: 读 JEDEC ID 验证
    pub fn init(&mut self) -> Result<u32, ()> {
        let id = self.read_jedec_id()?;
        // W25Q64 JEDEC ID: 0xEF4017
        if id != 0xEF4017 {
            // 也接受 0xC84017 (Winbond clone) 和其他兼容型号
            // 只要厂商 ID 非 0 即可
            if (id >> 16) == 0 {
                return Err(());
            }
        }
        Ok(id)
    }

    /// 读取 JEDEC ID (3 bytes)
    pub fn read_jedec_id(&mut self) -> Result<u32, ()> {
        let mut rx = [0u8; 4];
        let tx = [cmd::READ_ID, 0, 0, 0];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut rx)?;
        self.spi.cs_high();
        Ok((rx[1] as u32) << 16 | (rx[2] as u32) << 8 | rx[3] as u32)
    }

    /// 读状态寄存器
    fn read_status(&mut self) -> Result<u8, ()> {
        let mut rx = [0u8; 2];
        let tx = [cmd::READ_STATUS, 0];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut rx)?;
        self.spi.cs_high();
        Ok(rx[1])
    }

    /// 等待写入完成
    fn wait_busy(&mut self) {
        while self.read_status().map_or(true, |s| s & STATUS_BUSY != 0) {
            self.spi.delay_ms(1);
        }
    }

    /// 写使能
    fn write_enable(&mut self) -> Result<(), ()> {
        self.wait_busy();
        let tx = [cmd::WRITE_ENABLE];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 1])?;
        self.spi.cs_high();
        Ok(())
    }

    /// 读数据
    pub fn read(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), ()> {
        let mut tx = [0u8; 5];
        tx[0] = cmd::READ_DATA;
        tx[1] = (addr >> 16) as u8;
        tx[2] = (addr >> 8) as u8;
        tx[3] = addr as u8;

        self.spi.cs_low();
        self.spi.spi_transfer(&tx[..4], &mut [0; 4])?;
        self.spi.spi_transfer(&[0; buf.len()], buf)?;
        self.spi.cs_high();
        Ok(())
    }

    /// 写数据 (page program, 最大 256 bytes per op)
    pub fn write(&mut self, addr: u32, data: &[u8]) -> Result<(), ()> {
        let mut offset = 0usize;
        while offset < data.len() {
            let chunk = &data[offset..core::cmp::min(offset + 256, data.len())];
            let page_offset = (addr as usize + offset) % 256;

            if page_offset + chunk.len() > 256 {
                // 跨页对齐, 分两次写
                let first_len = 256 - page_offset;
                self.write_page(addr + offset as u32, &chunk[..first_len])?;
                offset += first_len;
            } else {
                self.write_page(addr + offset as u32, chunk)?;
                offset += chunk.len();
            }
        }
        Ok(())
    }

    fn write_page(&mut self, addr: u32, data: &[u8]) -> Result<(), ()> {
        self.write_enable()?;
        let mut tx = [0u8; 4];
        tx[0] = cmd::PAGE_PROGRAM;
        tx[1] = (addr >> 16) as u8;
        tx[2] = (addr >> 8) as u8;
        tx[3] = addr as u8;

        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 4])?;
        self.spi.spi_transfer(data, &mut [0; data.len()])?;
        self.spi.cs_high();
        self.wait_busy();
        Ok(())
    }

    /// 扇区擦除 (4KB)
    pub fn sector_erase(&mut self, addr: u32) -> Result<(), ()> {
        self.write_enable()?;
        let mut tx = [0u8; 4];
        tx[0] = cmd::SECTOR_ERASE;
        tx[1] = (addr >> 16) as u8;
        tx[2] = (addr >> 8) as u8;
        tx[3] = addr as u8;
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 4])?;
        self.spi.cs_high();
        self.wait_busy();
        Ok(())
    }

    /// 进入深度功耗模式
    pub fn power_down(&mut self) -> Result<(), ()> {
        let tx = [cmd::POWER_DOWN];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 1])?;
        self.spi.cs_high();
        Ok(())
    }

    /// 唤醒
    pub fn wake_up(&mut self) -> Result<(), ()> {
        let tx = [cmd::RELEASE_PD];
        self.spi.cs_low();
        self.spi.spi_transfer(&tx, &mut [0; 1])?;
        self.spi.cs_high();
        self.spi.delay_ms(3); // tRES1 = 3us, 用 ms 更安全
        Ok(())
    }

    /// 擦除 + 写入 (用于参数保存等, 自动处理扇区擦除)
    pub fn erase_write(&mut self, addr: u32, data: &[u8]) -> Result<(), ()> {
        let sector_start = addr & !0x0FFF; // 4KB 对齐
        self.sector_erase(sector_start)?;
        self.write(addr, data)
    }
}
