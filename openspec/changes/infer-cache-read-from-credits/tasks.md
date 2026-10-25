# 任务清单：infer-cache-read-from-credits

## 状态：DONE

## 任务
- [x] 任务 1：在 `src/model/usage/pricing.rs`（及 `src/model/usage/mod.rs` 导出）中实现 Claude 模型计费反推费率查询与 `infer_cache_read_from_credits` / `refine_prefix_cache_read` 纯函数，覆盖 Haiku 5.5（`<= 100K` / `> 100K` 双档）与其余 Claude 模型（`get_model_pricing × get_k_ref`）、非 Claude 模型过滤、`final_input_tokens >= 4096` 阈值及 `min(prefix_estimated_tokens, inferred_read)` 上界融合，并编写单元测试验证各模型档位与边界条件（通过 `cargo test pricing` 验证；参考 `specs/cache/spec.md`）。
- [x] 任务 2：在 `src/anthropic/stream/context/mod.rs` 的 `StreamContext::derive_report_cache_usage` 中接入 `refine_prefix_cache_read`，使 `UsageTracker` 落库在收到 `metering_usage` 后按计费输出 `self.output_tokens()` 自动校准 `cache_read_input_tokens`；流式 `message_delta` 与 `message_start` 同为静态口径，`cache_accounting` 开启时保持原有行为，并补充流式结算单元测试（通过 `cargo test stream` 验证；不涉及 `token_manager`/`provider` 多账号调度逻辑修改；参考 `specs/cache/spec.md`）。
- [x] 任务 3：在 `src/anthropic/handlers/nonstream.rs` 的非流式结算路径中，提取包含 thinking 的计费输出 token 数并调用 `refine_prefix_cache_read` 修正 `prefix_estimated` 后再传入 `crate::cache::select_final_usage`，使 `/v1/messages` 与 `/cc/v1/messages` 非流式落库与响应 `usage` 同步生效，并通过 `cargo test`、`cargo fmt --check` 与 `cargo clippy` 全量验证（不涉及 `token_manager`/`provider` 多账号逻辑修改；参考 `specs/cache/spec.md`）。

## 验收标准
- [x] 修复 1：非流式生产纯函数在过滤前计算计费 thinking，保持过滤后展示与观测语义，覆盖原生/文本 thinking 及两个输出展示开关（全量 `cargo test` 通过）。
- [x] 修复 2：流式/非流式成功接入续流后只禁用 credits 反推，真实 harvest 接线、流式落库静态值与积分保留测试通过；非流式续请求失败组合尚无直接 handler 回归测试。
- [x] 修复 3：补齐 4095/4096、100000/100001、常态全量 miss、断点账本创建/读取与 metering 优先测试（`cargo test pricing`、`cargo test stream_context` 通过）。
- [x] 修复 4：同步未标定费率假设、隐藏 thinking、多轮降级与 OpenAI 兼容影响；本轮增量代码审查无新增问题，全量测试通过。第二轮需求回溯仅剩已披露的 M2 文档偏离（NON_CONFORMS），经用户接受残留后结束修复；不代表消除了该偏离。
- [x] `claude-haiku-5-5` 在 `<= 100K` 与 `> 100K` 档位下，给定实测未命中/部分命中/全量命中 `creditsUsed` 样本时，`refine_prefix_cache_read` 反推结果与实测值一致（首轮/未命中为 `0`，部分失效如实下修，全量命中不超过 `prefix_estimated_tokens`）。
- [x] Sonnet / Opus / Haiku 4.5 模型在收到高额未命中 `creditsUsed` 时下修 `cache_read_input_tokens`，正常命中时受 `prefix_estimated_tokens` 上界约束；非 Claude 模型（如 `gpt-5.6-luna`）及 `final_input_tokens < 4096` 请求保持静态前缀估算不变。
- [x] 全量 `cargo test` 为 1025 passed / 0 failed，`cargo fmt --check` 通过；`cargo clippy --all-targets` 退出码为 0，但有 95 条既有告警，并非 clean。本轮未测量覆盖率。
