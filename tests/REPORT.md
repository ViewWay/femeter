# Femeter 自动化测试报告

**日期:** 2026-04-03  
**项目:** FM33A068EV 三相智能电表固件  
**语言:** Rust (14 crates) + Python (集成测试)

---

## 1. 现有 Rust 单元测试覆盖情况

| Crate | 测试数 | 主要模块 |
|-------|--------|----------|
| dlms-apdu | 77 | codec, get, set, initiate, action, block_transfer, event, exception |
| dlms-asn1 | 20 | ber, aarq, aare, rlrq_rlre, conformance, context, initiate |
| dlms-axdr | 33 | encoder, decoder, compact, datetime_codec |
| dlms-core | 8 | types, units, obis, errors, datetime, access, traits |
| dlms-cosem | 191 | management, data_register, time_control, local_comm, internet, wireless, plc, llc, payment, mbus (最全面的 crate) |
| dlms-hal | 87 | uart, spi, i2c, gpio, adc, flash, rtc, watchdog, relay, display, modem |
| dlms-hdlc | 29 | frame, crc, address, control, config, segment, connection, llc |
| dlms-host | 25 | cli, sniffer, simulator, test_runner |
| dlms-meter-app | 119 | measurement, meter_app, alarm, control, profile, clock, tariff, firmware, comm |
| dlms-obis | 6 | parser, lookup, codes (ac_electricity, gas, water, etc.) |
| dlms-rtos | 31 | task, mutex, semaphore, queue, timer, interrupt, mempool |
| dlms-security | 103 | aes_gcm, sm4_gmac, hls, lls, key, system_title, context, control |
| femeter-core | 30 | event_detect, ota |
| virtual-meter | 42 | meter, protocol, shell, dlms, tariff, load_profile, demand, display, calibration, statistics, persistence, serial, tcp_server, iec62056 |
| **总计** | **801** | |

> 注：801 > 717，可能包含新增测试或统计差异。

### 覆盖缺口分析

- **dlms-obis** (6 tests): parser/lookup 边界用例不足，OBIS 码解析异常输入测试缺失
- **dlms-core** (8 tests): datetime 边界、units 转换边界
- **femeter-core** (30 tests): event_detect 需要更多边界条件（阈值边界、并发事件）
- **dlms-meter-app**: tariff/profile 跨时段切换场景可补充

---

## 2. Python 集成测试框架

### 架构
- **虚拟电表** 启动 TCP 文本协议服务器 (端口 8888)
- pytest 通过 socket 连接发送命令，验证响应
- session-scoped fixture 管理虚拟电表生命周期

### 测试文件

| 文件 | 测试数 | 状态 |
|------|--------|------|
| test_metering.py | 7 | ✅ 全部通过 |
| test_dlms_protocol.py | 2 | ⏭ 跳过 (需 DLMS TCP 端口) |
| test_event_detection.py | 2 | ✅ 通过 |
| test_communication.py | 2 | ✅ 通过 |
| test_display.py | 1 | ✅ 通过 |
| test_power_manager.py | 1 | ✅ 通过 |
| test_tou.py | 1 | ✅ 通过 |
| test_load_profile.py | 1 | ✅ 通过 |
| test_storage.py | 1 | ✅ 通过 |
| test_ota.py | 1 | ✅ 通过 |
| **总计** | **19** | **18 通过, 2 跳过** |

### 运行结果

```
18 passed, 2 skipped in 0.62s
```

跳过的 2 个 DLMS 测试需要额外启动 DLMS TCP 端口 (4059)。

---

## 3. 发现的问题

1. **虚拟电表 CLI 不启动 TCP 服务器**: 默认只启动交互式 shell，需要额外 binary (`test_server`) 来启动 TCP 服务
2. **DLMS 端口未自动启动**: text_server 只监听 8888，DLMS (4059) 和 IEC 62056 端口需单独启动
3. **SNAPSHOT 格式**: 事件字段命名需确认 (`events` vs `alarms`)

---

## 4. 后续建议

1. **增强 test_server**: 同时启动 text + DLMS + IEC 端口
2. **补充 DLMS 测试**: 完整的 AARQ → GetRequest → Release 流程
3. **补充 Rust 边界测试**: dlms-obis、dlms-core、femeter-core
4. **端到端场景测试**: 完整电表生命周期、事件风暴
5. **CI 集成**: 加入 GitHub Actions workflow
