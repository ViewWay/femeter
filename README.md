# Femeter — 三相智能电表固件

> 基于 Rust + Cortex-M0+ 的三相智能电表固件，完整 DLMS/COSEM 协议栈，支持 RS485、红外、LoRaWAN 通信。

## 特性

- **DLMS/COSEM 完整协议栈** — AXDR、ASN.1、HDLC、OBIS、APDU、COSEM 接口类、安全层
- **RS485 HDLC 通信** — IEC 62056-21 兼容，支持光学口和 RS485 串口
- **LoRaWAN 远程通信** — 基于 RN8615V2 模块，Class A/C
- **三相电能计量** — 支持 ATT7022E / RN8302B 计量芯片
- **电能质量监测** — 谐波分析、电压/电流 THD、功率因数
- **防窃电检测** — 开盖检测、电压缺失、电流异常、磁干扰
- **OTA 双 Bank 升级** — 安全固件更新，掉电保护
- **费率/时段管理** — TOU（分时计费）、负荷曲线、需量统计
- **生产校表** — 脉冲校准、误差计算、出厂测试模式
- **FreeRTOS 集成** — 基于复旦微 FM33A068EV，FreeRTOS 11.x

## 硬件平台

| 组件 | 型号 | 说明 |
|------|------|------|
| MCU | FM33A068EV | 复旦微 Cortex-M0+，512KB Flash，256KB SRAM |
| 计量芯片 | ATT7022E / RN8302B | 三相多功能电能计量 |
| LoRa 模块 | RN8615V2 | LoRaWAN 1.0.3 Class A/C |
| 通信 | RS485 + 红光口 | HDLC/IEC 62056-21 |
| 显示 | LCD 段码屏 | 轮显/按键切换 |

## 架构

```
┌─────────────────────────────────────────────────────┐
│                  femeter-firmware                     │
│            (Cortex-M0+ FreeRTOS App)                  │
├────────┬──────────┬──────────┬───────────┬───────────┤
│ board  │  meter   │  comm    │ display   │  ota      │
│  HAL   │ 计量引擎 │ RS485/IR │ LCD 轮显   │ 双Bank   │
├────────┴──────────┴──────────┴───────────┴───────────┤
│                   femeter-core                        │
│    电能质量 │ 防窃电 │ 事件检测 │ 负荷预测 │ OTA      │
├─────────────────────────────────────────────────────┤
│              DLMS/COSEM 协议栈 (17 crates)            │
│  dlms-core → axdr → asn1 → hdlc → obis → apdu      │
│  → security → cosem → meter-app → hal → rtos → host │
├─────────────────────────────────────────────────────┤
│         virtual-meter (桌面模拟器/测试)                │
│    Python 测试框架 │ Fuzz 测试 │ 基准测试              │
└─────────────────────────────────────────────────────┘
```

## 模块清单

| Crate | 说明 |
|-------|------|
| `femeter-firmware` | 固件主程序，FreeRTOS 任务调度 |
| `femeter-core` | 电能质量、防窃电、OTA、事件检测、负荷预测 |
| `dlms-core` | 核心数据类型、错误定义、计量单位、日期时间 |
| `dlms-axdr` | DLMS 编码规则 (BER/AXDR) 编解码 |
| `dlms-asn1` | ASN.1 基础类型 (Integer/BitString/OID/OctetString) |
| `dlms-hdlc` | HDLC 帧协议，CRC-16，帧分割/组装 |
| `dlms-obis` | OBIS 代码定义与解析 (IEC 62056-61) |
| `dlms-apdu` | DLMS APDU 类型 (Get/Set/Action/Event) |
| `dlms-security` | 安全层 (AES-GCM/LLS/HLS/SecurityContext) |
| `dlms-cosem` | COSEM 接口类 (Clock/Register/Tariff/Profile 等 43 类) |
| `dlms-meter-app` | 完整电表应用层 (抄表/费率/负荷/需量) |
| `dlms-hal` | 硬件抽象层 trait 定义 (UART/SPI/Flash/RTC) |
| `dlms-rtos` | RTOS 适配层 (FreeRTOS 信号量/队列/任务) |
| `dlms-host` | 主站工具 CLI (模拟器/嗅探器/测试运行器) |
| `virtual-meter` | 跨平台虚拟电表模拟器 |
| `bench-tests` | 性能基准测试 |
| `fuzz-tests` | 模糊测试与内存安全断言 |

## 构建

### 前置条件

```bash
rustup target add thumbv6m-none-eabi
cargo install probe-rs
# ARM 工具链 (可选, 用于 openocd)
brew install arm-none-eabi-gcc
```

### 编译固件

```bash
cd firmware
cargo build --release --target thumbv6m-none-eabi
```

### 烧录

```bash
# probe-rs (推荐)
probe-rs run --chip FM33A068EV target/thumbv6m-none-eabi/release/femeter-firmware

# OpenOCD + J-Link
openocd -f interface/jlink.cfg -f target/fm33a0xx.cfg \
  -c "program target/thumbv6m-none-eabi/release/femeter-firmware verify reset exit"
```

### 构建 Host 工具

```bash
cargo build -p dlms-host
```

## 测试

### Rust 单元测试

```bash
# 全部 workspace (排除固件，需交叉编译工具链)
cargo test --workspace --exclude femeter-firmware

# 单个 crate
cargo test -p dlms-cosem
```

### Python 集成测试

```bash
cd crates/virtual-meter
pip install -r requirements.txt
python -m pytest tests/ -v
```

### 代码质量

```bash
cargo fmt --check
cargo clippy --workspace --exclude femeter-firmware -- -D warnings
cargo doc --workspace --exclude femeter-firmware --no-deps
```

## 通信协议

### RS485 / HDLC (IEC 62056-46)

- 波特率：2400 / 9600 / 19200 bps
- HDLC 帧格式：`7E ... FCS 7E`
- 支持 SN/ LN 寻址模式
- 最大帧长度 1024 字节

### 红外光学口 (IEC 62056-21)

- 波特率：300 ~ 115200 bps (自适应)
- 支持握手协商模式

### LoRaWAN

- Class A/C 双模
- ABP/OTAA 入网
- 上行：定时抄表 + 事件告警
- 下行：参数配置 + OTA 触发

## 贡献

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feat/xxx`)
3. 确保所有测试通过 (`cargo test --workspace --exclude femeter-firmware`)
4. 确保代码质量 (`cargo fmt --check && cargo clippy --workspace --exclude femeter-firmware -- -D warnings`)
5. 提交 PR

## 许可证

MIT OR Apache-2.0

## 致谢

- 复旦微电子 — FM33A068EV 开发板及技术支持
- 锐能微 — RN8302B / RN8615V2 模块文档
- 珠海鼎信 — ATT7022E 计量芯片参考设计
