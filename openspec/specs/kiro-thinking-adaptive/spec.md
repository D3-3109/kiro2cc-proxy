# Spec: 账号级 thinking adaptive 注入（kiro-thinking-adaptive）

## Purpose

提供账号粒度的 Kiro thinking 调度开关：管理员为某账号开启后，客户端请求携带 `thinking: {"type": "adaptive"}` 且路由到该账号时，代理向 Kiro 上游 `additionalModelRequestFields` 注入 `thinking: {"type": "adaptive"}`，恢复该账号的 thinking 能力；其余一切情况（开关关闭、客户端未请求 adaptive、模型不支持）与 v3.3.0 以来"不发 thinking 字段"的现状逐字节一致。

## Requirements

### Requirement: 账号级 thinking adaptive 开关持久化

系统 SHALL 在 `KiroCredentials` 上持久化账号级开关 `thinkingAdaptive`（bool，serde default=false），经 Admin API `PUT /credentials/{id}` 的 `thinkingAdaptive` 字段（`Option<bool>`，未提供时不更新）变更，并随账号快照（`CredentialEntrySnapshot`）透出给前端。

#### Scenario: 旧配置文件向后兼容

- **GIVEN** credentials.json 由旧版本生成，某账号对象不含 `thinkingAdaptive` 字段
- **WHEN** 服务加载该 credentials.json
- **THEN** 该账号的 `thinking_adaptive` 反序列化为 `false`
- **AND** credentials.json 中不因加载而出现序列化噪音（bool 字段照常输出，与 `disabled` 字段行为一致）

#### Scenario: Admin API 部分更新开关

- **GIVEN** 存在账号 #N，当前 `thinkingAdaptive = false`
- **WHEN** Admin 调用 `PUT /credentials/N`，body 携带 `{"thinkingAdaptive": true}`
- **THEN** 账号 #N 的 `thinking_adaptive` 更新为 `true` 并持久化
- **AND** 同请求未提供的其他字段（refresh_token、proxy_url 等）不被修改
- **AND** 快照接口返回该账号 `thinkingAdaptive: true`

#### Scenario: 未提供该字段时不更新

- **WHEN** Admin 调用 `PUT /credentials/N`，body 不含 `thinkingAdaptive`
- **THEN** 账号 #N 的 `thinking_adaptive` 保持原值不变

### Requirement: 仅在开关开启且客户端请求 adaptive 时注入 thinking 字段

Provider 层 SHALL 在确定账号（`CallContext`）后，对请求体执行账号开关判定：账号 `thinkingAdaptive == true` 且请求体含 `additionalModelRequestFields.thinking`（converter 按 enabled/adaptive 注入）时 SHALL 保留该字段；账号 `thinkingAdaptive == false` 时 SHALL 剥离该字段。`thinking_adaptive_requested` 标记的语义扩展为"客户端请求 thinking（enabled 或 adaptive）"，透传链保留。模型侧 GPT 系 / 4.5 代际排除条件已上移至 converter 注入层，provider 层 SHALL NOT 重复判定模型类型。

#### Scenario: 开关开启 + 客户端 enabled thinking → 注入保留

- **GIVEN** 账号 A 的 `thinkingAdaptive = true`，客户端请求携带 `thinking: {"type": "enabled"}`，模型为 claude-sonnet-4.6
- **WHEN** 请求路由到账号 A 并发往 Kiro 上游
- **THEN** request body 的 `additionalModelRequestFields` 含 `"thinking": {"type": "adaptive"}`
- **AND** 其余字段（output_config、max_tokens）保持既有构建逻辑不变

#### Scenario: 开关关闭 → 剥离（现状不变）

- **GIVEN** 账号 B 的 `thinkingAdaptive = false`，客户端请求携带 thinking（enabled 或 adaptive）
- **WHEN** 请求路由到账号 B
- **THEN** request body 不含 `additionalModelRequestFields.thinking`
- **AND** body 与 v3.3.0 行为逐字节一致

#### Scenario: 故障转移后按新账号重新判定

- **GIVEN** 账号 A（开关开）与账号 B（开关关）均可用，客户端请求携带 thinking
- **WHEN** 首选账号 A 请求失败，故障转移到账号 B 重试
- **THEN** 发往账号 B 的 request body 不含 thinking（按账号 B 的开关值判定）
- **AND** 每次重试都以当次实际选中的账号状态为准

### Requirement: Claude 系模型 thinking 字段注入
当客户端请求携带 `thinking` 且 `thinking.is_enabled()`（enabled 或 adaptive），且映射后的 Kiro 模型为 Claude 系（非 4.5 代际、非 luna、非 GPT 系）时，系统 SHALL 在 converter 层生成的 `additionalModelRequestFields` 中包含 `thinking` 对象（值为 `{"type": "adaptive"}`，Kiro 私有协议唯一实证接受的形态），并与 `output_config`、`max_tokens` 字段合并共存。

#### Scenario: CC 常规请求携带 enabled thinking
- **WHEN** 客户端向 `/v1/messages` 发送 `thinking: {"type": "enabled", "budget_tokens": 20000}`，模型映射为 Claude 非 4.5 代际
- **THEN** converter 生成的 `additionalModelRequestFields` 包含 `thinking` 字段，且 `output_config.effort` 与 `max_tokens` 仍按现有规则存在

#### Scenario: 客户端未请求 thinking
- **WHEN** 请求不携带 `thinking` 字段
- **THEN** `additionalModelRequestFields` 不含 `thinking` 字段，与变更前逐字节一致

#### Scenario: 4.5 代际跳过
- **WHEN** 模型映射为 4.5 代际 Claude（如 claude-4-5-sonnet）
- **THEN** `additionalModelRequestFields` 整体为 None（维持现状），不含 `thinking` 字段

#### Scenario: GPT 系与 luna 不注入
- **WHEN** 请求携带 thinking 但模型为 GPT 系或 luna
- **THEN** `additionalModelRequestFields` 不含 `thinking` 字段（GPT 系维持 `reasoning.effort` 现状，luna 维持现状）

### Requirement: 上游推理事件转发（现有通路回归约束）
当上游返回 `reasoningContentEvent` 且 `thinking_enabled` 为 true 时，系统 SHALL 通过现有 `process_native_reasoning` 通路产生 Anthropic SSE `thinking_delta` 与 `signature_delta` 事件；本变更 SHALL NOT 修改该通路的行为。

#### Scenario: 思考文案渲染前提
- **WHEN** 上游返回非空 `reasoningContentEvent`
- **THEN** SSE 流包含 `content_block_delta`（`thinking_delta`）事件，CC 据此渲染思考文案

## 接口说明

本变更为 Admin 管理面行为，不影响 `/v1/messages` 与 `/cc/v1/messages` 的对外协议：两端点对客户端的请求/响应格式零变化，差异仅体现在代理 → Kiro 上游的 request body（且仅当账号开关开启）。
