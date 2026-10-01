#!/bin/bash
# kiro2cc-proxy macOS 本地 debug 运行脚本
# 以 debug 日志级别（RUST_LOG=debug）+ debug 编译模式直跑，便于排查问题

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

CONFIG_DIR="$SCRIPT_DIR/app/config"
CONFIG_FILE="$CONFIG_DIR/config.json"
CREDENTIALS_FILE="$CONFIG_DIR/credentials.json"

echo "=================================================="
echo "  kiro2cc-proxy debug 运行脚本"
echo "=================================================="

if [ ! -f "$CONFIG_FILE" ]; then
    echo "[!] 未找到配置: $CONFIG_FILE"
    echo "[*] 请先运行 ./run-local-service-mac.sh 完成初始配置"
    read -p "按回车退出..."
    exit 1
fi

if ! command -v cargo &>/dev/null; then
    echo "[!] 未找到 cargo，请先安装 Rust: https://rustup.rs"
    read -p "按回车退出..."
    exit 1
fi

export RUST_LOG="${RUST_LOG:-debug}"

echo "[*] RUST_LOG=$RUST_LOG"
echo "[*] cargo run（debug 编译，首次需要几分钟）..."
echo ""

exec cargo run -- --config "$CONFIG_FILE" --credentials "$CREDENTIALS_FILE"
