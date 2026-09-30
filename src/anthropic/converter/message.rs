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
                            if let Some(tool_use_id) = block.tool_use_id {
                                let (mut result_content, result_images) =
                                    extract_tool_result_parts(&block.content);
                                // Kiro 的 ToolResult 只接受文本 content，图片块无法内嵌。
                                // 图片挂到所在 user 消息的 images 上（与 user 消息图片
                                // 同一通路，Kiro 仅在该字段接受图片），并在工具结果文本
                                // 末尾留占位，让模型知道图片与该工具结果的对应关系
                                if !result_images.is_empty() {
                                    let marker = if result_images.len() == 1 {
                                        "[image attached]".to_string()
                                    } else {
                                        format!("[{} images attached]", result_images.len())
                                    };
                                    if result_content.trim().is_empty() {
                                        result_content = marker;
                                    } else {
                                        result_content.push('\n');
                                        result_content.push_str(&marker);
                                    }
                                    images.extend(result_images);
                                }
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

/// 提取工具结果内容：文本部分拼接为字符串，图片块单独收集
///
/// Kiro 的 ToolResult.content 只接受 `{text}` 形态，不支持内嵌图片；
/// 图片由调用方挂到所在 user 消息的 images 上转发（同 user 消息图片通路）。
/// 与顶层 image 块同一口径：仅支持 base64 + jpeg/png/gif/webp，其余静默忽略。
fn extract_tool_result_parts(content: &Option<serde_json::Value>) -> (String, Vec<KiroImage>) {
    match content {
        Some(serde_json::Value::Array(arr)) => {
            let mut parts = Vec::new();
            let mut images = Vec::new();
            for item in arr {
                if let Ok(block) = serde_json::from_value::<ContentBlock>(item.clone()) {
                    if block.block_type == "image"
                        && let Some(source) = block.source
                        && let Some(format) = get_image_format(&source.media_type)
                    {
                        images.push(KiroImage::from_base64(format, source.data));
                        continue;
                    }
                    if let Some(text) = block.text {
                        parts.push(text);
                    }
                } else if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    parts.push(text.to_string());
                }
            }
            (parts.join("\n"), images)
        }
        Some(serde_json::Value::String(s)) => (s.clone(), Vec::new()),
        Some(v) => (v.to_string(), Vec::new()),
        None => (String::new(), Vec::new()),
    }
}
