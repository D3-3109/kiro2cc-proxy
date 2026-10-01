// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! convert_request 主入口与触发类型判定

use uuid::Uuid;

use crate::anthropic::types::MessagesRequest;
use crate::kiro::model::requests::conversation::{
    ConversationState, CurrentMessage, Message, UserInputMessage, UserInputMessageContext,
};

use super::fields::build_additional_model_request_fields;
use super::history::build_history;
use super::message::process_message_content;
use super::model::map_model;
use super::prompt::{append_output_format_instruction, append_recent_knowledge_hints};
use super::result::{ConversionError, ConversionResult};
use super::session::{
    derive_agent_continuation_id, derive_fallback_conversation_id, extract_session_id,
    is_compact_request,
};
use super::tools::{convert_tools, remove_orphaned_tool_uses, validate_tool_pairing};
use super::websearch::{
    collect_history_tool_names, create_placeholder_tool, create_web_search_bridge_tool,
    split_web_search_tool,
};

/// 将 Anthropic 请求转换为 Kiro 请求
pub fn convert_request(req: &MessagesRequest) -> Result<ConversionResult, ConversionError> {
    // 1. 映射模型
    let model_id = map_model(&req.model)
        .ok_or_else(|| ConversionError::UnsupportedModel(req.model.clone()))?;

    // 2. 检查消息列表
    if req.messages.is_empty() {
        return Err(ConversionError::EmptyMessages);
    }

    // 2.5. 预处理 prefill：Kiro 不支持末尾 assistant prefill
    let messages: &[_] = if req.messages.last().is_some_and(|m| m.role != "user") {
        tracing::info!("检测到末尾 assistant 消息（prefill），静默丢弃");
        let last_user_idx = req
            .messages
            .iter()
            .rposition(|m| m.role == "user")
            .ok_or(ConversionError::EmptyMessages)?;
        &req.messages[..=last_user_idx]
    } else {
        &req.messages
    };

    // 3. 生成会话 ID 和代理 ID
    // 优先级：
    //   1. metadata.user_id 中的 session UUID（Claude Code 标准格式 / 纯 UUID 直通）
    //   2. system + 工具名 + 首条消息的 SHA-256 派生（含裸请求，全部走首条消息 seed）
    //   3. 完全随机 UUID（仅防御路径：messages 为空时 fallback 不可用，实际不可达）
    let (conversation_id, id_source) = req
        .metadata
        .as_ref()
        .and_then(|m| m.user_id.as_ref())
        .and_then(|user_id| extract_session_id(user_id))
        .map(|id| (id, "metadata"))
        .or_else(|| derive_fallback_conversation_id(req).map(|id| (id, "fallback")))
        .unwrap_or_else(|| (Uuid::new_v4().to_string(), "random"));
    // agentContinuationId 基于 conversationId 派生，保持同一会话内稳定
    // 这样 Kiro 后端能识别连续请求，对历史消息做跨请求 prompt caching
    let agent_continuation_id = derive_agent_continuation_id(&conversation_id);
    tracing::info!(
        "[session] conversationId={} agentContinuationId={} source={} (同一会话的连续请求这两个值应保持不变)",
        conversation_id,
        agent_continuation_id,
        id_source
    );

    // 4. 确定触发类型
    let chat_trigger_type = determine_chat_trigger_type(req);

    // 5. 处理最后一条消息作为 current_message（经过 prefill 预处理，末尾必为 user）
    let last_message = messages.last().unwrap();
    let (text_content, images, tool_results) = process_message_content(&last_message.content)?;
    let text_content = append_recent_knowledge_hints(text_content);
    let text_content = append_output_format_instruction(text_content, &req.output_config);

    // 6. 转换工具定义
    // web_search server tool 不发给 Kiro（Kiro 不识别该格式），由 handlers 层桥接到
    // Kiro MCP 执行；未命中时 split_web_search_tool 返回 None，与直接转换逐字节一致。
    // 借用实现：未命中路径直接引用 req.tools，避免每个请求深拷贝全部工具定义
    let split_owned;
    let (web_search_max_uses, split_tools) = match split_web_search_tool(req) {
        Some((max_uses, ordinary)) => {
            split_owned = Some(ordinary);
            (Some(max_uses), &split_owned)
        }
        None => (None, &req.tools),
    };
    let mut tools = convert_tools(split_tools);

    // 6b. web_search server tool 命中时注入桥接工具定义（普通 tool spec 格式）
    // 剔除 server tool 后若不在 context.tools 中补一份 Kiro 可识别的普通定义，
    // 模型不知道自己具备搜索能力，不会发起 web_search toolUse，桥接永不触发
    // （表现为模型回复"我没有 websearch 工具可用"）。
    // collect_history_tool_names 会跳过历史中的 web_search toolUse，因此该
    // 定义不会被占位符逻辑重复生成。
    // 有效 max_uses（未声明默认 5）为 0 时桥接层不会截获任何轮次，注入反而
    // 会让 toolUse 按普通 tool_use 透传给客户端，故跳过注入。
    if web_search_max_uses.is_some_and(|max_uses| max_uses.unwrap_or(5) > 0) {
        tools.push(create_web_search_bridge_tool());
    }

    // 7. 构建历史消息（需要先构建，以便收集历史中使用的工具）
    let mut history = build_history(req, messages, &model_id, &conversation_id)?;

    // 8. 验证并过滤 tool_use/tool_result 配对
    // 移除孤立的 tool_result（没有对应的 tool_use）
    // 同时返回孤立的 tool_use_id 集合，用于后续清理
    let (validated_tool_results, orphaned_tool_use_ids) =
        validate_tool_pairing(&history, &tool_results);

    // 9. 从历史中移除孤立的 tool_use（Kiro API 要求 tool_use 必须有对应的 tool_result）
    remove_orphaned_tool_uses(&mut history, &orphaned_tool_use_ids);

    // 10. 收集历史中使用的工具名称，为缺失的工具生成占位符定义
    // Kiro API 要求：历史消息中引用的工具必须在 tools 列表中有定义
    // 注意：Kiro 匹配工具名称时忽略大小写，所以这里也需要忽略大小写比较
    let history_tool_names = collect_history_tool_names(&history);
    let existing_tool_names: std::collections::HashSet<_> = tools
        .iter()
        .map(|t| t.tool_specification.name.to_lowercase())
        .collect();

    for tool_name in history_tool_names {
        if !existing_tool_names.contains(&tool_name.to_lowercase()) {
            tools.push(create_placeholder_tool(&tool_name));
        }
    }

    // 11b. 构建 UserInputMessageContext —— tools 完整定义直接写入 context.tools（与 Kiro 官方 CLI 一致）
    let mut context = UserInputMessageContext::new();
    if !tools.is_empty() {
        context.tools = tools;
    }
    if !validated_tool_results.is_empty() {
        context = context.with_tool_results(validated_tool_results);
    }

    // 12. 构建当前消息
    // 保留文本内容，即使有工具结果也不丢弃用户文本
    // 空 content 兜底：Kiro 后端不接受空字符串。
    // 注意：此前用 "Continue" 会让模型把 tool_result-only 的 user 消息误判为
    // "用户让我继续" → 仅简短回 "已完成" 不复述工具结果（如 LS 输出）。
    // 改用中性提示词，明确告知"上方为工具结果"，让模型基于结果回复用户。
    let content = if text_content.is_empty() {
        "(tool result above)".to_string()
    } else {
        text_content
    };

    let mut user_input = UserInputMessage::new(content, &model_id)
        .with_context(context)
        .with_origin("AI_EDITOR");

    if !images.is_empty() {
        user_input = user_input.with_images(images);
    }

    let current_message = CurrentMessage::new(user_input);

    // 13. 构建 ConversationState
    let agent_task_type = determine_agent_task_type(req);
    tracing::debug!("[session] agentTaskType={}", agent_task_type);

    let conversation_state = ConversationState::new(conversation_id)
        .with_agent_continuation_id(agent_continuation_id)
        .with_agent_task_type(agent_task_type)
        .with_chat_trigger_type(chat_trigger_type)
        .with_current_message(current_message)
        .with_history(history);

    log_rule_diagnostics(req, &conversation_state);

    let additional_model_request_fields = build_additional_model_request_fields(req, &model_id);
    let is_compact = is_compact_request(messages);
    if is_compact {
        tracing::info!(
            "[COMPACT] 检测到 /compact 压缩请求，将使用压缩超时（见 provider::COMPACT_TIMEOUT_SECS）"
        );
    }

    Ok(ConversionResult {
        conversation_state,
        additional_model_request_fields,
        is_compact_request: is_compact,
        web_search_max_uses,
        thinking_adaptive_requested: req
            .thinking
            .as_ref()
            .map(|t| t.thinking_type == "adaptive")
            .unwrap_or(false),
    })
}

// 临时诊断：定位后删除；只记录元数据，不输出提示词或工具参数。
fn log_rule_diagnostics(req: &MessagesRequest, state: &ConversationState) {
    let span = tracing::info_span!(
        "rule_diag",
        session = %state.conversation_id,
        diagnostic_id = %Uuid::new_v4(),
        message_count = req.messages.len()
    );
    let _guard = span.enter();
    let output_texts: Vec<&str> = state
        .history
        .iter()
        .filter_map(|message| match message {
            Message::User(user) => Some(user.user_input_message.content.as_str()),
            Message::Assistant(_) => None,
        })
        .chain(std::iter::once(
            state.current_message.user_input_message.content.as_str(),
        ))
        .collect();
    let watched_texts: Vec<_> = req
        .system
        .iter()
        .flatten()
        .map(|block| ("system", block.text.as_str()))
        .chain(
            req.messages
                .iter()
                .filter(|message| message.role == "user")
                .flat_map(|message| {
                    message.content.as_str().into_iter().chain(
                        message
                            .content
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|block| {
                                block.get("type").and_then(|v| v.as_str()) == Some("text")
                            })
                            .filter_map(|block| block.get("text").and_then(|v| v.as_str())),
                    )
                })
                .map(|text| ("user", text)),
        )
        .filter(|(source, text)| {
            *source == "system"
                || text.contains("<system-reminder>")
                || text.contains("00-change-gate.md")
        })
        .collect();
    tracing::info!(
        watched_text_blocks = watched_texts.len(),
        input_tool_count = req.tools.as_ref().map_or(0, Vec::len),
        tool_choice_present = req.tool_choice.is_some(),
        "[RULE-DIAG] 请求概况"
    );
    for (index, (source, text)) in watched_texts.iter().enumerate() {
        tracing::info!(
            source = *source,
            watched_text_index = index,
            input_chars = text.chars().count(),
            reminder_count = text.matches("<system-reminder>").count(),
            has_cr_rule_file = text.contains("00-change-gate.md"),
            exact_text_present = output_texts.iter().any(|output| output.contains(*text)),
            "[RULE-DIAG] 规则候选文本"
        );
    }
    let output_tools = &state
        .current_message
        .user_input_message
        .user_input_message_context
        .tools;
    for name in ["Agent", "Task", "AskUserQuestion", "ToolSearch"] {
        let input = req
            .tools
            .iter()
            .flatten()
            .find(|tool| tool.name.trim().eq_ignore_ascii_case(name));
        let output = output_tools
            .iter()
            .map(|tool| &tool.tool_specification)
            .find(|tool| tool.name.eq_ignore_ascii_case(name));
        tracing::info!(
            tool = name,
            input_present = input.is_some(),
            output_present = output.is_some(),
            defer_loading = ?input.and_then(|tool| tool.defer_loading),
            input_description_chars = ?input.map(|tool| tool.description.trim().chars().count()),
            output_description_chars = ?output.map(|tool| tool.description.chars().count()),
            description_matches = ?input.zip(output).map(|(a, b)| a.description.trim() == b.description),
            "[RULE-DIAG] 关键工具"
        );
    }
}

/// 确定聊天触发类型
/// "AUTO" 模式可能会导致 400 Bad Request 错误
pub(super) fn determine_chat_trigger_type(_req: &MessagesRequest) -> String {
    "MANUAL".to_string()
}

/// 确定代理任务类型
///
/// - 请求携带任意工具 → "spectask"：触发 Kiro 原生 toolUseEvent 响应
/// - 无工具 → "vibe"
pub(super) fn determine_agent_task_type(req: &MessagesRequest) -> &'static str {
    match &req.tools {
        Some(tools) if !tools.is_empty() => "spectask",
        _ => "vibe",
    }
}
