# 变更提案：add-client-token-passthrough

## 背景

Issue #44（bug: 上下文长度统计严重偏低）：第三方客户端（如 Pi agent）接入本代理时，客户端显示的上下文占用严重偏低（上游已提示上下文溢出，客户端仅显示 60K 左右）。

根因：`src/anthropic/stream/calib.rs` 的 `CLIENT_TOKEN_DISPLAY_SCALE = 0.6657` 将所有返回给客户端的 `usage.input_tokens / cache_creation_input_tokens / cache_read_input_tokens` 等字段统一乘以约 0.7 的缩放系数。该系数是为 Claude Code 的 200K 窗口 auto-compact 触发时机校准的，但第三方客户端按显示值计算上下文占用，导致统计偏低约 33%。

## 目标范围

**在范围内：**
- `config.json` 新增顶层 bool 字段 `clientTokenPassthrough`（camelCase），默认 `false`
- 默认 `false`：完全维持现有 0.6657 缩放行为（Claude Code 用户体验零变化）
- 设置为 `true`：所有客户端展示 token 字段 1:1 上报真实值
- 支持环境变量 `CLIENT_TOKEN_PASSTHROUGH` 覆盖（与现有 `apply_env_overrides` 模式一致）
- 更新 `config.example.json` 与 `docs/使用指南.md` 配置说明
- 新增单元测试覆盖两种模式

**不在范围内：**
- 不改动内部计费与 usage_tracker 入库口径（始终真实值，已与展示分离）
- 不改动 `output_tokens`（本就不缩放）
- 不改动 `CLIENT_ASSUMED_CONTEXT_WINDOW`（超窗错误文案口径）
- 不为 admin/user UI 增加该字段的图形化配置项

## 技术方案

- `src/model/config.rs`：`Config` 新增 `client_token_passthrough: bool`（`#[serde(default)]`，默认 false）；`Default` 实现同步；`apply_env_overrides` 新增 `CLIENT_TOKEN_PASSTHROUGH` 解析
- `src/anthropic/stream/calib.rs`：新增 `static CLIENT_TOKEN_PASSTHROUGH: AtomicBool`（Relaxed，启动时设置一次）与 `set_client_token_passthrough(bool)`；`scale_for_client` 改为：标志为 true 时原样返回 n，否则走现有 0.6657 缩放；新增纯函数 `scale_for_client_with(n, model, passthrough: bool)` 承载两分支逻辑，`scale_for_client` 委托它（全局标志作参数），保证测试不触碰全局状态、无并行竞争
- `src/main.rs`：config 加载后调用 `set_client_token_passthrough(config.client_token_passthrough)`，并在 true 时输出一条 info 日志
- 测试：`scale_for_client_with` 两种模式的断言（passthrough=true 时 1:1，含 0/负数边界；false 时维持现有断言值）；config 解析测试（缺省 false / 显式 true / env 覆盖）

## 预期影响

- 默认行为零变化（默认 false，现有部署无感）
- 用户设置 `clientTokenPassthrough: true` 后：Claude Code 的 auto-compact 触发时机将提前（显示值变大，按 200K 口径更早到 82%），此为该模式的预期取舍，文档中说明
- 性能影响可忽略（一次 AtomicBool load）

## 风险

- **风险 1**：用户开启 passthrough 后 Claude Code auto-compact 提前触发 → 应对：文档明确该字段面向第三方客户端场景，Claude Code 用户建议保持默认
- **风险 2**：全局标志在测试中被误改导致并行测试不稳定 → 应对：两分支逻辑收口到纯函数 `scale_for_client_with`，测试仅测纯函数，不修改全局标志
