# kiro2cc-proxy 一键构建并启动脚本（Windows）
# 先执行 build-windows.ps1 完成前端 + Rust 构建，再执行 run-local-service-windows.ps1 启动服务
# 用法: .\build-and-run-windows.ps1
# 任一环节失败即中止（$ErrorActionPreference = "Stop"），不会用旧二进制启动

$ErrorActionPreference = "Stop"

$SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $SCRIPT_DIR

& "$SCRIPT_DIR\build-windows.ps1"
& "$SCRIPT_DIR\run-local-service-windows.ps1"
