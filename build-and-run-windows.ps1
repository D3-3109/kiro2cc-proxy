# kiro2cc-proxy 一键构建并启动脚本（Windows）
# 先执行 build-windows.ps1 完成前端 + Rust 构建，再执行 run-local-service-windows.ps1 启动服务
# 用法: .\build-and-run-windows.ps1
# 任一环节失败即中止，不会用旧二进制启动
# 注：$ErrorActionPreference 只拦 cmdlet 异常，不拦 native 命令非零退出码；
# 子脚本内部已逐步检查 $LASTEXITCODE 并 exit 1，此处再查一道作双保险

$ErrorActionPreference = "Stop"

$SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $SCRIPT_DIR

& "$SCRIPT_DIR\build-windows.ps1"
if ($LASTEXITCODE -ne 0) { Write-Host "[!] 构建失败，中止启动"; exit $LASTEXITCODE }

& "$SCRIPT_DIR\run-local-service-windows.ps1"
