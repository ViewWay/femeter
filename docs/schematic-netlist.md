# FeMeter 电路原理图 — 引脚连接表 (Netlist)

## FM33A068EV LQFP80 引脚分配

### 电源
| Net | MCU Pin | 连接 | 说明 |
|-----|---------|------|------|
| VDD | 10, 31, 48, 65, 75 | 3.3V | 数字电源 (4pin) |
| VSS | 9, 32, 47, 64, 74 | GND | 数字地 (5pin) |
| VDDA | 20 | 3.3V (LC滤波) | 模拟电源 |
| VSSA | 19 | GND (LC滤波) | 模拟地 |
| VBAT | 22 | ER26500+ | RTC后备电源 |
| VDDIO2 | 72 | 3.3V | PF口独立电源(可不同电压) |

### 时钟
| Net | MCU Pin | 外设 | 连接 | 说明 |
|-----|---------|------|------|------|
| XTHF_IN | 15 | XTHF | 8MHz 晶振 Y1 | 外部高速时钟 |
| XTHF_OUT | 16 | XTHF | 8MHz 晶振 Y1 | (12pF 负载电容 C1/C2) |
| XTLF_IN | 23 | XTLF | 32.768kHz 晶振 Y2 | RTC时钟 |
| XTLF_OUT | 24 | XTLF | 32.768kHz 晶振 Y2 | (6pF 负载电容 C3/C4) |

### 复位/调试
| Net | MCU Pin | 连接 | 说明 |
|-----|---------|------|------|
| NRST | 14 | 10K上拉+100nF→GND | 复位引脚 |
| SWDIO | 53 | SWD 调试口 | 2pin SWD |
| SWCLK | 54 | SWD 调试口 | (PA13/SWDIO, PA14/SWCLK) |

### SPI0 — 计量芯片 (ATT7022E/RN8302B)
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| SPI0_SCK | 77 | PF14 | 计量芯片 SCLK | SPI时钟 |
| SPI0_MISO | 76 | PF13 | 计量芯片 DOUT | SPI主入 |
| SPI0_MOSI | 75 | PF12 | 计量芯片 DIN | SPI主出 |
| SPI0_CSN | 78 | PF15 | 计量芯片 CS | 片选(低有效) |
| METER_RST | 79 | PG0 | 计量芯片 RST | 复位(高有效) |
| METER_IRQ | 80 | PG1 | 计量芯片 IRQ | 数据就绪中断 |

### SPI1 — 外部 Flash (W25Q64, 可选)
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| SPI1_SCK | 17 | PA5 | W25Q64 CLK | SPI时钟 |
| SPI1_MISO | 18 | PA6 | W25Q64 DO | 主入 |
| SPI1_MOSI | 21 | PA7 | W25Q64 DI | 主出 |
| SPI1_CSN | 16 | PA4 | W25Q64 CS | 片选 |
| W25Q64_WP | — | 3.3V | W25Q64 WP | 写保护(不保护) |
| W25Q64_HOLD | — | 3.3V | W25Q64 HOLD | 不暂停 |

### UART0 — RS485 (DLMS/COSEM)
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| UART0_TX | 62 | PG9 | RSM485MT5V DI | 数据输入 |
| UART0_RX | 61 | PG8 | RSM485MT5V RO | 数据输出 |
| RS485_DE | 57 | PF2 | RSM485MT5V DE/RE | 方向控制(高=发送) |
| RS485_AB_P | — | — | RSM485MT5V A | RS485总线A |
| RS485_AB_N | — | — | RSM485MT5V B | RS485总线B |
| RS485_GND | — | — | RSM485MT5V GND | 隔离地 |

> RSM485MT5V: 5V供电, 隔离DC-DC, 自动偏置, 120Ω终端电阻 onboard

### UART1 — 红外 (IEC 62056-21)
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| UART1_TX | 41 | PE4 | IR发射管驱动(NPN) | 38kHz载波调制 |
| UART1_RX | 40 | PE3 | IR接收模块 OUT | TSOP38238 或 HS0038B |
| IR_TX_EN | 42 | PE5 | NPN基极(限流电阻) | 红外发射使能 |

> 红外调制: 使用 UARTIR 独立模块或外部门电路生成 38kHz 载波
> 波特率: 300bps(初始) → 9600bps(协商后)

### UART2 — LoRaWAN (ASR6601)
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| UART2_TX | 27 | PB3 | E78-470LN22S RX | AT指令发送 |
| UART2_RX | 26 | PB2 | E78-470LN22S TX | AT指令接收 |
| LORA_RST | 28 | PB4 | E78-470LN22S RST | 模组复位 |
| LORA_EN | 29 | PB5 | E78-470LN22S PWR | 模组电源控制 |

> E78-470LN22S: CN470~510MHz, 22dBm, 内置天线匹配, 38400bps

### UART3 — 蜂窝模组 (EC800N / BC260Y)
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| UART3_TX | 39 | PB15 | EC800N RXD | AT指令 |
| UART3_RX | 38 | PB14 | EC800N TXD | 响应数据 |
| CELL_PWRKEY | 40 | PE2 | EC800N PWRKEY | 开关机 |
| CELL_RESET | 46 | PE7 | EC800N RESET | 硬件复位 |
| CELL_DTR | 47 | PE8 | EC800N DTR | 睡眠唤醒 |
| CELL_RI | 48 | PE9 | EC800N RI | 来电/短信指示 |

> 蜂窝模组电源: 3.4~4.2V (VBAT), 峰值电流 ~2A, 需大电容 (100µF+)

### LPUART0 — 调试/低功耗唤醒
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| LPUART0_TX | 73 | PF0 | 调试口 TX | defmt/日志输出 |
| LPUART0_RX | 71 | PF1 | 调试口 RX | 命令输入 |

> LPUART0 可在 DEEPSLEEP 模式下接收数据唤醒 MCU

### LCD — 4COM × 44SEG 段码显示
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| LCD_COM0 | 3 | PA0 | LCD COM1 | 公共端1 |
| LCD_COM1 | 4 | PA1 | LCD COM2 | 公共端2 |
| LCD_COM2 | 5 | PA2 | LCD COM3 | 公共端3 |
| LCD_COM3 | 6 | PA3 | LCD COM4 | 公共端4 |
| LCD_SEG0~7 | 7~14 | PA4~PA11 | LCD SEG0~7 | 段驱动(与其他复用) |
| LCD_SEG8~39 | — | PC/PE/PD | LCD SEG8~39 | 段驱动(根据封装) |

> LCD 配置: 1/3 bias, 1/4 duty, 片内电阻分压
> 掉电显示: 电池供电保持显示电能值

### LED 指示灯
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| LED_POWER | 34 | PA8 | LED1(绿)阳极, 阴极→1K→GND | 电源指示 |
| LED_COMM | 35 | PA9 | LED2(黄)阳极 | 通信指示 |
| LED_ALARM | 36 | PA10 | LED3(红)阳极 | 告警指示 |
| LED_PULSE_P | 37 | PA11 | LED4(红)阳极 | 有功脉冲 |
| LED_PULSE_Q | 38 | PA12 | LED5(绿)阳极 | 无功脉冲 |

### 蜂鸣器
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| BUZZER | 43 | PA15 | NPN基极→蜂鸣器(有源) | 告警蜂鸣 |

### 按键
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| KEY_PAGE | 25 | PB0 | 按键→GND, 10K上拉 | 翻页键(EXTI) |
| KEY_PROG | 26 | PB1 | 按键→GND, 10K上拉 | 编程键(EXTI) |

### 脉冲输出 (光耦隔离)
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| PULSE_P | 29 | PB6 | PC817 LED+ (经限流电阻) | 有功脉冲输出 |
| PULSE_Q | 30 | PB7 | PC817 LED+ (经限流电阻) | 无功脉冲输出 |

> PC817: CTR 50~300%, 输出集电极开路, 外部上拉
> 脉冲宽度: 80ms (可配), 脉冲常数: 6400 imp/kWh

### 防窃电检测
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| COVER_DET | 52 | PD8 | 微动开关→GND, 10K上拉 | 上盖检测 |
| TERMINAL_DET | 51 | PD9 | 微动开关→GND, 10K上拉 | 端子盖检测 |
| MAGNETIC_DET | 37 | PA13 | 霍尔传感器 OUT | 磁场检测(防窃电) |

### 电源监测
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| POWER_FAIL | 26 | PB1 | 比较器输出(或外部电压检测) | 掉电中断 |
| BAT_ADC | 67 | PF6(ADC5) | 电池分压(R1/R2) | 电池电压采样 |

### SIM 卡
| Net | MCU Pin | GPIO | 连接 | 说明 |
|-----|---------|------|------|------|
| SIM_DET | 50 | PD7 | SIM卡座检测开关 | 插卡检测 |

> SIM 信号 (SIM_CLK/SIM_DATA/SIM_RST/SIM_VDD) 由蜂窝模组直接管理

## 外部器件清单

### 计量芯片周边 (ATT7022E)
| 器件 | 值 | 连接 | 说明 |
|------|-----|------|------|
| C5, C6, C7 | 33nF | 各相电压采样输入 | 抗混叠滤波 |
| C8, C9, C10 | 33nF | 各相电流采样输入 | 抗混叠滤波 |
| R1~R6 | 1KΩ | 采样电阻分压网络 | 电压/电流通道 |
| Y3 | 5.5296MHz | ATT7022E OSC | 计量晶振 |
| R7 | 1MΩ | Y3 并联 | 晶振反馈电阻 |

### 计量芯片周边 (RN8302B)
| 器件 | 值 | 连接 | 说明 |
|------|-----|------|------|
| C11~C16 | 1µF | 抗混叠 | Sigma-Delta输入滤波 |
| Y4 | 8.192MHz | RN8302B OSC | 高精度晶振 |

### RS485 周边
| 器件 | 值 | 连接 | 说明 |
|------|-----|------|------|
| U2 | RSM485MT5V | 隔离RS485收发 | 亿佰特模组 |
| R8 | 120Ω | A-B之间 | 终端电阻(可选Jumper) |
| TVS1 | SMDJ6.5A | A-B之间 | 防浪涌 |
| C17 | 100nF | RSM485 VCC-GND | 去耦 |
| C18 | 10µF | RSM485 VISO-GNDISO | 隔离侧去耦 |

### 电源
| 器件 | 值 | 说明 |
|------|-----|------|
| U3 | LM2576-3.3 或 LDO | 3.3V 主电源 (工频变压器副边) |
| U4 | HT7133 | VBAT域 3.3V (电池供电) |
| C19 | 100µF | 3.3V 大电容 |
| C20~C24 | 100nF | 各IC去耦 |
| L1 | 10µH | VDDA LC滤波电感 |
| D1 | 1N5819 | 电池防反灌 |

### 蜂窝模组电源
| 器件 | 值 | 说明 |
|------|-----|------|
| C25 | 100µF (MLCC) | EC800N VBAT 去耦(峰值2A) |
| C26 | 10µF | VBAT 补充 |
| C27 | 33pF | RF 去耦 |

## PCB 布局注意事项

1. **计量采样**: 抗混叠电容紧贴计量芯片引脚, 差分走线等长
2. **RS485**: 隔离模组跨接在隔离带两侧, GND不直连
3. **晶振**: 远离高速信号, 走线短, 地平面屏蔽
4. **蜂窝**: VBAT 走线宽, 100µF电容贴近模组引脚
5. **LCD**: 段码走线远离计量采样区, 避免串扰
6. **EMC**: RS485接口加TVS, 蜂窝天线远离模拟采样

---

*此文档等效于原理图 netlist，可用于 PCB 设计参考。*
*实际引脚分配需根据 FM33A068EV LQFP80 最终 datasheet 确认。*
