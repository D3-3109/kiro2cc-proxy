# 变更提案：enable-thinking-display

## 背景

Claude Code 经本代理使用时，即使客户端开启深度思考（请求携带 `thinking` 配置），UI 上也从不出现"Thinking for Ns…"灰色思考文案。

代码链路实证（三个根因）：

1. **Claude 模型 + `thinking_type == "enabled"`（CC 常规请求路径）**：converter 只在 history[0] 注入 `<thinking_mode>enabled</thinking_mode>` 文本标签（`converter/thinking.rs::generate_thinking_prefix`），从不注入原生 `additionalModelRequestFields.thinking`（`converter/fields.rs` 明确注释"不发 thinking 字段"）。实测上游不因文本标签产出可解析的 thinking 内容，响应侧无 `reasoningContentEvent` 也无 `<thinking>` 标签 → `stream/context/mod.rs::process_assistant_response` 走纯 text 分支 → 无 thinking_delta → CC 无思考文案。
2. **Claude 模型 + `thinking_type == "adaptive"`（模型名带 thinking 后缀时）**：`provider/errors.rs::rewrite_request_body` 的原生注入条件要求 `credentials.thinking_adaptive == true`（账号级开关）。该开关默认 False，且即使开启（用户已实测）上游仍未返回 reasoning 事件，说明仅注入 `{"type":"adaptive"}` 不足以让上游产出推理内容。
3. **GPT 系模型**：`gpt_anti_pseudo_tag_hint` 仅注入"反伪标签"提示语阻止模型自造标签，无任何 thinking 转发通路（luna 强制关闭，其余 GPT 系无协议支持）。

> 行为前提（用户可见）：`thinkingAdaptive` 账号开关语义保持不变（默认 False，需在 Admin UI 逐账号开启后功能才生效）。开关关闭时所有请求与变更前逐字节一致（零回归底线）。变更核心：开关开启后，CC 常规 enabled thinking 请求（旧逻辑下不注入）也能触发上游推理产出。

目标：让客户端开启深度思考时，Kiro 上游实际产出推理内容，并以 Anthropic SSE `thinking_delta` 流式转发给 CC，使其渲染灰色思考文案。

## 目标范围

**在范围内：**
- Claude 系模型：客户端请求 `thinking`（enabled 或 adaptive）时，向上游注入原生 thinking 配置，使上游产出 `reasoningContentEvent`，复用现有 `process_native_reasoning` 通路转发 thinking_delta。
- 注入条件与账号级 `thinkingAdaptive` 开关的关系调整：客户端请求为权威信号，账号开关降级为"强制关闭"豁免（保持对不想产生 TTFB 代价的账号的退出能力）。
- GPT 系模型：沿用现有反伪标签引导语路径，不新增原生协议注入（上游无对应字段，超出本变更范围）。
- 4.5 代际维持跳过（`additional_fields_skipped` 现有约束不变）。
- 协议层变更配套单元测试（内联 `#[cfg(test)]`）。

**不在范围内：**
- GPT 系模型的推理内容产出与转发（上游无原生协议支撑）。
- thinking 预算/budget_tokens 的精确映射调优。
- prompt cache history[0] 冻结机制的改动（本次注入发生在 `additionalModelRequestFields`，不触碰 history[0]）。
- luna 模型（维持强制关闭）。

## 技术方案

1. **`src/anthropic/converter/fields.rs::build_additional_model_request_fields`**：Claude 系（非 4.5 代际）在请求 `thinking.is_enabled()` 且账号 `thinkingAdaptive == true` 时，追加 `thinking: {"type": "adaptive"}`（Kiro 私有协议仅接受 adaptive 型）；与现有 `output_config.effort`、`max_tokens` 合并输出。删除"不发 thinking 字段"的旧注释，替换为新的行为说明。注入矩阵单元测试落在现有测试文件 `src/anthropic/converter/tests/fields.rs`（若该测试文件组织方式不允许，则内联 `#[cfg(test)]`）。
   - **开关语义保持不变**：`thinkingAdaptive` 维持旧语义（`true` 才注入），不做"豁免剥离"反转。但注入条件从旧的"仅 adaptive 请求"扩展为"enabled 或 adaptive 请求均注入"（这是本变更的核心：CC 常规请求走 enabled，旧逻辑下永远不注入）。
2. **`src/kiro/provider/errors.rs::rewrite_request_body`**：注入判定上移至 converter 后，该函数的注入分支改为仅当请求体尚未含 `additionalModelRequestFields.thinking` 且账号开关开启时补充注入（保持故障转移后按实际选中账号重判的能力）；模型类型判定（GPT/4.5）已在 converter 层完成，provider 层不重复。`thinking_adaptive_requested` 透传链保留并加注释说明新语义（enabled/adaptive 均视为已请求 thinking）。
3. **响应侧零改动**：`process_native_reasoning` / `process_content_with_thinking` 已具备完整转发能力，无需修改。
4. **真机验证**：实施完成后在 CC 中实测（账号开关开启 + 客户端开启深度思考），确认灰色思考文案可见；不达标则回到调研阶段分析上游真实响应。
5. **测试**：fields.rs 注入矩阵单元测试（Claude enabled/adaptive/无 thinking × 代际 × GPT × luna × 开关开/关）；errors.rs 补充注入逻辑测试。

## 预期影响

- 正向：CC 中可见灰色思考文案；thinking 内容计入现有 usage 统计链路（count_token_chars 已在 process_native_reasoning 中处理）。
- 风险 1：TTFB 增加 —— 注入原生 thinking 字段可能触发 Kiro 后端推理调度路径。缓解：保留账号开关强制关闭豁免。
- 风险 2：部分代际对 thinking 字段返回 400 —— 现有多账号故障转移与 429/400 重试机制兜底；4.5 代际已跳过。
- 风险 3：thinking 内容与正文内容重复（模型把推理复述进正文）—— 现有 `native_thinking_seen` 分支已处理两种来源互斥。
- 兼容性：未开启 thinking 的请求完全不受影响（注入条件以请求携带 thinking 为前提）。

## 风险

- Kiro 私有协议对 `thinking` 字段的具体 schema（type 取值、是否需要 budget 等字段）仅能凭 `rewrite_request_body` 现有实现推断，存在上游拒绝的可能；需真机验证后调整。
- 上游对 `{"type":"adaptive"}` 的实际响应行为未经抓包确认，若上游仍不产出 reasoningContentEvent，需要退回文本标签+上游调度的组合实验（记为 design.md 待决问题）。
