## 上下文
`kiro2cc-proxy` 目前在 `/v1/messages` 与 `/cc/v1/messages` 的流式（`StreamContext`）和非流式（`handle_non_stream_request`）路径中，使用 `helpers::estimate_prefix_tokens` 计算静态缓存前缀 `prefix_estimated_tokens`。该值在 `prefix_cached == false` 时为 `0`，在 `prefix_cached == true` 时等于 `system + tools + messages[..n-1]` 的 token 数。
然而 Kiro 上游并不保证所有历史前缀永久命中缓存（存在 TTL 过期、跨后端节点路由 miss、或仅 `history[0]` 命中而后续对话历史未命中）。由于 Kiro `meteringEvent` 只返回 `usage: f64`（`creditsUsed`）而不返回 `cache_read_input_tokens`，纯静态前缀估算无法感知上游缓存失效。

## 目标 / 非目标
- **目标**：
  1. 在不引入新外部依赖、不改变现有请求转换与账号调度逻辑的前提下，利用 `meteringEvent.usage`（`creditsUsed`）估算全部 Claude 模型的缓存读取量；非 Haiku 5.5 费率属于未标定假设。
  2. 采用「静态前缀估算提供上界 + `creditsUsed` 反推下修失效部分」的混合策略（`min(prefix_estimated_tokens, inferred_read)`），兼顾首轮 `0%` 确定性、物理前缀上限保护与上游失效检测。
  3. 同步作用于 `UsageTracker` 落库及非流式 JSON 响应；流式末尾 `message_delta` 保持静态口径（见风险 / 权衡）。
- **非目标**：
  1. 不对 GPT / 第三方模型进行反推（保持静态前缀估算）。
  2. 不改动 `message_start` 阶段行为，也不覆盖显式开启的 `promptCacheAccounting` 断点模拟账本。

## 决策
1. **反推公式与 `0.90` 缓存节省率**：
   - 历史 commit `16532248` 曾尝试按 `0.50` 缓存折扣反推，因真实缓存折扣为 `0.10`（节省 `0.90`），除以 `0.50` 导致反推值被放大 `1.8` 倍并频繁顶满 `100%` clamp。
   - 本次基于 112 条 `claude-haiku-5-5` 真实记录（含 49 条固定输出受控差分序列）确证缓存命中单价严格为未命中单价的 `1/10`（节省 `0.90`），反推公式定为：
     $$\hat{R} = \operatorname{clamp}\!\left(\operatorname{round}\!\left(\frac{r_{\text{miss}} \cdot I + r_{\text{out}} \cdot O_{\text{billed}} - \text{creditsUsed} \times 10^6}{0.90 \cdot r_{\text{miss}}}\right),\; 0,\; I\right)$$
2. **Claude 全模型费率决策**：
   - `claude-haiku-5-5`：采用实测标定的双档参数（`I <= 100_000` 时 $r_{\text{miss}} = 1.6978, r_{\text{out}} = 19.1873$；`I > 100_000` 时 $r_{\text{miss}} = 6.7830, r_{\text{out}} = 93.9200$）。
   - 其余 Claude 模型（`haiku-4.5`、`sonnet-4.5/4.6/5`、`opus-4.5/4.6/4.7/4.8/5/5.5`、`fable`）：直接复用 `src/model/usage/pricing.rs` 中已有的 `get_model_pricing(model)` 与 `get_k_ref(model)` 计算 $r_{\text{miss}} = \text{input\_per\_mtok} \times k_{\text{ref}}$、$r_{\text{out}} = \text{output\_per\_mtok} \times k_{\text{ref}}$，避免维护两套不一致的模型档位表。
3. **计费输出 token（$O_{\text{billed}}$）口径对齐**：
   - 上游 `creditsUsed` 包含 thinking 输出的计费。
   - 流式路径中，`StreamContext::output_tokens()` 已累计全部输出字符（含 `<thinking>` 标签与原生 `ReasoningContent`），直接作为 $O_{\text{billed}}$ 传入。
   - 非流式路径在账号过滤 thinking 前统计计费 thinking，反推始终使用可见输出加剔除前 thinking；过滤后再统计落库观测字段。`outputTokensIncludeThinking` 仍仅影响原有存留内容的输出展示，不因本次修复计入已隐藏的 thinking。
4. **最小输入阈值 `MIN_INFER_INPUT_TOKENS = 4096` 与上界融合**：
   - 当 `final_input_tokens < 4096` 时，输入积分占总 `creditsUsed` 比重过小，输出分词误差易造成反推抖动，故直接跳过反推。
   - 当 `final_input_tokens >= 4096` 且满足其他反推条件时，取 `prefix_estimated.min(inferred_read)`，限制结果不超过静态前缀；下修是否准确取决于费率、折扣及 token 估算，不能保证识别真实 miss。

## 风险 / 权衡
- **已接受的文档偏离：M2（fable）**：
  - 用户未选择修正决策 2 的统一费率描述，该描述对 fable 不成立。实际仅反推路径使用 Opus 定价 `$5/$25 × 2.36`；此处记录偏离，不将统一公式视为 fable 实际行为。
  - fable 的 `get_model_pricing` / `calculate_cost` 保持既有默认定价行为，不因反推使用 Opus 费率而改变。
- **假设：非 Haiku 5.5 费率尚未实测标定**：
  - 保留全部 Claude 模型反推，但除 Haiku 5.5 双档外，派生费率与 90% 节省率的组合未经同等受控样本验证。
  - `get_k_ref` 保留历史 d=0.50 标定背景，不表示已按当前 d=0.90 重新标定；结果可能系统性偏差，前缀上界仅限制高估幅度，不能保证识别真实 miss 或真实命中量。
- **降级：web_search 成功续流后停用反推**：
  - 流式在收割 `Continued`、非流式在成功读取续请求响应后标记多轮。仅令前缀反推不使用 credits；原有 metering 缓存真值、断点账本及积分记录不变。
  - 不能用搜索截获次数或预先存在的 `evolution_base` 替代成功续流标记。
- **兼容层继承**：
  - Chat Completions 的 `prompt_tokens_details.cached_tokens`、Responses 的 `input_tokens_details.cached_tokens` 原值继承 Anthropic 缓存读取；总输入由净输入、缓存读取、缓存创建相加。
  - 非流式继承校准后的展示值；流式读取 `message_delta`，继承静态展示值。OpenAI 层不再缩放，不修改协议字段结构。
- **权衡：流式响应不上报反推值，落库与响应口径分离**：
  - 项目注释记载客户端按字段取 max 合并 `message_start` 与 `message_delta`。若两者分别为静态值与反推值，合并结果会同时放大 input 与 cache_read。该行为尚未在真实客户端验证。
  - 因此流式两端统一静态前缀口径，反推值仅写入 `UsageTracker`（Admin UI 与积分统计）及非流式 JSON 响应。代价是流式客户端看到的 cache_read 不反映上游失效。
- **权衡：Sonnet / Opus 长上下文或特殊折扣波动**：
  - `min(prefix_estimated_tokens, inferred_read)` 确保不超过静态前缀，但费率、折扣与输出估算偏差仍可能导致误下修，不能将推导结果视为上游真值。

## 迁移方案
- 纯后端计算逻辑演进，无需迁移历史 JSON 数据文件，不影响配置文件格式与前端打包产物。
