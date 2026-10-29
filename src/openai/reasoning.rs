// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 推理签名在 OpenAI 协议中的往返编码
//!
//! 上游 thinking 块的签名能让下一轮的模型看到上一轮的推理（`src/anthropic/converter/history.rs`
//! 把它回传为 `reasoningContent`）。OpenAI 协议没有 thinking 块，这里约定两种载体：
//!
//! - Responses：reasoning item 的 `encrypted_content` = [`ENCRYPTED_CONTENT_PREFIX`] + 签名。
//!   前缀用来区分其他服务商的加密推理（客户端换服务商后原样回传的内容不可用）。
//! - Chat Completions：assistant 消息的 `reasoning_details`，沿用 OpenRouter 的 Anthropic 形态
//!   `{"type": "reasoning.text", "text", "signature", "format": "anthropic-claude-v1", "index"}`。
//!
//! 代理伪造的签名（见 [`crate::anthropic::is_fake_signature`]）不下发给客户端。

use serde_json::{Value, json};

/// Responses `encrypted_content` 的前缀
const ENCRYPTED_CONTENT_PREFIX: &str = "kiro2cc.v1:";

/// Chat Completions `reasoning_details` 项的类型与格式标识（OpenRouter 约定）
const REASONING_DETAIL_TYPE: &str = "reasoning.text";
const REASONING_DETAIL_FORMAT: &str = "anthropic-claude-v1";

/// 签名能否回传给上游：非空且不是代理伪造的
pub(super) fn is_replayable_signature(signature: &str) -> bool {
    !signature.is_empty() && !crate::anthropic::is_fake_signature(signature)
}

/// 签名 → Responses reasoning item 的 `encrypted_content`
pub(super) fn encode_encrypted_content(signature: &str) -> String {
    format!("{ENCRYPTED_CONTENT_PREFIX}{signature}")
}

/// Responses reasoning item 的 `encrypted_content` → 签名；不是本代理产出的返回 `None`
pub(super) fn decode_encrypted_content(encrypted: &str) -> Option<&str> {
    encrypted
        .strip_prefix(ENCRYPTED_CONTENT_PREFIX)
        .filter(|signature| !signature.is_empty())
}

/// 一个 Chat Completions `reasoning_details` 项
pub(super) fn reasoning_detail(text: &str, signature: &str, index: usize) -> Value {
    json!({
        "type": REASONING_DETAIL_TYPE,
        "text": text,
        "signature": signature,
        "format": REASONING_DETAIL_FORMAT,
        "index": index,
    })
}

/// Chat Completions `reasoning_details` → Anthropic thinking 块（只取带签名的项）
pub(super) fn thinking_blocks_from_details(details: Option<&Value>) -> Vec<Value> {
    let Some(details) = details.and_then(Value::as_array) else {
        return Vec::new();
    };
    details
        .iter()
        .filter(|d| d.get("type").and_then(Value::as_str) == Some(REASONING_DETAIL_TYPE))
        .filter_map(|d| {
            let signature = d.get("signature").and_then(Value::as_str)?;
            let text = d.get("text").and_then(Value::as_str).unwrap_or_default();
            is_replayable_signature(signature).then(|| thinking_block(text, signature))
        })
        .collect()
}

/// Anthropic thinking 块
pub(super) fn thinking_block(text: &str, signature: &str) -> Value {
    json!({"type": "thinking", "thinking": text, "signature": signature})
}
