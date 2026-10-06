// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 思考内容「文本化」渲染（opt-in，仅 Claude Code 客户端）
//!
//! Claude Code 携带 `redact-thinking` beta（或默认折叠 thinking）时，界面几乎看不到
//! 流式思考内容，长时间推理期间表现为「卡住」。开启 `thinkingAsText` 后，本模块把出站
//! SSE 中的 thinking 块改写为普通 text 块（逐行流式输出），使思考过程
//! 像 Kiro CLI 一样实时可见。
//!
//! 每行内容用 ANSI dim（变暗）转义包裹，接近 Kiro CLI 的灰色思考文字；
//! 是否生效取决于客户端是否放行转义字符。首行为「💭 Thinking」标记，
//! 不带 markdown 引用前缀（客户端不显示竖线）。
//!
//! 代价：这些文本块会被客户端当作 assistant 正文存入对话历史并随后续请求回传。
//! 回传时由 `converter::history::convert_assistant_message` 识别 [`THOUGHT_HEADER`]
//! 并剥离标记行，思考正文按普通助手文本保留回传（上下文略增，已确认接受）。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;

use super::state::SseEvent;

/// 文本化思考块的首行标记（不含换行符）。同时是历史剥离的识别依据，修改需保持两侧一致。
/// 不带 `> ` 引用前缀（客户端渲染时无竖线）；历史回传仅按此标记行识别并剥离。
pub(crate) const THOUGHT_HEADER: &str = "💭 Thinking";

/// ANSI：变暗开始 / 复位（dim 样式，仅包在每行内容两侧，不跨行）
const DIM_ON: &str = "\x1b[2m";
const DIM_OFF: &str = "\x1b[0m";

static THINKING_AS_TEXT: AtomicBool = AtomicBool::new(false);

/// 启动时由 main 根据配置设置（进程级，与 `set_client_token_passthrough` 同模式）
pub fn set_thinking_as_text(enabled: bool) {
    THINKING_AS_TEXT.store(enabled, Ordering::Relaxed);
}

/// 配置是否开启了思考文本化
pub fn thinking_as_text_enabled() -> bool {
    THINKING_AS_TEXT.load(Ordering::Relaxed)
}

/// 单行文本是否为渲染出的思考标记行（兼容 dim 转义包裹）
fn is_rendered_header_line(line: &str) -> bool {
    line.replace(DIM_ON, "").replace(DIM_OFF, "") == THOUGHT_HEADER
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
/// 仅当文本首行是 [`THOUGHT_HEADER`]（允许被 dim 转义包裹）时处理：移除该标记行，
/// 保留其后的正文。无标记时原样返回。
pub(crate) fn strip_rendered_thinking(text: &str) -> &str {
    match text.split_once('\n') {
        // 首行即标记行（允许被 dim 转义包裹）时剥掉，其余原样返回
        Some((first, rest)) if is_rendered_header_line(first) => rest,
        _ => text,
    }
}

/// 把 thinking 块事件改写为 text 块事件的有状态转换器
#[derive(Debug, Default)]
pub struct ThinkingTextRewriter {
    /// 已被改写为 text 的块索引
    indices: HashSet<i32>,
    /// 已输出首行标记的块
    header_sent: HashSet<i32>,
    /// 各块当前是否位于行首（决定是否补 ANSI dim 起始转义，避免样式跨行）
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
                        // 首行：dim(💭 Thinking)，即 THOUGHT_HEADER + 换行
                        rendered.push_str(DIM_ON);
                        rendered.push_str(THOUGHT_HEADER);
                        rendered.push_str(DIM_OFF);
                        rendered.push('\n');
                    }
                    let at_start = self.at_line_start.entry(i).or_insert(true);
                    for ch in text.chars() {
                        if *at_start {
                            rendered.push_str(DIM_ON);
                            *at_start = false;
                        }
                        if ch == '\n' {
                            // 换行前复位，样式不跨行
                            rendered.push_str(DIM_OFF);
                            rendered.push('\n');
                            *at_start = true;
                        } else {
                            rendered.push(ch);
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
                    // 块在行中间结束时补一个复位，避免 dim 样式泄漏到后续正文
                    if self.at_line_start.remove(&i) == Some(false) {
                        out.push(SseEvent::new(
                            "content_block_delta",
                            json!({
                                "type": "content_block_delta",
                                "index": i,
                                "delta": {"type": "text_delta", "text": DIM_OFF}
                            }),
                        ));
                    }
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

    /// 去掉 dim 转义，便于断言可见文本
    fn plain(s: &str) -> String {
        s.replace(DIM_ON, "").replace(DIM_OFF, "")
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
            plain(&text_of(&out)),
            "💭 Thinking\nfirst line\nsecond\n\nthird"
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
    fn strip_removes_only_header_and_keeps_thinking_body() {
        // 仅剥「💭 Thinking」标记行，思考正文保留（作为普通助手文本回传上游）
        let rendered = "💭 Thinking\na\n\nb";
        assert_eq!(strip_rendered_thinking(rendered), "a\n\nb");
        // 客户端把相邻 text 块合并的情形
        let merged = "💭 Thinking\na\n\nb\nanswer";
        assert_eq!(strip_rendered_thinking(merged), "a\n\nb\nanswer");
        // 普通文本（含普通引用）不受影响
        assert_eq!(strip_rendered_thinking("> quote\nx"), "> quote\nx");
        assert_eq!(strip_rendered_thinking("plain"), "plain");
    }

    #[test]
    fn rendered_thinking_body_survives_history_strip() {
        let mut r = ThinkingTextRewriter::new();
        let mut out = r.rewrite(vec![start(0, "thinking")]);
        out.extend(r.rewrite(vec![
            delta(0, "thinking_delta", "thinking", "想一想\n\n再想想\n"),
            stop(0),
        ]));
        let rendered = plain(&text_of(&out));
        let stripped = strip_rendered_thinking(&rendered);
        assert_eq!(stripped, "想一想\n\n再想想\n");
        // 尾部换行与中间空行原样保留，不得被裁剪
        assert!(stripped.ends_with('\n'));
        assert_eq!(stripped.lines().count(), 3);
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
                assert_eq!(plain(&all_text), "💭 Thinking\n第一行\n第二行最终答案");
            } else {
                assert!(has_thinking, "关闭时保持原生 thinking 块");
                assert_eq!(all_text, "最终答案");
            }
        }
    }

    #[test]
    fn dim_style_wraps_each_line_and_resets_at_block_end() {
        let mut r = ThinkingTextRewriter::new();
        let mut out = r.rewrite(vec![start(0, "thinking")]);
        out.extend(r.rewrite(vec![
            delta(0, "thinking_delta", "thinking", "ab\n\ncd"),
            stop(0),
        ]));
        let text = text_of(&out);
        assert_eq!(
            text,
            "\x1b[2m💭 Thinking\x1b[0m\n\x1b[2mab\x1b[0m\n\x1b[2m\x1b[0m\n\x1b[2mcd\x1b[0m"
        );
        // 行中结束 → 末尾补复位；stop 事件仍在最后
        assert_eq!(out.last().unwrap().event, "content_block_stop");
        // dim 渲染的内容同样能被历史剥离识别（仅剥标记行，正文保留）
        let body = &text[text.find('\n').unwrap() + 1..];
        assert_eq!(strip_rendered_thinking(&text), body);
        assert_eq!(
            strip_rendered_thinking(&format!("{text}\nanswer")),
            format!("{body}\nanswer")
        );
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
