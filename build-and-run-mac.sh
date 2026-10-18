#!/bin/bash
# kiro2cc-proxy 一键构建并启动脚本（macOS）
# 先执行 build-mac.sh 完成前端 + Rust 构建，再执行 run-local-service-mac.sh 启动服务
# 任一环节失败即中止，不会用旧二进制启动

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

"$SCRIPT_DIR/build-mac.sh"
"$SCRIPT_DIR/run-local-service-mac.sh"
