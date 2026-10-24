// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 消息内容解析：文本/图片/tool_result 提取

use crate::anthropic::types::ContentBlock;
use crate::kiro::model::requests::conversation::KiroImage;
use crate::kiro::model::requests::tool::ToolResult;

use super::pdf::extract_pdf_text_from_base64;
use super::result::ConversionError;

/// 处理消息内容，提取文本、图片和工具结果
pub(super) fn process_message_content(
    content: &serde_json::Value,
) -> Result<(String, Vec<KiroImage>, Vec<ToolResult>), ConversionError> {
    process_message_content_with_tool_filter(content, 0, |_| true)
}

/// 过滤工具结果时，连同其内嵌图片一起过滤，保留顶层图片的原始顺序。
///
/// `image_offset` 是同一条 Kiro 消息里排在本内容之前的图片数（合并多条 user 消息时非 0），
/// 用于让工具图片的序号等于它在最终 `images` 里的位置。
pub(super) fn process_message_content_with_tool_filter(
    content: &serde_json::Value,
    image_offset: usize,
    mut keep_tool_result: impl FnMut(&str) -> bool,
) -> Result<(String, Vec<KiroImage>, Vec<ToolResult>), ConversionError> {
    let mut text_parts = Vec::new();
    let mut images = Vec::new();
    let mut tool_results = Vec::new();

    match content {
        serde_json::Value::String(s) if !s.trim().is_empty() => {
            text_parts.push(s.clone());
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                match serde_json::from_value::<ContentBlock>(item.clone()) {
                    Ok(block) => match block.block_type.as_str() {
                        "text" => {
                            if let Some(text) = block.text
                                && !text.trim().is_empty()
                            {
                                text_parts.push(text);
                            }
                        }
                        "image" => {
                            if let Some(source) = block.source
                                && let Some(format) = get_image_format(&source.media_type)
                            {
                                images.push(KiroImage::from_base64(format, source.data));
                            }
                        }
                        "document" => {
                            if let Some(source) = block.source
                                && source.media_type == "application/pdf"
                            {
                                match extract_pdf_text_from_base64(&source.data) {
                                    Some(text) if !text.is_empty() => {
                                        text_parts.push(format!(
                                                "<document media_type=\"application/pdf\">\n{}\n</document>",
                                                text
                                            ));
                                    }
                                    _ => {
                                        text_parts.push(
                                            "[PDF document attached; text extraction unavailable]"
                                                .to_string(),
                                        );
                                    }
                                }
                            }
                        }
                        "tool_result" => {
                            if let Some(tool_use_id) = block.tool_use_id
                                && keep_tool_result(&tool_use_id)
                            {
                                let result_content = extract_tool_result_content(
                                    &block.content,
                                    &tool_use_id,
                                    image_offset,
                                    &mut images,
                                );
                                let is_error = block.is_error.unwrap_or(false);

                                let mut result = if is_error {
                                    ToolResult::error(&tool_use_id, result_content)
                                } else {
                                    ToolResult::success(&tool_use_id, result_content)
                                };
                                result.status =
                                    Some(if is_error { "error" } else { "success" }.to_string());

                                tool_results.push(result);
                            }
                        }
                        "tool_use" => {
                            // tool_use 在 assistant 消息中处理，这里忽略
                        }
                        other => {
                            tracing::warn!("丢弃未支持的内容块类型: {}", other);
                        }
                    },
                    Err(e) => {
                        tracing::warn!("内容块反序列化失败，块被丢弃: {}", e);
                    }
                }
            }
        }
        _ => {}
    }

    Ok((text_parts.join("\n"), images, tool_results))
}
/// 从 media_type 获取图片格式
pub(super) fn get_image_format(media_type: &str) -> Option<String> {
    match media_type {
        "image/jpeg" => Some("jpeg".to_string()),
        "image/png" => Some("png".to_string()),
        "image/gif" => Some("gif".to_string()),
        "image/webp" => Some("webp".to_string()),
        _ => None,
    }
}

/// 提取工具结果文字，同时保留 Claude Code Read 返回的内嵌图片。
/// 工具结果文字仍放在 toolResults；图片放入同条 user 消息的 images 字段，
/// 并在原工具结果位置留下带序号和 tool_use_id 的说明（`[Image n attached ...]`）。
///
/// 序号 n 是该图片在整条 user 消息 `images` 里的位置（从 1 开始，含顶层图片和
/// 其他工具结果的图片）。Kiro 的 images 是不带标签的平铺列表，模型只能按顺序数图；
/// 若每个工具结果各自从 1 编号（如 1/1、1/1、1/1），并行读多张图时模型无法把图对应到
/// 工具结果。无法转发的图片留下 `[Image omitted ...]` 占位，避免模型把图片丢失
/// 误认为工具返回了空内容。
fn extract_tool_result_content(
    content: &Option<serde_json::Value>,
    tool_use_id: &str,
    image_offset: usize,
    images: &mut Vec<KiroImage>,
) -> String {
    match content {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(arr)) => {
            let mut parts = Vec::new();
            for item in arr {
                if item.get("type").and_then(|v| v.as_str()) == Some("image") {
                    match parse_tool_result_image(item) {
                        Some((format, data)) => {
                            images.push(KiroImage::from_base64(format, data));
                            let n = image_offset + images.len();
                            parts.push(format!(
                                "[Image {n} attached from tool_result {tool_use_id}]"
                            ));
                        }
                        None => parts.push(format!(
                            "[Image omitted from tool_result {tool_use_id}: unsupported or invalid image]"
                        )),
                    }
                } else if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    parts.push(text.to_string());
                }
            }
            parts.join("\n")
        }
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

/// 解析工具结果里的一张图片，返回 `(format, base64 data)`。
/// 需要受支持格式的非空 base64 图片；其余情况返回 `None`。
fn parse_tool_result_image(item: &serde_json::Value) -> Option<(String, String)> {
    match serde_json::from_value::<ContentBlock>(item.clone()) {
        Ok(block) => {
            if let Some(source) = block.source
                && source.source_type == "base64"
                && !source.data.is_empty()
                && let Some(format) = get_image_format(&source.media_type)
            {
                Some((format, source.data))
            } else {
                tracing::warn!("工具结果图片未转发：需要受支持格式的非空 base64 图片");
                None
            }
        }
        Err(_) => {
            // serde 错误可能包含原始载荷，避免把图片数据写入日志。
            tracing::warn!("工具结果图片格式无效，未转发");
            None
        }
    }
}
