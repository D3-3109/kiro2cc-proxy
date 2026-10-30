// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 最终事件测试（自 handlers/tests.rs 拆出，纯代码搬移）
#[cfg(test)]
mod tests {

    use crate::anthropic::stream::StreamContext;
    use crate::kiro::model::events::Event;

    #[test]
    fn test_generate_final_events_message_stop_exactly_once() {
        // 收尾恰好一次：generate_final_events 的 message_stop 由 message_ended
        // 门控——首次调用补发 message_stop，重复调用不再产生（防客户端双 message_stop）
        let mut ctx = StreamContext::new_with_thinking("claude-sonnet-4", 1000, false);
        ctx.process_kiro_event(&Event::AssistantResponse(
            serde_json::from_str(r#"{"content":"回答正文"}"#).unwrap(),
        ));
        let final_events = ctx.generate_final_events();
        let stops: Vec<_> = final_events
            .iter()
            .filter(|e| e.event == "message_stop")
            .collect();
        assert_eq!(stops.len(), 1, "首次收尾应恰好发出 1 个 message_stop");
        let repeated = ctx.generate_final_events();
        assert!(
            !repeated.iter().any(|e| e.event == "message_stop"),
            "重复收尾不得再次发出 message_stop"
        );
    }
}
