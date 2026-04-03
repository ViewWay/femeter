#!/bin/bash
echo "=========================================="
echo "虚拟电表三相控制功能快速测试"
echo "=========================================="
echo ""

# 构建并启动虚拟电表（后台模式）
cd /Users/yimiliya/.openclaw/workspace/femeter

# 检查构建
echo "1. 检查构建..."
if [ ! -f target/release/virtual-meter ]; then
    echo "需要先构建: cargo build -p virtual-meter --release"
    exit 1
fi
echo "✅ 构建文件存在"

# 显示帮助信息
echo ""
echo "2. 显示帮助信息..."
./target/release/virtual-meter --help | head -20

echo ""
echo "=========================================="
echo "✅ 快速检查完成!"
echo "=========================================="
echo ""
echo "交互模式测试命令:"
echo "  ./target/release/virtual-meter"
echo ""
echo "推荐测试序列:"
echo "  set three-phase 230 5 50 0.95"
echo "  get voltage"
echo "  get current"
echo "  get angle"
echo "  get power"
echo "  get energy"
echo "  get frequency"
echo "  get power-factor"
echo "  get status-word"
echo ""
echo "异常场景测试:"
echo "  set ua 0          # A相失压"
echo "  get status-word"
echo "  set ua 280        # A相过压"
echo "  get status-word"
echo "  set ia 70         # A相过流"
echo "  get status-word"
echo "  set angle-a 180   # 反向功率"
echo "  get status-word"
echo ""
