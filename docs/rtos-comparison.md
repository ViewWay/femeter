# FeMeter RTOS 选型对比

## 候选方案

| 维度 | Embassy (Rust async) | FreeRTOS | RT-Thread | Zephyr |
|------|---------------------|----------|-----------|--------|
| **语言** | 纯 Rust | C | C (有 Rust 绑定) | C (有 Rust 绑定) |
| **架构** | async/await 协作式 | 抢占式多任务 | 抢占式 + 协作式 | 抢占式多任务 |
| **Flash 占用** | ~8~15KB | ~6~10KB | ~10~20KB | ~20~40KB |
| **RAM 占用** | ~2~4KB (不含栈) | ~1~3KB (每任务栈) | ~2~5KB | ~5~10KB |
| **最小栈/任务** | 无任务概念, 共享栈 | 256B~4KB/任务 | 512B~4KB/任务 | 512B~4KB/任务 |
| **中断延迟** | ~15 cycles | ~12 cycles | ~15 cycles | ~20 cycles |
| **调度延迟** | ~50 cycles (poll) | ~30 cycles (上下文切换) | ~35 cycles | ~40 cycles |
| **Cortex-M0+ 支持** | ✅ 完整 | ✅ 完整 | ✅ 完整 | ✅ 完整 |
| **低功耗支持** | WFE/WFI 自动 | Tickless Idle | Tickless + PM组件 | PM子系统完善 |
| **DLMS 协议栈** | 需自写 | 需自写 | 有第三方包 | 有 LwM2M/CoAP |
| **DMA 支持** | async 天然支持 | 手动回调 | 驱动封装 | 驱动封装 |
| **调试** | defmt/probe-rs | GDB/OpenOCD | RT-Thread Studio | west/Zephyr SDK |
| **社区** | Rust embedded 核心 | 最大嵌入式生态 | 中国社区强 | Linux基金会 |
| **许可证** | MIT/Apache-2.0 | MIT | Apache-2.0 | Apache-2.0 |

## 详细分析

### 1. Embassy (Rust async/await)

**优势：**
- **零成本抽象**: async 编译为状态机，无运行时开销
- **共享栈**: 所有 async task 共享调用栈，RAM 占用极低
- **类型安全**: Rust 所有权系统避免数据竞争，编译期保证
- **与项目完美契合**: 项目已用 Rust，无需 FFI 桥接
- **DMA 天然适配**: `await` 等待 DMA 完成，代码清晰
- **defmt**: 零开销日志，Release 模式零成本

**劣势：**
- 学习曲线陡峭（async embedded 概念较新）
- 第三方驱动少（FM33 更没有现成 HAL）
- 不支持抢占式调度（协作式，必须 yield）
- 商用案例相对少

**资源估算 (FM33A068EV):**
```
Flash: ~12KB (Embassy executor + util)
RAM:   ~3KB  (executor + 8 async tasks)
       每个 task 仅需局部变量，无独立栈
```

**代码示例:**
```rust
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = Board::init();
    
    // 并发运行多个任务
    spawner.spawn(metering_task(board.spi0, board.metering)).ok();
    spawner.spawn(rs485_task(board.uart0)).ok();
    spawner.spawn(lcd_task(board.lcd)).ok();
    spawner.spawn(lorawan_task(board.uart2)).ok();
}

#[embassy_executor::task]
async fn metering_task(spi: Spi, mut chip: Att7022e) {
    let mut timer = Timer::periodic(Duration::from_millis(200));
    loop {
        timer.next().await;
        let data = chip.read_instant_data().await.unwrap();
        // 发布到 channel
    }
}
```

### 2. FreeRTOS

**优势：**
- **行业标准**: 数十亿设备部署，最成熟
- **极小体积**: 最小配置 ~6KB Flash
- **丰富文档**: 书籍、培训、社区资源海量
- **认证**: SIL3 / IEC 61508 认证版本
- **FM33 支持**: 复旦微官方可能提供 BSP

**劣势：**
- **C 语言**: 与 Rust 项目需要 FFI 桥接，unsafe 满天飞
- **每任务栈**: 10 个任务 × 512B 栈 = 5KB RAM 仅栈
- **数据竞争**: C 无保护，需手动 mutex
- **错误易发**: 队列/信号量误用导致死锁

**资源估算:**
```
Flash: ~8KB  (kernel + port)
RAM:   ~8KB  (kernel + 10 tasks × ~600B stack + queues)
```

**代码示例 (Rust FFI):**
```rust
// 需要 C 桥接层
extern "C" {
    fn xTaskCreate(pxTaskCode: ..., pcName: ..., usStackDepth: u16, ...) -> BaseType;
    fn vTaskDelay(xTicksToDelay: TickType);
}

fn rs485_task(_param: *mut c_void) -> *mut c_void {
    loop {
        unsafe { vTaskDelay(pdMS_TO_TICKS(100)); }
        // 处理 RS485
    }
}
```

### 3. RT-Thread

**优势：**
- **中国生态**: 国产 RTOS，中文文档完善
- **组件丰富**: DFS/VFS, FinSH, 设备框架, PM 框架
- **FM33**: 有 FM33LC0xx BSP（灵动微电子官方）
- **ULog/FinSH**: 实用调试工具
- **软件包**: AESDLMS、cJSON、agile_ftp 等现成包

**劣势：**
- **C 语言**: 同 FreeRTOS 的 FFI 问题
- **体积偏大**: nano 版 ~10KB，完整版 ~20KB+
- **过抽象**: 设备驱动框架对电表场景过重
- **Rust 绑定**: 实验性，生产环境不成熟

**资源估算:**
```
Flash: ~15KB (RT-Thread nano + 设备框架)
RAM:   ~6KB  (kernel + 8 任务栈 + 消息队列)
```

### 4. Zephyr

**优势：**
- **最完善**: 设备树 + Kconfig + 驱动模型
- **LwM2M/CoAP**: 原生 IoT 协议支持
- **安全**: PSA Certified Level 1/2/3
- **PM 子系统**: 最完善的电源管理框架
- **测试**: Twister 自动化测试框架

**劣势：**
- **臃肿**: 最小 ~20KB Flash, 有电表场景下浪费
- **学习曲线**: 最陡峭（设备树、Kconfig、驱动模型）
- **编译慢**: CMake + West 构建系统复杂
- **Rust 支持**: 实验性
- **Cortex-M0+ 性能**: 框架开销在 64MHz 下可感知

**资源估算:**
```
Flash: ~25KB (kernel + 最低驱动)
RAM:   ~8KB  (kernel + 5 threads + net_buf)
```

## 评分矩阵 (权重针对智能电表场景)

| 权重 | 维度 | Embassy | FreeRTOS | RT-Thread | Zephyr |
|------|------|---------|----------|-----------|--------|
| 30% | 与 Rust 项目集成 | ★★★★★ | ★★☆☆☆ | ★★☆☆☆ | ★★☆☆☆ |
| 20% | Flash/RAM 效率 | ★★★★★ | ★★★★☆ | ★★★☆☆ | ★★☆☆☆ |
| 15% | 低功耗支持 | ★★★★☆ | ★★★☆☆ | ★★★★☆ | ★★★★★ |
| 10% | DLMS/IoT 协议 | ★★☆☆☆ | ★★☆☆☆ | ★★★★☆ | ★★★★★ |
| 10% | 开发效率 | ★★★★☆ | ★★★☆☆ | ★★★★☆ | ★★★☆☆ |
| 10% | 长期维护 | ★★★★☆ | ★★★★★ | ★★★★☆ | ★★★★☆ |
| 5% | 调试工具 | ★★★★☆ | ★★★★☆ | ★★★★☆ | ★★★★★ |
| **100%** | **加权总分** | **4.30** | **2.95** | **3.10** | **3.15** |

## 推荐方案

### 🏆 首选: Embassy (Rust async)

**理由：**
1. 项目已 100% Rust，Embassy 无需 FFI，编译期安全保证
2. 共享栈模型完美适配 FM33A068EV 的 80KB SRAM
3. async/await 天然适合 DMA、UART 等异步操作
4. 低功耗: `Timer::periodic().await` 自动进入 WFE
5. 社区活跃，probe-rs + defmt 调试体验优秀

### 实施路径

```
Phase 1 (当前): 裸机 + 手动调度 (已有)
    ↓ 添加 embassy-executor, embassy-time
Phase 2: Embassy executor + async tasks
    - metering_task: 200ms 定时读计量
    - rs485_task: HDLC 收发
    - infrared_task: IEC 62056-21
    - lorawan_task: AT 指令状态机
    - cellular_task: MQTT/CoAP
    - lcd_task: 显示刷新
    - pulse_task: 脉冲输出
    - tamper_task: 防窃电检测
Phase 3: Embassy USB (如需) + OTA
```

### Embassy 依赖配置

```toml
[dependencies]
embassy-executor = { version = "0.7", features = ["arch-cortex-m", "executor-thread"] }
embassy-time = "0.4"
embassy-futures = "0.2"
# 对于 FM33, 需要自写 embassy-time driver (基于 BSTIM 或 LPTIM)
```

### 备选: 如果团队偏好 C RTOS
**RT-Thread nano** — 国产生态好，FM33 有 BSP，体积可接受。但 Rust FFI 桥接成本高。

---

*结论: 对于纯 Rust 的 FeMeter 项目，Embassy 是综合最优解。*
