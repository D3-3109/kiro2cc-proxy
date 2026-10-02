# 技术方案：enable-thinking-display

## 上下文

Kiro 私有协议中 thinking 的唯一已知可接受结构是 `additionalModelRequestFields.thinking = {"type":"adaptive"}`（来自 `provider/errors.rs::rewrite_request_body` 现有实现与 Kiro CLI 行为）。响应侧通路（`reasoningContentEvent` → `process_native_reasoning` → thinking_delta）已完整存在且无需改动。

## 目标 / 非目标

**目标：**
- 客户端开启深度思考 → 上游产出推理内容 → CC 渲染思考文案，全链路打通。

**非目标：**
- GPT 系推理产出（无上游协议支撑，维持反伪标签引导语现状）。
- history[0] 冻结机制、prompt cache、usage 统计链路的改动。

## 决策

### 决策 1：注入点上移至 converter（fields.rs），provider 层保留账号开关判定

**现状有两条注入路径：**
- 路径 A（`converter/fields.rs`）：构建期生成 `additionalModelRequestFields`，当前不发 thinking。
- 路径 B（`provider/errors.rs::rewrite_request_body`）：发送前按 `thinking_adaptive_requested && credentials.thinking_adaptive` 合并注入。

**选型：注入决策（按请求/模型）上移到路径 A（converter 层），路径 B 保留但职责收窄为账号开关判定。**

理由：
1. converter 层已持有完整请求上下文（thinking 配置、模型代际、luna/GPT 判定），是模型侧注入决策的自然位置；provider 层仅有凭据与布尔标记。
2. 职责清晰分层：converter 决定"请求想要什么"，provider 决定"该账号允不允许"（故障转移后按新账号重判，现有 spec 场景延续）。
3. `thinking_adaptive_requested` 透传链保留，语义扩展为"客户端请求 thinking（enabled 或 adaptive）"。

### 决策 2：注入值固定 `{"type": "adaptive"}`

Kiro 私有协议仅证实接受 adaptive 型（现有代码唯一实证）。文本标签协议的 `enabled` 型不适用于 `additionalModelRequestFields`。即使客户端请求 `enabled`，注入值也统一为 adaptive——客户端请求的语义是"要思考"，上游协议形态由本代理决定。

### 决策 3：账号开关语义保持不变（仅扩展触发面）

经用户裁决（步骤 4.6 核验超出项），**不反转** `thinkingAdaptive` 开关语义：
- 旧语义保留：`thinkingAdaptive == true` 才保留 thinking 字段（默认 False → 默认不注入，零回归底线）。
- 变更点仅在**触发面扩展**：旧逻辑仅 `thinking_type == "adaptive"` 请求触发注入（CC 常规请求走 enabled，永远命中不了）；新逻辑 enabled 与 adaptive 均触发。
- 用户启用路径：Admin UI 逐账号开启开关（行为与既有 spec `kiro-thinking-adaptive` 一致）。

### 决策 4：豁免判定保留在 rewrite_request_body（发送前最后一步）

converter 层无凭据信息，账号开关（`thinkingAdaptive`）判定只能在实际选中账号后执行——即 `rewrite_request_body`（profileArn 改写管线中，持有当前凭据）。职责划分：
- converter 层（fields.rs）：按请求 thinking 配置 + 模型类型决定"是否请求 thinking 注入"。
- provider 层（errors.rs）：按实际选中账号的开关决定"该账号是否保留注入"（故障转移后按新账号重判）。
- `thinking_adaptive_requested` 标记语义扩展为"客户端请求 thinking（enabled 或 adaptive）"，透传链保留不清理（经用户裁决剔除清理项）。

## 风险 / 权衡

- **TTFB 增加**：adaptive 注入可能触发上游推理调度路径（代码注释曾警告）。权衡：保留账号级豁免，用户可按账号关闭。
- **上游 schema 未知面**：若 `thinking` 字段还需要 budget 等子字段，注入会被 400 拒绝；现有 9 次重试/多账号故障转移兜底，失败表现为请求失败而非静默错误，可观测。
- **上游仍不产出 reasoningContentEvent 的可能性**：用户实测开关开启时仍无文案，说明可能存在其他缺失条件（如 effort 联动、chatTriggerType 约束）。若真机验证失败，后续变更需抓包 Kiro CLI 对照——本变更不阻塞在该未知项上。

## 待决问题

（无阻塞项。上游真实响应行为需真机验证，验证失败时的备选方向已记录在风险段，不改变本变更的任务拆分。）

## 迁移方案

不需要数据迁移。配置向后兼容：`thinkingAdaptive` 字段语义变化（False 从"默认不注入"变为"主动剥离"），对未开启 thinking 的客户端请求无可观察差异。
