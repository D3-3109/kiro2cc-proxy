// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 上游元数据事件
//!
//! 处理 metadataEvent：流末尾下发，携带上游对本次生成的终止判定 `stopReason`
//! （如 END_TURN / TOOL_USE / MAX_TOKENS / MALFORMED_TOOL_USE）。

use serde::Deserialize;

use crate::kiro::parser::error::ParseResult;
use crate::kiro::parser::frame::Frame;

use super::base::EventPayload;

/// 元数据事件（其余字段如 tokenUsage 暂不消费）
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataEvent {
    /// 上游终止原因；部分 metadataEvent 不携带
    #[serde(default)]
    pub stop_reason: Option<String>,
}

impl EventPayload for MetadataEvent {
    fn from_frame(frame: &Frame) -> ParseResult<Self> {
        frame.payload_as_json()
    }
}

/// 上游 stopReason 归类（大小写不敏感）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamStop {
    /// END_TURN / STOP_SEQUENCE：正常结束
    Completed,
    /// TOOL_USE：上游声称发起了工具调用（须与实际收到的 toolUseEvent 核对）
    ToolUse,
    /// MAX_TOKENS / LENGTH_LIMIT：输出达到上限
    MaxTokens,
    /// MODEL_CONTEXT_WINDOW_EXCEEDED：上下文窗口超限
    ContextWindowExceeded,
    /// MALFORMED_TOOL_USE / MALFORMED_MODEL_OUTPUT：上游丢弃了无法解析的输出
    Malformed,
    /// 其他取值（CONTENT_FILTERED / MAX_TIME 等），仅记录
    Other,
}

impl MetadataEvent {
    /// 归类 stopReason；未携带时返回 None
    pub fn upstream_stop(&self) -> Option<UpstreamStop> {
        let reason = self.stop_reason.as_deref()?.trim().to_ascii_uppercase();
        let stop = match reason.as_str() {
            "END_TURN" | "STOP_SEQUENCE" => UpstreamStop::Completed,
            "TOOL_USE" => UpstreamStop::ToolUse,
            "MAX_TOKENS" | "LENGTH_LIMIT" => UpstreamStop::MaxTokens,
            "MODEL_CONTEXT_WINDOW_EXCEEDED" => UpstreamStop::ContextWindowExceeded,
            "MALFORMED_TOOL_USE" | "MALFORMED_MODEL_OUTPUT" => UpstreamStop::Malformed,
            _ => UpstreamStop::Other,
        };
        Some(stop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop_of(json: &str) -> Option<UpstreamStop> {
        serde_json::from_str::<MetadataEvent>(json)
            .unwrap()
            .upstream_stop()
    }

    #[test]
    fn test_upstream_stop_classification() {
        assert_eq!(stop_of(r#"{}"#), None);
        assert_eq!(
            stop_of(r#"{"stopReason":"end_turn"}"#),
            Some(UpstreamStop::Completed)
        );
        assert_eq!(
            stop_of(r#"{"stopReason":"TOOL_USE"}"#),
            Some(UpstreamStop::ToolUse)
        );
        assert_eq!(
            stop_of(r#"{"stopReason":"LENGTH_LIMIT"}"#),
            Some(UpstreamStop::MaxTokens)
        );
        assert_eq!(
            stop_of(r#"{"stopReason":"MODEL_CONTEXT_WINDOW_EXCEEDED"}"#),
            Some(UpstreamStop::ContextWindowExceeded)
        );
        assert_eq!(
            stop_of(r#"{"stopReason":"MALFORMED_MODEL_OUTPUT"}"#),
            Some(UpstreamStop::Malformed)
        );
        assert_eq!(
            stop_of(r#"{"stopReason":"MAX_TIME","tokenUsage":{"outputTokens":1}}"#),
            Some(UpstreamStop::Other)
        );
    }
}
