// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! web_search 服务端工具识别与剔除、历史占位工具

use crate::anthropic::types::MessagesRequest;
use crate::kiro::model::requests::conversation::Message;
use crate::kiro::model::requests::tool::{InputSchema, Tool, ToolSpecification};

/// Anthropic 服务端 web_search 工具的 `type` 前缀（如 `web_search_20250305`）
const WEB_SEARCH_SERVER_TOOL_TYPE_PREFIX: &str = "web_search_";

/// 是否为 Anthropic 服务端 web_search 工具
///
/// 只按 `type` 识别：服务端工具由 API 侧执行，没有 input_schema；客户端自己实现、
/// 恰好也叫 `web_search` 的普通工具（无 `type` 或 `type: "custom"`）必须原样透传。
pub(crate) fn is_web_search_server_tool(t: &crate::anthropic::types::Tool) -> bool {
    t.tool_type
        .as_deref()
        .is_some_and(|ty| ty.starts_with(WEB_SEARCH_SERVER_TOOL_TYPE_PREFIX))
}

/// 剔除 web_search 服务端工具后的工具列表
///
/// Kiro 不认识服务端工具格式，代理也不代为执行（客户端普遍自带搜索 / 抓取工具）。
/// 没有可剔除的工具时返回 `None`，调用方直接借用原列表，避免深拷贝全部工具定义。
pub(crate) fn strip_web_search_server_tool(
    req: &MessagesRequest,
) -> Option<Vec<crate::anthropic::types::Tool>> {
    let tools = req.tools.as_ref()?;
    if !tools.iter().any(is_web_search_server_tool) {
        return None;
    }
    tracing::debug!("剔除 web_search 服务端工具（上游不支持，代理不代为执行）");
    Some(
        tools
            .iter()
            .filter(|t| !is_web_search_server_tool(t))
            .cloned()
            .collect(),
    )
}

/// 收集历史消息中使用的所有工具名称
pub(super) fn collect_history_tool_names(history: &[Message]) -> Vec<String> {
    let mut tool_names = Vec::new();

    for msg in history {
        if let Message::Assistant(assistant_msg) = msg
            && let Some(ref tool_uses) = assistant_msg.assistant_response_message.tool_uses
        {
            for tool_use in tool_uses {
                if !tool_names.contains(&tool_use.name) {
                    tool_names.push(tool_use.name.clone());
                }
            }
        }
    }

    tool_names
}

/// 为历史中使用但不在 tools 列表中的工具创建占位符定义
/// Kiro API 要求：历史消息中引用的工具必须在 currentMessage.tools 中有定义
pub(super) fn create_placeholder_tool(name: &str) -> Tool {
    Tool {
        tool_specification: ToolSpecification {
            name: name.to_string(),
            description: "Tool used in conversation history".to_string(),
            input_schema: InputSchema::from_json(serde_json::json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": true
            })),
        },
    }
}
