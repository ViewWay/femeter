# Femeter ↔ dlms-cosem 互操作性测试报告

**日期**: 2026-04-03  
**测试环境**: macOS (arm64), Python 3.14, Rust stable

## 测试结果

```
13 passed, 1 xfailed in 5.22s
```

## 测试分类

### ✅ 基础连接 (2/2)
| 测试 | 结果 |
|------|------|
| TCP 文本端口 (8888) 连接 | PASS |
| DLMS HDLC 端口 (4059) 连接 | PASS |

### ✅ HDLC/APDU 级别 (8/9)
| 测试 | 结果 |
|------|------|
| HDLC 帧往返 (AARQ/AARE) | PASS |
| 释放请求 (RLRQ/RLRE) | PASS |
| 读取时钟 (Clock 0.0.1.0.0.255 attr 2) | PASS |
| 读取有功电能 (Register 1.0.1.8.0.255) | PASS |
| 读取 A 相电压 (Register 1.0.32.7.0.255) | PASS |
| SET 请求 (Clock timezone) | PASS |
| ACTION 请求 (Association method 1) | XFAIL* |
| 未知 OBIS 读取 | PASS |

*XFAIL: Action request 格式需要参数字段，待进一步调查

### ✅ Python dlms-cosem 集成 (3/3)
| 测试 | 结果 |
|------|------|
| Python AARQ 被 Rust 正确解码 | PASS |
| AARE 包含 LN OID | PASS |
| 完整会话 (AARQ→GET→GET→RLRE) | PASS |

### ✅ 文本协议 (1/1)
| 测试 | 结果 |
|------|------|
| help 命令 | PASS |

## 发现并修复的 Bug

### 1. test_server HDLC 帧解析 (已修复)
- **问题**: `handle_dlms_client` 剥离 0x7E 标志后直接传递给 `HdlcFrame::decode`，但 decode 期望带标志的完整帧
- **修复**: 在传递前包装 0x7E 标志，并正确处理字节反转义

### 2. test_server AARQ tag 匹配 (已修复)
- **问题**: `process_apdu` 仅匹配 `0xE0`，但标准 AARQ 使用 `0x60` tag
- **修复**: 同时匹配 `0x60 | 0xE0`

### 3. AARQ 解码兼容性 (已修复)
- **问题**: Python dlms-cosem 生成的 AARQ 使用 context-1 (A1) 和 context-6 (A6) 标签，Rust 仅接受 context-0 和 context-30
- **修复**: 扩展 AARQ 解码器接受两种标签格式

### 4. AARQ user-information 格式 (已修复)
- **问题**: Python 库将 initiate request 放在 OCTET STRING 中，Rust 期望 BER context-1 constructed
- **修复**: 同时支持两种格式（OCTET STRING 包装和直接 BER）

## 已知不兼容性

### 1. AARE 响应格式
- Rust 生成的 AARE 使用不同的 BER 内部标签映射
- Python dlms-cosem 的 `AARE.from_bytes()` 无法解析 Rust AARE
- **影响**: 无法直接使用 Python DlmsConnection 的高层 API 解析 AARE
- **规避**: 使用 raw APDU 层通信

### 2. HDLC 链路层
- Rust 虚拟电表没有 SNRM/UA 链路层状态机
- 无法使用 Python `HdlcTransport` 的标准连接流程
- **规避**: 使用 `DirectHdlcTransport` 直接发送 I-frame

### 3. HDLC 帧格式差异
- Rust HDLC 使用 HCS (Header Check Sequence)，标准 HDLC 仅使用 FCS
- **已处理**: 测试代码同时生成 HCS 和 FCS
