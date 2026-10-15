# 变更提案：add-request-duration-column

## 背景

请求日志表（API Key 详情页 / 每日明细页 / 凭据详情页共用）目前只记录 token、费用与积分，无法看出单条请求「慢不慢」：上游首字延迟（TTFT）与请求总耗时是排障与体验评估的关键指标。需求：为每条使用日志新增一列「用时」，以两个主题风格徽章展示「首字 X.Xs」与「耗时 XX.Xs」。

## 目标范围

**在范围内：**
- 后端 `UsageRecord` 新增可选字段 `firstTokenMs` / `durationMs`（Option，None=旧数据，serde `skip_serializing_if` 保持对旧日志 JSON 的兼容）
- 流式路径（`src/anthropic/handlers/stream.rs` / `stream/context`）：记录请求起点与首个上游数据事件时刻，落库时计算首字延迟与总耗时
- 非流式路径（`src/anthropic/handlers/nonstream.rs`）：记录请求起点，落库时计算总耗时（无首字概念，`firstTokenMs` 为 None）
- `UsageTracker::record()` 签名扩展耗时参数，两个调用点（stream context / nonstream）同步适配
- admin-ui `usage-log-table.tsx` 新增「用时」列：两个圆角胶囊徽章（首字 / 耗时），遵循仓库现有主题 token（bg-surface-3 / ok-soft / warn-soft / ink-3 等），不引入新配色
- 旧数据无对应字段时，该徽章渲染「—」占位（非流式记录不渲染首字徽章胶囊，仅耗时；旧数据两徽章均「—」），保证列宽稳定
- i18n：zh / en 新增列头与徽章文案 key
- 前端类型 `UsageRecord`（`types/api`）同步新增可选字段

**不在范围内：**
- 不改动 OpenAI 兼容层（`src/openai/`），其复用 anthropic 链路自动获得耗时记录
- 不做耗时统计聚合（均值 / P95 等）与任何汇总卡改动
- 不改实时日志流页（log-viewer-page）与导出格式
- 不迁移 / 回填历史日志数据

## 技术方案

- **计时口径**：流式以 `stream_started_at = Instant::now()`（stream.rs 已有）为总耗时起点；首个上游数据事件（unfold 流内首个非 keepalive 帧）为首字时刻，需在 unfold 状态元组或 ctx 上携带计时状态；非流式在 handler 进入处新增 `Instant::now()`。
- **落库**：`record()` 增加 `first_token_ms: Option<u64>`、`duration_ms: Option<u64>` 两参数（有意继续追加至 12 参数、维持既有 `#[allow(clippy::too_many_arguments)]`，不收敛为参数 struct——调用点仅 2 处，最小改动优先）；`UsageRecord` 以毫秒整数存储，前端格式化为 `X.Xs`。
- **API**：admin 日志接口（get_key_usage_records / get_daily_usage_records）返回 serde 自动透传，无显式改动。
- **前端**：`LOG_COLS` 插入「用时」列；单元格内两个 `rounded-[4px] bg-surface-3` 类胶囊（参考 log-viewer-page repeat 徽章），首字延迟按快慢用 ok / warn 语义色分档，耗时同理；None 显示「—」。

## 预期影响

- 数据模型向后兼容：新字段 Option + `skip_serializing_if`，旧记录不落新字段，前端可选读取，旧 UI 不受影响
- `record()` 签名变更影响仅 2 个调用点，全在本次范围内
- 计时逻辑为纯内存 `Instant` 差值，无额外 I/O，性能影响可忽略
- 前端表格多一列，三处复用页面同步生效

## 风险

- **流式首字埋点遗漏**（如 error/压缩帧先到）：以「首个携带业务数据的上游事件」为准，keepalive 与纯控制帧不计首字；实现时以单测覆盖
- **耗时口径歧义**（是否含客户端 SSE 下行全程）：定义为「请求开始 → 落库前」的服务端处理总时长，spec 中固化口径
- **旧数据展示**：None 时显示「—」，不误导为 0s
