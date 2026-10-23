// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 工具声明收集与转换：namespace 展开、custom 自由文本工具降级、同名去重

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::openai::chat_request::convert_tool;

/// `namespace` 容器的嵌套深度上限；超出即跳过，避免畸形请求把栈递归穿了
pub(crate) const MAX_NAMESPACE_DEPTH: usize = 4;

/// Codex remote compaction v2 系统提示
///
/// 当检测到 `compaction_trigger` input item 时，注入该提示替代原始压缩指令。
/// 要求模型以"对话摘要"形式生成一段文字，作为后续轮次的上下文压缩内容。
pub(crate) const COMPACTION_SYSTEM_PROMPT: &str = "\
You are performing a CONTEXT COMPACTION operation. \
Your task is to produce a concise but comprehensive summary of the current conversation that \
will allow seamlessly continuing the work in a future context window.\n\
\n\
Include:\n\
- The current task/goal and its status\n\
- Key decisions and conclusions reached\n\
- Important context, file paths, or technical details\n\
- Work completed so far\n\
- Pending actions or next steps\n\
\n\
Write this as a first-person internal note to yourself (as if you are writing notes to a \
successor instance of yourself who will continue this exact task). Be concise yet complete. \
Do not include any preamble or explanation — just the summary content.";

/// 将代理自制的 compaction `encrypted_content` 解码为原始摘要文本
///
/// 本代理不产出真正的 Fernet 加密内容，而是用标准 base64 编码摘要文字。
/// 解码成功时返回摘要文本；格式不匹配或解码失败时返回 `None`（由调用方决定降级行为）。
pub(crate) fn base64_decode_compaction(encoded: &str) -> Option<String> {
    use base64::Engine as _;
    // 尝试标准 base64 解码（含 padding 变种）
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(encoded))
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(encoded))
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded))
        .ok()?;
    String::from_utf8(decoded_bytes).ok()
}

/// `custom` 工具的入参在 Responses 协议里是自由文本，Anthropic 只接受 JSON schema。
/// 降级为单字段对象，配合 [`ToolInputForm::FreeText`] 把原始文本塞进 `input` 字段。
pub(crate) fn custom_tool_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"input": {"type": "string"}},
        "required": ["input"],
        "additionalProperties": false,
    })
}

/// 追加到 `custom`（自由文本）工具 description 末尾的适配说明
///
/// **必需**：这类工具的原始描述往往明确要求"raw text, not JSON, no code fences"
/// （Codex 的 `exec` 逐字如此），与 [`custom_tool_schema`] 的单字段包装直接冲突。
/// 不加说明时模型会照原描述吐裸文本，导致入参解析不出来。只在末尾追加，不改动原描述。
pub(crate) const FREEFORM_ADAPTATION_NOTE: &str = concat!(
    "\n\n---\n[Gateway adaptation] This freeform tool is exposed as a JSON tool. ",
    "Put the raw tool text (unquoted, no code fences) into the `input` string field."
);

/// OpenAI 工具协议的默认命名空间名：其子工具按裸名声明、按裸名回调，展开时不加前缀
pub(crate) const DEFAULT_TOOL_NAMESPACE: &str = "functions";

/// 扁平工具名里分隔 namespace 与叶子名的分隔符
const NAMESPACE_SEP: &str = "__";

/// 扁平工具名的长度上限（Anthropic 工具名 `^[a-zA-Z0-9_-]{1,64}$`）
const MAX_FLAT_TOOL_NAME_LEN: usize = 64;

/// 扁平工具名 → (namespace, 叶子名)
///
/// 请求侧把非默认 namespace 的子工具展开成扁平名发给上游；上游回调时给的是扁平名，
/// 响应侧据此映射表还原成 Codex 要求的 `{namespace, name}` 两个字段。
pub(crate) type NamespaceMap = HashMap<String, (String, String)>;

/// 把 `namespace` + 叶子名拼成上游可接受的扁平工具名：`<namespace>__<leaf>`
///
/// Codex 的路由按 `{namespace, name}` 做严格匹配，裸叶子名、`ns.名`、`ns__名` 当作单个名字回调
/// 都会被拒（`unsupported call`），所以必须记映射表、在响应里拆回两个字段。
///
/// 结果是确定性的：同样的输入永远得到同样的名字，历史里回放的 `function_call` 才能
/// 与本轮声明的工具名对上，不依赖映射表。含非法字符或超长时，把非法字符换成 `_`、
/// 截断到上限，并在末尾附上原始全名的 SHA-256 前 4 字节（8 位十六进制）以免撞名。
pub(crate) fn flatten_tool_name(namespace: &str, leaf: &str) -> String {
    let full = format!("{namespace}{NAMESPACE_SEP}{leaf}");
    let clean: String = full
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean == full && clean.len() <= MAX_FLAT_TOOL_NAME_LEN {
        return clean;
    }
    let digest = Sha256::digest(full.as_bytes());
    let suffix: String = digest.iter().take(4).map(|b| format!("{b:02x}")).collect();
    // clean 逐字符映射后全是 ASCII，按字节截断不会切到字符中间
    let keep = (MAX_FLAT_TOOL_NAME_LEN - 1 - suffix.len()).min(clean.len());
    format!("{}_{suffix}", &clean[..keep])
}

/// 工具调用入参的两种形态
#[derive(Clone, Copy)]
pub(crate) enum ToolInputForm {
    /// `function_call.arguments`：JSON 字符串
    Json,
    /// `custom_tool_call.input`：自由文本
    FreeText,
}

/// 请求转换结果
#[derive(Debug)]
pub(crate) struct ConvertedResponsesRequest {
    /// 客户端请求的原始模型名，响应必须回写该值
    pub(crate) client_model: String,
    /// 客户端是否要求流式
    pub(crate) stream: bool,
    /// 转换后的 Anthropic 请求体
    pub(crate) anthropic_body: Value,
    /// 声明为 `custom` 的工具名。响应侧必须把这些工具的调用还原成
    /// `custom_tool_call` item（自由文本入参），否则客户端认不出来
    pub(crate) custom_tools: HashSet<String>,
    /// 非默认 namespace 的子工具：扁平名 → (namespace, 叶子名)。响应侧据此把上游的
    /// 扁平调用还原成 `function_call.namespace` + 叶子 `name`，否则 Codex 认不出
    pub(crate) namespaced_tools: NamespaceMap,
    /// 是否为 Codex remote compaction v2 压缩请求
    ///
    /// 当 `input` 末尾包含 `{"type":"compaction_trigger"}` 时置为 `true`。
    /// 响应侧需要将模型的文字摘要包装为 `type: "compaction"` output item，
    /// 否则 Codex 会报 "expected exactly one compaction output item, got 0"。
    pub(crate) is_compaction: bool,
}

/// 汇总各来源的工具声明
///
/// 两个来源：顶层 `tools`，以及 `input` 里的 `additional_tools` item。先收的优先——
/// 顶层声明在遍历 `input` 之前入栈，同名时后来者被丢弃。
#[derive(Default)]
pub(crate) struct ToolCollector {
    /// 已转换为 Anthropic 形态的工具
    pub(crate) tools: Vec<Value>,
    /// 已收录的工具名，用于同名去重
    seen: HashSet<String>,
    /// 其中入参为自由文本（`custom`）的工具名
    pub(crate) custom: HashSet<String>,
    /// 其中来自非默认 namespace 的工具：扁平名 → (namespace, 叶子名)
    pub(crate) namespaced: NamespaceMap,
}

impl ToolCollector {
    pub(crate) fn push_list(&mut self, list: &[Value], depth: usize) {
        self.push_list_in(list, depth, None);
    }

    /// `ns` 是最近一层非默认 namespace 的名字；`None` 表示工具按裸名声明
    fn push_list_in(&mut self, list: &[Value], depth: usize, ns: Option<&str>) {
        for tool in list {
            self.push_one(tool, depth, ns);
        }
    }

    fn push_one(&mut self, tool: &Value, depth: usize, ns: Option<&str>) {
        let tool_type = tool
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("function");

        // namespace 本身不可调用，只是分组容器，真正的工具在它的 tools 数组里。
        //
        // - `functions`（OpenAI 默认命名空间）：子工具按裸名声明、按裸名回调，直接剥掉容器。
        // - 其余 namespace（Codex 的 MCP server、multi_agent_v1 等）：Anthropic 工具名不接受
        //   `.`，且客户端的路由按 `{namespace, name}` 严格匹配，所以子工具展开成扁平名
        //   `ns__叶子名` 发给上游，同时记入映射表，响应侧再拆回两个字段（见 `flatten_tool_name`）。
        // "是不是默认命名空间"在协议里没有字段可表达，只能按名字判别。
        if tool_type == "namespace" {
            let ns_name = tool.get("name").and_then(Value::as_str).unwrap_or_default();
            if ns_name.is_empty() {
                tracing::warn!("namespace 容器缺少 name，无法生成工具名，已跳过");
                return;
            }
            let inner_ns = if ns_name == DEFAULT_TOOL_NAMESPACE {
                ns
            } else {
                Some(ns_name)
            };
            match tool.get("tools").and_then(Value::as_array) {
                Some(inner) if depth < MAX_NAMESPACE_DEPTH => {
                    self.push_list_in(inner, depth + 1, inner_ns)
                }
                Some(_) => {
                    tracing::warn!(depth, "namespace 嵌套超过深度上限，已跳过");
                }
                None => {}
            }
            return;
        }

        // namespace 内只支持 function：custom 工具的回调 item 没有 namespace 字段可还原
        if let Some(ns) = ns
            && tool_type != "function"
        {
            tracing::warn!(
                namespace = %ns,
                tool_type = %tool_type,
                "namespace 内的非 function 工具无法还原调用，已跳过"
            );
            return;
        }

        let Some(mut converted) = convert_one_tool(tool) else {
            return;
        };
        let Some(leaf) = converted
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            return;
        };
        let name = match ns {
            Some(ns) => {
                let flat = flatten_tool_name(ns, &leaf);
                converted["name"] = Value::String(flat.clone());
                flat
            }
            None => leaf.clone(),
        };
        if !self.seen.insert(name.clone()) {
            tracing::warn!(tool_name = %name, "工具重名，保留先出现的声明");
            return;
        }
        if let Some(ns) = ns {
            self.namespaced.insert(name, (ns.to_string(), leaf));
        } else if tool_type == "custom" {
            self.custom.insert(name);
        }
        self.tools.push(converted);
    }
}

/// 单个工具声明 → Anthropic tool
pub(crate) fn convert_one_tool(tool: &Value) -> Option<Value> {
    match tool
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("function")
    {
        "function" => convert_tool(tool),
        "custom" => {
            let name = tool
                .get("name")
                .and_then(Value::as_str)
                .filter(|n| !n.is_empty())?;
            let description = tool
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default();
            tracing::warn!(
                tool_name = %name,
                "custom 工具的自由文本入参已降级为单字段 JSON schema（上游只接受 JSON schema）"
            );
            Some(json!({
                "name": name,
                "description": format!("{description}{FREEFORM_ADAPTATION_NOTE}"),
                "input_schema": custom_tool_schema(),
            }))
        }
        // web_search / local_shell / image_generation 等内建工具由 OpenAI 侧执行，上游没有对等实现
        other => {
            tracing::warn!(tool_type = %other, "不支持的工具类型，已跳过（上游仅支持函数工具）");
            None
        }
    }
}
