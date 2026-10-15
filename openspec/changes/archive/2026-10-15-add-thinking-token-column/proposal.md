# 变更提案：add-thinking-token-column

## 背景

请求日志的 Token 用量列目前展示输入/输出/cache read 三个数字，但「输出」值无法体现深度思考的开销构成：
- **流式路径**：入库 output 为计费口径（`billed_output_tokens`，**含 thinking**），但 thinking 占比无法从日志拆分看到
- **非流式路径**：入库 output 为 `estimate_output_tokens`（text+tool_use，**不含 thinking**），思考开销完全漏算
- 两条路径入库口径本就不一致的既有事实，本变更不改变（仅新增观测字段）

管理员无法从日志判断某条请求的思考开销（credits 消耗与耗时的重要构成）。本变更在 Token 用量列追加一个 🧠 徽章展示 thinking token 数，样式遵循 admin-ui 主题（参考截图：`🧠 197` 形式，位于输入/输出之后，语义色展示）。

## 目标范围

**在范围内：**
- 流式路径：StreamContext 新增 thinking 字符独立统计（thinking 块内 delta 内容），落库为新字段 `thinking_token_count`
- 非流式路径：从组装后的 content 块遍历 `type == "thinking"` 块统计 token
- `UsageRecord`/`UsageRecordItem` 新增 `thinking_token_count: Option<i32>` 字段（skip_serializing_if 兼容旧数据），record() 签名扩展，全调用点透传
- admin-ui Token 用量列追加 🧠 徽章（仅 thinking_token_count > 0 时展示），i18n 中英文案，类型定义
- user-ui 请求日志页同步追加同款徽章（复用同字段，样式遵循 user-ui 主题，经用户确认纳入）
- 单元测试：流式统计口径、非流式统计、serde 兼容、透传断言

**不在范围内：**
- 历史记录回填（旧记录无此字段，展示为无徽章）
- 请求日志之外的页面（汇总卡、每日明细沿用现有表组件自动获得）
- 客户端 SSE 协议中 output_tokens 口径的任何变化（对外上报值不变，本字段仅入库存观测）

## 技术方案

**流式口径（零埋点推导 vs 独立计数，选独立计数）：**
- 现有 `output_chars_*`（计费口径，含 thinking）与 `visible_chars_*`（上报口径，不含）理论上可差值推导，但差值混入 echo_guard 剥离、web_search 截获输入（`output_chars_other` 计入而可见性块不计 visible）、空格兜底文本等噪声，推导值不可信
- 方案：新增 `thinking_chars_cn`/`thinking_chars_other` 两个计数器，在两个「客户端实际收到 thinking 内容」的发射点累计：
  1. `native.rs::process_native_reasoning` 的 thinking_delta 分支——native reasoning 主路径（state_manager 接受 delta 时）
  2. `mod.rs::create_thinking_delta_event` 非空内容路径——`<thinking>` 标签解析与流结束 flush 兜底路径
- 落库值：`tokens_from_chars(thinking_chars_cn, thinking_chars_other)`，与 output_tokens 同口径换算

**非流式口径：**
- 在 `drop_thinking_blocks` 剔除**之后**、`estimate_output_tokens` 同位置遍历 content，统计 `type == "thinking"` 块的 `thinking` 字段（`count_tokens`，与 `estimate_output_tokens` 同函数族）
- 被剔除（账号关闭 thinking）的内容**不计**——与流式「客户端实际收到」口径对齐（被剔除内容客户端不可见）

**前端展示：**
- `DurationCell` 同级的 Token 用量列内追加徽章：`🧠`（lucide `Brain` 图标）+ 数字，语义色沿用现有 `text-warn`（与截图橙色一致），数值千分位格式化与现有 token 数字同源，>0 才渲染，旧数据/无思考自然隐藏
- user-ui 请求日志页同步追加同款徽章（复用 `thinkingTokenCount` 字段，样式遵循 user-ui 主题）

## 预期影响

- 对外协议（/v1/messages、/cc/v1/messages、OpenAI 兼容层）零变化：thinking 统计仅写入访问日志
- record() 签名扩至 13 参，全部调用点（流式/非流式/测试）同步；旧持久化 JSON 缺字段反序列化为 None（serde 兼容）
- 性能：两次 i64 加法/ thinking delta，可忽略

## 风险

- thinking 内容产出点分散（native/标签/flush 三路径）→ 收敛到双发射点统计规避；以单测穷举锁定
- `create_thinking_delta_event` 需从 `&self` 改 `&mut self`（内部累计计数器）：8 处调用点无冲突；mod.rs:829 一处参数借用 `self.thinking_buffer`，改用 `std::mem::take` 后调用（该分支本就 clear，语义无损）
- record() 13 参持续膨胀 → 已知技术债（cr-known-issues 有案），本次沿用位置参数不做 struct 重构（超出最小变更范围）
- user-ui 已纳入本期范围（任务 4b，复用同字段，样式遵循 user-ui 主题）
