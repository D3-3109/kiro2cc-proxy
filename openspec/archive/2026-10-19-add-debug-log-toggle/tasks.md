# 任务清单：add-debug-log-toggle

## 状态：DONE

## 任务
- [x] 1. `src/model/config.rs` 新增 `debug_logs: bool`（`#[serde(default)]`，默认 false）与
      `DEBUG_LOGS` 环境变量覆盖；补 serde 缺省/显式值单测（验证：`cargo test config::tests`）
- [x] 2. 新建 `src/logging.rs`：`build_filter` / `install` / `set_debug_enabled` /
      `debug_logs_enabled`；句柄未安装时 `set_debug_enabled` 返回明确错误而非静默失败；
      `main.rs` 注册模块（验证：`cargo check`）
- [x] 3. `src/main.rs` 日志初始化改为全局 `reload::Layer` 过滤，三个 layer 移除各自
      `with_filter`，启动时按 `config.debug_logs` 构建并安装句柄（验证：`cargo check`）
- [x] 4. `src/admin/types.rs` 新增 `DebugLogsResponse` / `SetDebugLogsRequest`；
      `src/admin/handlers.rs` 新增 get/set 与 `persist_debug_logs`，含回写单测
      （验证：`cargo test persist_debug_logs`）
- [x] 5. `src/admin/router.rs` 注册 `/config/debug-logs` GET/PUT（验证：`cargo check`）
- [x] 6. `web-ui/admin-ui`��增 API 函数与 React Query hooks，设置页「调试」分区接入 `Sw` 开关
      （验证：`npx tsc --noEmit`）
- [x] 7. `web-ui/admin-ui/src/i18n/locales/{zh,en}.json` 新增 `settings.debugLogs` /
      `settings.debugLogsDesc`（验证：设置页中英切换文案正确）
- [x] 8. `README.md` / `README.en.md` 配置表与设置页表新增 `debugLogs` 行
      并说明「开关状态 vs RUST_LOG 实际生效档位」的关系（验证：表格字段名与实现一致）

## 验收标准
- [ ] 未设置 RUST_LOG 且字段缺失或 false 时启动，日志页无 DEBUG 记录，行为与变更前一致
- [ ] 打开开关后无需重启即可在日志页看到新的 DEBUG 记录，config.json 中 `debugLogs` 为 true
- [ ] 关闭开关后不再产生新的 DEBUG 记录，已有记录保留
- [ ] 重启服务后开关状态从 config.json 恢复
- [ ] 控制台与日志文件（配置 `logFile` 时）随开关同步变化
- [ ] `cargo fmt` / `cargo clippy` / `cargo test` / `npx tsc --noEmit` 通过
