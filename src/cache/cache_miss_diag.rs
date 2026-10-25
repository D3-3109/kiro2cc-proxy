// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! credits 反推发现上游缓存部分未命中时的 INFO 诊断日志。
//!
//! 触发：creditsUsed 反推实际生效，且反推读取 < 前缀估算 × 0.8（命中率 < 80%）。
//! 诊断字段与候选原因的对应：
//! - `gap_since_prev_ms` 大（> 5min）→ 指向上游 TTL 逐出
//! - `credential_changed=true` → 换账号必然全量 miss
//! - 间隔小且同凭据 → 倾向上游负载均衡换实例 / 缓存抖动（本地无法观测，只能排除前两者）
//!
//! 上游真实实例路由本地不可见，本日志只负责缩小范围。注册表为会话级
//! （256 会话上限，满员整体清空，同 drift_diag 口径），只记录每会话上次
//! 结算时刻与凭据，不记录任何请求内容。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::model::usage::{MIN_INFER_INPUT_TOKENS, infer_cache_read_from_credits};

/// 命中率低于该比例视为部分未命中，输出诊断
const HIT_RATIO_TRIGGER: f64 = 0.8;
/// 最多记录的会话数；超出后整体清空（诊断数据，丢失无影响）
const MAX_SESSIONS: usize = 256;

struct LastSettlement {
    at: Instant,
    credential_id: u64,
}

static LAST_SETTLEMENT: OnceLock<Mutex<HashMap<String, LastSettlement>>> = OnceLock::new();

/// 流式 / 非流式结算路径共用：反推生效且命中率低于阈值时输出 `[cache-miss]` 诊断，
/// 并无条件更新会话注册表（供下次结算计算间隔与凭据变化）。
///
/// `inference_applied` 为 false 表示本次未走反推（metering 真值 / 断点账本优先 /
/// 桥接续流回退静态前缀），诊断不触发，但注册表照常更新。
#[allow(clippy::too_many_arguments)]
pub fn maybe_log(
    conversation_id: &str,
    model: &str,
    prefix_estimated_tokens: i32,
    final_input_tokens: i32,
    billed_output_tokens: i32,
    credits_used: Option<f64>,
    inference_applied: bool,
    credential_id: u64,
) {
    let now = Instant::now();
    let (prev_at, prev_credential) = {
        let map = LAST_SETTLEMENT.get_or_init(|| Mutex::new(HashMap::new()));
        let mut map = map.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() >= MAX_SESSIONS && !map.contains_key(conversation_id) {
            map.clear();
        }
        let prev = map.insert(
            conversation_id.to_string(),
            LastSettlement {
                at: now,
                credential_id,
            },
        );
        (prev.as_ref().map(|p| p.at), prev.map(|p| p.credential_id))
    };

    // 门槛复用 pricing 的 MIN_INFER_INPUT_TOKENS：pricing 内作用于 final_input_tokens，
    // 此处作用于前缀估算，两者必须同值（反推的输入基数与前缀同量级才可比）
    if !inference_applied
        || prefix_estimated_tokens <= 0
        || prefix_estimated_tokens < MIN_INFER_INPUT_TOKENS
        || final_input_tokens <= 0
    {
        return;
    }
    let Some(credits) = credits_used else {
        return;
    };
    let Some(inferred) =
        infer_cache_read_from_credits(model, final_input_tokens, billed_output_tokens, credits)
    else {
        return;
    };
    if inferred as f64 >= prefix_estimated_tokens as f64 * HIT_RATIO_TRIGGER {
        return;
    }
    tracing::info!(
        session = %conversation_id,
        prefix_estimated = prefix_estimated_tokens,
        inferred_read = inferred,
        miss_tokens = (prefix_estimated_tokens - inferred).max(0),
        hit_ratio = format!("{:.0}%", (inferred as f64 / prefix_estimated_tokens as f64) * 100.0),
        input = final_input_tokens,
        output = billed_output_tokens,
        credits,
        gap_since_prev_ms = prev_at.map(|at| now.duration_since(at).as_millis() as u64),
        credential_changed = prev_credential.is_none_or(|c| c != credential_id),
        "[cache-miss] credits 反推显示上游缓存部分未命中（间隔大→TTL 逐出；换凭据→必然 miss；均否→倾向上游换实例/抖动）"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 只清自己的 key：cargo test 默认并行，clear() 整表清空会清掉其他并行测试
    /// 两次 maybe_log 之间插入的条目，导致 gap/credential 断言间歇性 flaky
    fn clear(key: &str) {
        LAST_SETTLEMENT
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap()
            .remove(key);
    }

    /// 与 2026-10-25 实测 0.0539 事件同构：prefix 53650、input 53776、输出 346、
    /// credits 0.06617 → 反推约 21000，命中率约 39%，应触发
    fn event_credits() -> (i32, i32, i32, f64) {
        (53_650, 53_776, 346, 0.06617)
    }

    #[derive(Clone, Default)]
    struct BufWriter(std::sync::Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn capture<F: FnOnce()>(f: F) -> String {
        let buf = BufWriter::default();
        let writer = buf.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        tracing::subscriber::with_default(subscriber, f);
        String::from_utf8(buf.0.lock().unwrap().clone()).unwrap()
    }

    #[test]
    fn partial_miss_logs_diag() {
        clear("miss-diag-1");
        let (prefix, input, output, credits) = event_credits();
        let out = capture(|| {
            maybe_log(
                "miss-diag-1",
                "claude-haiku-5-5",
                prefix,
                input,
                output,
                Some(credits),
                true,
                64,
            );
        });
        assert_eq!(out.matches("[cache-miss]").count(), 1, "{out}");
        assert!(out.contains("prefix_estimated=53650"), "{out}");
        assert!(out.contains("hit_ratio=\"39%\""), "{out}");
        // 首次结算：无间隔字段、凭据视为已变化
        assert!(!out.contains("gap_since_prev_ms"), "{out}");
        assert!(out.contains("credential_changed=true"), "{out}");
    }

    #[test]
    fn high_hit_ratio_does_not_log() {
        clear("miss-diag-2");
        let out = capture(|| {
            // 全量命中：credits 只含 10% 缓存读取价 → 反推 ≈ prefix
            maybe_log(
                "miss-diag-2",
                "claude-haiku-5-5",
                53_650,
                53_776,
                346,
                // 53.65K 读 × 10% + 346 输出 + 未命中尾部
                Some(0.0108),
                true,
                64,
            );
        });
        assert!(!out.contains("[cache-miss]"), "{out}");
    }

    #[test]
    fn second_settlement_reports_gap_and_same_credential() {
        clear("miss-diag-3");
        let (prefix, input, output, credits) = event_credits();
        let out = capture(|| {
            maybe_log(
                "miss-diag-3",
                "claude-haiku-5-5",
                prefix,
                input,
                output,
                Some(credits),
                true,
                64,
            );
            maybe_log(
                "miss-diag-3",
                "claude-haiku-5-5",
                prefix,
                input,
                output,
                Some(credits),
                true,
                64,
            );
        });
        let second = out
            .lines()
            .filter(|l| l.contains("[cache-miss]"))
            .nth(1)
            .expect("第二次应触发");
        // 同会话两次结算间隔极短（毫秒级，可能为 0 或 1），只断言字段存在
        assert!(second.contains("gap_since_prev_ms="), "{second}");
        assert!(second.contains("credential_changed=false"), "{second}");
    }

    #[test]
    fn credential_switch_is_reported() {
        clear("miss-diag-4");
        let (prefix, input, output, credits) = event_credits();
        let out = capture(|| {
            maybe_log(
                "miss-diag-4",
                "claude-haiku-5-5",
                prefix,
                input,
                output,
                Some(credits),
                true,
                64,
            );
            maybe_log(
                "miss-diag-4",
                "claude-haiku-5-5",
                prefix,
                input,
                output,
                Some(credits),
                true,
                99,
            );
        });
        let second = out
            .lines()
            .filter(|l| l.contains("[cache-miss]"))
            .nth(1)
            .expect("第二次应触发");
        assert!(second.contains("credential_changed=true"), "{second}");
    }

    #[test]
    fn inference_not_applied_only_updates_registry() {
        clear("miss-diag-5");
        let out = capture(|| {
            maybe_log(
                "miss-diag-5",
                "claude-haiku-5-5",
                53_650,
                53_776,
                346,
                Some(0.06617),
                false,
                64,
            );
        });
        assert!(!out.contains("[cache-miss]"), "{out}");
        // 注册表已更新：下次同会话结算能拿到间隔
        let (prefix, input, output, credits) = event_credits();
        let out2 = capture(|| {
            maybe_log(
                "miss-diag-5",
                "claude-haiku-5-5",
                prefix,
                input,
                output,
                Some(credits),
                true,
                64,
            );
        });
        assert!(out2.contains("gap_since_prev_ms="), "{out2}");
        assert!(out2.contains("credential_changed=false"), "{out2}");
    }
}
