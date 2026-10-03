#!/bin/bash
# kiro2cc-proxy 一键构建脚本
# 依次构建 web-ui/admin-ui、web-ui/user-ui 前端，再编译 Rust 二进制

set -eo pipefail

NPM_REGISTRY="https://registry.npmmirror.com"

log() { echo "[$(date '+%H:%M:%S')] $*"; }

cd "$(dirname "$0")"

echo "=================================================="
echo "  kiro2cc-proxy 构建脚本"
echo "=================================================="

# 检测 npm
if ! command -v npm &>/dev/null; then
    echo "[!] 未找到 npm，请先安装 Node.js"
    echo "    brew install node"
    exit 1
fi

# 检测 cargo
if ! command -v cargo &>/dev/null; then
    echo "[!] 未找到 cargo，请先安装 Rust"
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo ""
echo "[1/3] 构建 web-ui/admin-ui..."
cd web-ui/admin-ui
log "npm install 开始 (registry: $NPM_REGISTRY)"
npm install --registry "$NPM_REGISTRY" --progress
log "npm install 完成，开始 build..."
npm run build
cd ../..
log "web-ui/admin-ui 构建完成 ✓"

echo ""
echo "[2/3] 构建 web-ui/user-ui..."
cd web-ui/user-ui
log "npm install 开始 (registry: $NPM_REGISTRY)"
npm install --registry "$NPM_REGISTRY" --progress
log "npm install 完成，开始 build..."
npm run build
cd ../..
log "web-ui/user-ui 构建完成 ✓"

echo ""
echo "[3/3] 编译 Rust 二进制..."
log "cargo build --release 开始..."
cargo build --release
log "编译完成 ✓"

echo ""
echo "=================================================="
echo "  构建成功！"
echo "  二进制位置: ./target/release/kiro2cc-proxy"
echo "  运行: ./run-local-service-mac.command"
echo "=================================================="
