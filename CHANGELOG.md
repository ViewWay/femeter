# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-03

### Added

#### 项目初始化
- FeMeter 项目骨架与 workspace 结构
- CI/CD GitHub Actions 工作流
- FreeRTOS 11.x 集成与 FFI 绑定
- 构建脚本、linker 脚本、OTA 打包工具

#### DLMS/COSEM 协议栈 (12 crates)
- `dlms-core`: 核心数据类型、错误定义、计量单位、日期时间
- `dlms-axdr`: DLMS 编码规则 (BER/AXDR) 完整编解码
- `dlms-asn1`: ASN.1 基础类型 (Integer/BitString/OID/OctetString)
- `dlms-hdlc`: HDLC 帧协议，CRC-16 校验，帧分割/组装
- `dlms-obis`: OBIS 代码完整定义与解析 (IEC 62056-61)
- `dlms-apdu`: DLMS APDU 全类型 (Get/Set/Action/Event/Initiate)
- `dlms-security`: AES-GCM 加密、LS/HS/HLS 认证、SecurityContext 管理
- `dlms-cosem`: 43 个 COSEM 接口类实现
- `dlms-meter-app`: 完整电表应用 (抄表/费率/负荷曲线/需量)
- `dlms-hal`: 硬件抽象层 trait (UART/SPI/Flash/RTC/GPIO)
- `dlms-rtos`: FreeRTOS 适配层 (信号量/队列/任务/定时器)
- `dlms-host`: 主站 CLI 工具 (模拟器/嗅探器/测试运行器)

#### 固件
- FM33A068EV 寄存器定义 (CMSIS)
- ATT7022E 三相计量芯片驱动
- RN8302B / RN8615V2 驱动
- Bootloader + OTA 双 Bank 升级
- FreeRTOS 11.x 任务调度器迁移
- LCD 段码显示 + 轮显 + 按键切换
- RS485 / 红光口通信驱动
- LoRaWAN Class A/C 通信
- J-Link 调试支持

#### 核心功能
- `femeter-core`: 电能质量监测 (THD/谐波/功率因数)
- `femeter-core`: 防窃电检测 (开盖/电压缺失/电流异常/磁干扰)
- `femeter-core`: 事件检测与记录
- `femeter-core`: OTA 升级管理
- `femeter-core`: 负荷预测

#### 虚拟电表
- 跨平台桌面模拟器 (virtual-meter)
- TCP 服务器 + IEC 62056 通信
- 分时计费 (TOU) / 负荷曲线 / 需量统计
- 校准接口 / 显示模拟 / 统计持久化
- 场景引擎与自动测试

#### 测试与质量
- 950+ Rust 单元测试
- 58 Python 集成测试
- Fuzz 测试 + 内存安全断言
- 性能基准测试
- Clippy 0 warnings (workspace)
- 统一错误处理 + 安全审计
