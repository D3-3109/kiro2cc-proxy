// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 空响应检测与上下文窗口测试（自 stream/tests.rs 拆出，纯代码搬移）
#[cfg(test)]
mod tests {
    use crate::anthropic::stream::{StreamContext, context_window_for_model};

    #[test]
    fn test_empty_response_detected() {
        // 上游完全没发任何内容事件 → is_empty_response 为 true
        let mut ctx = StreamContext::new_with_thinking("test-model", 1, false);
        let _ = ctx.generate_initial_events();
        assert!(
            ctx.is_empty_response(),
            "未收到任何内容事件时应判定为空响应"
        );
    }

    #[test]
    fn test_empty_response_oversized_context_by_threshold() {
        // contextUsage 本地化后所有模型按 1M 窗口，阈值 = 1M * 0.28 = 280_000
        // 大输入(>=28万)的空响应 → 判定为上下文过大
        let big = StreamContext::new_with_thinking("test-model", 300_000, true);
        assert!(
            big.empty_response_is_oversized_context(),
            "input 30万应判定为上下文过大"
        );
        // 小输入的空响应 → 视为偶发，可重试
        let small = StreamContext::new_with_thinking("test-model", 100_000, true);
        assert!(
            !small.empty_response_is_oversized_context(),
            "input 10万不应判定为上下文过大"
        );
    }

    #[test]
    fn test_non_empty_response_not_flagged() {
        // 收到了文本内容 → 不应判定为空响应
        let mut ctx = StreamContext::new_with_thinking("test-model", 1, false);
        let _ = ctx.generate_initial_events();
        ctx.process_assistant_response("hello");
        assert!(!ctx.is_empty_response(), "已产生文本内容时不应判定为空响应");
    }

    #[test]
    fn test_tool_only_response_not_flagged() {
        // 只有工具调用、无文本 → 不是空响应
        let mut ctx = StreamContext::new_with_thinking("test-model", 1, false);
        let _ = ctx.generate_initial_events();
        ctx.process_tool_use(&crate::kiro::model::events::ToolUseEvent {
            name: "Bash".to_string(),
            tool_use_id: "toolu_1".to_string(),
            input: "{}".to_string(),
            stop: true,
        });
        assert!(!ctx.is_empty_response(), "仅有工具调用时不应判定为空响应");
    }

    #[test]
    fn test_near_empty_response_oversized_context_flagged() {
        // 大上下文（>28万）+ 极短输出（< 30 tokens）+ 无工具调用 → 视为退化空响应
        let mut ctx = StreamContext::new_with_thinking("test-model", 300_000, false);
        let _ = ctx.generate_initial_events();
        // 模拟极短输出（40 个非中文字符 = 10 token）
        ctx.output_chars_other = 40;
        assert!(
            ctx.is_empty_response(),
            "大上下文+极短输出应判定为近似空响应"
        );
    }

    #[test]
    fn test_near_empty_response_small_context_not_flagged() {
        // 小上下文 + 极短输出 → 不应判定为空响应（可能是正常的短回复）
        let mut ctx = StreamContext::new_with_thinking("test-model", 50_000, false);
        let _ = ctx.generate_initial_events();
        ctx.output_chars_other = 40;
        assert!(
            !ctx.is_empty_response(),
            "小上下文+极短输出不应判定为空响应"
        );
    }

    #[test]
    fn test_near_empty_response_with_tool_use_not_flagged() {
        // 大上下文 + 极短输出 + 有工具调用 → 不应判定为空响应（工具调用是有效响应）
        let mut ctx = StreamContext::new_with_thinking("test-model", 300_000, false);
        let _ = ctx.generate_initial_events();
        ctx.output_chars_other = 40;
        ctx.state_manager.set_has_tool_use(true);
        assert!(!ctx.is_empty_response(), "有工具调用时不应判定为空响应");
    }

    #[test]
    fn test_context_window_opus_4_6_is_1m() {
        assert_eq!(context_window_for_model("claude-opus-4-6"), 1_000_000);
        assert_eq!(
            context_window_for_model("claude-opus-4-6-thinking"),
            1_000_000
        );
    }

    #[test]
    fn test_context_window_fable_5_is_1m() {
        assert_eq!(context_window_for_model("claude-fable-5"), 1_000_000);
        assert_eq!(
            context_window_for_model("claude-fable-5-thinking"),
            1_000_000
        );
    }

    #[test]
    fn test_context_window_all_models_unified_to_1m() {
        // contextUsage 本地化后所有模型统一返回 1M
        assert_eq!(
            context_window_for_model("claude-haiku-4-5-20251001"),
            1_000_000
        );
        assert_eq!(context_window_for_model("unknown-model"), 1_000_000);
        assert_eq!(context_window_for_model(""), 1_000_000);
    }

    #[test]
    fn test_context_window_existing_branches_unchanged() {
        // 回归：4-7 / 4-8 / sonnet-4-6 仍为 1M
        assert_eq!(context_window_for_model("claude-opus-4-7"), 1_000_000);
        assert_eq!(context_window_for_model("claude-opus-4-8"), 1_000_000);
        assert_eq!(context_window_for_model("claude-sonnet-4-6"), 1_000_000);
    }

    #[test]
    fn test_first_token_marked_on_business_event() {
        // 首个客户端可见产出（AssistantResponse 非空 content）应记录首字时刻
        // 埋点在 stream.rs 按「本帧是否产出非空 SSE 事件」判定后调用
        let mut ctx = StreamContext::new_with_thinking("test-model", 10, false)
            .with_request_started_at(std::time::Instant::now());
        let inner = serde_json::from_str::<crate::kiro::model::events::AssistantResponseEvent>(
            r#"{"content":"hi"}"#,
        )
        .unwrap();
        let event = crate::kiro::model::events::Event::AssistantResponse(inner);
        ctx.mark_first_token(); // 模拟 stream.rs 在非空 SSE 产出处的埋点调用
        ctx.process_kiro_event(&event);
        assert!(ctx.first_token_at.is_some(), "非空产出应计首字");
    }

    #[test]
    fn test_first_token_not_marked_by_non_business_events() {
        // meteringEvent 产出空 SSE 事件，不计首字（stream.rs 埋点按产出非空判定，
        // 元信息帧 / 空输出帧天然产出空事件）；keepalive 在 SSE 层合成，同样不经过埋点
        let mut ctx = StreamContext::new_with_thinking("test-model", 10, false)
            .with_request_started_at(std::time::Instant::now());
        let event = crate::kiro::model::events::Event::Metering(
            crate::kiro::model::events::MeteringEvent {
                usage: 0.5,
                unit_plural: "credits".to_string(),
                unit: String::new(),
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
            },
        );
        assert!(
            ctx.process_kiro_event(&event).is_empty(),
            "metering 帧产出应为空，锁定空产出不计首字的语义"
        );
        assert!(ctx.first_token_at.is_none(), "空产出不应计首字");
    }

    #[test]
    fn test_first_token_mark_is_idempotent() {
        // 多次 mark 仅首次生效
        let mut ctx = StreamContext::new_with_thinking("test-model", 10, false)
            .with_request_started_at(std::time::Instant::now());
        ctx.mark_first_token();
        let first = ctx.first_token_at;
        ctx.mark_first_token();
        assert_eq!(ctx.first_token_at, first);
    }

    #[tokio::test]
    async fn test_record_timing_none_without_started_at() {
        // 未注入起点（旧调用路径）时落库的 first_token_ms / duration_ms 均为 None
        let dir = std::env::temp_dir().join(format!("kiro2cc_ctx_none_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("api_key_usage.json");
        let tracker = std::sync::Arc::new(crate::model::usage::UsageTracker::load(&path).unwrap());
        let mut ctx = StreamContext::new_with_thinking("test-model", 10, false)
            .with_usage_tracking(
                Some(std::sync::Arc::clone(&tracker)),
                Some(1),
                None,
                None,
                None,
            );
        ctx.mark_first_token();
        let _ = ctx.generate_final_events();
        let page = tracker.get_records_paged(1, 1, 10, &std::collections::HashMap::new());
        assert_eq!(page.records.len(), 1);
        assert!(page.records[0].first_token_ms.is_none());
        assert!(page.records[0].duration_ms.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
