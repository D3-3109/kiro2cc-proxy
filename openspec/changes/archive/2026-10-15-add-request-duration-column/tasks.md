# 任务清单：add-request-duration-column

状态：COMPLETED

## 任务

- [x] 1. `src/model/usage/mod.rs`：`UsageRecord` 新增 `first_token_ms` / `duration_ms`（`Option<u64>`，camelCase `firstTokenMs`/`durationMs`，`skip_serializing_if = "Option::is_none"`），`record()` 增加 `first_token_ms: Option<u64>` / `duration_ms: Option<u64>` 两参数；单测覆盖新字段序列化与旧数据反序列化兼容（cargo test usage）
- [x] 2. `src/anthropic/handlers/nonstream.rs`：handler 进入处新增 `Instant::now()` 起点，record 调用点计算 `duration_ms`（首字传 None）；cargo check + 既有测试通过
- [x] 3. 流式路径（`src/anthropic/handlers/stream.rs` + `src/anthropic/stream/context/mod.rs`）：unfold 状态携带计时状态（起点已有 `stream_started_at`），在首个上游业务数据事件处记录首字时刻，最终 record 调用点计算 `first_token_ms` / `duration_ms`；附单测覆盖「keepalive 不计首字」「首个数据帧计首字」（cargo test stream）
- [x] 4. 后端集成自检：`cargo fmt` + `cargo clippy` clean、`cargo test` 全量通过
- [x] 5. 前端类型与表格：`web-ui/admin-ui/src/types/api` 的 `UsageRecord` 增加可选 `firstTokenMs`/`durationMs`；`usage-log-table.tsx` 新增「用时」列与双徽章渲染。i18n key 清单：列头 `common.colDuration` + 徽章 `common.firstTokenLabel`（首字）/ `common.durationLabel`（耗时），zh/en 同步；快慢分档阈值固化（首字 ≤1s 快 / ≥5s 慢，耗时 ≤10s 快 / ≥60s 慢，常量集中定义）；非流式不渲染首字胶囊、旧数据两徽章均「—」占位保证列宽稳定；`pnpm build` 通过
- [x] 6. 前端单测：徽章格式化（ms → X.Xs）、None 降级展示逻辑（vitest，遵循仓库现有测试配置；仓库无 vitest 配置则以 `pnpm build` + tsc 通过为准）

## 验收标准

- [x] 新请求落库后日志 API 返回 `firstTokenMs` / `durationMs` 字段，流式首字为首个上游数据事件延迟
- [x] 请求日志表三个页面均出现「用时」列：新流式请求显示「首字 X.Xs」「耗时 XX.Xs」双徽章，非流式仅「耗时」（不渲染首字胶囊），旧数据两徽章均显示「—」占位，保证列宽稳定
- [x] UI 遵循 admin-ui 现有主题 token，无新增自定义颜色
- [x] `cargo fmt` / `cargo clippy` clean，`cargo test` 全量通过
