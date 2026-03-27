# FeMeter 🦀⚡

DLMS/COSEM 智能电表系统 — Rust + Embedded RTOS

## 项目结构

```
femeter/
├── crates/
│   ├── dlms-core/       核心数据类型与错误定义
│   ├── dlms-axdr/       A-XDR 编解码
│   ├── dlms-asn1/       ASN.1 BER 编解码
│   ├── dlms-hdlc/       HDLC 链路层
│   ├── dlms-obis/       OBIS 码定义
│   ├── dlms-apdu/       应用层 APDU
│   ├── dlms-security/   安全机制
│   ├── dlms-cosem/      COSEM 接口类 (105个)
│   ├── dlms-meter-app/  电表应用层
│   ├── dlms-rtos/       RTOS 抽象层
│   ├── dlms-hal/        硬件抽象层
│   └── dlms-host/       上位机工具 (std)
└── 需求/                标准文档与分析
```

## 目标平台

- STM32F4 (Cortex-M4F), RAM ≤ 256KB, Flash ≤ 1MB
- STM32F1 (Cortex-M3) 兼容
- QEMU 模拟调试
- Host (std) 上位机工具

## 构建

```bash
# Host 开发测试
cargo build --workspace

# STM32F4 交叉编译
cargo build --target thumbv7em-none-eabihf -p dlms-core
```
