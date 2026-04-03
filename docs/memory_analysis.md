# 固件内存优化分析

> FM33A068EV: 512KB Flash, 80KB SRAM, Cortex-M0+ @ 64MHz

## 1. 各模块 RAM/Flash 占用估算

| 模块 | 源码行数 | Flash (KB) | RAM (B) | 说明 |
|------|---------|-----------|---------|------|
| `main.rs` | 1194 | 2.0 | 128 | 任务创建/调度/共享状态 |
| `comm.rs` | 1468 | 3.5 | 512 | 通信管理器 (收发缓冲区) |
| `board.rs` | 1398 | 2.5 | 96 | 板级驱动 (GPIO/ADC) |
| `at_parser.rs` | 1233 | 2.5 | 256 | AT 指令解析缓冲区 |
| `quectel.rs` | 1229 | 2.5 | 256 | 蜂窝模组驱动 |
| `calibration.rs` | 1031 | 2.0 | 64 | 校准参数 (flash 存储) |
| `event_detect.rs` | 985 | 2.0 | 384 | 事件检测 + 日志缓冲 |
| `hal.rs` | 927 | 1.5 | 0 | 纯 trait/类型定义 |
| `storage.rs` | 884 | 2.0 | 512 | 闪存分区管理 + 缓冲 |
| `asr6601.rs` | 839 | 2.0 | 256 | LoRaWAN AT 驱动 |
| `att7022e.rs` | 797 | 1.5 | 128 | 计量芯片驱动 (SPI) |
| `metering.rs` | 712 | 1.5 | 256 | 计量管理器 |
| `power_manager.rs` | 685 | 1.5 | 32 | 低功耗管理 |
| `rn8302b.rs` | 671 | 1.5 | 128 | 计量芯片驱动 (备选) |
| `rtc.rs` | 669 | 1.0 | 32 | RTC 时间管理 |
| `fm33lg0.rs` | 665 | 1.0 | 0 | MCU HAL 底层 |
| `rn8615v2.rs` | 662 | 1.5 | 128 | 计量芯片驱动 (备选) |
| `ota.rs` | 644 | 1.5 | 512 | OTA 双 Bank 管理 |
| `display.rs` | 569 | 1.0 | 64 | LCD 段码驱动 |
| `uart_driver.rs` | ~400 | 1.0 | 256 | UART 收发缓冲区 |
| `key_scan.rs` | ~300 | 0.5 | 16 | 按键去抖 |
| `watchdog.rs` | ~200 | 0.5 | 8 | IWDT 管理 |
| `freertos.rs` | ~500 | 1.5 | 1024 | FreeRTOS 内核 (port) |
| `freertos_hooks.rs` | ~100 | 0.3 | 0 | 钩子函数 |
| `dlms_stack.rs` | ~800 | 3.0 | 1024 | DLMS/COSEM 协议栈 |
| **总计** | **~20500** | **~42** | **~5800** | |

### 注意
- Flash 估算基于 ARM Thumb-2 指令密度 (~4 bytes/行有效代码)
- RAM 不含 FreeRTOS 任务栈 (单独计算)
- DLMS 协议栈依赖外部 crate (dlms-cosem), 额外 Flash ~30KB

## 2. 可优化方向

### 2.1 静态分配 → 动态分配
- **comm.rs**: 收发缓冲区 256B 固定, 可根据波特率动态调整
- **storage.rs**: 写入缓冲区 512B, 可改为双缓冲 256B
- **ota.rs**: OTA 接收缓冲区 512B, 可按 chunk 动态使用

### 2.2 数据结构压缩
- **event_detect.rs**: 事件日志可用位域压缩事件类型+时间戳
- **PhaseData**: u16→u8 电压电流 (精度 0.04V/0.04A, 足够)
- **EnergyData**: u64→u32 (电量 <4M kWh, u32 足够, 节省 12B)

### 2.3 编译优化
- `-C opt-level=z` 或 `-C opt-level=s` (size 优化)
- `panic = "abort"` (省 ~2KB Flash)
- `lto = true` (Link-Time Optimization, 省 10~15% Flash)
- `codegen-units = 1` (配合 LTO)

### 2.4 功能裁剪
- 非必要模块 feature gate: `cellular`, `ext-flash`, `dlms`
- 日志级别运行时可调, Release 仅 error/warn

## 3. FreeRTOS 任务栈大小建议

| 任务 | 当前栈 (words→bytes) | 建议 (bytes) | 实际使用估算 | 备注 |
|------|---------------------|-------------|-------------|------|
| rs485 | 384 (1536B) | 1536 | ~1200 | DLMS APDU 解析需要大栈 |
| infrared | 256 (1024B) | 1024 | ~800 | 同 RS485 |
| metering | 192 (768B) | 768 | ~400 | SPI 读取 + 计算 |
| event_detect | 128 (512B) | 384 | ~200 | 可减小 |
| pulse | 128 (512B) | 384 | ~150 | 可减小 |
| key | 96 (384B) | 256 | ~100 | 可减小 |
| display | 192 (768B) | 768 | ~500 | LCD 段码映射 |
| storage | 192 (768B) | 768 | ~600 | Flash 写入需要 |
| watchdog | 96 (384B) | 256 | ~64 | 最小任务 |
| rtc_sync | 128 (512B) | 384 | ~200 | 可减小 |
| power_mgr | 128 (512B) | 384 | ~150 | 可减小 |
| tamper | 128 (512B) | 512 | ~300 | GPIO 读取 |
| temperature | 96 (384B) | 256 | ~100 | ADC 读取 |
| lorawan | 192 (768B) | 768 | ~500 | AT 指令拼接 |
| ota | 128 (512B) | 512 | ~300 | 版本比较 |
| cellular | 192 (768B) | 768 | ~600 | AT 指令 |

**优化后总栈: ~10.4KB** (当前 ~12.6KB, 节省 ~2.2KB)

## 4. SRAM 80KB 预算分配

| 区域 | 大小 (KB) | 说明 |
|------|----------|------|
| FreeRTOS 内核 + 全局 | 4.0 | TCB × 16 + 信号量/队列/事件组 |
| 任务栈 (16个) | 10.4 | 优化后 |
| 全局静态数据 (SharedState) | 2.0 | 计量管理器 + LCD + 事件检测器 |
| UART 收发缓冲区 | 2.0 | RS485 + IR + LoRa |
| DLMS APDU 缓冲区 | 3.0 | 解析/构建 APDU |
| 计量数据缓存 | 1.0 | 瞬时量 + 电能 + 需量 |
| 事件日志缓冲 | 1.0 | 循环缓冲 |
| OTA 接收缓冲 | 1.0 | 固件分块接收 |
| 校准参数 | 0.5 | NV 备份 |
| 杂项/对齐/预留 | 55.1 | **大量剩余** |
| **合计** | **80.0** | |

> 💡 SRAM 预算非常充裕 (仅用 ~25KB), 可考虑增加负荷曲线记录深度或 DLMS 对象缓存。

## 5. Flash 512KB 预算分配

| 区域 | 大小 (KB) | 说明 |
|------|----------|------|
| 中断向量表 + 启动 | 2 | Vector table + boot |
| 固件代码 (优化后) | 50 | 含 LTO + size 优化 |
| DLMS 协议栈 crate | 35 | dlms-cosem 全量 |
| 计量芯片驱动 | 5 | ATT7022E 或 RN8302B (编译时选择) |
| 通信驱动 | 10 | RS485 + IR + LoRa + Cellular |
| 功能模块 | 15 | 存储/显示/校准/OTA/事件/低功耗 |
| FreeRTOS 内核 | 6 | Cortex-M0+ port |
| 常量/字符串 | 3 | 版本号/错误信息/日志 |
| 外设初始化 | 2 | FM33LG0 HAL |
| 应用层 (main) | 3 | 任务创建/共享状态 |
| 保留 | 381 | 内部 Flash 空间富余 |
| **合计** | **512** | |

> 💡 Flash 预算极为充裕。可考虑增加: 负荷曲线记录容量、事件日志深度、更多 OBIS 对象支持。
