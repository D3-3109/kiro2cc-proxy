## ADDED Requirements

### Requirement: 基于 creditsUsed 与静态前缀上界的缓存读取混合校准
系统 SHALL 在收到 Kiro 上游 `meteringEvent`（经 AWS Event Stream 二进制帧 CRC32 校验解码，`usage` 字段为 `f64` 类型的 `creditsUsed`，且 `cache_read_input_tokens` / `cache_creation_input_tokens` 为 `null`）时，对全部 Claude 模型在 `final_input_tokens >= 4096` 条件下按 `0.90` 缓存节省率反推缓存读取 token 数，并以静态前缀估算 `prefix_estimated_tokens` 作为上界进行混合校准；`/v1/messages` 与 `/cc/v1/messages` 两端点行为一致（仅 `/cc/v1/messages` 额外带 300s 流式全局 deadline），且在多账号 priority / balanced 负载均衡与故障转移（单凭据最多 3 次、单请求最多 9 次重试）下仅基于最终成功响应的 `meteringEvent` 计算。

#### Scenario: Haiku 5.5 常态档（<= 100K）缓存命中与部分失效校准
- **GIVEN** 请求模型为 `claude-haiku-5-5`，`final_input_tokens == 53776`（`<= 100_000`），静态前缀估算 `prefix_estimated_tokens == 53650`（`prefix_cached == true`），计费输出 token（含 thinking）为 `346`
- **WHEN** 上游返回 `meteringEvent.usage == 0.06617`（对应上游仅部分前缀命中），且未透传 `cache_read_input_tokens`
- **THEN** 系统按常态档费率（`r_miss = 1.6978`, `r_out = 19.1873`）反推并取 `min(prefix_estimated_tokens, inferred_read)`，最终结算 `cache_read_input_tokens` 落在 `[20000, 22500]` 区间（约 `38%~41%` 命中率），替代未下修的 `53650`
- **AND** `UsageTracker` 落库记录与非流式 JSON `usage` 采用该校准值；流式 `message_delta` 仍为静态口径，与 `message_start` 一致

#### Scenario: 隐藏 thinking 仍参与非流式计费反推
- **WHEN** 账号关闭 thinking，响应内容中的 thinking 块被剔除
- **THEN** 反推计费输出包含剔除前 thinking token，响应不包含隐藏块，落库 thinking 观测字段仍仅统计存留块
- **AND** `outputTokensIncludeThinking` 的原有展示口径不变

#### Scenario: web_search 多轮桥接回退静态前缀
- **WHEN** 请求成功接入 web_search 续流，导致输出累计多轮而 credits 仅保留末轮
- **THEN** 前缀分支停用 credits 反推，取 `min(prefix_estimated_tokens, final_input_tokens)`
- **AND** metering 缓存真值与断点账本优先级、原有 credits 落库口径不变

#### Scenario: OpenAI 兼容端点继承 usage
- **WHEN** Anthropic 响应经 `/v1/chat/completions` 或 `/v1/responses` 兼容层转换
- **THEN** 缓存读取分别映射到 `prompt_tokens_details.cached_tokens` 或 `input_tokens_details.cached_tokens`，OpenAI 层不额外缩放
- **AND** 非流式继承校准展示值，流式继承静态 `message_delta` 展示值；Chat 流式遵循 `stream_options.include_usage`

#### Scenario: 流式 message_delta 与落库口径分离
- **GIVEN** 流式请求已收到 `meteringEvent`，且 `prefix_estimated_tokens > 0`
- **WHEN** 生成末尾 `message_delta` 并写入 `UsageTracker`
- **THEN** `message_delta` 的 `usage.cache_read_input_tokens` 等于 `message_start` 同口径的静态值 `min(prefix_estimated_tokens, final_input_tokens)`（经 `scale_for_client` 展示缩放）
- **AND** `UsageTracker` 落库的 `cache_read_input_tokens` 采用 `creditsUsed` 校准值

#### Scenario: Haiku 5.5 长上下文档（> 100K）全量未命中与半截命中校准
- **GIVEN** 请求模型为 `claude-haiku-5-5`，`final_input_tokens == 122222`（`> 100_000`），静态前缀估算 `prefix_estimated_tokens == 121100`（`prefix_cached == true`），计费输出 token（含 thinking）为 `857`
- **WHEN** 上游返回 `meteringEvent.usage == 0.70897`
- **THEN** 系统按长上下文档费率（`r_miss = 6.7830`, `r_out = 93.9200`）反推并下修 `cache_read_input_tokens` 至 `[30000, 35000]` 区间（约 `25%~28%` 命中率）
- **AND** 当同规模请求返回全价 `meteringEvent.usage >= 0.86` 时，校准后 `cache_read_input_tokens == 0`

#### Scenario: Sonnet / Opus / Haiku 4.5 模型按定价与 k_ref 派生费率校准
- **GIVEN** 请求模型属于其余 Claude 系列（如 `claude-sonnet-4-6`、`claude-opus-4-6`、`claude-opus-5-5`、`claude-haiku-4-5`），`final_input_tokens >= 4096` 且 `prefix_estimated_tokens > 0`
- **WHEN** 上游返回 `meteringEvent.usage` 高于或等于无缓存基准输入加输出费用（代表上游缓存已过期或跨实例全量 miss）
- **THEN** 系统按 `get_model_pricing(model) × get_k_ref(model)` 派生的 `(r_miss, r_out)` 反推出 `inferred_read == 0`，最终结算 `cache_read_input_tokens == 0`
- **AND** 当上游正常命中缓存使 `inferred_read >= prefix_estimated_tokens` 时，受上界约束最终结算 `cache_read_input_tokens == prefix_estimated_tokens`

#### Scenario: 首轮或 history[0] 重置门控优先（prefix_cached 为 false）
- **GIVEN** 新会话首条消息、单条消息重发或 `history[0]` 首次出现/漂移重置导致 `prefix_estimated_tokens == 0`
- **WHEN** 请求完成结算（无论 `meteringEvent.usage` 取何值）
- **THEN** 最终结算 `cache_read_input_tokens == 0`

#### Scenario: 短输入请求（< 4096 tokens）跳过反推
- **GIVEN** 请求 `final_input_tokens < 4096`（如 `input_tokens == 500` 的短对话或标题生成请求），`prefix_estimated_tokens == 300`
- **WHEN** 上游返回任意 `meteringEvent.usage`
- **THEN** 系统跳过 `creditsUsed` 反推，保持 `cache_read_input_tokens == 300`

#### Scenario: 非 Claude 模型与无 meteringEvent 回退静态估算
- **GIVEN** 请求模型为非 Claude 模型（如 `gpt-5.6-luna`、`deepseek-3.2`、`qwen3-coder-next`）或上游未返回 `meteringEvent`
- **WHEN** 请求完成结算
- **THEN** 系统跳过 `creditsUsed` 反推，保持 `cache_read_input_tokens == min(prefix_estimated_tokens, final_input_tokens)`

#### Scenario: 无效 credits 回退静态估算
- **WHEN** `creditsUsed` 缺失、为非有限值或不大于零
- **THEN** 前缀分支跳过反推，保持 `cache_read_input_tokens == min(prefix_estimated_tokens, final_input_tokens)`
- **AND** 不改变 metering 缓存真值与断点账本的优先级

#### Scenario: 流式 message_start 与开启 promptCacheAccounting 的优先级约束
- **GIVEN** 流式请求正在生成初始 `message_start` 事件，或实例已开启 `promptCacheAccounting`（`cache_accounting` 为 `Some`）
- **WHEN** 派生上报的 `usage` 字段
- **THEN** `message_start` 阶段因尚未收到 `meteringEvent` 仍使用静态前缀估算值
- **AND** 当 `cache_accounting` 为 `Some` 且上游未直接透传 `cache_read_input_tokens` 时，系统优先使用 `CacheAccounting::usage_for` 的断点模拟结果，不被 `creditsUsed` 反推覆盖
