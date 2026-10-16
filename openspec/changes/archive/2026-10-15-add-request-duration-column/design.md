# 技术方案：add-request-duration-column

## 上下文

`UsageTracker::record()`（src/model/usage/mod.rs:328）当前 10 参数，全仓仅 2 个调用点：流式（src/anthropic/stream/context/mod.rs:898）与非流式（src/anthropic/handlers/nonstream.rs:644）。流式路径已有计时起点 `stream_started_at = Instant::now()`（src/anthropic/handlers/stream.rs:258，unfold 构建前），但 unfold 状态元组未携带首字时刻。请求日志表为 `web-ui/admin-ui/src/components/usage-log-table.tsx`，列结构由 `LOG_COLS` 驱动，被三处页面复用。

## 目标 / 非目标

**目标：** 流式与非流式请求均落库 `duration_ms`；流式额外落库 `first_token_ms`；前端「用时」列双徽章展示，主题内配色。

**非目标：** 耗时聚合统计、历史数据回填、OpenAI 层独立改动（自动继承）、实时日志流页。

## 决策

1. **存储单位毫秒整数（`Option<u64>`）**：落库即取整，前端仅做格式化（÷1000，一位小数）；避免存储浮点。字段命名 `first_token_ms` / `duration_ms`，serde camelCase 对齐现有字段，`skip_serializing_if = "Option::is_none"` 保证旧数据零迁移。
2. **首字时刻埋点位置**：unfold 流内**首个携带业务数据的上游事件**处（`EventStreamDecoder` 解出的非 keepalive 帧），非 SSE 首个下行字节——后者包含 keepalive 干扰，且与 Kiro 上游 TTFT 语义对齐。计时状态放 unfold 状态元组（或 ctx 上的 `Option<Instant>` 字段，取改动最小者），首字只记一次。
3. **总耗时口径**：请求进入 handler（流式 = `stream_started_at`；非流式 = 新增起点）→ record 落库前。不含进程启动等无关阶段；「落库前」而非「SSE 下行完成」，因 record 在收尾路径上本就是最后动作，差值可忽略且口径稳定可测。
4. **非流式起点**：nonstream handler 进入处新增 `Instant::now()`；`first_token_ms` 传 None（非流式无流式语义）。
5. **前端徽章**：沿用 log-viewer-page repeat 徽章原语 `rounded-[4px] border border-transparent px-1 text-[9.5px] font-semibold`，底色走 `bg-surface-3 text-ink-2`（默认档）与 `bg-ok-soft text-ok` / `bg-warn-soft text-warn`（快 / 慢语义分档，阈值：首字 ≤1s 为快、≥5s 为慢；耗时 ≤10s 为快、≥60s 为慢，中间档用默认）。中文「首字 X.Xs / 耗时 XX.Xs」走 i18n key。
6. **API 零改动**：日志接口返回 serde 透传，前端 `UsageRecord` 类型补两个可选字段即可。

## 风险 / 权衡

- unfold 状态元组已含 9 个元素，再加计时状态进一步降低可读性 —— 权衡：改 ctx 字段需跨模块传递生命周期，元组追加是改动最小路径；若实施时发现元组过载严重，可在任务 3 中改用小结构体聚合，不影响行为契约。
- 徽章快慢阈值（1s/5s、10s/60s）为主观分档 —— 作为常量集中定义并注释，后续可调。
- 首字语义依赖上游帧分类正确性 —— 以「解码出首个非 keepalive 业务帧」为判定，与现有事件分类逻辑复用同一分支，单测覆盖。

## 迁移方案

无迁移：新字段全部 Option，旧记录缺字段即显示「—」。

## 待决问题

无 —— 阈值与口径均已在上方定为默认值，用户在步骤 5 确认时如有异议可调整。
