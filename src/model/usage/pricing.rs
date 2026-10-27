//! 模型定价与费用估算：每百万 tokens 美元定价、credits/USD 换算率、单次请求费用。
/// 模型定价（每百万 tokens，美元）
/// 使用 200K context 标准定价
pub(crate) struct ModelPricing {
    input_per_mtok: f64,
    output_per_mtok: f64,
}

/// 判断模型名是否属于 haiku 5.5（5.5 代际）。
///
/// 与 `crate::anthropic::converter::model::is_haiku_55` 保持同一判定口径
/// （同一模型族在映射层与定价层必须一致，否则会出现「映射到 5.5 但按 4.5 计价」）。
/// 不跨层引用是为避免 `model` 层反向依赖 `anthropic` 层。
fn is_haiku_55(model_lower: &str) -> bool {
    model_lower.replace(['.', ' '], "-").contains("haiku-5-5")
}

/// Haiku 5.5 长上下文阈值（tokens）。
///
/// 官方口径：单次请求 prompt 超过 100K 时，**整个请求**按 5 倍高档计费
/// （非超出部分累进）。低档 $0.10/$0.50，高档 $0.50/$2.50。
const HAIKU_55_LONG_CONTEXT_THRESHOLD: i32 = 100_000;

/// 触发 `creditsUsed` 缓存读取反推的最小输入 token 阈值。
///
/// 输入过短时，冷/热两条计费曲线差值过小，输出分词微差会被放大；
/// 且 1.5K 以内首段单价更高（实测曲线仅在 4K 以上呈直线），故直接保留静态前缀估算。
pub(crate) const MIN_INFER_INPUT_TOKENS: i32 = 4096;

/// 反推命中比例允许超出 [0, 1] 的幅度；超出即视为模型无法解释本次计费，不反推。
///
/// 实测曲线残差 ≤ 5%，余量覆盖输出分词与隐藏推理估算误差。
const HIT_RATIO_TOLERANCE: f64 = 0.15;

/// Kiro 实测 credits 计费曲线（pinned 单账号标定，输入 4K–130K）。
///
/// 上游计费可由「全未命中」与「全命中」两条直线描述，命中比例在两者之间线性插值：
///
/// ```text
/// credits   = cold(I) − hit · (cold(I) − cached(I)) + output_per_mtok · O
/// cold(I)   = cold_base   + cold_per_mtok   · I
/// cached(I) = cached_base + cached_per_mtok · I
/// ```
///
/// 截距包含 Kiro 注入的隐藏系统提示（本地 token 计数不含）。opus / sonnet 实测：
/// 新增尾部不短于已缓存前缀时整单按 cold 计价，否则部分命中；haiku 为常规前缀缓存。
/// 插值直接给出「计费意义上的」命中比例，两种行为无需分别建模。
struct CreditCurve {
    cold_base: f64,
    cold_per_mtok: f64,
    cached_base: f64,
    cached_per_mtok: f64,
    /// 散文类输出单价（cl100k 口径）；纯数字列表实测约低 30%
    output_per_mtok: f64,
    /// 隐藏推理输出 tokens ≈ signature_slope · 签名字符数 + signature_intercept · 签名块数
    signature_slope: f64,
    signature_intercept: f64,
}

const OPUS_55_CURVE: CreditCurve = CreditCurve {
    cold_base: 0.08284,
    cold_per_mtok: 26.486,
    cached_base: 0.02381,
    cached_per_mtok: 7.191,
    output_per_mtok: 320.0,
    signature_slope: 0.2807,
    signature_intercept: -155.7,
};

const SONNET_55_CURVE: CreditCurve = CreditCurve {
    cold_base: 0.05144,
    cold_per_mtok: 17.270,
    cached_base: 0.01515,
    cached_per_mtok: 4.666,
    output_per_mtok: 208.0,
    signature_slope: 0.2672,
    signature_intercept: -110.1,
};

/// Haiku 5.5 常态档（<= 100K）。输出单价按 opus 散文/数字比例由数字列表实测值折算。
const HAIKU_55_CURVE_STANDARD: CreditCurve = CreditCurve {
    cold_base: 0.00083,
    cold_per_mtok: 2.485,
    cached_base: 0.00082,
    cached_per_mtok: 0.248,
    output_per_mtok: 8.4,
    signature_slope: 0.2989,
    signature_intercept: -170.7,
};

/// Haiku 5.5 长上下文档（> 100K，整单 5 倍跳价）。
const HAIKU_55_CURVE_LONG_CONTEXT: CreditCurve = CreditCurve {
    cold_base: -0.00151,
    cold_per_mtok: 12.497,
    cached_base: 0.00355,
    cached_per_mtok: 1.250,
    output_per_mtok: 42.0,
    signature_slope: 0.2989,
    signature_intercept: -170.7,
};

/// Opus 4.5+ 定价：$5 / $25。
const OPUS_PRICING: ModelPricing = ModelPricing {
    input_per_mtok: 5.0,
    output_per_mtok: 25.0,
};

/// 根据模型名获取定价
pub(crate) fn get_model_pricing(model: &str) -> ModelPricing {
    pricing_of_lower(&model.to_lowercase())
}

fn pricing_of_lower(model_lower: &str) -> ModelPricing {
    if model_lower.contains("opus") {
        OPUS_PRICING
    } else if is_haiku_55(model_lower) {
        // Haiku 5.5: $0.10 / $0.50（≤100K 低档，官方称覆盖约 90% 请求量）。
        // >100K 的高档由 calculate_cost 按整档跳价处理（本函数不持有 token 数）。
        ModelPricing {
            input_per_mtok: 0.10,
            output_per_mtok: 0.50,
        }
    } else if model_lower.contains("haiku") {
        // Haiku 4.5: $1 / $5
        ModelPricing {
            input_per_mtok: 1.0,
            output_per_mtok: 5.0,
        }
    } else {
        // Sonnet 4 / sonnet-5 / haiku: $3 / $15
        // claude-sonnet-5 Rate = 1.3 Credit，与 sonnet-4.x 同档，定价一致
        ModelPricing {
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
        }
    }
}

/// 平台级 credits/USD 换算率，按模型档位差异化（代理实测 2026-06-25）。
/// 供 usage 报表 credits_saved（estimated_cost × k_ref - credits_used）使用。
/// 2026-06-30 重校：按 opus 版本分档，基于实测 d=0.50 缓存折扣反推。
/// 此处保留历史标定背景；缓存读取反推已改用实测计费曲线（`CreditCurve`），不依赖本值。
/// 2026-07-25 追加 opus-5 与 4.7/4.8 同档。
pub(crate) fn get_k_ref(model: &str) -> f64 {
    k_ref_of_lower(&model.to_lowercase())
}

fn k_ref_of_lower(m: &str) -> f64 {
    if m.contains("opus-4-7")
        || m.contains("opus-4.7")
        || m.contains("opus-4-8")
        || m.contains("opus-4.8")
        || m.contains("opus-5")
        || m.contains("opus.5")
    {
        // opus 4.7/4.8/5 共用同档（实测 4.8 ≈ 2.36，5 沿用 4.8 档位）
        2.36
    } else if m.contains("opus-4-5")
        || m.contains("opus-4.5")
        || m.contains("opus-4-6")
        || m.contains("opus-4.6")
    {
        // 旧 opus 4.5/4.6（实测 4.6 ≈ 1.90）
        1.90
    } else if m.contains("opus") || m.contains("fable") {
        // 未知 opus / fable 兜底沿用最新档
        2.36
    } else if m.contains("sonnet-5") || m.contains("sonnet.5") {
        // claude-sonnet-5: Rate = 1.3 Credit，与 sonnet-4.5/4.6 同档（实测确认）
        1.43
    } else {
        // sonnet 系列 / haiku（含 haiku 5.5）默认
        1.43
    }
}

/// 计算单次请求的估算费用
///
/// Haiku 5.5 按官方「整档跳价」口径：输入超 100K 时整个请求走高档
/// （见 HAIKU_55_LONG_CONTEXT_THRESHOLD）。
///
/// 已知偏差：`input_tokens` 为**不含 cache token** 的净输入
/// （cache_read / cache_creation 单独字段存储，未参与定价计算），
/// 故阈值按净输入判定，与官方「prompt 总长度」口径存在系统性低估，
/// 带长缓存前缀的会话可能被判入低档。
pub(crate) fn calculate_cost(model: &str, input_tokens: i32, output_tokens: i32) -> f64 {
    let model_lower = model.to_lowercase();
    let pricing = if is_haiku_55(&model_lower) && input_tokens > HAIKU_55_LONG_CONTEXT_THRESHOLD {
        ModelPricing {
            input_per_mtok: 0.50,
            output_per_mtok: 2.50,
        }
    } else {
        get_model_pricing(model)
    };
    let input_cost = (input_tokens as f64 / 1_000_000.0) * pricing.input_per_mtok;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * pricing.output_per_mtok;
    input_cost + output_cost
}

/// 本次响应的计费输出观测值，供 credits 反推估算上游真实输出。
///
/// 原生推理只回传摘要，真实推理输出隐藏在签名里：签名长度与隐藏输出强相关
/// （实测 R² 0.94–0.98），摘要 tokens 则只有 0.7–12.7 倍的松散关系。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BilledOutput {
    /// 本地 BPE 计费口径输出 tokens（可见输出 + 推理摘要 + 旧标签 thinking）
    pub tokens: i32,
    /// `tokens` 中的原生推理摘要部分
    pub native_reasoning_tokens: i32,
    /// 原生推理签名字符数之和
    pub signature_chars: usize,
    /// 带签名的原生推理块数
    pub signature_blocks: usize,
}

/// 已实测标定的模型取计费曲线；未标定模型返回 `None`（跳过反推，保留静态前缀估算）。
fn credit_curve(model: &str, input_tokens: i32) -> Option<&'static CreditCurve> {
    let model_lower = model.to_lowercase();
    let normalized = model_lower.replace(['.', ' '], "-");
    if normalized.contains("opus-5-5") {
        Some(&OPUS_55_CURVE)
    } else if normalized.contains("sonnet-5-5") {
        Some(&SONNET_55_CURVE)
    } else if is_haiku_55(&model_lower) {
        if input_tokens > HAIKU_55_LONG_CONTEXT_THRESHOLD {
            Some(&HAIKU_55_CURVE_LONG_CONTEXT)
        } else {
            Some(&HAIKU_55_CURVE_STANDARD)
        }
    } else {
        None
    }
}

impl CreditCurve {
    /// 上游计费的输出 tokens：推理摘要替换为按签名估算的隐藏推理输出（不低于摘要）。
    ///
    /// 有推理摘要却无签名时无法估算隐藏输出，返回 `None`。
    fn upstream_output_tokens(&self, output: &BilledOutput) -> Option<f64> {
        let summary = output.native_reasoning_tokens as f64;
        if output.native_reasoning_tokens > 0 && output.signature_blocks == 0 {
            return None;
        }
        let from_signature = self.signature_slope * output.signature_chars as f64
            + self.signature_intercept * output.signature_blocks as f64;
        Some(output.tokens as f64 - summary + summary.max(from_signature))
    }
}

/// 由上游 `meteringEvent.usage`（`credits_used`）反推计费意义上的缓存读取 token 数。
///
/// `hit = (cold(I) + r_out · O − credits) / (cold(I) − cached(I))`，返回 `round(clamp(hit, 0, 1) · I)`。
/// 仅对已标定模型、`input_tokens >= 4096`、`credits_used > 0` 生效；
/// `hit` 超出 `[−0.15, 1.15]` 说明本次计费不符合模型（如上游计费口径变化），返回 `None`。
pub(crate) fn infer_cache_read_from_credits(
    model: &str,
    input_tokens: i32,
    output: &BilledOutput,
    credits_used: f64,
) -> Option<i32> {
    if input_tokens < MIN_INFER_INPUT_TOKENS || !credits_used.is_finite() || credits_used <= 0.0 {
        return None;
    }
    let curve = credit_curve(model, input_tokens)?;
    let upstream_output = curve.upstream_output_tokens(output)?;

    let input_mtok = input_tokens as f64 / 1_000_000.0;
    let cold = curve.cold_base + curve.cold_per_mtok * input_mtok;
    let cached = curve.cached_base + curve.cached_per_mtok * input_mtok;
    let output_cost = curve.output_per_mtok * upstream_output / 1_000_000.0;
    let hit_ratio = (cold + output_cost - credits_used) / (cold - cached);

    let explainable = (-HIT_RATIO_TOLERANCE..=1.0 + HIT_RATIO_TOLERANCE).contains(&hit_ratio);
    if !explainable {
        return None;
    }
    Some((hit_ratio.clamp(0.0, 1.0) * input_tokens as f64).round() as i32)
}

/// 结合静态前缀上界与 `creditsUsed` 反推值校准最终 `cache_read_input_tokens`。
///
/// - `prefix_estimated_tokens <= 0`（首轮或 `history[0]` 重置）时锁定为 `0`；
/// - 满足反推条件时取 `min(prefix_estimated_tokens, inferred_read, final_input_tokens)`；
/// - 其余情况（`< 4096`、无 `credits_used`、未标定模型、计费无法解释）保留
///   `min(prefix_estimated_tokens, final_input_tokens)`。
pub(crate) fn refine_prefix_cache_read(
    model: &str,
    prefix_estimated_tokens: i32,
    final_input_tokens: i32,
    output: &BilledOutput,
    credits_used: Option<f64>,
) -> i32 {
    if prefix_estimated_tokens <= 0 || final_input_tokens <= 0 {
        return 0;
    }
    let capped_prefix = prefix_estimated_tokens.min(final_input_tokens);
    match credits_used
        .and_then(|cu| infer_cache_read_from_credits(model, final_input_tokens, output, cu))
    {
        Some(inferred) => capped_prefix.min(inferred),
        None => capped_prefix,
    }
}
