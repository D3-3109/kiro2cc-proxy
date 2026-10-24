# 变更提案：add-debug-log-toggle

## 背景
Admin「查看日志」页当前只显示 INFO 及以上级别：`src/main.rs` 初始化 tracing 时用 `EnvFilter`
默认 `info` 过滤，DEBUG 事件在进入控制台 / 日志文件 / `LogCapture` 层之前就被丢弃。排障时只能
修改 `RUST_LOG` 环境变量并重启服务，无法在运行中临时开启。需要在 Admin 设置页提供 DEBUG 日志
开关，热生效并持久化到 config.json。

## 目标范围
**在范围内：**
- `config.json` 新增 `debugLogs` 布尔字段（默认 `false`），支持环境变量 `DEBUG_LOGS` 覆盖
- 启动时按该字段决定是否把全局日志过滤级别放开到 DEBUG
- Admin API 新增 `GET/PUT /api/admin/config/debug-logs`，PUT 热切换 + 原子写回 config.json
- Admin 设置页「调试」分区新增开关行（中英双语文案）
- 影响范围覆盖控制台、日志文件（`logFile`）与 Admin 日志页三处（同一全局过滤器）
- 关闭开关只停止捕获新的 DEBUG，已进入 ring buffer 的 DEBUG 记录保留

**不在范围内：**
- 不新增 TRACE 级别开关
- 不改变前端日志页现有的级别筛选 / 折叠 / 下载逻辑
- 不调整 ring buffer 容量与 SSE 协议

## 技术方案
1. tracing 初始化改为「单一全局可重载过滤器」：用 `reload::Layer::new(EnvFilter)` 生成
   `(filter, handle)`，把 filter 直接 `registry.with(filter)` 作为全局过滤器；控制台 / 日志文件 /
   `LogCapture` 三个 layer 不再各自 `with_filter`。
2. `EnvFilter` ��建规则：优先 `RUST_LOG`，缺省 `info`；开关开启时追加全局 `debug` 指令（覆盖全局
   默认档，保留 `RUST_LOG` 中的 target 定向指令）。`reload::Handle::modify` 内部调用
   `callsite::rebuild_interest_cache()`，刷新全局 max level 与 callsite interest，热切换即时生效。
3. 新增 `src/logging.rs`：持有 `OnceLock<Handle>` 与 `AtomicBool` 当前开关值，提供
   `build_filter` / `install` / `set_debug_enabled` / `debug_logs_enabled`。
4. Admin handler 复用现有 `persist_config_field` + `persist_lock` 模式（与 `thinkingAsText` /
   `dumpKiroBody` 一致），先热更新再持久化，失败返回「持久化失败，但运行时已生效」。
5. 前端复用现有 `useXxx` / `useSetXxx` React Query 模式与 `Sw` 开关组件，设置项置于「调试」分区。

## 预期影响
- 默认 `debugLogs=false`，行为与现在完全一致（仍为 info 级过滤）。
- 开启后控制台、日志文件与 Admin 日志页同时出现 DEBUG 记录，日志量显著增加。
- 关闭后新事件恢复 info 级过滤，缓冲区内已有 DEBUG 保留至自然淘汰或被手动清空。
- config.json 向后兼容：字段缺失按 false 处理。

## 风险
- **日志洪峰**：DEBUG 全开会显著增加日志量，加快 ring buffer 轮转并增加 SSE 推送压力；通过默认
  关闭 + 设置页说明缓解，ring buffer 容量保持不变。
- **RUST_LOG 语义冲突**：开关只追加全局 `debug` 指令，`RUST_LOG` 中更具体的 target 定向指令
  仍优先；GET 接口返回的是开关状态而非实际生效档位，需在 README 说明。
- **reload 句柄未安装**：其他入口复用 `set_debug_enabled` 时句柄缺失，须返回明确错误而非静默失败。
