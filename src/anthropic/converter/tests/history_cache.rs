//! converter 测试（自 tests.rs 拆分，纯代码搬移）
#![cfg(test)]

use super::super::convert::convert_request;
use super::super::result::ConversionResult;
#[allow(unused_imports)]
use crate::anthropic::types::ContentBlock as _;
use crate::anthropic::types::MessagesRequest;
#[allow(unused_imports)]
use crate::kiro::model::requests::conversation::Message;

#[test]
fn test_system_history_refreshes_when_content_changes() {
    use crate::anthropic::types::{Message as AnthropicMessage, Metadata, SystemMessage};

    let session = Some(Metadata {
        user_id: Some("user_account__session_7b2e9c4d-1a6f-4b8e-9d3c-5f0a2e7b6c11".to_string()),
    });
    let make_req = |system_text: &str| MessagesRequest {
        model: "claude-sonnet-4".to_string(),
        max_tokens: 1024,
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: serde_json::json!("Hello"),
        }],
        stream: false,
        system: Some(vec![SystemMessage {
            text: system_text.to_string(),
        }]),
        tools: None,
        tool_choice: None,
        thinking: None,
        output_config: None,
        metadata: session.clone(),
    };

    let first = convert_request(&make_req("original system prompt")).unwrap();
    let second = convert_request(&make_req("compacted system prompt")).unwrap();

    let history_content = |result: &ConversionResult| -> String {
        let Message::User(history_user) = &result.conversation_state.history[0] else {
            panic!("系统提示应转换为 history user 消息");
        };
        history_user.user_input_message.content.clone()
    };

    assert!(history_content(&first).contains("original system prompt"));
    assert!(history_content(&second).contains("compacted system prompt"));
    assert!(!history_content(&second).contains("original system prompt"));
}

#[test]
fn test_reminders_stay_in_original_messages() {
    use crate::anthropic::types::{Message as AnthropicMessage, Metadata, SystemMessage};

    let req = MessagesRequest {
        model: "claude-sonnet-4".to_string(),
        max_tokens: 1024,
        messages: vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: serde_json::json!("<system-reminder>old reminder</system-reminder>"),
            },
            AnthropicMessage {
                role: "assistant".to_string(),
                content: serde_json::json!("Acknowledged."),
            },
            AnthropicMessage {
                role: "user".to_string(),
                content: serde_json::json!(
                    "<system-reminder>current reminder</system-reminder>Continue."
                ),
            },
        ],
        stream: false,
        system: Some(vec![SystemMessage {
            text: "Follow the user request.".to_string(),
        }]),
        tools: None,
        tool_choice: None,
        thinking: None,
        output_config: None,
        metadata: Some(Metadata {
            user_id: Some("user_account__session_5c8e1d2a-7f4b-4a9c-8e6d-1b3f0a2c9d44".to_string()),
        }),
    };

    let result = convert_request(&req).unwrap();
    let Message::User(history_user) = &result.conversation_state.history[0] else {
        panic!("系统提示应转换为 history user 消息");
    };
    let content = &history_user.user_input_message.content;

    assert_eq!(content, "Follow the user request.");
    let Message::User(original_user) = &result.conversation_state.history[2] else {
        panic!("原始 user 消息应保留在历史中");
    };
    assert_eq!(
        original_user.user_input_message.content,
        "<system-reminder>old reminder</system-reminder>"
    );
    assert_eq!(
        result
            .conversation_state
            .current_message
            .user_input_message
            .content,
        "<system-reminder>current reminder</system-reminder>Continue."
    );
}

fn reminder_request(system: Option<&str>, messages: serde_json::Value) -> MessagesRequest {
    let request = serde_json::from_value(serde_json::json!({
        "model": "claude-sonnet-4",
        "max_tokens": 1024,
        "messages": messages,
        "metadata": {
            "user_id": "user_account__session_23083fc0-9423-4a8b-9f54-28667ec939fe"
        }
    }))
    .unwrap();
    MessagesRequest {
        system: system.map(|text| {
            vec![crate::anthropic::types::SystemMessage {
                text: text.to_string(),
            }]
        }),
        ..request
    }
}

#[test]
fn test_current_reminders_preserve_text_with_or_without_system() {
    let texts = [
        "<system-reminder>z: 修改前确认</system-reminder>\n\
         <system-reminder>a: 修改后调用 Agent 做 CR</system-reminder>\n\
         <system-reminder>z: 修改前确认</system-reminder>\n请修复代码。",
        "<system-reminder>未闭合提醒也不能删除后续内容",
    ];
    for system in [None, Some(""), Some("Follow the user request.")] {
        for text in texts {
            for content in [
                serde_json::json!(text),
                serde_json::json!([{"type": "text", "text": text}]),
            ] {
                let req = reminder_request(
                    system,
                    serde_json::json!([{"role": "user", "content": content}]),
                );
                let result = convert_request(&req).unwrap();
                assert_eq!(
                    result
                        .conversation_state
                        .current_message
                        .user_input_message
                        .content,
                    text
                );
            }
        }
    }
}

#[test]
fn test_rules_survive_read_and_edit_results_without_changing_system_history() {
    let rules =
        "<system-reminder>修改前必须 AskUserQuestion；修改后必须 Agent CR。</system-reminder>";
    for content in [
        serde_json::json!(rules),
        serde_json::json!([{"type": "text", "text": rules}]),
    ] {
        let messages = serde_json::json!([
            {"role": "user", "content": content},
            {"role": "assistant", "content": [
                {"type": "tool_use", "id": "read_1", "name": "Read",
                 "input": {"file_path": "/tmp/example.rs"}}
            ]},
            {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "read_1", "content": "file contents"}
            ]},
            {"role": "assistant", "content": [
                {"type": "tool_use", "id": "edit_1", "name": "Edit",
                 "input": {"file_path": "/tmp/example.rs", "old_string": "old", "new_string": "new"}}
            ]},
            {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "edit_1", "content": "edit completed"}
            ]}
        ]);
        for count in [1, 3, 5] {
            let req = reminder_request(
                Some("Follow the user request."),
                serde_json::json!(&messages.as_array().unwrap()[..count]),
            );
            let state = convert_request(&req).unwrap().conversation_state;
            let Message::User(system_user) = &state.history[0] else {
                panic!("系统历史应保留");
            };
            assert_eq!(
                system_user.user_input_message.content,
                "Follow the user request."
            );
            let serialized = serde_json::to_string(&state).unwrap();
            assert_eq!(serialized.matches(rules).count(), 1);
            if count > 1 {
                let results = &state
                    .current_message
                    .user_input_message
                    .user_input_message_context
                    .tool_results;
                assert_eq!(results.len(), 1);
                assert_eq!(
                    results[0].tool_use_id,
                    if count == 3 { "read_1" } else { "edit_1" }
                );
            }
        }
    }
    let compacted = reminder_request(
        Some("Follow the user request."),
        serde_json::json!([{
            "role": "user",
            "content": "<system-reminder>新的规则</system-reminder>压缩后的上下文"
        }]),
    );
    let state = convert_request(&compacted).unwrap().conversation_state;
    let serialized = serde_json::to_string(&state).unwrap();
    assert!(!serialized.contains(rules));
    assert!(serialized.contains("<system-reminder>新的规则</system-reminder>"));
}

#[test]
fn test_client_workflow_tool_descriptions_are_not_augmented() {
    let description = "修改前必须确认；修改后必须执行独立审查。";
    let req: MessagesRequest = serde_json::from_value(serde_json::json!({
        "model": "claude-sonnet-4",
        "max_tokens": 1024,
        "system": "Follow the user request.",
        "messages": [{"role": "user", "content": "请修复代码。"}],
        "tools": (["Write", "Edit", "Agent", "AskUserQuestion"].map(|name| {
            serde_json::json!({
                "name": name,
                "description": description,
                "input_schema": {"type": "object", "properties": {}}
            })
        }))
    }))
    .unwrap();
    let state = convert_request(&req).unwrap().conversation_state;
    let tools = &state
        .current_message
        .user_input_message
        .user_input_message_context
        .tools;
    assert_eq!(tools.len(), 4);
    for tool in tools {
        assert_eq!(tool.tool_specification.description, description);
    }
}
