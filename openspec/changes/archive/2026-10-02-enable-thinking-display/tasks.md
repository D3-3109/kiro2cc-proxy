# 任务清单：enable-thinking-display

## 状态：ARCHIVED

## 任务

- [x] 1. `src/anthropic/converter/fields.rs`：Claude 系（非 4.5 代际）请求 `thinking.is_enabled()`（enabled 或 adaptive）且账号 `thinkingAdaptive == true` 时在 `additionalModelRequestFields` 注入 `thinking: {"type":"adaptive"}`，与 output_config.effort / max_tokens 合并；更新旧注释；注入矩阵单元测试落点为 `src/anthropic/converter/tests/fields.rs`（若测试组织不适用则内联 `#[cfg(test)]`）。验证：`cargo test additional_model_request_fields` + `cargo check`。
  - 说明：converter 层不持有账号凭据，账号开关判定按 design.md 决策 4 由 provider 层补充执行；本任务先按"请求携带 thinking 即注入"实现，开关判定在任务 2 联动。
- [x] 2. `src/kiro/provider/errors.rs::rewrite_request_body`：保留账号开关判定（`thinkingAdaptive == true` 才保留 thinking 字段，开关关闭时剥离/不补充注入，故障转移后按实际选中账号重判）；模型类型判定不重复；`thinking_adaptive_requested` 语义扩展为"enabled 或 adaptive 均视为已请求 thinking"并加注释（透传链保留不清理）。验证：`cargo test rewrite_request_body`。
- [x] 3. 全量回归：`cargo fmt` + `cargo clippy` + `cargo test` 全部通过；验证"账号开关默认 False + 请求 thinking"时请求体与变更前逐字节一致（零回归底线，用现有 converter 测试验证）。
- [x] 4. 真机验证：结论为**协议限制**（2026-10-02 裁决记录）。mitmdump 抓包官方 kiro-cli 2.22.0（含 MITM 注入伪 Claude 模型条目绕过前端校验，产物 /tmp/kiro_claude_real.json）证实：官方客户端对 GPT 系与 Claude 系模型发出的 GenerateAssistantResponse 请求体均**不含** additionalModelRequestFields/thinking/reasoning/output_config，无伴随字段可补；`thinking:{"type":"adaptive"}` 系服务端 schema 的合法实验位，上游仅产出 1~3 个短 reasoningContentEvent 后即转正文。保留现有注入实现（账号开关可控），灰色思考文案可见性受上游能力边界限制，非本代理缺陷。

## 验收标准

- [ ] 客户端请求携带 `thinking`（enabled 或 adaptive）且账号开关为 true 时，发往 Kiro 的请求体 `additionalModelRequestFields` 含 `thinking` 字段。
- [ ] 账号 `thinkingAdaptive == false`（含默认缺失）时，无论请求如何配置，请求体不含 `thinking` 字段（与变更前逐字节一致）。
- [ ] 零回归底线：存量账号（开关默认 False）+ 任意请求（含携带 thinking）→ 请求体与变更前逐字节一致。
- [ ] 上游返回 `reasoningContentEvent` 时，SSE 流中出现 `thinking_delta` / `signature_delta` 事件（现有通路，回归验证）。
- [ ] GPT 系、4.5 代际、luna 模型行为与变更前一致。
- [x] 真机验证：**协议限制达成共识**——注入链路实现正确（请求体含 thinking 字段），灰色思考文案可见性受上游 reasoningContentEvent 产出极短的边界限制，官方客户端形态亦不含该字段，视为上游能力边界而非实现缺陷。
- [ ] `cargo fmt` + `cargo clippy` clean，全量测试通过。
