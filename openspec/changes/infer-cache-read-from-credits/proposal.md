# 变更提案：infer-cache-read-from-credits

## 背景
Kiro 上游 `meteringEvent` 仅返回总积分消耗 `usage`（即 `creditsUsed`），未透传 `cache_read_input_tokens` 与 `cache_creation_input_tokens`。当前代理通过 `estimate_prefix_tokens`（结合 `prefix_cached` 首轮/漂移门控）将除最后一条消息外的全部历史静态估算为 `cache_read_input_tokens`。但当上游发生缓存 TTL 过期、跨实例路由全量未命中、或长对话中途历史轮次部分失效（如仅 `history[0]` 命中而对话历史未命中）时，静态前缀估算仍报告 95%~99% 命中率，导致 Admin UI 缓存命中率胶囊、节省积分统计以及响应末尾上报的 `cache_read_input_tokens` 与真实 `creditsUsed` 严重背离。

通过对 Haiku 5.5 本地实测数据的回归与差分验证，采用缓存读取按未命中单价的 10% 计费（节省 90%，即 `d = 0.90`）的反推模型。结合静态前缀上界，在流结束结算与非流式响应中估算缓存读取量；其余 Claude 的派生费率未经本次实测标定，不保证精确还原。

## 目标范围
**在范围内：**
- 在 `src/model/usage/pricing.rs`（或 `src/cache/`）提供面向全部 Claude 模型（Haiku 5.5 双档实测费率，Haiku 4.5 / Sonnet / Opus 按 `get_model_pricing × get_k_ref` 派生费率）的 `creditsUsed` 缓存读取反推函数 `infer_cache_read_from_credits` 与混合校准函数 `refine_prefix_cache_read`。
- 采用「上界约束混合策略」：当 `prefix_cached == false`（`prefix_estimated_tokens == 0`）时锁定为 `0`；当 `prefix_cached == true` 且 `final_input_tokens >= 4096` 且收到有效 `creditsUsed` 时，取 `min(prefix_estimated_tokens, inferred_read)`；其余情况（`< 4096`、无 `creditsUsed`、非 Claude 模型）保留 `prefix_estimated_tokens` 原值。
- 在流式结算（`StreamContext::generate_final_events_raw`：仅作用于 `UsageTracker` 落库，末尾 `message_delta` 保持静态口径）及非流式响应结算（`handle_non_stream_request`：作用于 `UsageTracker` 落库与 JSON 响应 `usage`）中接入该混合校准值。
- 保持流式 `message_start` 事件使用静态估算值（此时 `meteringEvent` 尚未到达），并保持 `promptCacheAccounting`（断���模拟记账）开启时的优先级高于前缀估算与反推。
- 为全模型费率矩阵、边界阈值（`< 4096`、`> 100K`）、上界裁剪、流式与非流式结算路径补充完整单元测试。

**不在范围内：**
- 非 Claude 模型（`gpt-*`、`qwen*`、`glm*`、`deepseek*`、`minimax*`、`kimi*`）不启用 `creditsUsed` 反推，保持现有静态前缀估算。
- 不修改前端（`web-ui/admin-ui`、`web-ui/user-ui`）代码与接口字段结构。
- 不改变流式 `message_start` 早期事件的生成逻辑。

## 技术方案
- **反推数学模型**：
  设总输入 token 为 $I = \text{final\_input\_tokens}$，计费输出 token（含 thinking）为 $O_{\text{billed}}$，模型未命中输入单价为 $r_{\text{miss}}$（credits/MTok），输出单价为 $r_{\text{out}}$（credits/MTok），缓存读取按 $10\%$ 计费（节省比例 $d = 0.90$）：
  $$\hat{R} = \operatorname{clamp}\!\left(\operatorname{round}\!\left(\frac{r_{\text{miss}} \cdot I + r_{\text{out}} \cdot O_{\text{billed}} - \text{creditsUsed} \times 10^6}{0.90 \cdot r_{\text{miss}}}\right),\; 0,\; I\right)$$
- **Claude 模型费率矩阵**：
  - `claude-haiku-5-5`（`is_haiku_55`）：
    - `I <= 100_000`：$r_{\text{miss}} = 1.6978$, $r_{\text{out}} = 19.1873$
    - `I > 100_000`：$r_{\text{miss}} = 6.7830$, $r_{\text{out}} = 93.9200$
  - 其余 Claude 模型（模型名小写含 `claude` / `sonnet` / `opus` / `haiku` / `fable`）：
    - 由 `get_model_pricing(model)` 与 `get_k_ref(model)` 派生：$r_{\text{miss}} = \text{input\_per\_mtok} \times k_{\text{ref}}$, $r_{\text{out}} = \text{output\_per\_mtok} \times k_{\text{ref}}$。
- **混合校准门控（`refine_prefix_cache_read`）**：
  - 若 `prefix_estimated <= 0`，直接返回 `0`（首轮或 `history[0]` 重置门控）。
  - 若 `final_input_tokens < 4096`、模型非 Claude 系列，或 `credits_used` 缺失、非有限值、非正数，返回 `prefix_estimated.min(final_input_tokens)`。
  - 若反推得到 `Some(inferred)`，返回 `prefix_estimated.min(inferred).min(final_input_tokens)`。

## 预期影响
- **兼容端点**：`/v1/chat/completions` 的 `usage.prompt_tokens_details.cached_tokens` 与 `/v1/responses` 的 `usage.input_tokens_details.cached_tokens` 直接继承 Anthropic `cache_read_input_tokens`；非流式继承校准值，流式继承静态 `message_delta` 值，OpenAI 层不额外缩放。Chat 流式仅在 `stream_options.include_usage=true` 时输出用量帧。
- **可观测性提升**：Admin UI 使用日志中的「缓存读取」、缓存命中率胶囊与「省 X.XXXX」积分采用混合校准结果。在费率与 token 估算准确的前提下，可下修静态前缀高估；非 Haiku 5.5 费率尚未标定，不能保证结果代表真实缓存命中。
- **协议与性能**：反推函数使用纯内存浮点运算，不新增外部 crate；本次未测量单请求耗时，不作具体延迟承诺。

## 风险
- **多轮口径不匹配**：成功接入 web_search 续请求后，累计输出与末轮积分不同域，因此停用 credits 反推，退回静态前缀；metering 缓存真值、断点账本及原有积分落库优先级不变。
- **短请求输出分词偏差放大**：当输入极小而输出较大时，本地输出 token 估算误差除以较小的 $0.90 \cdot r_{\text{miss}}$ 会产生较大波动。**应对**：强制 `final_input_tokens >= 4096` 门控阈值，低于阈值直接回退静态前缀估算。
- **输出 token 估算负偏差导致反推偏高**：当实际输出费低于估算值时，反推 $\hat{R}$ 可能略高于实际缓存前缀。**应对**：采用 `min(prefix_estimated_tokens, inferred_read)` 上界约束，确保绝不超过物理可缓存前缀上限。
