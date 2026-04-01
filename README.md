# FeMeter 🦀⚡

三相智能电表固件 — Rust + FreeRTOS 11.x (bare-metal, `#[no_std]`)

## 项目概述

基于 FM33A068EV (Cortex-M0+) 的三相智能电表固件，支持 DLMS/COSEM 协议，运行 FreeRTOS 实时操作系统。

## 目标硬件

| 组件 | 型号 | 说明 |
|------|------|------|
| MCU | FM33A068EV | Cortex-M0+ @ 64MHz, 512KB Flash, 80KB SRAM, LQFP80 |
| 计量芯片 | ATT7022E / RN8302B / RN8615V2 | trait 抽象，运行时切换 |
| RS485 | RSM485MT5V | 隔离收发，HDLC/DLMS |
| LoRaWAN | ASR6601 (E78-470LN22S) | CN470~510MHz, AT 指令 |
| LCD | 内置段码控制器 | 4COM×44SEG |
| 蜂窝 | EC800N (Cat.1) / BC260Y (NB-IoT) | 双模预留 |

## 项目结构

```
femeter/
├── firmware/                    # 主固件 (thumbv6m-none-eabi)
│   ├── src/
│   │   ├── main.rs              # 入口 + 12 个 FreeRTOS 任务
│   │   ├── fm33lg0.rs           # MCU 寄存器定义 (CMSIS SVD)
│   │   ├── board.rs             # 硬件初始化 (GPIO/SPI/UART/LCD)
│   │   ├── hal.rs               # 硬件抽象层 (trait MeteringChip)
│   │   ├── att7022e.rs          # ATT7022E SPI 驱动
│   │   ├── rn8302b.rs           # RN8302B SPI 驱动
│   │   ├── rn8615v2.rs          # RN8615V2 SPI 驱动
│   │   ├── comm.rs              # DLMS HDLC 帧处理 (945行)
│   │   ├── metering.rs          # 计量数据管理 (翻转/费率累计)
│   │   ├── display.rs           # LCD 段码显示
│   │   ├── asr6601.rs           # LoRaWAN AT 指令驱动 (444行)
│   │   ├── quectel.rs           # 蜂窝模组驱动 (Cat.1/NB-IoT)
│   │   ├── power_manager.rs     # 低功耗管理
│   │   ├── freertos.rs          # FreeRTOS FFI 绑定
│   │   ├── freertos_hooks.rs    # FreeRTOS Rust hooks
│   │   └── boot.rs              # Bootloader
│   ├── FreeRTOSConfig.h         # FreeRTOS 配置
│   ├── freertos_hooks.c         # FreeRTOS C hooks
│   ├── memory.x                 # 链接器内存布局
│   ├── build.rs                 # 构建脚本 (cc + FreeRTOS)
│   └── Cargo.toml
├── crates/                      # DLMS/COSEM 协议栈 (12 crate)
│   ├── dlms-core/               # 核心数据类型与错误定义
│   ├── dlms-axdr/               # A-XDR 编解码
│   ├── dlms-asn1/               # ASN.1 BER 编解码
│   ├── dlms-hdlc/               # HDLC 链路层
│   ├── dlms-obis/               # OBIS 码定义
│   ├── dlms-apdu/               # 应用层 APDU
│   ├── dlms-security/           # 安全机制 (AES/GCM/ECDSA/SHA256)
│   ├── dlms-cosem/              # COSEM 接口类 (105个)
│   ├── dlms-meter-app/          # 电表应用层
│   ├── dlms-rtos/               # RTOS 抽象层
│   ├── dlms-hal/                # 硬件抽象层
│   └── dlms-host/               # 上位机工具 (std)
├── docs/                        # 文档
│   ├── rtos-comparison.md       # RTOS 选型对比
│   ├── schematic-netlist.md     # 原理图网表
│   ├── system-diagram.md        # 系统框图
│   ├── BOM.md                   # 物料清单
│   └── communication-plan.md    # 通信方案
└── virtual-meter/               # 虚拟电表模拟器
```

## 通信通道

| CH | 外设 | 用途 | 协议 |
|----|------|------|------|
| CH0 | UART0 + RSM485MT5V | RS-485 | HDLC/DLMS, 9600~115200 |
| CH1 | UART1 | 红外 | IEC 62056-21, 300~9600 |
| CH2 | UART2 + ASR6601 | LoRaWAN | AT指令, 38400 |
| CH3 | UART3 | 调试 | defmt/日志 |

## Flash 布局 (双 Bank OTA)

```
0x000000 ┌──────────────┐
         │  Bootloader  │ 4KB
0x001000 ├──────────────┤
         │   App Bank 1 │ 252KB (当前运行)
0x040000 ├──────────────┤
         │   App Bank 2 │ 256KB (升级暂存)
0x080000 ├──────────────┤
         │  OTA Data    │ 512KB (接收区)
0x100000 ├──────────────┤
         │   Reserved   │ 512KB
0x200000 └──────────────┘
```

### OTA 升级流程
1. 通过 RS485/LoRaWAN/蜂窝接收新固件 → OTA Data 区
2. CRC32 校验 + 版本号验证
3. 擦除目标 Bank (非活动 Bank)
4. 拷贝固件到目标 Bank
5. 设置 Bootloader 启动标志
6. 系统复位，Bootloader 从新 Bank 启动
7. 记录升级历史（最近 8 次）

## 烧录

```bash
# 首次烧录 (Bootloader + 固件)
./scripts/flash.sh boot    # 烧录 Bootloader 到 0x0
./scripts/flash.sh            # 烧录主固件到 0x1000

# OTA 远程升级 (通过 DLMS/COSEM)
# 或手动：将 OTA bin 写入 OTA 区后重启

# 调试
./scripts/debug.sh
```

```bash
# 主固件 (FreeRTOS)
cargo build --release --target thumbv6m-none-eabi --bin femeter

# 纯裸机 (无 RTOS)
cargo build --release --target thumbv6m-none-eabi --bin femeter --no-default-features --features bare

# Bootloader
cargo build --release --target thumbv6m-none-eabi --bin femeter-boot

# DLMS 协议栈 (host)
cargo build --workspace
```

## 构建产物

| 二进制 | Flash | RAM | 说明 |
|--------|-------|-----|------|
| femeter | 19.0 KB | 50.6 KB | 主固件 (FreeRTOS) |
| femeter-boot | 1.7 KB | — | Bootloader |

## FreeRTOS 任务架构

```
Priority 5: task_comm_rs485    (RS485 通信)
Priority 4: task_comm_ir       (红外通信)
Priority 4: task_lora          (LoRaWAN)
Priority 3: task_metering      (计量采集)
Priority 3: task_energy        (电能累计)
Priority 2: task_display       (LCD 显示)
Priority 2: task_pulse         (脉冲输出)
Priority 2: task_key           (按键处理)
Priority 1: task_event         (事件检测)
Priority 1: task_power         (功耗管理)
Priority 1: task_diagnostics  (诊断)
Priority 0: task_watchdog      (喂狗)
```

## 技术栈

- **语言**: Rust (stable, `#[no_std]`)
- **RTOS**: FreeRTOS 11.x (MIT license)
- **工具链**: `thumbv6m-none-eabi` (ARM GCC 13.3.1)
- **链接器**: rust-lld + flip-link
- **日志**: defmt + RTT
- **协议**: DLMS/COSEM (IEC 62056)

## 开发阶段

- [x] **Phase 0** — 项目搭建 + DLMS 协议栈 (12 crate, 693 tests, 35K 行)
- [x] **Phase 1** — 硬件驱动层 (MCU/计量/通信/LCD/LoRaWAN)
- [x] **Phase 1.5** — FreeRTOS 迁移 (12 任务 + mutex)
- [ ] **Phase 2** — 数据采集 (定时读计量、校表、电能累计、事件检测)
- [ ] **Phase 3** — DLMS/COSEM 协议栈集成
- [ ] **Phase 4** — 应用层 (费率/存储/显示调度/低功耗/告警)

## License

MIT
