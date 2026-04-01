#!/bin/bash
# FeMeter 固件烧录脚本
# 使用 J-Link + OpenOCD
#
# 用法:
#   ./scripts/flash.sh              # 烧录主固件
#   ./scripts/flash.sh boot         # 烧录 bootloader
#   ./scripts/flash.sh --erase      # 全片擦除后烧录
#   ./scripts/flash.sh --debug      # 烧录后保持调试连接

set -e

FIRMWARE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$FIRMWARE_DIR/target/thumbv6m-none-eabi/release"
OPENOCD_CFG="$FIRMWARE_DIR/openocd/fm33a068ev.cfg"

# 参数解析
MODE="main"
EXTRA_CMDS=""
for arg in "$@"; do
    case "$arg" in
        boot)   MODE="boot" ;;
        --erase) EXTRA_CMDS="init reset halt; flash erase_sector 0 0 last; $EXTRA_CMDS" ;;
        --debug) EXTRA_CMDS="$EXTRA_CMDS; echo 'Debug mode: GDB on port 3333'" ;;
        *) echo "Unknown arg: $arg"; exit 1 ;;
    esac
done

# 选择二进制
if [ "$MODE" = "boot" ]; then
    BIN="$TARGET_DIR/femeter-boot"
    ADDR="0x0"
    echo ">> Flashing bootloader..."
else
    BIN="$TARGET_DIR/femeter"
    ADDR="0x0"
    echo ">> Flashing main firmware..."
fi

if [ ! -f "$BIN" ]; then
    echo "Error: Binary not found: $BIN"
    echo "Run 'cargo build --release --target thumbv6m-none-eabi' first"
    exit 1
fi

# 显示二进制信息
echo "Binary: $BIN"
echo "Address: $ADDR"
arm-none-eabi-size "$BIN" 2>/dev/null || echo "(arm-none-eabi-size not found)"

# 烧录
echo ">> Flashing via OpenOCD..."
openocd -f "$OPENOCD_CFG" \
    -c "$EXTRA_CMDS" \
    -c "program $BIN $ADDR verify reset exit"

echo ">> Done!"
