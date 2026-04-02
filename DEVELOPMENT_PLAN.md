# FeMeter 三相智能电表 — 开发计划

> 生成时间: 2026-04-03 | 项目路径: firmware/ + crates/

---

## 一、项目现状分析

### 1.1 规模

| 指标 | 数值 |
|------|------|
| Workspace crates | 15 (14 + firmware) |
| Rust 源文件 | ~259 |
| 代码行数 | ~90,559 |
| 单元测试 | **717 (全部通过)** |
| 固件模块 | 28 个 .rs 文件 |
| 目标 MCU | FM33A068EV (Cortex-M0+ @ 64MHz) |

### 1.2 测试分布

| Crate | 测试数 | 状态 |
|-------|--------|------|
| dlms-apdu | 77 | ✅ |
| dlms-meter-app | 191 | ✅ |
| dlms-security | 119 | ✅ |
| dlms-hdlc | 42 | ✅ |
| dlms-rtos | 29 | ✅ |
| dlms-cosem | 25 | ✅ |
| dlms-axdr | 33 | ✅ |
| dlms-obis | 6 | ✅ |
| dlms-core | 8 | ✅ |
| dlms-hal | 20 | ✅ |
| virtual-meter | 27 | ✅ |
| dlms-host | 30 | ✅ |
| dlms-asn1 | 8 | ✅ |
| femeter-core | 0 | ⚠️ 无测试 |
| firmware | N/A (no_std, ARM only) | — |

---

## 二、各模块完成度评估

### 2.1 协议栈 crates (dlms-*) — ⭐⭐⭐⭐⭐ 完成度: 85%

- **dlms-axdr**: ASN.1 编解码完整，有 datetime/compact 扩展
- **dlms-asn1**: AARQ/AARE/RlRq/RlRe 完整
- **dlms-hdlc**: 帧编解码、CRC、LLC、连接管理完整
- **dlms-obis**: OBIS 码解析与查找完整
- **dlms-apdu**: Get/Set/Action/Initiate/Event/BlockTransfer 完整
- **dlms-security**: AES-GCM、SM4-GMAC、HLS、LLS 完整
- **dlms-cosem**: COSEM IC 对象模型，100+ IC 定义，PLC/无线/MBus/LPwan 全覆盖
- **dlms-meter-app**: 电表应用层（量测、时钟、费率、负荷曲线、告警）
- **dlms-rtos**: FreeRTOS 安全封装（队列、信号量、事件组、定时器）
- **dlms-hal**: 硬件抽象层 trait 定义（UART/SPI/I2C/GPIO/ADC/Flash/RTC）
- **dlms-host**: 宿主工具（CLI、模拟器、抓包器、测试运行器）

**差距**: COSEM IC 对象多为数据结构定义，缺少与实际计量数据的绑定逻辑。

### 2.2 固件 firmware/ — ⭐⭐⭐ 完成度: 60%

| 模块 | 状态 | 说明 |
|------|------|------|
| main.rs | 🟡 框架完整 | FreeRTOS 多任务架构完整，但多处 TODO |
| board.rs | 🟡 大部分完成 | LCD/GPIO/UART 初始化，缺 NVIC 和 ADC |
| metering.rs | 🟡 框架完整 | 计量数据采集循环，依赖计量芯片驱动 |
| storage.rs | 🟡 框架完整 | Flash 分区管理，缺时间戳寻址写入 |
| display.rs | 🟡 大部分完成 | LCD 段码映射基本完成，缺符号类型写入 |
| comm.rs | 🟡 框架完整 | RS485/红外/LoRaWAN 通信框架 |
| event_detect.rs | 🟢 完成 | 过压/欠压/断相/电流不平衡/逆功率检测 |
| power_manager.rs | 🟡 框架完整 | 低功耗管理，多处硬件相关 TODO |
| ota.rs | 🟡 框架完整 | OTA 升级流程，缺实际通信实现 |
| rtc.rs | 🟢 完成 | RTC 读写、时间戳转换 |
| key_scan.rs | 🟡 大部分完成 | 按键扫描，密码校验为 placeholder |
| watchdog.rs | 🟢 完成 | 独立看门狗配置 |
| freertos.rs | 🟢 **已修复** | FreeRTOS 11.x FFI 绑定（见 Phase A） |
| hal.rs | 🟢 完成 | HAL trait 抽象 |
| fm33lg0.rs | 🟡 大部分完成 | FM33LG0 寄存器定义 |
| att7022e.rs | 🟡 基础完成 | SPI 读写、寄存器读取，缺校准参数写入 |
| rn8302b.rs | 🟡 基础完成 | 类似 att7022e |
| rn8615v2.rs | 🟡 基础完成 | 类似 att7022e |
| asr6601.rs | 🟡 框架 | LoRaWAN AT 指令框架 |
| quectel.rs | 🟡 框架 | 蜂窝模组 AT 指令框架 |
| at_parser.rs | 🟡 框架 | AT 响应解析，多处 TODO |

### 2.3 虚拟电表 virtual-meter/ — ⭐⭐⭐⭐ 完成度: 75%

- 完整的电表模拟（电压/电流/功率/电能）
- DLMS SN 协议处理
- IEC 62056-21 协议
- TCP 服务器 + Shell 控制台
- 负荷曲线、需量、校准、显示
- 自动化测试套件

---

## 三、已修复问题 (Phase A)

### 3.1 ✅ FreeRTOS 11.x FFI 符号不匹配

**问题**: FreeRTOS 11.x 中以下 API 是 C 宏而非导出函数，Rust FFI 链接时找不到符号：
- `xQueueCreate` → `xQueueGenericCreate`
- `xQueueSend` → `xQueueGenericSend`
- `xQueueSendFromISR` → `xQueueGenericSendFromISR`
- `xQueueReset` → `xQueueGenericReset`

**修复**: 将 `freertos.rs` 中的 FFI 声明改为映射到实际函数，并更新安全封装层调用。

### 3.2 TODO 注释清单 (共 42 处)

| 分类 | 数量 | 优先级 |
|------|------|--------|
| main.rs 任务集成 | 12 | P1 |
| power_manager.rs 硬件相关 | 5 | P2 |
| board.rs 硬件相关 | 3 | P2 |
| comm.rs / asr6601.rs / quectel.rs | 3 | P1 |
| at_parser.rs 响应解析 | 3 | P1 |
| storage.rs 时间戳寻址 | 1 | P1 |
| key_scan.rs 密码校验 | 2 | P3 |
| 其他 (boot/display/att7022e/watchdog) | 6 | P3 |

> 注意: 无 `todo!()` 或 `unimplemented!()` 宏调用（不会 panic），全部是 TODO 注释，代码可以编译运行。

---

## 四、分阶段开发计划

### Phase A: 代码质量与稳定性 ✅ (部分完成)

- [x] 扫描所有 TODO/FIXME/unimplemented!/todo!()
- [x] 修复 FreeRTOS 11.x FFI 链接问题
- [x] 确认所有 717 测试通过
- [ ] 为 femeter-core 补充单元测试
- [ ] 为 firmware 纯逻辑模块编写 host 测试（cfg(test) + conditional compilation）
- [ ] 修复 CI `cargo clippy --workspace` (firmware 在 host 环境可能 clippy 失败)

### Phase B: 核心功能完善

| 任务 | 工作量 | 说明 |
|------|--------|------|
| DLMS 端到端测试 | 3d | SN 连接 → GetRequest(1.0.0.0.0.0.255) → 响应验证 |
| 计量数据完整流 | 5d | 驱动采样 → metering.rs 处理 → storage 写入 → display 显示 |
| TOU 分时费率 | 3d | 多时段/多费率/节假日/特殊日 |
| 负荷曲线 | 2d | 周期记录 + 时间戳寻址 + 读取 |
| 事件上报 | 2d | event_detect → event_log → DLMS 事件通知 |

### Phase C: 通信完善

| 任务 | 工作量 | 说明 |
|------|--------|------|
| RS485 HDLC 完整 | 5d | 帧收发、CRC-16、超时重传、多从站 |
| 红外 IEC 62056-21 | 3d | 38kHz 调制、波特率协商、数据读取 |
| LoRaWAN AT 指令 | 3d | 入网、发送、接收、下行处理 |
| 蜂窝模组通信 | 5d | MQTT/CoAP、NTP 同步、远程固件升级 |
| 多通道调度 | 3d | RS485 + 红外 + 无线 优先级调度 |

### Phase D: 生产就绪

| 任务 | 工作量 | 说明 |
|------|--------|------|
| 虚拟电表自动化测试 | 5d | 全协议覆盖、异常场景、压力测试 |
| OTA 端到端验证 | 3d | 固件签名、升级流程、回滚机制 |
| 掉电保护 | 3d | 关键数据即时保存、恢复验证 |
| 看门狗复位恢复 | 1d | 复位原因检测、状态恢复 |
| 生产校表流程 | 5d | 精度校准、参数写入、出厂测试 |

---

## 五、已知问题和风险

### 5.1 高优先级

1. **CI firmware-build 可能仍失败**: build.rs 依赖 `arm-none-eabi-gcc`，CI 已安装但需验证 `cc` crate 能否正确找到
2. **CI host-build-test 包含 firmware**: `cargo clippy --workspace` 会尝试编译 firmware (no_std)，需 `--exclude femeter-firmware`
3. **femeter-core 无测试**: 事件检测和 OTA 模块缺少 host 测试

### 5.2 中优先级

4. **计量芯片驱动缺校准**: att7022e/rn8302b/rn8615v2 的校准参数写入未实现
5. **storage 时间戳寻址**: 负荷曲线等需要基于时间的高效寻址
6. **main.rs 中 12 处 TODO**: 核心任务循环中多个关键步骤未实现

### 5.3 低优先级

7. **board.rs NVIC**: 中断配置需要 PAC crate 支持
8. **power_manager 低功耗**: tickless 模式、外设唤醒未完成

---

## 六、下一步行动

1. **立即**: 修复 CI — 在 host-build-test 中排除 firmware，在 firmware-build 中验证 FreeRTOS 链接
2. **本周**: 为 femeter-core 编写 host 测试
3. **下周**: 实现 Phase B 中的 DLMS 端到端测试和计量数据流
4. **持续**: 逐步推进 TODO 清单中的 P1 项目
