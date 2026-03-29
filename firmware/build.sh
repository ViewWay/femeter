#!/bin/bash
# ================================================================== #
#                                                                    #
#  build.sh — FeMeter 一键编译所有物料组合                             #
#                                                                    #
#  用法:                                                              #
#    ./build.sh                    # 编译所有预设                     #
#    ./build.sh highend            # 仅编译高端型                     #
#    ./build.sh standard           # 仅编译标准型                     #
#    ./build.sh economy            # 仅编译经济型                     #
#    ./build.sh custom <features>  # 自定义 feature 组合              #
#    ./build.sh clean              # 清理                             #
#                                                                    #
#  输出: output/ 目录                                                 #
#    femeter-<preset>-normal.bin   # 应用固件                        #
#    femeter-<preset>-boot.bin     # Bootloader                     #
#    femeter-<preset>-ff.bin       # 出厂烧录 (Boot + Normal)        #
#    femeter-<preset>.hex          # 调试用 HEX                     #
#    femeter-<preset>-ota.bin      # OTA 升级包 (Normal + Header)    #
#                                                                    #
#  (c) 2026 FeMeter Project — ViewWay                                #
# ================================================================== #

set -euo pipefail

# ── 路径 ──
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FIRMWARE_DIR="$SCRIPT_DIR"
OUTPUT_DIR="$SCRIPT_DIR/../output"
TOOLCHAIN="thumbv6m-none-eabi"
OBJCOPY="arm-none-eabi-objcopy"

# ── 颜色 ──
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()    { echo -e "${GREEN}[ OK ]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERR]${NC} $1"; }

# ── 预设定义 ──
# 格式: "名称:features"
PRESETS=(
    "highend:rn8615v2,ec800n,bc260y,gps,lwm2m,coap,sms,http,ftp,mqtt,ext-flash,bat-er26500,pq-analysis,dlms"
    "standard:rn8302b,bc260y,mqtt,sms,bat-er26500,dlms"
    "economy:att7022e,bat-er17335,dlms"
)

# ── 工具检查 ──
check_tools() {
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Install Rust: https://rustup.rs"
        exit 1
    fi
    if ! rustup target list --installed | grep -q "$TOOLCHAIN"; then
        log_info "Installing target $TOOLCHAIN..."
        rustup target add "$TOOLCHAIN"
    fi
    # objcopy 可选 (HEX 生成)
    if ! command -v "$OBJCOPY" &> /dev/null; then
        log_warn "$OBJCOPY not found. HEX files will not be generated."
        log_warn "Install: brew install arm-none-eabi-gcc"
    fi
}

# ── 编译单个预设 ──
build_preset() {
    local name="$1"
    local features="$2"
    local target_dir="$OUTPUT_DIR/$name"

    log_info "Building preset: ${YELLOW}$name${NC}"
    log_info "  Features: $features"

    mkdir -p "$target_dir"

    # 编译应用固件
    cd "$FIRMWARE_DIR"
    log_info "  Compiling firmware..."
    if cargo build --release --target "$TOOLCHAIN" --features "$features" 2>&1; then
        log_ok "  Firmware compiled"
    else
        log_error "  Firmware compilation failed!"
        return 1
    fi

    local elf="target/$TOOLCHAIN/release/femeter"

    if [ -f "$elf" ]; then
        # 生成 bin
        cargo objcopy --release -- -O binary "$target_dir/femeter-normal.bin"
        log_ok "  → femeter-normal.bin ($(stat -f%z "$target_dir/femeter-normal.bin" 2>/dev/null || stat -c%s "$target_dir/femeter-normal.bin") bytes)"

        # 生成 hex (如果 objcopy 可用)
        if command -v "$OBJCOPY" &> /dev/null; then
            "$OBJCOPY" "$elf" -O ihex "$target_dir/femeter.hex"
            log_ok "  → femeter.hex"
        fi

        # 编译 Bootloader
        log_info "  Compiling bootloader..."
        if cargo build --release --target "$TOOLCHAIN" --bin femeter-boot --features "$features" 2>&1; then
            local boot_elf="target/$TOOLCHAIN/release/femeter-boot"
            cargo objcopy --release --bin femeter-boot -- -O binary "$target_dir/femeter-boot.bin"
            log_ok "  → femeter-boot.bin"
        else
            log_warn "  Bootloader compilation skipped"
        fi

        # 生成 FF (Factory Flash = Boot + Normal)
        if [ -f "$target_dir/femeter-boot.bin" ] && [ -f "$target_dir/femeter-normal.bin" ]; then
            cat "$target_dir/femeter-boot.bin" "$target_dir/femeter-normal.bin" \
                > "$target_dir/femeter-ff.bin"
            log_ok "  → femeter-ff.bin (Factory Flash)"
        fi

        # 生成 OTA 包 (添加 header)
        # Header: magic(4) + size(4) + crc32(4) + version(4) + timestamp(4) + hw_hash(4) + reserved(8) = 32 bytes
        if command -v python3 &> /dev/null; then
            python3 - "$target_dir/femeter-normal.bin" "$target_dir/femeter-ota.bin" <<'PYEOF'
import sys, struct, zlib, time
input_file = sys.argv[1]
output_file = sys.argv[2]
with open(input_file, 'rb') as f:
    data = f.read()
crc = zlib.crc32(data) & 0xFFFFFFFF
magic = 0x46544D52  # "FTMR"
size = len(data)
version = (0, 2, 0, 0)  # 0.2.0
timestamp = int(time.time())
hw_hash = 0  # TODO: 根据物料组合生成
header = struct.pack('<IIIIIII8x', magic, size, crc,
                     version[0] | (version[1]<<8) | (version[2]<<16) | (version[3]<<24),
                     timestamp, hw_hash, 0)
with open(output_file, 'wb') as f:
    f.write(header)
    f.write(data)
print(f"  OTA package: {len(data)} bytes, CRC32=0x{crc:08X}")
PYEOF
            log_ok "  → femeter-ota.bin (OTA upgrade package)"
        fi

        # 输出固件大小统计
        local size=$(stat -f%z "$target_dir/femeter-normal.bin" 2>/dev/null || stat -c%s "$target_dir/femeter-normal.bin")
        local pct=$((size * 100 / (128 * 1024)))
        log_info "  Flash usage: $size / 131072 bytes ($pct%)"

        log_ok "Preset ${GREEN}$name${NC} build complete!"
        echo ""
    else
        log_error "  ELF not found: $elf"
        return 1
    fi
}

# ── 清理 ──
do_clean() {
    log_info "Cleaning build artifacts..."
    cd "$FIRMWARE_DIR"
    cargo clean
    rm -rf "$OUTPUT_DIR"
    log_ok "Clean done"
}

# ── 主入口 ──
main() {
    check_tools
    mkdir -p "$OUTPUT_DIR"

    local cmd="${1:-all}"

    case "$cmd" in
        clean)
            do_clean
            ;;
        highend|standard|economy)
            for preset in "${PRESETS[@]}"; do
                local name="${preset%%:*}"
                local features="${preset#*:}"
                if [ "$name" = "$cmd" ]; then
                    build_preset "$name" "$features"
                    break
                fi
            done
            ;;
        custom)
            if [ -z "${2:-}" ]; then
                log_error "Usage: $0 custom <features>"
                exit 1
            fi
            build_preset "custom" "$2"
            ;;
        all)
            log_info "Building all presets..."
            echo ""
            local failed=0
            for preset in "${PRESETS[@]}"; do
                local name="${preset%%:*}"
                local features="${preset#*:}"
                build_preset "$name" "$features" || ((failed++))
            done
            echo ""
            if [ $failed -eq 0 ]; then
                log_ok "All presets built successfully!"
                echo ""
                log_info "Output:"
                ls -la "$OUTPUT_DIR/"*/
            else
                log_error "$failed preset(s) failed!"
                exit 1
            fi
            ;;
        *)
            echo "Usage: $0 [all|highend|standard|economy|custom|clean]"
            echo ""
            echo "Presets:"
            echo "  all       Build all presets"
            echo "  highend   RN8615V2 + Cat.1 + NB-IoT + GPS + 全功能"
            echo "  standard  RN8302B + NB-IoT + MQTT"
            echo "  economy   ATT7022E + 无蜂窝"
            echo "  custom    自定义 feature 组合"
            echo "  clean     清理编译产物"
            exit 1
            ;;
    esac
}

main "$@"
