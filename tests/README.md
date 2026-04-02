# Femeter 测试框架

## 快速开始

```bash
# 1. 编译虚拟电表
cargo build -p virtual-meter --release

# 2. 安装 Python 依赖
pip install -r tests/requirements.txt

# 3. 运行测试
pytest tests/ -v

# 4. 运行单个模块
pytest tests/test_dlms_protocol.py -v
```

## 测试层级

### Layer 1: Rust 单元测试 (717 个)
直接运行 `cargo test`

### Layer 2: Python 集成测试
通过虚拟电表 TCP 接口 (端口 8888 文本协议, 4059 DLMS) 进行集成测试

### Layer 3: 端到端场景
覆盖完整电表生命周期

## 测试模块

| 文件 | 覆盖范围 |
|------|----------|
| test_metering.py | 电压/电流/功率/电能读取 |
| test_dlms_protocol.py | DLMS/COSEM 协议栈 |
| test_event_detection.py | 过压/过流/断相等事件 |
| test_communication.py | HDLC帧/串口/超时 |
| test_storage.py | 数据持久化 |
| test_display.py | LCD显示 |
| test_power_manager.py | 功耗/资源管理 |
| test_tou.py | 分时费率 |
| test_load_profile.py | 负荷曲线 |
| test_ota.py | 固件升级 |
