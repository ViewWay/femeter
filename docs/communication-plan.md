# FeMeter 通信方案 — 自研设计文档

## 一、硬件选料

### 1.1 LTE Cat.1 模组选型

| 型号 | 厂商 | 芯片平台 | 封装 | UART | 尺寸mm | 供电V | 价格档 | 推荐度 |
|------|------|---------|------|------|--------|-------|--------|--------|
| **EC800N** | 移远 | 展锐8910DM | LCC | 3路 | 24×27.6×2.4 | 3.3~4.3 | ¥50~70 | ⭐⭐⭐⭐⭐ |
| **EC200U** | 移远 | 展锐8910DM | LCC | 3路 | 24×27.6×2.4 | 3.3~4.3 | ¥40~55 | ⭐⭐⭐⭐ |
| **L610** | 广和通 | ASR1803S | LGA | 3路 | 29×25×2.35 | 3.3~4.2 | ¥55~80 | ⭐⭐⭐⭐ |
| **ML302** | 中移物联 | ASR1803S | LCC | 2路 | 26×24×2.4 | 3.4~4.2 | ¥45~65 | ⭐⭐⭐ |
| **Air724UG** | 合宙 | 展锐8910DM | LCC | 3路 | 24.5×24.5×2.5 | 3.4~4.2 | ¥35~50 | ⭐⭐⭐⭐ |
| **SLM322** | 移柯 | ASR1803S | LCC | 2路 | ~26×24 | 3.4~4.2 | ¥45~65 | ⭐⭐⭐ |

**推荐: 移远 EC800N**
- 行业标杆，资料最全，文档完善
- 展锐 8910DM 平台成熟稳定
- 3 路 UART（主UART + 辅助UART + 调试UART）
- 内置 GNSS（可选版本带定位）
- 工业级温度范围 -40~+85°C
- 供电兼容 3.3V（FM33LG0xx IO 电平）

### 1.2 备选: NB-IoT 模组（低功耗场景）

| 型号 | 厂商 | 芯片 | 价格 | 推荐度 |
|------|------|------|------|--------|
| **BC260Y** | 移远 | 海思Hi2115 | ¥20~30 | ⭐⭐⭐⭐⭐ |
| **BC28F** | 移远 | 海思Hi2115 | ¥25~35 | ⭐⭐⭐⭐ |
| **L620** | 广和通 | ASR | ¥25~40 | ⭐⭐⭐ |
| **ML307A** | 中移物联 | ASR | ¥20~35 | ⭐⭐⭐ |

**推荐: 移远 BC260Y**（纯 NB-IoT 低功耗抄表方案）

### 1.3 其他外设确认

| 通道 | 器件 | 厂商 | 接口 | 状态 |
|------|------|------|------|------|
| RS485 | RSM485MT5V | 亿佰特 | UART0 | ✅ 已选 |
| 红外 | 分立红外收发 (Vishay TSOP38438 + TSAL6200) | Vishay | UART1 | 需确认 |
| LoRaWAN | E78-470LN22S (ASR6601) | 亿佰特 | UART2 | ✅ 已选 |
| LTE Cat.1 | EC800N | 移远 | 新增UART | 🆕 新增 |
| 调试 | USB-TTL / SWD | - | SWD | 方案调整 |

### 1.4 UART 资源重新分配

FM33LG0xx 有 **4 个 UART**，原方案全部分配：

| UART | 原方案 | 新方案 |
|------|--------|--------|
| UART0 | RS485 (RSM485MT5V) | RS485 (不变) |
| UART1 | 红外 | 红外 (不变) |
| UART2 | LoRaWAN (ASR6601) | **LTE Cat.1 (EC800N)** |
| UART3 | 调试日志 | LoRaWAN (ASR6601) |
| SWD | - | 调试日志 (defmt over SWO) |

**设计决策**:
- UART2 改为 Cat.1 模组（主力上云通道，需要高波特率）
- UART3 改为 LoRaWAN（AT 指令速率低，38400 足够）
- 调试改用 SWO (Serial Wire Output) — defmt 支持，不占 UART
- 如果仍需串口调试，可通过 Cat.1 模组的辅助 UART 做远程日志

---

## 二、硬件电路设计

### 2.1 EC800N 典型应用电路

```
                    FM33LG0xx                              EC800N
               ┌──────────────┐                    ┌──────────────┐
               │              │                    │              │
               │     UART2_TX ├───────────────────►│ MAIN_RXD    │
               │              │    9600~460800     │              │
               │     UART2_RX │◄───────────────────┤ MAIN_TXD    │
               │              │                    │              │
               │          PWR │─────┐              │ PWRKEY       │
               │              │     │              │              │
               │          RST │─────┤              │ RESET_N      │
               │              │     │              │              │
               │              │  ┌──┴──┐           │              │
               │         GPIO │──┤ 电平 │───────────│ DTR / RI    │
               │              │  │转换  │           │              │
               └──────────────┘  └─────┘           └──────┬───────┘
                                                         │
                                                    ┌────┴────┐
                                                    │  UICC   │
                                                    │ (SIM卡) │
                                                    └────┬────┘
                                                         │
                                                    ┌────┴────┐
                                                    │  天线    │
                                                    │ LTE天线  │
                                                    └─────────┘
```

### 2.2 关键设计要点

1. **电平转换**
   - FM33LG0xx IO: 3.3V
   - EC800N IO: 1.8V (典型) / 3.3V tolerant
   - 需要 1.8V↔3.3V 电平转换芯片 (如 TXS0108E / BSS138 MOS 桥)

2. **供电设计**
   - EC800N 峰值电流可达 2A (发射突发)
   - 不能直接从 MCU LDO 取电
   - 需独立 DC-DC: 输入 5V/12V → 输出 3.8V@2A
   - 大电容: 1000µF+ 钽电容靠近模组 VBAT

3. **SIM 卡接口**
   - UICC 接口: SIM_CLK, SIM_DATA, SIM_RST, SIM_VDD
   - ESD 保护: TVS 管 (如 TPD4E05U06)
   - 走线短、远离天线

4. **射频设计**
   - 50Ω 阻抗匹配
   - 天线: FPC 天线或 PCB 板载天线
   - 频段: LTE FDD B1/B3/B5/B8, TDD B34/B38/B39/B40/B41

5. **控制信号**
   - PWRKEY: 开机控制 (拉低 500ms 开机)
   - RESET_N: 硬件复位
   - DTR: 睡眠唤醒
   - RI: 模组唤醒 MCU (来电/数据)

---

## 三、驱动开发

### 3.1 Cat.1 驱动架构 (cat1.rs)

```rust
// firmware/src/cat1.rs — LTE Cat.1 模组驱动

/// Cat.1 模组 AT 指令驱动 trait
pub trait Cat1Module {
    /// 初始化模组 (开机序列)
    fn power_on(&mut self) -> Result<(), Cat1Error>;
    /// 关机
    fn power_off(&mut self) -> Result<(), Cat1Error>;
    /// 检查模组是否就绪
    fn is_ready(&mut self) -> bool;
    /// 查询信号强度
    fn get_signal_quality(&mut self) -> Result<(u8, u8), Cat1Error>; // (rssi, ber)
    /// 查询网络注册状态
    fn get_network_status(&mut self) -> Result<NetworkStatus, Cat1Error>;
    /// 查询 SIM 卡状态
    fn get_sim_status(&mut self) -> Result<SimStatus, Cat1Error>;
    /// 查询 IMEI
    fn get_imei(&mut self) -> Result<[u8; 15], Cat1Error>;
    /// 查询 ICCID
    fn get_iccid(&mut self) -> Result<String<32>, Cat1Error>;
    /// 建立 TCP/UDP 连接
    fn connect(&mut self, proto: Protocol, addr: &str, port: u16) -> Result<u8, Cat1Error>;
    /// 发送数据
    fn send(&mut self, conn_id: u8, data: &[u8]) -> Result<(), Cat1Error>;
    /// 接收数据 (非阻塞)
    fn recv(&mut self, conn_id: u8, buf: &mut [u8]) -> Result<usize, Cat1Error>;
    /// 关闭连接
    fn close(&mut self, conn_id: u8) -> Result<(), Cat1Error>;
    /// 进入低功耗
    fn sleep(&mut self) -> Result<(), Cat1Error>;
    /// 唤醒
    fn wakeup(&mut self) -> Result<(), Cat1Error>;
    /// MQTT 连接 (EC800N 内置 MQTT)
    fn mqtt_connect(&mut self, broker: &str, port: u16, client_id: &str) -> Result<(), Cat1Error>;
    /// MQTT 发布
    fn mqtt_publish(&mut self, topic: &str, data: &[u8], qos: QoS) -> Result<(), Cat1Error>;
    /// MQTT 订阅
    fn mqtt_subscribe(&mut self, topic: &str, qos: QoS) -> Result<(), Cat1Error>;
}

/// EC800N 具体实现
pub struct Ec800n<UART: UartDriver, PIN_PWR: OutputPin, PIN_RST: OutputPin> {
    uart: UART,
    pwrkey: PIN_PWR,
    reset: PIN_RST,
    rx_buf: Vec<u8, 2048>,
    urc_handler: Option<fn(UrcEvent)>,
}

// AT 指令核心实现
impl<UART, PIN_PWR, PIN_RST> Cat1Module for Ec800n<UART, PIN_PWR, PIN_RST>
where
    UART: UartDriver,
    PIN_PWR: OutputPin,
    PIN_RST: OutputPin,
{
    fn power_on(&mut self) -> Result<(), Cat1Error> {
        // 1. PWRKEY 拉低 500ms
        self.pwrkey.set_low();
        delay_ms(500);
        self.pwrkey.set_high();
        // 2. 等待 RDY 响应 (最长 10s)
        self.wait_response("RDY", 10000)?;
        // 3. 关闭回显
        self.send_at_cmd("ATE0", 1000)?;
        // 4. 检查 SIM 卡
        self.send_at_cmd("AT+CPIN?", 1000)?;
        // 5. 查询信号
        self.send_at_cmd("AT+CSQ", 1000)?;
        // 6. 注册网络
        self.send_at_cmd("AT+CREG?", 1000)?;
        Ok(())
    }
    // ... 其他实现
}
```

### 3.2 AT 指令层 (at_parser.rs)

```rust
// AT 指令解析器 — 通用组件，所有模组共用

pub struct AtParser<UART: UartDriver> {
    uart: UART,
    line_buf: String<256>,
}

impl<UART: UartDriver> AtParser<UART> {
    /// 发送 AT 指令并等待响应
    pub fn send_cmd(&mut self, cmd: &str, timeout_ms: u32) -> Result<AtResponse, AtError> {
        self.uart.write(cmd.as_bytes());
        self.uart.write(b"\r\n");
        self.wait_ok_or_error(timeout_ms)
    }

    /// 发送带数据前缀的 AT 指令 (如 AT+QSEND)
    pub fn send_data_cmd(
        &mut self,
        cmd: &str,
        data: &[u8],
        timeout_ms: u32,
    ) -> Result<AtResponse, AtError> {
        self.uart.write(cmd.as_bytes());
        self.uart.write(b"\r\n");
        // 等待 "> " 提示符
        self.wait_prompt(timeout_ms)?;
        self.uart.write(data);
        self.uart.write(&[0x1A]); // CTRL+Z 发送
        self.wait_ok_or_error(timeout_ms)
    }

    /// URC (主动上报) 处理
    pub fn poll_urc(&mut self) -> Option<UrcEvent> {
        while let Some(byte) = self.uart.read_byte() {
            if byte == b'\n' {
                return self.parse_urc(&self.line_buf);
            }
            self.line_buf.push(byte as char).ok();
        }
        None
    }
}

/// URC 事件类型
pub enum UrcEvent {
    /// 网络注册状态变化 +CREG: <stat>
    NetworkRegChanged(u8),
    /// 信号强度变化 +CSQ: <rssi>,<ber>
    SignalChanged(u8, u8),
    /// 收到数据 +QIRD: <id>,<size>
    DataReceived(u8, usize),
    /// 连接关闭 +QIURC: "closed",<id>
    ConnectionClosed(u8),
    /// MQTT 消息 +QMTPUB: <client>,<pktid>,<result>
    MqttMessage { topic: String<128>, data: Vec<u8, 512> },
    /// 模组关机 NORMAL POWER DOWN
    PowerDown,
    /// 准备就绪 RDY
    Ready,
}
```

### 3.3 多模组适配 (trait 体系)

```rust
// 所有模组统一 trait — 跟 MeteringChip 设计一致

/// NB-IoT 模组 (BC260Y)
pub struct Bc260y<UART: UartDriver, PIN: OutputPin> { /* ... */ }
impl<UART, PIN> Cat1Module for Bc260y<UART, PIN> where UART: UartDriver, PIN: OutputPin { /* ... */ }

/// LoRaWAN 模组 (ASR6601)
pub struct Asr6601<UART: UartDriver> { /* ... */ }
impl<UART> LorawanModule for Asr6601<UART> where UART: UartDriver { /* ... */ }

/// 红外通信
pub struct Irda<UART: UartDriver> { /* ... */ }
impl<UART> SerialChannel for Irda<UART> where UART: UartDriver { /* ... */ }

/// RS485
pub struct Rs485<UART: UartDriver, PIN_DE: OutputPin> { /* ... */ }
impl<UART, PIN> SerialChannel for Rs485<UART, PIN_DE>
where UART: UartDriver, PIN: OutputPin { /* ... */ }
```

---

## 四、通信协议适配

### 4.1 四通道协议栈架构

```
┌──────────────────────────────────────────────────────────────┐
│                     应用层 (Application)                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐ │
│  │ 电能数据  │ │ 费率管理  │ │ 事件告警  │ │ OTA 升级       │ │
│  └─────┬────┘ └─────┬────┘ └─────┬────┘ └───────┬────────┘ │
│        │            │            │               │           │
├────────┼────────────┼────────────┼───────────────┼───────────┤
│        │      统一消息抽象层 (Message Bus)        │           │
│  ┌─────┴────────────┴────────────┴───────────────┴────────┐ │
│  │              obis.rs — OBIS 短码 → 数据点映射           │ │
│  │              codec.rs — 统一编解码                       │ │
│  └────┬───────────┬────────────┬───────────────┬───────────┘ │
│       │           │            │               │             │
├───────┼───────────┼────────────┼───────────────┼─────────────┤
│  ┌────┴────┐ ┌────┴────┐ ┌────┴────┐   ┌─────┴──────┐      │
│  │  HDLC   │ │ IEC     │ │ AT+MQTT │   │ AT+LoRaWAN │      │
│  │  /COSEM │ │ 62056   │ │         │   │            │      │
│  │ 协议栈  │ │ -21     │ │ MQTT    │   │ CN470      │      │
│  └────┬────┘ └────┬────┘ └────┬────┘   └─────┬──────┘      │
│       │           │           │               │             │
├───────┼───────────┼───────────┼───────────────┼─────────────┤
│  ┌────┴────┐ ┌────┴────┐ ┌───┴─────┐   ┌─────┴──────┐      │
│  │  UART0  │ │  UART1  │ │ UART2   │   │  UART3     │      │
│  │ RS485   │ │  红外   │ │ Cat.1   │   │ LoRaWAN    │      │
│  └─────────┘ └─────────┘ └─────────┘   └────────────┘      │
└──────────────────────────────────────────────────────────────┘
```

### 4.2 统一消息抽象

```rust
// 所有通道共用统一的消息格式

/// 通信消息
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    /// OBIS 短码 (如 1.0.1.7.0.255 = 电压)
    pub obis: ObisCode,
    /// 数据值
    pub value: DataValue,
    /// 时间戳 (可选)
    pub timestamp: Option<UnixTimestamp>,
    /// 数据质量
    pub quality: Quality,
}

/// 统一通道 trait
pub trait Channel {
    /// 通道类型
    fn channel_type(&self) -> ChannelType;
    /// 发送消息
    fn send(&mut self, msg: &ChannelMessage) -> Result<(), ChannelError>;
    /// 接收消息 (非阻塞)
    fn recv(&mut self) -> Option<ChannelMessage>;
    /// 通道是否在线
    fn is_online(&self) -> bool;
    /// 获取通道统计
    fn stats(&self) -> ChannelStats;
}

/// 四通道实例化
pub struct CommManager<CH0, CH1, CH2, CH3>
where
    CH0: Channel, // RS485
    CH1: Channel, // 红外
    CH2: Channel, // Cat.1
    CH3: Channel, // LoRaWAN
{
    ch0: CH0,
    ch1: CH1,
    ch2: CH2,
    ch3: CH3,
    /// 路由表: OBIS → 通道优先级
    route_table: RouteTable,
}
```

### 4.3 通道协议映射

| 通道 | 物理层 | 链路层 | 应用层 | 方向 | 波特率 |
|------|--------|--------|--------|------|--------|
| CH0 RS485 | RSM485MT5V | HDLC/IEC 62056-46 | DLMS/COSEM | 双向 | 9600~115200 |
| CH1 红外 | TSOP+TSAL | IEC 62056-21 | 协议模式 A/B/C/D | 双向 | 300~9600 |
| CH2 Cat.1 | EC800N | TCP/UDP | MQTT | 双向 | 115200 |
| CH3 LoRaWAN | ASR6601 | LoRaWAN Class A | AT 指令封装 | 上行为主 | 38400 |

### 4.4 MQTT 协议设计 (Cat.1 上云通道)

```rust
// MQTT Topic 设计

// 上行 (电表 → 云)
"meter/{device_id}/data/realtime"     // 实时数据 (电压/电流/功率/电能)
"meter/{device_id}/data/daily"        // 日冻结数据
"meter/{device_id}/event/alarm"       // 告警事件
"meter/{device_id}/event/status"      // 状态变化 (上电/掉电/复位)
"meter/{device_id}/ota/report"        // OTA 状态上报

// 下行 (云 → 电表)
"meter/{device_id}/cmd/read"          // 读数据命令
"meter/{device_id}/cmd/write"         // 写参数命令
"meter/{device_id}/cmd/relay"         // 拉合闸命令
"meter/{device_id}/ota/command"       // OTA 升级命令

// QoS 策略
// 实时数据: QoS 0 (丢包可接受)
// 日冻结:   QoS 1 (必须到达)
// 告警:     QoS 1 (必须到达)
// 控制命令: QoS 1 (必须到达 + 应答)
```

---

## 五、开发阶段规划

### Phase 1.5: Cat.1 通道 (新增)

| 模块 | 文件 | 内容 | 优先级 |
|------|------|------|--------|
| AT 解析器 | `at_parser.rs` | 通用 AT 指令收发、URC 处理 | P0 |
| EC800N 驱动 | `ec800n.rs` | 开机/关机/网络注册/TCP/MQTT | P0 |
| MQTT 客户端 | `mqtt_client.rs` | 连接/发布/订阅/遗嘱 | P1 |
| 消息编解码 | `codec.rs` | JSON/CBOR 编解码 | P1 |
| 通道管理 | `comm.rs` | 四通道路由、优先级、缓冲 | P1 |

### 开发顺序

```
1. at_parser.rs (通用 AT 指令框架)
   └── 串口收发、行缓冲、超时处理、URC 解析

2. ec800n.rs (EC800N 驱动)
   └── 基于 at_parser 实现 Cat1Module trait
   └── 开机序列、网络注册、信号查询
   └── TCP/UDP Socket 操作
   └── 内置 MQTT 客户端

3. codec.rs (数据编解码)
   └── OBIS 数据 → JSON/CBOR
   └── 命令解析: JSON → 内部命令

4. comm.rs 重构 (四通道管理)
   └── Channel trait 统一接口
   └── 消息路由、优先级、重发
   └── 统计和错误处理
```

---

## 六、BOM 增量成本 (Cat.1 方案)

| 器件 | 型号 | 数量 | 单价(¥) | 小计 |
|------|------|------|---------|------|
| Cat.1 模组 | EC800N | 1 | 60 | 60 |
| SIM 卡座 | nano-SIM | 1 | 0.5 | 0.5 |
| TVS 保护 | TPD4E05U06 | 1 | 1.5 | 1.5 |
| 电平转换 | TXS0108E | 1 | 2 | 2 |
| DC-DC | TPS63020 (5V→3.8V/2A) | 1 | 5 | 5 |
| 电容 | 1000µF 钽电容 | 1 | 1 | 1 |
| 天线 | FPC LTE天线 | 1 | 3 | 3 |
| 阻容 | 去耦/匹配 | ~10 | - | 2 |
| **合计** | | | | **~¥75** |

---

## 七、总结

**UART 分配调整**:
- UART0 → RS485 (DLMS/COSEM)
- UART1 → 红外 (IEC 62056-21)
- UART2 → **Cat.1 EC800N (MQTT 上云)** ← 主力远程通道
- UART3 → LoRaWAN ASR6601
- SWO → 调试日志 (defmt)

**核心代码结构**:
```
firmware/src/
├── fm33lg0.rs      — MCU 寄存器
├── att7022e.rs     — ATT7022E 计量芯片
├── rn8302b.rs      — RN8302B 计量芯片 (待开发)
├── at_parser.rs    — AT 通用框架 (多模组共用) 🆕 ✅ 已完成
├── board.rs        — 硬件初始化
├── metering.rs     — 计量数据管理
├── comm.rs         — 多通道通信管理 (重构)
├── codec.rs        — 数据编解码 🆕
├── display.rs      — LCD 显示
└── main.rs         — 入口
```

**at_parser.rs trait 体系**:
```
UartTransport          ← 硬件串口抽象
AtParser<T>            ← 通用 AT 收发/URC 检测
├── ModuleBase         ← 基础 (AT测试/版本/复位)
├── CellularModule     ← 蜂窝 (SIM/信号/网络)
│   ├── SocketOps      ← TCP/UDP
│   ├── MqttOps        ← MQTT (EC800N内置)
│   └── CoapOps        ← CoAP (BC260Y特有)
├── LorawanOps         ← LoRaWAN (入网/发送/DR/信道)
├── PowerControl       ← 低功耗控制
└── 通道管理:
    Rs485Channel       ← DLMS/COSEM
    IrdaChannel        ← IEC 62056-21
    CloudChannel       ← MQTT 上云
    LorawanChannel     ← LoRaWAN
    ChannelManager     ← 四通道路由
```
