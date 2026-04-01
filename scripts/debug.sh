#!/bin/bash
# FeMeter 固件调试脚本
# 使用 J-Link + OpenOCD + GDB
#
# 用法:
#   ./scripts/debug.sh              # 调试主固件
#   ./scripts/debug.sh boot         # 调试 bootloader

set -e

FIRMWARE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$FIRMWARE_DIR/target/thumbv6m-none-eabi/release"
OPENOCD_CFG="$FIRMWARE_DIR/openocd/fm33a068ev.cfg"

MODE="${1:-main}"
if [ "$MODE" = "boot" ]; then
    ELF="$TARGET_DIR/femeter-boot"
else
    ELF="$TARGET_DIR/femeter"
fi

if [ ! -f "$ELF" ]; then
    echo "Error: $ELF not found"
    echo "Run 'cargo build --release --target thumbv6m-none-eabi' first"
    exit 1
fi

echo ">> Starting OpenOCD (port 3333 for GDB)..."
echo ">> ELF: $ELF"

# 后台启动 OpenOCD
openocd -f "$OPENOCD_CFG" &
OPENOCD_PID=$!

cleanup() {
    kill $OPENOCD_PID 2>/dev/null
}
trap cleanup EXIT

sleep 1

# 启动 GDB
arm-none-eabi-gdb \
    -ex "file $ELF" \
    -ex "target remote :3333" \
    -ex "load" \
    -ex "break main" \
    -ex "continue"
