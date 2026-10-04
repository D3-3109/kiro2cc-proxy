// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 思考内容「文本化」渲染（opt-in，仅 Claude Code 客户端）
//!
//! Claude Code 携带 `redact-thinking` beta（或默认折叠 thinking）时，界面几乎看不到
//! 流式思考内容，长时间推理期间表现为「卡住」。开启 `thinkingAsText` 后，本模块把出站
//! SSE 中的 thinking 块改写为普通 text 块（markdown 引用，逐行流式输出），使思考过程
//! 像 Kiro CLI 一样实时可见。
//!
//! 代价：这些文本块会被客户端当作 assistant 正文存入对话历史并随后续请求回传。
//! 回传时由 `converter::history::convert_assistant_message` 识别 [`THOUGHT_HEADER`]
//! 并剥离，上游模型不会看到自己的旧思考（与原生 thinking 块在历史中被剥离的语义一致）。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;

use super::state::SseEvent;

/// 文本化思考块的首行标记。同时是历史剥离的识别依据，修改需保持两侧一致。
pub(crate) const THOUGHT_HEADER: &str = "> 💭 Thinking\n";

static THINKING_AS_TEXT: AtomicBool = AtomicBool::new(false);

/// 启动时由 main 根据配置设置（进程级，与 `set_client_token_passthrough` 同模式）
pub fn set_thinking_as_text(enabled: bool) {
    THINKING_AS_TEXT.store(enabled, Ordering::Relaxed);
}

/// 配置是否开启了思考文本化
pub fn thinking_as_text_enabled() -> bool {
    THINKING_AS_TEXT.load(Ordering::Relaxed)
}

/// 仅对 Claude Code 客户端生效：OpenAI 兼容端点等其他调用方需要保留原生 thinking 语义
pub fn is_claude_code_client(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ua| ua.starts_with("claude-cli") || ua.starts_with("claude-code"))
}

/// 剥离历史 text 块中由本模块渲染的思考前缀。
///
/// 仅当文本以 [`THOUGHT_HEADER`] 开头时处理：移除开头连续以 `>` 起始的行
/// （渲染时每行都带 `> ` 前缀，含空行），保留其后的正文。无前缀时原样返回。
pub(crate) fn strip_rendered_thinking(text: &str) -> &str {
    if !text.starts_with(THOUGHT_HEADER) {
        return text;
    }
    let mut rest = text;
    while rest.starts_with('>') {
        match rest.find('\n') {
            Some(i) => rest = &rest[i + 1..],
            None => return "",
        }
    }
    rest
}

/// 把 thinking 块事件改写为 text 块事件的有状态转换器
#[derive(Debug, Default)]
pub struct ThinkingTextRewriter {
    /// 已被改写为 text 的块索引
    indices: HashSet<i32>,
    /// 已输出首行标记的块
    header_sent: HashSet<i32>,
    /// 各块当前是否位于行首（决定是否补 `> ` 前缀）
    at_line_start: HashMap<i32, bool>,
}

impl ThinkingTextRewriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// 改写一批事件；非 thinking 块事件原样透传
    pub fn rewrite(&mut self, events: Vec<SseEvent>) -> Vec<SseEvent> {
        let mut out = Vec::with_capacity(events.len());
        for event in events {
            let index = event
                .data
                .get("index")
                .and_then(|v| v.as_i64())
                .map(|i| i as i32);
            match (event.event.as_str(), index) {
                ("content_block_start", Some(i))
                    if event.data["content_block"]["type"] == "thinking" =>
                {
                    self.indices.insert(i);
                    self.at_line_start.insert(i, true);
                    out.push(SseEvent::new(
                        "content_block_start",
                        json!({
                            "type": "content_block_start",
                            "index": i,
                            "content_block": {"type": "text", "text": ""}
                        }),
                    ));
                }
                ("content_block_delta", Some(i)) if self.indices.contains(&i) => {
                    match event.data["delta"]["type"].as_str() {
                        Some("thinking_delta") => {}
                        // signature_delta 对 text 块无意义，丢弃
                        Some("signature_delta") => continue,
                        // 已是改写后的 text_delta 等：原样透传（幂等）
                        _ => {
                            out.push(event);
                            continue;
                        }
                    }
                    let text = event.data["delta"]["thinking"].as_str().unwrap_or("");
                    if text.is_empty() {
                        continue;
                    }
                    let mut rendered = String::new();
                    if self.header_sent.insert(i) {
                        rendered.push_str(THOUGHT_HEADER);
                    }
                    let at_start = self.at_line_start.entry(i).or_insert(true);
                    for ch in text.chars() {
                        if *at_start {
                            rendered.push_str("> ");
                            *at_start = false;
                        }
                        rendered.push(ch);
                        if ch == '\n' {
                            *at_start = true;
                        }
                    }
                    out.push(SseEvent::new(
                        "content_block_delta",
                        json!({
                            "type": "content_block_delta",
                            "index": i,
                            "delta": {"type": "text_delta", "text": rendered}
                        }),
                    ));
                }
                ("content_block_stop", Some(i)) if self.indices.remove(&i) => {
                    self.header_sent.remove(&i);
                    self.at_line_start.remove(&i);
                    out.push(event);
                }
                _ => out.push(event),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start(i: i32, ty: &str) -> SseEvent {
        SseEvent::new(
            "content_block_start",
            json!({"type":"content_block_start","index":i,"content_block":{"type":ty}}),
        )
    }
    fn delta(i: i32, kind: &str, field: &str, v: &str) -> SseEvent {
        SseEvent::new(
            "content_block_delta",
            json!({"type":"content_block_delta","index":i,"delta":{"type":kind,(field):v}}),
        )
    }
    fn stop(i: i32) -> SseEvent {
        SseEvent::new(
            "content_block_stop",
            json!({"type":"content_block_stop","index":i}),
        )
    }

    fn text_of(events: &[SseEvent]) -> String {
        events
            .iter()
            .filter_map(|e| e.data["delta"]["text"].as_str())
            .collect()
    }

    #[test]
    fn thinking_block_becomes_quoted_text_block() {
        let mut r = ThinkingTextRewriter::new();
        let mut out = r.rewrite(vec![start(0, "thinking")]);
        out.extend(r.rewrite(vec![
            delta(0, "thinking_delta", "thinking", "first line\nsec"),
            delta(0, "thinking_delta", "thinking", "ond\n\nthird"),
            delta(0, "signature_delta", "signature", "SIG"),
            stop(0),
        ]));
        assert_eq!(out[0].data["content_block"]["type"], "text");
        assert_eq!(
            text_of(&out),
            "> 💭 Thinking\n> first line\n> second\n> \n> third"
        );
        assert!(
            out.iter()
                .all(|e| e.data["delta"]["type"] != "thinking_delta"
                    && e.data["delta"]["type"] != "signature_delta"),
            "不得残留 thinking/signature delta"
        );
        assert_eq!(out.last().unwrap().event, "content_block_stop");
    }

    #[test]
    fn non_thinking_blocks_pass_through_untouched() {
        let mut r = ThinkingTextRewriter::new();
        let input = vec![
            start(1, "text"),
            delta(1, "text_delta", "text", "hi"),
            stop(1),
            start(2, "tool_use"),
            delta(2, "input_json_delta", "partial_json", "{}"),
        ];
        let out = r.rewrite(input.clone());
        assert_eq!(out.len(), input.len());
        for (a, b) in out.iter().zip(input.iter()) {
            assert_eq!(a.data, b.data);
        }
    }

    #[test]
    fn empty_thinking_emits_no_header() {
        let mut r = ThinkingTextRewriter::new();
        let out = r.rewrite(vec![
            start(0, "thinking"),
            delta(0, "thinking_delta", "thinking", ""),
            delta(0, "signature_delta", "signature", "SIG"),
            stop(0),
        ]);
        assert_eq!(out.len(), 2, "仅剩 start 与 stop");
        assert_eq!(text_of(&out), "");
    }

    #[test]
    fn strip_removes_rendered_prefix_separate_and_merged() {
        let rendered = "> 💭 Thinking\n> a\n> \n> b";
        assert_eq!(strip_rendered_thinking(rendered), "");
        // 客户端把相邻 text 块合并的情形
        let merged = "> 💭 Thinking\n> a\n> \n> b\nanswer";
        assert_eq!(strip_rendered_thinking(merged), "answer");
        // 普通文本（含普通引用）不受影响
        assert_eq!(strip_rendered_thinking("> quote\nx"), "> quote\nx");
        assert_eq!(strip_rendered_thinking("plain"), "plain");
    }

    #[test]
    fn rendered_then_stripped_roundtrip() {
        let mut r = ThinkingTextRewriter::new();
        let mut out = r.rewrite(vec![start(0, "thinking")]);
        out.extend(r.rewrite(vec![
            delta(0, "thinking_delta", "thinking", "想一想\n\n再想想\n"),
            stop(0),
        ]));
        assert_eq!(strip_rendered_thinking(&text_of(&out)), "");
    }

    #[test]
    fn stream_context_renders_native_reasoning_as_text_blocks() {
        use crate::anthropic::stream::StreamContext;
        use crate::kiro::model::events::{AssistantResponseEvent, Event, ReasoningContentEvent};

        let reasoning = |text: &str, sig: &str| {
            Event::ReasoningContent(
                serde_json::from_value::<ReasoningContentEvent>(
                    json!({"text": text, "signature": sig}),
                )
                .unwrap(),
            )
        };
        let answer = Event::AssistantResponse(
            serde_json::from_value::<AssistantResponseEvent>(json!({"content": "最终答案"}))
                .unwrap(),
        );

        for enabled in [false, true] {
            let mut ctx = StreamContext::new_with_thinking("claude-sonnet-4-6", 100, true)
                .with_thinking_as_text(enabled);
            let mut events = ctx.generate_initial_events();
            events.extend(ctx.process_kiro_event(&reasoning("第一行\n第二", "")));
            events.extend(ctx.process_kiro_event(&reasoning("行", "SIG")));
            events.extend(ctx.process_kiro_event(&answer));
            events.extend(ctx.generate_final_events());

            let has_thinking = events.iter().any(|e| {
                e.data["content_block"]["type"] == "thinking"
                    || e.data["delta"]["type"] == "thinking_delta"
                    || e.data["delta"]["type"] == "signature_delta"
            });
            let all_text: String = events
                .iter()
                .filter_map(|e| e.data["delta"]["text"].as_str())
                .collect();
            if enabled {
                assert!(!has_thinking, "开启后不应再出现 thinking 块/delta");
                assert_eq!(all_text, "> 💭 Thinking\n> 第一行\n> 第二行最终答案");
            } else {
                assert!(has_thinking, "关闭时保持原生 thinking 块");
                assert_eq!(all_text, "最终答案");
            }
        }
    }

    #[test]
    fn claude_code_user_agent_detection() {
        let mut h = axum::http::HeaderMap::new();
        assert!(!is_claude_code_client(&h));
        h.insert(
            axum::http::header::USER_AGENT,
            "claude-cli/2.1.231 (external, cli)".parse().unwrap(),
        );
        assert!(is_claude_code_client(&h));
        h.insert(axum::http::header::USER_AGENT, "codex/1.0".parse().unwrap());
        assert!(!is_claude_code_client(&h));
    }
}
