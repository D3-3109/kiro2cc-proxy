// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! web_search 桥接 Handler：解析 bridge 查询参数并构建桥接上下文，将客户端 web_search 工具请求转发到 Kiro 侧执行。

use super::stream::stream_interrupted_error_event;
use crate::anthropic::stream::{SseEvent, StreamContext};

use crate::kiro::model::events::Event;
use crate::kiro::model::requests::conversation::{
    AssistantMessage, ConversationState, HistoryAssistantMessage, Message,
};
use crate::kiro::model::requests::kiro::KiroRequest;
use crate::kiro::model::requests::tool::{ToolResult, ToolUseEntry};
use crate::kiro::parser::decoder::EventStreamDecoder;
use bytes::Bytes;
use serde_json::json;
use std::collections::VecDeque;

use super::super::websearch;

#[derive(Debug, Clone)]
pub(crate) struct BridgeContext {
    /// 首次转换的 ConversationState clone（多轮桥接的演进基底，D3）
    pub conversation_state: ConversationState,
    /// 续请求需要同参序列化
    pub profile_arn: Option<String>,
    /// 模型专属请求参数（thinking、output_config、max_tokens）
    pub additional_model_request_fields: Option<serde_json::Value>,
    /// server tool 声明的次数上限（未声明时为 None，桥接层兜底 5）
    pub max_uses: Option<i32>,
    /// call_api_stream / call_mcp_api 均需要；unfold 闭包作用域内不可得，必须随 BridgeContext 携带
    pub bound_ids: Vec<u64>,
    /// 决定续请求的上游超时分档（普通 180s / compact 1000s）
    pub is_compact_request: bool,
    /// 客户端是否请求了 thinking adaptive（续请求与首轮同参注入，D1）
    pub thinking_adaptive_requested: bool,
}

/// 构造桥接上下文（D5/D7）
///
/// `split_web_search_tool` 命中（请求携带 web_search server tool）时返回
/// `Some(BridgeContext)`，未命中返回 `None`。流式/非流式共用同一判定（D7）。
///
/// 必须在 KiroRequest 构建（conversation_state 被 move）之前调用。
pub(crate) fn build_bridge_context(
    conversion_result: &super::super::converter::ConversionResult,
    profile_arn: Option<String>,
    bound_ids: Vec<u64>,
) -> Option<BridgeContext> {
    // 外层 None = 请求未携带 web_search server tool → 不构造桥接上下文
    let max_uses = conversion_result.web_search_max_uses?;
    Some(BridgeContext {
        conversation_state: conversion_result.conversation_state.clone(),
        profile_arn,
        additional_model_request_fields: conversion_result.additional_model_request_fields.clone(),
        max_uses,
        bound_ids,
        is_compact_request: conversion_result.is_compact_request,
        thinking_adaptive_requested: conversion_result.thinking_adaptive_requested,
    })
}

/// web_search server tool 桥接状态机（D4 流式段，嵌入 create_sse_stream 的 unfold 状态）
///
/// `None`（bridge 整体不存在）= 非桥接请求，全部分支短路，零行为变化。
#[derive(Debug)]
pub(crate) struct BridgeState {
    /// 当前所处阶段
    pub(crate) phase: BridgePhase,
    /// 已完成的截获轮数
    pub(crate) rounds_used: usize,
    /// 多轮搜索硬上限 `min(max_uses, 5)`（D8）
    pub(crate) max_rounds: usize,
    /// 已截获完成、待在流结束后执行的搜索队列（D8：Kiro 流读到自然结束才发起 MCP；
    /// 队列化支持 Collecting 期间上游连发多次 web_search 的场景，按截获顺序执行）
    pub(crate) pending: VecDeque<PendingSearch>,
    /// 多轮桥接的演进基底（D3）：初始为 BridgeContext.conversation_state 的 clone；
    /// 每轮续请求基于上一轮续请求所用的状态演进，保证 conversationId/
    /// agentContinuationId/history 跨轮次逐字节不变
    pub(crate) evolution_base: Option<ConversationState>,
    /// 一致性基底：与 evolution_base 不同，此状态始终满足"末位 assistant 的
    /// 每个 tool_use 都被 currentMessage 的 tool_result 应答"的完整配对约束。
    ///
    /// 常规续请求会整体替换 currentMessage.tool_results（客户端原始结果被换成
    /// 本轮搜索结果），配对依赖上游服务端会话状态；服务端状态缺失（超长会话）
    /// 时该请求被 400 TOOL_USE_RESULT_MISMATCH 拒绝，且替换语义使客户端原始
    /// tool_results 已不在演进链中、无法从演进基底恢复。此基底从原始请求状态
    /// 出发，每轮结束时把本轮 toolUse（补进 history 末位 assistant）与结果
    /// （追加到 tool_results 尾部）累积进来，重试时由它重建完整配对请求。
    pub(crate) consolidated_base: Option<ConversationState>,
}

/// 一轮已截获完成、待执行的搜索
#[derive(Debug)]
pub(crate) struct PendingSearch {
    /// Kiro 流中截获的 toolUse id（续请求回填 ToolResult 必须用它，
    /// 而非 create_mcp_request 返回的 srvtoolu_ id）
    pub(crate) tool_use_id: String,
    /// 聚合出的搜索词
    pub(crate) query: String,
}

/// 桥接阶段
#[derive(Debug)]
pub(crate) enum BridgePhase {
    /// 正常透传：非 web_search 事件全部走现有 process_kiro_event 路径
    PassThrough,
    /// 聚合中：input 分片累积到 `input_buffer`，`stop == true` 时截获完成；
    /// 不透传为普通 tool_use SSE（客户端看不到裸 tool_use 块）
    Collecting {
        tool_use_id: String,
        input_buffer: String,
    },
}

impl BridgeState {
    /// 创建桥接状态（初始为 PassThrough）
    ///
    /// `max_uses` 为 server tool 声明的次数上限（None 时兜底 5），
    /// 实际上限取 `min(max_uses, 5)`（D8：多轮硬上限 5）。
    pub(crate) fn new(max_uses: Option<i32>) -> Self {
        Self {
            phase: BridgePhase::PassThrough,
            rounds_used: 0,
            max_rounds: max_uses.unwrap_or(5).clamp(0, 5) as usize,
            pending: VecDeque::new(),
            evolution_base: None,
            consolidated_base: None,
        }
    }

    /// 是否还有剩余截获轮次
    pub(crate) fn has_remaining_rounds(&self) -> bool {
        self.rounds_used < self.max_rounds
    }
}

/// 解析截获聚合的 input JSON 中的 `query` 字段（解析失败返回空串，由 MCP 侧报错）
pub(crate) fn parse_bridge_query(input_json: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(input_json) else {
        return String::new();
    };
    v.get("query")
        .and_then(|q| q.as_str())
        .unwrap_or_default()
        .to_string()
}

/// 桥接状态机对单个 Kiro 事件的处理（D4/D8）
///
/// 返回 `(consumed, events)`：
/// - `consumed == true`：事件被桥接截获，**不**透传给 `process_kiro_event`
///   （即不产生普通 tool_use SSE）；`events` 为需发给客户端的可见性块
/// - `consumed == false`：事件按现有路径透传（含 Collecting 期间的
///   AssistantResponse 说明文字与非目标工具调用）
///
/// 计费口径（对齐 `process_tool_use`）：截获的 input 分片同样无条件计入
/// `output_chars_other`——上游已生成这段内容即已计费；可见性块把 query 回传给了
/// 客户端，故 `visible_chars_other` 同步累加。
pub(crate) fn bridge_handle_event(
    ctx: &mut StreamContext,
    bridge: &mut Option<BridgeState>,
    event: &Event,
) -> (bool, Vec<SseEvent>) {
    let Some(state) = bridge else {
        return (false, Vec::new());
    };

    match &mut state.phase {
        BridgePhase::Collecting {
            tool_use_id,
            input_buffer,
        } => {
            if let Event::ToolUse(tu) = event
                && tu.tool_use_id == *tool_use_id
            {
                input_buffer.push_str(&tu.input);
                if tu.stop {
                    // 截获完成：解析 query → 发可见性块 → 记录待执行搜索 → 回 PassThrough
                    let chars = input_buffer.len() as i64;
                    ctx.output_chars_other += chars;
                    ctx.visible_chars_other += chars;

                    let query = parse_bridge_query(input_buffer);
                    let events = build_web_search_visibility_events(ctx, tool_use_id, &query);
                    state.pending.push_back(PendingSearch {
                        tool_use_id: tool_use_id.clone(),
                        query,
                    });

                    state.phase = BridgePhase::PassThrough;
                    state.rounds_used += 1;
                    return (true, events);
                }
                return (true, Vec::new());
            }
            // Collecting 期间的其他事件（AssistantResponse 说明文字、非目标
            // ToolUse）正常透传（D8）
            (false, Vec::new())
        }
        BridgePhase::PassThrough => {
            if let Event::ToolUse(tu) = event
                && tu.name == "web_search"
                && state.has_remaining_rounds()
            {
                // 轮次未达上限 → 截获，转 Collecting 聚合
                let id = tu.tool_use_id.clone();
                state.phase = BridgePhase::Collecting {
                    tool_use_id: id.clone(),
                    input_buffer: tu.input.clone(),
                };
                // 首个分片即带 stop=true（单事件完整调用）→ 立即完成截获
                if tu.stop {
                    let chars = tu.input.len() as i64;
                    ctx.output_chars_other += chars;
                    ctx.visible_chars_other += chars;

                    let query = parse_bridge_query(&tu.input);
                    let events = build_web_search_visibility_events(ctx, &id, &query);
                    state.pending.push_back(PendingSearch {
                        tool_use_id: id.clone(),
                        query,
                    });
                    state.phase = BridgePhase::PassThrough;
                    state.rounds_used += 1;
                    return (true, events);
                }
                return (true, Vec::new());
            }
            // 非 web_search，或轮次已耗尽 → 按现有普通 tool_use 逻辑透传（D8）
            (false, Vec::new())
        }
    }
}

/// 构造截获可见性块：`server_tool_use`（D4 客户端可见性，截获完成时立即发送）
///
/// 复用 websearch.rs 拦截式 `generate_websearch_events` 的块格式（②-④ 步），差异：
/// - 不重发 message_start（主响应的 message_start 已在 initial_events 发出）
/// - 不发 text 摘要块与 message_delta/message_stop——模型解读与流收尾由
///   续流轮次与 `generate_final_events` 统一处理
/// - 块索引经 `state_manager.next_block_index()` 分配（单调延续，不与已有块冲突）
///
/// `web_search_tool_result` 块由 `build_web_search_result_events` 在 MCP 调用
/// 完成后（流自然结束、续请求发起前）携带真实结果发出。
fn build_web_search_visibility_events(
    ctx: &mut StreamContext,
    tool_use_id: &str,
    query: &str,
) -> Vec<SseEvent> {
    let mut events = ctx.finish_native_thinking();

    // server_tool_use 块：start + input_json_delta + stop
    let idx = ctx.state_manager.next_block_index();
    events.extend(ctx.state_manager.handle_content_block_start(
        idx,
        "server_tool_use",
        json!({
            "type": "content_block_start",
            "index": idx,
            "content_block": {
                "id": tool_use_id,
                "type": "server_tool_use",
                "name": "web_search",
                "input": {}
            }
        }),
    ));
    let input_json = json!({ "query": query });
    if let Some(delta) = ctx.state_manager.handle_content_block_delta(
        idx,
        json!({
            "type": "content_block_delta",
            "index": idx,
            "delta": {
                "type": "input_json_delta",
                "partial_json": serde_json::to_string(&input_json).unwrap_or_default()
            }
        }),
    ) {
        events.push(delta);
    }
    if let Some(stop) = ctx.state_manager.handle_content_block_stop(idx) {
        events.push(stop);
    }

    events
}

/// 构造 `web_search_tool_result` 可见性块（D4，携带真实 MCP 搜索结果）
///
/// 复用 websearch.rs 拦截式 `generate_websearch_events` 的条目格式（⑤-⑥ 步）：
/// 每条结果为 `{type, title, url, encrypted_content(snippet), page_age}`。
/// `search_results` 为 None（MCP 失败/解析失败）时 content 为空数组。
pub(crate) fn build_web_search_result_events(
    ctx: &mut StreamContext,
    tool_use_id: &str,
    search_results: &Option<websearch::WebSearchResults>,
) -> Vec<SseEvent> {
    let mut events = ctx.finish_native_thinking();

    let search_content = search_results_to_json_array(search_results);

    let result_idx = ctx.state_manager.next_block_index();
    events.extend(ctx.state_manager.handle_content_block_start(
        result_idx,
        "web_search_tool_result",
        json!({
            "type": "content_block_start",
            "index": result_idx,
            "content_block": {
                "type": "web_search_tool_result",
                "tool_use_id": tool_use_id,
                "content": search_content
            }
        }),
    ));
    if let Some(stop) = ctx.state_manager.handle_content_block_stop(result_idx) {
        events.push(stop);
    }

    events
}

/// 构建续请求体（D3：手工构建，绕过 validate_tool_pairing）
///
/// 基于演进基底 clone（多轮时为上一轮续请求所用状态，首轮为
/// `BridgeContext.conversation_state`），仅替换 `current_message` 的
/// `tool_results` 为本轮结果；`conversation_id` / `agent_continuation_id` /
/// `history` / `agent_task_type` / `chat_trigger_type` 逐字节不变。
///
/// MCP 失败（`search_results == None`）→ `ToolResult::error` 降级，
/// 仍发续请求让模型自行告知用户搜索失败，流不中断。
///
/// 每轮续请求回填恰好 1 条 PendingSearch 的结果；多条截获搜索按队列顺序
/// 在各自续流结束后逐轮 drain（每条一次 MCP 调用 + 一次续请求）。
pub(crate) fn build_continuation_request(
    bridge_ctx: &BridgeContext,
    evolution_base: Option<ConversationState>,
    tool_results: Vec<ToolResult>,
) -> KiroRequest {
    let mut conversation_state =
        evolution_base.unwrap_or_else(|| bridge_ctx.conversation_state.clone());
    conversation_state
        .current_message
        .user_input_message
        .user_input_message_context
        .tool_results = tool_results;

    KiroRequest {
        conversation_state,
        profile_arn: bridge_ctx.profile_arn.clone(),
        additional_model_request_fields: bridge_ctx.additional_model_request_fields.clone(),
    }
}

/// 把本轮截获的 web_search toolUse 显式补进 history 尾部
///
/// 常规续请求只回填 `current_message.tool_results`，配对依赖上游服务端
/// 会话状态保留其刚产出的 toolUse；超长会话等场景下服务端状态缺失，
/// 续请求会收到 400 `TOOL_USE_RESULT_MISMATCH`。此函数把 toolUse 写入
/// history 使配对显式化：末位是 assistant 消息则并入其 `tool_uses`
/// （避免产生连续两条 assistant 消息），否则追加一条仅含 toolUse 的
/// assistant 消息（content 用单空格占位，与 merge_assistant_messages
/// 对"只有 toolUse 的 assistant 消息"的既有约定一致）。
pub(crate) fn pair_web_search_tool_use_in_history(
    state: &mut ConversationState,
    pending: &PendingSearch,
) {
    let entry = ToolUseEntry {
        tool_use_id: pending.tool_use_id.clone(),
        name: "web_search".to_string(),
        input: json!({ "query": pending.query }),
    };
    match state.history.last_mut() {
        Some(Message::Assistant(last)) => {
            last.assistant_response_message
                .tool_uses
                .get_or_insert_with(Vec::new)
                .push(entry);
        }
        _ => {
            let mut assistant = AssistantMessage::new(" ");
            assistant.tool_uses = Some(vec![entry]);
            state
                .history
                .push(Message::Assistant(HistoryAssistantMessage {
                    assistant_response_message: assistant,
                }));
        }
    }
}

/// 把一轮 web_search 的配对信息（toolUse + 结果）累积并入给定状态
///
/// 重试请求构建与轮末一致性基底维护共用。补写 toolUse 到 history 末位
/// assistant 的同时，把本轮结果**追加**到 currentMessage.tool_results 尾部
/// （区别于常规续请求的整体替换），使状态满足退化校验模式下
/// "assistant 每个 tool_use 都被紧随消息的 tool_result 应答、
/// 每个 tool_result 都有配对 tool_use"的双向配对约束。
pub(crate) fn apply_round_pairing(
    state: &mut ConversationState,
    pending: &PendingSearch,
    round_result: &ToolResult,
) {
    pair_web_search_tool_use_in_history(state, pending);
    state
        .current_message
        .user_input_message
        .user_input_message_context
        .tool_results
        .push(round_result.clone());
}

/// 按单条待执行搜索构建其 ToolResult（MCP 成功 → success 摘要；失败 → error 降级）
pub(crate) fn build_search_tool_result(
    tool_use_id: &str,
    query: &str,
    search_results: &Option<websearch::WebSearchResults>,
) -> ToolResult {
    match search_results {
        Some(_) => ToolResult::success(
            tool_use_id,
            websearch::generate_search_summary(query, search_results),
        ),
        None => ToolResult::error(
            tool_use_id,
            format!("Web search failed for query: {}", query),
        ),
    }
}

/// 一轮桥接的执行结果
///
/// `Continued`：MCP 搜索与续流建连均成功，携带新响应流与搜索结果；
/// `Failed(search_results)`：续请求发起/序列化失败，但 MCP 搜索结果可能
/// 已产出——调用方必须先把 `web_search_tool_result` 结果块发给客户端
/// （与已发出的 `server_tool_use` 块配对），再补发 error 事件收尾。
#[allow(clippy::large_enum_variant)] // 变体大小差异是桥接语义所需（Failed 不携带流）
pub(crate) enum BridgeRoundOutcome {
    Continued(
        reqwest::Response,
        EventStreamDecoder,
        Option<websearch::WebSearchResults>,
    ),
    Failed(Option<websearch::WebSearchResults>),
}

/// in-flight 桥接轮（修复③ v2：select! 条件分支保活）
///
/// 一轮桥接（MCP 搜索 + 续流建连）spawn 到后台任务执行，`JoinHandle` 连同
/// 本轮 tool_use_id 存入 unfold 状态元组第 10 元。收割通过 select! 的条件
/// 分支完成（`if round_in_flight.is_some()`）：轮次执行期间该分支挂起等待，
/// ping/deadline 分支照常就绪触发——桥接轮执行期间下游心跳不再中断，
/// 且耗尽的 body_stream 借条件前置不再被 poll（避免 flatten 重入误收尾）。
pub(crate) type InFlightRound = (
    tokio::task::JoinHandle<(BridgeState, BridgeRoundOutcome)>,
    // 本轮对应的 web_search tool_use_id（轮次完成后构建配对结果块用）
    String,
);

/// 执行一轮桥接：MCP 真实搜索 → 构建续请求 → 发起续流（D3/D4/D8）
///
/// 在 unfold 的上游 None 分支内调用（Kiro 流已自然结束）。成功时返回
/// `BridgeRoundOutcome::Continued((新响应流, 新解码器, 本轮搜索结果))`——
/// unfold 状态元组的 `body_stream`/`decoder` 被替换为续请求的响应，
/// `ctx`/`bridge` 原样携带，对 unfold 而言续流只是"换了一个上游 body 继续
/// unfold"；搜索结果供调用方构建 `web_search_tool_result` 可见性块；
/// `Failed` 表示续请求发起失败（调用方须先发结果块再发 error 事件收尾）。
///
/// 流程：
/// 1. `call_mcp_api` 真实搜索；失败 → `ToolResult::error` 降级，流不中断
/// 2. 基于演进基底 clone 构建 KiroRequest（仅替换 current_message 的
///    tool_results，conversationId/agentContinuationId/history 逐字节不变，
///    绕过 validate_tool_pairing）
/// 3. `call_api_stream` 续流；失败 → `Failed(search_results)`（调用方收尾）
#[allow(clippy::type_complexity)]
pub(crate) async fn bridge_execute_round(
    provider: &crate::kiro::provider::KiroProvider,
    bridge_ctx: &BridgeContext,
    bridge: &mut BridgeState,
    pending: PendingSearch,
) -> BridgeRoundOutcome {
    // 1. MCP 真实搜索（失败降级为 error ToolResult，仍发续请求让模型解读）
    let (_mcp_tool_use_id, mcp_request) = websearch::create_mcp_request(&pending.query);
    let search_results =
        match websearch::call_mcp_api(provider, &mcp_request, &bridge_ctx.bound_ids).await {
            Ok(response) => websearch::parse_search_results(&response),
            Err(e) => {
                tracing::warn!(
                    tool_use_id = %pending.tool_use_id,
                    "web_search MCP 调用失败，降级为 error ToolResult: {}",
                    e
                );
                None
            }
        };

    // 2. 基于演进基底构建续请求（D3 多轮语义：第 N+1 轮 clone 第 N 轮所用状态）
    let tool_result =
        build_search_tool_result(&pending.tool_use_id, &pending.query, &search_results);
    // 一致性基底懒初始化：从原始请求状态出发累积各轮配对（重试时使用）
    if bridge.consolidated_base.is_none() {
        bridge.consolidated_base = Some(bridge_ctx.conversation_state.clone());
    }
    let kiro_request = build_continuation_request(
        bridge_ctx,
        bridge.evolution_base.take(),
        vec![tool_result.clone()],
    );
    let request_body = match serde_json::to_string(&kiro_request) {
        Ok(body) => body,
        Err(e) => {
            tracing::error!("web_search 续请求序列化失败: {}", e);
            // 与 call_api_stream Err 分支对齐：写回取出的演进基底
            bridge.evolution_base = Some(kiro_request.conversation_state);
            // 本轮配对仍需累积进一致性基底，供后续轮次（若有）重试使用
            if let Some(consolidated) = bridge.consolidated_base.as_mut() {
                apply_round_pairing(consolidated, &pending, &tool_result);
            }
            return BridgeRoundOutcome::Failed(search_results);
        }
    };

    // 3. 续流：call_api_stream（换上游 body 继续 unfold，ctx/bridge 原样携带）
    let outcome = match provider
        .call_api_stream(
            &request_body,
            bridge_ctx.is_compact_request,
            bridge_ctx.thinking_adaptive_requested,
            &bridge_ctx.bound_ids,
        )
        .await
    {
        Ok((response, _credential_id)) => {
            // 演进基底更新为本轮续请求所用的状态（下一轮 clone 它）
            bridge.evolution_base = Some(kiro_request.conversation_state);
            BridgeRoundOutcome::Continued(response, EventStreamDecoder::new(), search_results)
        }
        Err(e) => {
            // 上游服务端会话状态缺失（超长会话等场景）时，续请求会因配对断裂被拒
            // （400 TOOL_USE_RESULT_MISMATCH，两个方向：tool_result 无配对 tool_use、
            // 或 tool_use 无紧随的 tool_result——后者源于续请求整体替换 tool_results
            // 使客户端原始结果悬空）。降级重试一次：从一致性基底重建完整配对请求
            // （原始结果 + 各轮累积 + 本轮结果，toolUse 补进 history 末位），
            // 不再依赖服务端状态。常规路径（history 逐字节不变）不受影响。
            if e.to_string().contains("TOOL_USE_RESULT_MISMATCH") {
                let mut retry_state = bridge
                    .consolidated_base
                    .clone()
                    .expect("consolidated_base 已在步骤 2 懒初始化");
                apply_round_pairing(&mut retry_state, &pending, &tool_result);
                let retried = KiroRequest {
                    conversation_state: retry_state,
                    profile_arn: bridge_ctx.profile_arn.clone(),
                    additional_model_request_fields: bridge_ctx
                        .additional_model_request_fields
                        .clone(),
                };
                tracing::warn!(
                    tool_use_id = %pending.tool_use_id,
                    "web_search 续请求配对被拒(TOOL_USE_RESULT_MISMATCH)，从一致性基底重建完整配对后重试"
                );
                match serde_json::to_string(&retried) {
                    Ok(retry_body) => {
                        match provider
                            .call_api_stream(
                                &retry_body,
                                bridge_ctx.is_compact_request,
                                bridge_ctx.thinking_adaptive_requested,
                                &bridge_ctx.bound_ids,
                            )
                            .await
                        {
                            Ok((response, _credential_id)) => {
                                tracing::info!(
                                    tool_use_id = %pending.tool_use_id,
                                    "web_search 续请求完整配对重试成功"
                                );
                                bridge.evolution_base = Some(retried.conversation_state);
                                BridgeRoundOutcome::Continued(
                                    response,
                                    EventStreamDecoder::new(),
                                    search_results,
                                )
                            }
                            Err(retry_err) => {
                                tracing::error!("web_search 续请求重试仍失败: {}", retry_err);
                                bridge.evolution_base = Some(retried.conversation_state);
                                BridgeRoundOutcome::Failed(search_results)
                            }
                        }
                    }
                    Err(se) => {
                        tracing::error!("web_search 续请求重试序列化失败: {}", se);
                        bridge.evolution_base = Some(retried.conversation_state);
                        BridgeRoundOutcome::Failed(search_results)
                    }
                }
            } else {
                tracing::error!("web_search 续请求发起失败: {}", e);
                // 演进基底保持取出的状态，避免下一轮基于未知状态演进
                bridge.evolution_base = Some(kiro_request.conversation_state);
                // 搜索结果必须带回：客户端已收到 server_tool_use 块，缺结果块会破坏配对
                BridgeRoundOutcome::Failed(search_results)
            }
        }
    };

    // 4. 轮末：本轮配对（toolUse + 结果）累积进一致性基底，供后续轮次重试重建
    if let Some(consolidated) = bridge.consolidated_base.as_mut() {
        apply_round_pairing(consolidated, &pending, &tool_result);
    }
    outcome
}

/// 执行一轮桥接的 owned 变体（修复③ v2：供 `tokio::spawn` 后台任务调用）
///
/// 与 `bridge_execute_round` 逻辑一致，差别仅在所有权形态：`BridgeState`
/// 按值进出（后台任务无法持有 unfold 状态元组的借用）。`InFlightRound`
/// 持有的 JoinHandle 完成时把更新后的 `BridgeState` 一并带回。
pub(crate) async fn bridge_execute_round_owned(
    provider: std::sync::Arc<crate::kiro::provider::KiroProvider>,
    bridge_ctx: BridgeContext,
    mut bridge: BridgeState,
    pending: PendingSearch,
) -> (BridgeState, BridgeRoundOutcome) {
    let outcome = bridge_execute_round(&provider, &bridge_ctx, &mut bridge, pending).await;
    (bridge, outcome)
}

/// 收割已完成的桥接轮（修复③）：按 outcome 补发配对结果块与收尾事件，
/// 返回本轮应下发的 SSE 字节流与 `body_stream`/`decoder` 换流结果及 finished
/// 标志（状态元组其余元素由调用方原样回填）。
///
/// `Continued` → 先补发 `web_search_tool_result` 结果块（与已下发的
/// `server_tool_use` 配对），再换入续流（finished = false，unfold 对续流的
/// 处理与普通上游流完全一致）；
/// `Failed` → 补发结果块（携带已产出的搜索结果，MCP 失败为空数组）后补发
/// error 事件收尾（finished = true）；后台任务 panic 兜底复用 `Failed(None)`。
pub(crate) struct BridgeRoundHarvest {
    pub(crate) events: Vec<SseEvent>,
    pub(crate) new_body_stream: reqwest::Response,
    pub(crate) new_decoder: EventStreamDecoder,
    pub(crate) finished: bool,
}

pub(crate) fn harvest_bridge_round(
    outcome: BridgeRoundOutcome,
    result_tool_use_id: &str,
    ctx: &mut StreamContext,
) -> BridgeRoundHarvest {
    match outcome {
        BridgeRoundOutcome::Continued(response, new_decoder, search_results) => {
            let events = build_web_search_result_events(ctx, result_tool_use_id, &search_results);
            // 续流的首个事件前先补发本轮配对结果块，保持块序：
            // server_tool_use → web_search_tool_result → 续流内容
            BridgeRoundHarvest {
                events,
                new_body_stream: response,
                new_decoder,
                finished: false,
            }
        }
        BridgeRoundOutcome::Failed(search_results) => {
            let mut events =
                build_web_search_result_events(ctx, result_tool_use_id, &search_results);
            events.push(stream_interrupted_error_event());
            // Failed 无续流，调用方以 finished = true 收尾，body_stream/decoder
            // 原值不再被消费（decoder 原样带回占位）
            BridgeRoundHarvest {
                events,
                new_body_stream: reqwest::Response::from(http::Response::new(body_dummy_bytes())),
                new_decoder: EventStreamDecoder::new(),
                finished: true,
            }
        }
    }
}

pub(crate) fn body_dummy_bytes() -> reqwest::Body {
    reqwest::Body::from(Bytes::new())
}

/// 非流式降级收尾前，为 pending_search 队列中尚未执行的搜索补发空结果块
///
/// 三个 `break 'rounds` 降级路径（序列化失败/响应读取失败/续请求发起失败）共
/// 用：队列中每条 pending 的 `server_tool_use` 块均已进入 visibility_blocks，
/// 缺对应结果块会破坏 server_tool_use / web_search_tool_result 成对不变量。
pub(crate) fn flush_unpaired_search_blocks(
    pending_search: &mut VecDeque<PendingSearch>,
    visibility_blocks: &mut Vec<serde_json::Value>,
) {
    for leftover in pending_search.drain(..) {
        visibility_blocks.push(build_web_search_result_block(&leftover.tool_use_id, &None));
    }
}

/// 将 MCP 搜索结果转换为 `web_search_tool_result` 块的 content 数组
///
/// 流式 `build_web_search_result_events` 与非流式 `build_web_search_result_block`
/// 共用的条目构建逻辑：`search_results` 为 None 时返回空数组。
fn search_results_to_json_array(
    search_results: &Option<websearch::WebSearchResults>,
) -> Vec<serde_json::Value> {
    match search_results {
        Some(results) => results
            .results
            .iter()
            .map(|r| {
                json!({
                    "type": "web_search_result",
                    "title": r.title,
                    "url": r.url,
                    "encrypted_content": r.snippet.clone().unwrap_or_default(),
                    "page_age": null
                })
            })
            .collect(),
        None => vec![],
    }
}

/// 构造非流式 `web_search_tool_result` 可见性块（D5 非流式段，直接组 JSON）
///
/// 条目格式与流式 `build_web_search_result_events` 一致：
/// `{type, title, url, encrypted_content(snippet), page_age}`。
/// `search_results` 为 None（MCP 失败/解析失败）时 content 为空数组。
pub(crate) fn build_web_search_result_block(
    tool_use_id: &str,
    search_results: &Option<websearch::WebSearchResults>,
) -> serde_json::Value {
    let content = search_results_to_json_array(search_results);

    json!({
        "type": "web_search_tool_result",
        "tool_use_id": tool_use_id,
        "content": content
    })
}
