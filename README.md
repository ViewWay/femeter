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

## 项目架构

```
┌──────────────────────────────────────────────────────────────┐
│                    FeMeter System Architecture                │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   ATT7022E  │  │   RN8302B   │  │  RN8615V2   │          │
│  │  计量芯片    │  │  计量芯片    │  │  计量芯片    │          │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
│         └────────┬───────┘────────────────┘                  │
│                  ▼                                            │
│  ┌──────────────────────────────────────┐                    │
│  │         HAL (trait MeteringChip)      │                    │
│  ├──────────────────────────────────────┤                    │
│  │  metering │ event │ storage │ display │                    │
│  │  ota      │ tamper│ power_mgr│ rtc   │                    │
│  └──────────────────────────────────────┘                    │
│                  ▼                                            │
│  ┌──────────────────────────────────────┐                    │
│  │         DLMS/COSEM 协议栈 (12 crates) │                    │
│  │  HDLC → A-XDR → APDU → COSEM → App   │                    │
│  │  Security: AES-128-GCM, ECDSA, SM4    │                    │
│  └──────────────────────────────────────┘                    │
│                  ▼                                            │
│  ┌──────────────────────────────────────┐                    │
│  │           通信通道                     │                    │
│  │  RS485 │ 红外 │ LoRaWAN │ Cat.1/NB   │                    │
│  └──────────────────────────────────────┘                    │
│                                                              │
│  ┌──────────────────────────────────────┐                    │
│  │      femeter-core (Host 可测逻辑)      │                    │
│  │  power_quality │ load_forecast        │                    │
│  │  tamper_detection │ event_detect      │                    │
│  └──────────────────────────────────────┘                    │
│                                                              │
│  ┌──────────────────────────────────────┐                    │
│  │      virtual-meter (跨平台模拟器)      │                    │
│  │  TCP Server │ DLMS HDLC │ HTML Report │                    │
│  └──────────────────────────────────────┘                    │
└──────────────────────────────────────────────────────────────┘
```

## 项目结构

```
femeter/
├── firmware/                    # 主固件 (thumbv6m-none-eabi)
│   ├── src/
│   │   ├── main.rs              # 入口 + 15 个 FreeRTOS 任务
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
├── crates/
│   ├── dlms-core/               # 核心数据类型与错误定义
│   ├── dlms-axdr/               # A-XDR 编解码
│   ├── dlms-asn1/               # ASN.1 BER 编解码
│   ├── dlms-hdlc/               # HDLC 链路层
│   ├── dlms-obis/               # OBIS 码定义
│   ├── dlms-apdu/               # 应用层 APDU
│   ├── dlms-security/           # 安全机制 (AES/GCM/ECDSA/SHA256/SM4)
│   ├── dlms-cosem/              # COSEM 接口类 (105个)
│   ├── dlms-meter-app/          # 电表应用层
│   ├── dlms-rtos/               # RTOS 抽象层
│   ├── dlms-hal/                # 硬件抽象层
│   ├── dlms-host/               # 上位机工具 (std)
│   ├── femeter-core/            # 核心业务逻辑 (host 可测)
│   │   ├── event_detect.rs      # 事件检测 (过压/欠压/断相/过流)
│   │   ├── power_quality.rs     # 电能质量 (THD/暂降暂升/闪变)
│   │   ├── load_forecast.rs     # 负荷预测 (EWMA/线性回归)
│   │   ├── tamper_detection.rs  # 防窃电检测 (CT短路/PT断线)
│   │   └── ota.rs              # OTA 升级管理
│   └── virtual-meter/           # 跨平台虚拟电表模拟器
│       ├── src/
│       │   ├── meter.rs         # 虚拟电表核心
│       │   ├── dlms.rs          # DLMS 协议处理
│       │   ├── protocol.rs      # TCP 文本协议
│       │   ├── tcp_server.rs    # TCP 服务器
│       │   ├── html_report.rs   # HTML 报告生成
│       │   ├── demand.rs        # 需量计算
│       │   ├── statistics.rs    # 统计记录
│       │   └── tariff.rs        # 费率管理
│       └── bin/
│           ├── test_server.rs   # 测试服务器 (TCP + DLMS)
│           └── virtual_meter.rs # 交互式终端
├── tests/                       # 端到端集成测试
│   ├── test_e2e.py              # Python E2E 测试
│   └── FULL_REPORT.md           # 测试报告
├── docs/                        # 文档
│   ├── task_optimization.md     # FreeRTOS 任务优化
│   ├── rtos-comparison.md       # RTOS 选型对比
│   ├── schematic-netlist.md     # 原理图网表
│   ├── system-diagram.md        # 系统框图
│   ├── BOM.md                   # 物料清单
│   └── communication-plan.md    # 通信方案
└── .github/workflows/ci.yml     # CI/CD
```

## 模块清单

| 模块 | 说明 | 测试 |
|------|------|------|
| `dlms-core` | 核心数据类型与错误定义 | 86 |
| `dlms-axdr` | A-XDR 编解码 | 34 |
| `dlms-asn1` | ASN.1 BER 编解码 | 54 |
| `dlms-hdlc` | HDLC 链路层 | 8 |
| `dlms-obis` | OBIS 码定义 | 198 |
| `dlms-apdu` | 应用层 APDU | 41 |
| `dlms-security` | 安全机制 (AES/GCM/ECDSA/SHA256/SM4) | 25 |
| `dlms-cosem` | COSEM 接口类 (105个) | 129 |
| `dlms-meter-app` | 电表应用层 | 17 |
| `dlms-rtos` | RTOS 抽象层 | 8 |
| `dlms-hal` | 硬件抽象层 | 110 |
| `dlms-host` | 上位机工具 | 121 |
| `femeter-core` | 核心业务逻辑 | 50 |
| `virtual-meter` | 虚拟电表 | 27 |
| **总计** | | **908 Rust + 12 E2E Python** |

## 通信通道

| CH | 外设 | 用途 | 协议 |
|----|------|------|------|
| CH0 | UART0 + RSM485MT5V | RS-485 | HDLC/DLMS, 9600~115200 |
| CH1 | UART1 | 红外 | IEC 62056-21, 300~9600 |
| CH2 | UART2 + ASR6601 | LoRaWAN | AT指令, 38400 |
| CH3 | UART3 | 调试 | defmt/日志 |

## Flash 布局 (双 Bank OTA)

```
0x0000_0000 ┌──────────────────┐
            │  Interrupt Vector │
0x0000_0200 ├──────────────────┤
            │  App (Bank A/B)   │  2 × 180KB
            │  OTA 可切换        │
0x0005_C000 ├──────────────────┤
            │  Storage (EEPROM) │  60KB (参数/事件/负荷曲线)
0x0006_B000 ├──────────────────┤
            │  FreeRTOS + App   │  ~170KB
0x0009_0000 ├──────────────────┤
            │  Reserved         │
0x0009_F000 ├──────────────────┤
            │  Bootloader       │  4KB
0x000A_0000 └──────────────────┘
```

## 构建与测试

### 前置要求

- Rust nightly (推荐 1.96+)
- ARM 工具链: `rustup target add thumbv6m-none-eabi`
- FreeRTOS: 子模块已包含
- Python 3.8+ (用于 E2E 测试)

### Host 构建 (协议栈 + 虚拟电表)

```bash
# 格式化 + Lint + 测试
cargo fmt
cargo clippy --workspace --exclude femeter-firmware -- -D warnings
cargo test --workspace --exclude femeter-firmware

# 启动虚拟电表服务器
cargo run --bin test_server -p virtual-meter

# 运行 E2E 测试 (另一终端)
python3 tests/test_e2e.py

# 交互式虚拟电表
cargo run --bin virtual-meter -p virtual-meter
```

### 固件交叉编译

```bash
cd firmware
cargo build --release --target thumbv6m-none-eabi --bin femeter --features freertos
arm-none-eabi-size target/thumbv6m-none-eabi/release/femeter
```

## FreeRTOS 任务分配

| 任务 | 优先级 | 栈 | 说明 |
|------|--------|-----|------|
| metering | 最高(5) | 512W | 计量采集 + 事件检测 |
| event_detect | 最高(5) | 256W | 事件分类 + DLMS 事件队列 |
| display | 中(3) | 512W | LCD 段码刷新 |
| storage | 中低(2) | 512W | Flash 读写 (参数/事件/曲线) |
| rs485 | 高(4) | 512W | DLMS/HDLC 通信 |
| infrared | 高(4) | 256W | 红外抄表 |
| tamper | 低(1) | 256W | 防窃电检测 |
| ota | 低(1) | 256W | OTA 升级 |
| watchdog | 中低(2) | 64W | 看门狗喂狗 |
| rtc_sync | 中低(2) | 128W | RTC 校时 |
| power_mgr | 低(1) | 128W | 低功耗管理 |
| temperature | 低(1) | 64W | 温度补偿 |
| lorawan | 低(1) | 256W | LoRaWAN 上报 |
| cellular | 低(1) | 256W | 蜂窝通信 (feature gate) |

## 开发进度

- ✅ **Phase A**: 驱动层 — 计量芯片 SPI 驱动 (ATT7022E/RN8302B/RN8615V2)
- ✅ **Phase B**: 数据采集 — 计量/事件/存储/显示
- ✅ **Phase C**: DLMS 协议栈 — HDLC/A-XDR/APDU/COSEM (105个接口类)
- ✅ **Phase D**: 通信 — RS485/红外/LoRaWAN/蜂窝
- ✅ **Phase E**: 生产校表 — 校准系数/误差补偿/误差曲线
- ✅ **Phase F**: 系统集成 — 虚拟电表/E2E测试/CI/文档

## License

MIT
