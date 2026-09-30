//! converter 测试：tool_result 内图片块的提取与转发（message.rs）
#![cfg(test)]

use super::super::message::process_message_content;
use crate::anthropic::types::MessagesRequest;
use crate::kiro::model::requests::conversation::Message;

fn image_block(media_type: &str, data: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "image",
        "source": {"type": "base64", "media_type": media_type, "data": data}
    })
}

fn tool_result_content(blocks: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!([{
        "type": "tool_result",
        "tool_use_id": "toolu_01TEST",
        "content": blocks,
    }])
}

#[test]
fn tool_result_image_is_lifted_to_message_images() {
    // pi 的实际形态：tool_result 内 [text + image]
    let content = tool_result_content(vec![
        serde_json::json!({"type": "text", "text": "Read image file [image/png]"}),
        image_block("image/png", "aVBoRw=="),
    ]);

    let (text, images, tool_results) = process_message_content(&content).expect("转换应成功");

    assert_eq!(images.len(), 1, "tool_result 内图片应提取到消息级 images");
    assert_eq!(images[0].format, "png");
    assert_eq!(images[0].source.bytes, "aVBoRw==");
    assert_eq!(tool_results.len(), 1);
    let tr = &tool_results[0].content[0];
    let tr_text = tr.get("text").and_then(|v| v.as_str()).unwrap_or_default();
    assert!(
        tr_text.contains("Read image file [image/png]"),
        "原文本应保留，实际: {tr_text}"
    );
    assert!(
        tr_text.contains("[image attached]"),
        "应追加图片占位标记，实际: {tr_text}"
    );
    assert!(text.is_empty(), "工具结果文本不应泄漏到消息正文");
}

#[test]
fn tool_result_image_only_gets_placeholder_text() {
    // 纯图片 tool_result：文本为占位词，避免 Kiro 侧空内容
    let content = tool_result_content(vec![image_block("image/jpeg", "QUJD")]);

    let (text, images, tool_results) = process_message_content(&content).expect("转换应成功");

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].format, "jpeg");
    let tr_text = tool_results[0].content[0]
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(tr_text, "[image attached]");
    assert!(text.is_empty());
}

#[test]
fn tool_result_multiple_images_counted_in_marker() {
    let content = tool_result_content(vec![
        image_block("image/png", "QQ=="),
        image_block("image/gif", "Qg=="),
    ]);

    let (_, images, tool_results) = process_message_content(&content).expect("转换应成功");

    assert_eq!(images.len(), 2);
    let tr_text = tool_results[0].content[0]
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(tr_text, "[2 images attached]");
}

#[test]
fn tool_result_text_only_unchanged() {
    let content = tool_result_content(vec![serde_json::json!({
        "type": "text", "text": "plain output"
    })]);

    let (_, images, tool_results) = process_message_content(&content).expect("转换应成功");

    assert!(images.is_empty(), "无图片时不应产生 images");
    let tr_text = tool_results[0].content[0]
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(tr_text, "plain output", "纯文本结果不应追加占位标记");
}

#[test]
fn tool_result_unsupported_media_type_ignored() {
    // 与顶层 image 块同口径：非 jpeg/png/gif/webp 静默忽略，不追加标记
    let content = tool_result_content(vec![image_block("image/bmp", "QQ==")]);

    let (_, images, tool_results) = process_message_content(&content).expect("转换应成功");

    assert!(images.is_empty());
    let tr_text = tool_results[0].content[0]
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(tr_text.is_empty());
}

#[test]
fn tool_result_string_content_still_works() {
    let content = serde_json::json!([{
        "type": "tool_result",
        "tool_use_id": "toolu_01TEST",
        "content": "string form result",
    }]);

    let (_, images, tool_results) = process_message_content(&content).expect("转换应成功");

    assert!(images.is_empty());
    let tr_text = tool_results[0].content[0]
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(tr_text, "string form result");
}

/// 端到端：当前消息（最后一条 user）携带图片 tool_result 时，
/// 图片应挂到 Kiro currentMessage 的 images，工具结果文本保留在 context.toolResults
#[test]
fn convert_request_lifts_tool_result_image_on_current_message() {
    use crate::anthropic::types::{Message as AnthropicMessage, Tool};

    let req = MessagesRequest {
        model: "claude-sonnet-4".to_string(),
        max_tokens: 1024,
        messages: vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: serde_json::json!("Read the file test.png."),
            },
            AnthropicMessage {
                role: "assistant".to_string(),
                content: serde_json::json!([{
                    "type": "tool_use",
                    "id": "toolu_01TEST",
                    "name": "read",
                    "input": {"path": "test.png"}
                }]),
            },
            AnthropicMessage {
                role: "user".to_string(),
                content: serde_json::json!([{
                    "type": "tool_result",
                    "tool_use_id": "toolu_01TEST",
                    "content": [
                        {"type": "text", "text": "Read image file [image/png]"},
                        {"type": "image", "source": {
                            "type": "base64", "media_type": "image/png", "data": "aVBoRw=="
                        }}
                    ]
                }]),
            },
        ],
        stream: false,
        system: None,
        tools: Some(vec![Tool {
            tool_type: None,
            name: "read".to_string(),
            description: "Read a file.".to_string(),
            input_schema: Default::default(),
            max_uses: None,
            defer_loading: None,
        }]),
        tool_choice: None,
        thinking: None,
        output_config: None,
        metadata: None,
    };

    let result = super::super::convert::convert_request(&req).expect("转换应成功");
    let current = &result.conversation_state.current_message.user_input_message;

    assert_eq!(
        current.images.len(),
        1,
        "图片应挂到 currentMessage 的 images"
    );
    assert_eq!(current.images[0].format, "png");
    assert_eq!(current.images[0].source.bytes, "aVBoRw==");

    let tool_results = &current.user_input_message_context.tool_results;
    assert_eq!(tool_results.len(), 1, "tool_result 应保留在 context 中");
    let tr_text = tool_results[0].content[0]
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(tr_text.contains("[image attached]"));
}

/// 端到端：图片 tool_result 位于历史消息时，图片应挂到合并后的历史 user 消息
#[test]
fn convert_request_lifts_tool_result_image_in_history() {
    use crate::anthropic::types::{Message as AnthropicMessage, Tool};

    let req = MessagesRequest {
        model: "claude-sonnet-4".to_string(),
        max_tokens: 1024,
        messages: vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: serde_json::json!("Read the file test.png."),
            },
            AnthropicMessage {
                role: "assistant".to_string(),
                content: serde_json::json!([{
                    "type": "tool_use",
                    "id": "toolu_01TEST",
                    "name": "read",
                    "input": {"path": "test.png"}
                }]),
            },
            AnthropicMessage {
                role: "user".to_string(),
                content: serde_json::json!([{
                    "type": "tool_result",
                    "tool_use_id": "toolu_01TEST",
                    "content": [
                        {"type": "text", "text": "Read image file [image/png]"},
                        {"type": "image", "source": {
                            "type": "base64", "media_type": "image/png", "data": "aVBoRw=="
                        }}
                    ]
                }]),
            },
            AnthropicMessage {
                role: "user".to_string(),
                content: serde_json::json!("What do you see in the image?"),
            },
        ],
        stream: false,
        system: None,
        tools: Some(vec![Tool {
            tool_type: None,
            name: "read".to_string(),
            description: "Read a file.".to_string(),
            input_schema: Default::default(),
            max_uses: None,
            defer_loading: None,
        }]),
        tool_choice: None,
        thinking: None,
        output_config: None,
        metadata: None,
    };

    let result = super::super::convert::convert_request(&req).expect("转换应成功");

    let history_images: Vec<_> = result
        .conversation_state
        .history
        .iter()
        .filter_map(|m| match m {
            Message::User(u) => Some(u.user_input_message.images.clone()),
            Message::Assistant(_) => None,
        })
        .flatten()
        .collect();

    assert_eq!(
        history_images.len(),
        1,
        "历史中的 tool_result 图片应挂到合并后的 user 消息 images"
    );
    assert_eq!(history_images[0].format, "png");

    // 当前消息（最后的提问）不应重复携带该图片
    let current = &result.conversation_state.current_message.user_input_message;
    assert!(current.images.is_empty(), "当前消息无图片时不应携带 images");
}
