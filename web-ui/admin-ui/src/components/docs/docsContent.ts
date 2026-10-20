// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 使用文档页接口参考内容：5 个公开端点规格（中英双语，协议示例不进 i18n）

export type ParameterSpec = {
  name: string
  type: string
  required: boolean
  description: string
}

export type ResponseExample = {
  code: number
  body: string
  label?: string
}

export type EndpointSpec = {
  id: string
  method: string
  path: string
  title: string
  description: string
  curl: string
  defaultBody?: string
  responses: ResponseExample[]
  parameters?: ParameterSpec[]
  /** 参数表表头与必选列文案（随 locale 由构建侧切换） */
  paramHeaderName: string
  paramHeaderType: string
  paramHeaderRequired: string
  paramHeaderDesc: string
  requiredLabel: string
  optionalLabel: string
  requestLabel: string
  responseLabel: string
}

export type DocsLocale = 'zh' | 'en'

function copy(locale: DocsLocale, zh: string, en: string) {
  return locale === 'zh' ? zh : en
}

const MESSAGES_BODY_ZH = `{
  "model": "claude-sonnet-4-5",
  "max_tokens": 1024,
  "stream": true,
  "messages": [
    {"role": "user", "content": "你好，介绍一下你自己"}
  ]
}`

const MESSAGES_RESP = `event: message_start
data: {"type":"message_start","message":{"id":"msg_01Xy...","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[],"usage":{"input_tokens":25,"output_tokens":1}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"你好！"}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":15}}

event: message_stop
data: {"type":"message_stop"}`

const NONSTREAM_RESP = `{
  "id": "msg_01Xy...",
  "type": "message",
  "role": "assistant",
  "model": "claude-sonnet-4-5",
  "content": [
    {"type": "text", "text": "你好！我是 Claude。"}
  ],
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 25,
    "output_tokens": 15
  }
}`

const CHAT_BODY = `{
  "model": "claude-sonnet-4-5",
  "stream": true,
  "messages": [
    {"role": "user", "content": "你好，介绍一下你自己"}
  ]
}`

const CHAT_RESP = `data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1700000000,"model":"claude-sonnet-4-5","choices":[{"index":0,"delta":{"role":"assistant","content":"你好！"},"finish_reason":null}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1700000000,"model":"claude-sonnet-4-5","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]`

const RESPONSES_BODY = `{
  "model": "claude-sonnet-4-5",
  "stream": true,
  "input": [
    {"role": "user", "content": [{"type": "input_text", "text": "你好，介绍一下你自己"}]}
  ]
}`

const RESPONSES_RESP = `event: response.created
data: {"type":"response.created","response":{"id":"resp_xxx","model":"claude-sonnet-4-5"}}

event: response.output_text.delta
data: {"type":"response.output_text.delta","delta":"你好！"}

event: response.completed
data: {"type":"response.completed","response":{"id":"resp_xxx","usage":{"input_tokens":25,"output_tokens":15}}}`

const MODELS_RESP = `{
  "object": "list",
  "data": [
    {
      "id": "claude-sonnet-4-5",
      "object": "model",
      "created": 1730304000,
      "owned_by": "anthropic",
      "display_name": "Claude Sonnet 4.5",
      "type": "chat",
      "max_tokens": 64000
    }
  ]
}`

export function buildEndpointSpecs(baseUrl: string, locale: DocsLocale = 'zh'): EndpointSpec[] {
  const base = baseUrl.replace(/\/+$/, '')
  const key = '<API_KEY>'
  const authHeader = `--header 'Authorization: Bearer ${key}'`
  const common = {
    paramHeaderName: copy(locale, '参数', 'Parameter'),
    paramHeaderType: copy(locale, '类型', 'Type'),
    paramHeaderRequired: copy(locale, '必选', 'Required'),
    paramHeaderDesc: copy(locale, '说明', 'Description'),
    requiredLabel: copy(locale, '必选', 'Required'),
    optionalLabel: copy(locale, '可选', 'Optional'),
    requestLabel: copy(locale, '请求体', 'Request body'),
    responseLabel: copy(locale, '响应', 'Response'),
  }

  return [
    {
      id: 'api-messages',
      method: 'POST',
      path: '/v1/messages',
      title: copy(locale, '创建消息（Anthropic 原生）', 'Create a message (native Anthropic)'),
      description: copy(
        locale,
        'Anthropic Messages API 原生端点，Claude Code 直连入口。请求体为 Anthropic Messages 格式，模型名按关键词映射到 Kiro 模型（见开发指南）。支持流式 SSE 与非流式 JSON。',
        'Native Anthropic Messages endpoint, the direct entry for Claude Code. Accepts Anthropic Messages format; model names are keyword-mapped to Kiro models (see Development). Supports SSE streaming and non-streaming JSON.',
      ),
      parameters: [
        { name: 'model', type: 'string', required: true, description: copy(locale, '模型名，含 sonnet/opus/haiku/gpt 等关键词即路由到对应 Kiro 模型；含 thinking 后缀自动启用扩展思考', 'Model name; sonnet/opus/haiku/gpt keywords route to the corresponding Kiro model; a thinking suffix enables extended thinking') },
        { name: 'max_tokens', type: 'integer', required: true, description: copy(locale, '输出上限；下限 1024，上限按模型代际 64K/128K 档', 'Output cap; floor 1024, ceiling 64K/128K by model generation') },
        { name: 'messages', type: 'array', required: true, description: copy(locale, '消息数组，支持 text/image/tool_result 内容块与 thinking 块', 'Message array with text/image/tool_result blocks and thinking blocks') },
        { name: 'system', type: 'string | array', required: false, description: copy(locale, '系统提示；history[0] 冻结机制保证跨请求缓存稳定', 'System prompt; the history[0] freeze keeps prefix caching stable across requests') },
        { name: 'stream', type: 'boolean', required: false, description: copy(locale, 'true 返回 Anthropic SSE 流（message_start / content_block_delta / message_stop）', 'true returns an Anthropic SSE stream (message_start / content_block_delta / message_stop)') },
        { name: 'tools', type: 'array', required: false, description: copy(locale, '工具定义（JSON Schema 会被规范化）', 'Tool definitions (JSON Schema is normalized)') },
      ],
      defaultBody: MESSAGES_BODY_ZH,
      curl: `curl --request POST \\
  --url ${base}/v1/messages \\
  ${authHeader} \\
  --header 'content-type: application/json' \\
  --header 'anthropic-version: 2023-06-01' \\
  --data '${MESSAGES_BODY_ZH.replace(/\n\s*/g, '')}'`,
      responses: [
        { code: 200, label: copy(locale, '流式响应（SSE）', 'Streaming response (SSE)'), body: MESSAGES_RESP },
        { code: 200, label: copy(locale, '非流式响应（stream=false）', 'Non-streaming (stream=false)'), body: NONSTREAM_RESP },
      ],
      ...common,
    },
    {
      id: 'api-messages-cc',
      method: 'POST',
      path: '/cc/v1/messages',
      title: copy(locale, '创建消息（Claude Code 专用通道）', 'Create a message (Claude Code channel)'),
      description: copy(
        locale,
        '与 /v1/messages 同构，但额外附带 300 秒全局截止时间：单请求超过 300s（含重试与多账号故障转移）会被主动终止，适配 Claude Code 长会话。请求/响应格式与 /v1/messages 完全一致。',
        'Identical to /v1/messages but with a 300s global deadline: a single request (including retries and multi-account failover) is terminated past 300s, fitting Claude Code long sessions. Request/response format is exactly the same as /v1/messages.',
      ),
      parameters: undefined,
      defaultBody: MESSAGES_BODY_ZH,
      curl: `curl --request POST \\
  --url ${base}/cc/v1/messages \\
  ${authHeader} \\
  --header 'content-type: application/json' \\
  --data '${MESSAGES_BODY_ZH.replace(/\n\s*/g, '')}'`,
      responses: [
        { code: 200, label: copy(locale, '流式响应（SSE）', 'Streaming response (SSE)'), body: MESSAGES_RESP },
      ],
      ...common,
    },
    {
      id: 'api-chat-completions',
      method: 'POST',
      path: '/v1/chat/completions',
      title: copy(locale, '对话补全（OpenAI 兼容）', 'Chat completion (OpenAI compatible)'),
      description: copy(
        locale,
        'OpenAI Chat Completions 兼容端点：请求被翻译为 Anthropic Messages 格式走完整下游链路，响应转回 OpenAI 格式。任意 OpenAI SDK 客户端把 baseURL 指到 <地址>/v1 即可使用。',
        'OpenAI Chat Completions compatible endpoint: requests are translated to Anthropic Messages format and run through the full downstream pipeline, then translated back. Point any OpenAI SDK client at <base>/v1.',
      ),
      parameters: [
        { name: 'model', type: 'string', required: true, description: copy(locale, '模型名，同 /v1/messages 的关键词映射规则', 'Model name; same keyword mapping as /v1/messages') },
        { name: 'messages', type: 'array', required: true, description: copy(locale, 'OpenAI 格式消息数组（role/content）', 'OpenAI-format message array (role/content)') },
        { name: 'stream', type: 'boolean', required: false, description: copy(locale, 'true 返回 OpenAI chunk 流（data: {...} / data: [DONE]）', 'true returns an OpenAI chunk stream (data: {...} / data: [DONE])') },
        { name: 'max_tokens', type: 'integer', required: false, description: copy(locale, '输出上限（内部映射到 Anthropic max_tokens）', 'Output cap (mapped internally to Anthropic max_tokens)') },
      ],
      defaultBody: CHAT_BODY,
      curl: `curl --request POST \\
  --url ${base}/v1/chat/completions \\
  ${authHeader} \\
  --header 'content-type: application/json' \\
  --data '${CHAT_BODY.replace(/\n\s*/g, '')}'`,
      responses: [
        { code: 200, label: copy(locale, '流式响应（SSE）', 'Streaming response (SSE)'), body: CHAT_RESP },
      ],
      ...common,
    },
    {
      id: 'api-responses',
      method: 'POST',
      path: '/v1/responses',
      title: copy(locale, '创建响应（OpenAI Responses 兼容）', 'Create a response (OpenAI Responses compatible)'),
      description: copy(
        locale,
        'OpenAI Responses API 兼容端点，Codex CLI 的原生协议。请求被翻译为 Anthropic Messages 走下游链路，响应转回 Responses 事件流/JSON。',
        'OpenAI Responses API compatible endpoint, the native protocol of Codex CLI. Requests are translated to Anthropic Messages downstream, and responses translated back to Responses events/JSON.',
      ),
      parameters: [
        { name: 'model', type: 'string', required: true, description: copy(locale, '模型名，同 /v1/messages 的关键词映射规则', 'Model name; same keyword mapping as /v1/messages') },
        { name: 'input', type: 'string | array', required: true, description: copy(locale, '输入内容：纯文本或 Responses 格式的消息数组', 'Input: plain text or a Responses-format message array') },
        { name: 'stream', type: 'boolean', required: false, description: copy(locale, 'true 返回 Responses 事件流（response.created / output_text.delta / response.completed）', 'true returns a Responses event stream (response.created / output_text.delta / response.completed)') },
      ],
      defaultBody: RESPONSES_BODY,
      curl: `curl --request POST \\
  --url ${base}/v1/responses \\
  ${authHeader} \\
  --header 'content-type: application/json' \\
  --data '${RESPONSES_BODY.replace(/\n\s*/g, '')}'`,
      responses: [
        { code: 200, label: copy(locale, '流式响应（SSE）', 'Streaming response (SSE)'), body: RESPONSES_RESP },
      ],
      ...common,
    },
    {
      id: 'api-models',
      method: 'GET',
      path: '/v1/models',
      title: copy(locale, '模型列表', 'List models'),
      description: copy(
        locale,
        'OpenAI 格式的模型列表，返回服务提供的模型 ID 与显示名（列表有缓存，上游刷新失败时回退内置模型表）。开启 modelsOfficialAliases 后会追加官方连字符别名（如 claude-opus-4.6 → claude-opus-4-6）。',
        'OpenAI-format model list returning the model IDs and display names this service offers (the list is cached; when the upstream refresh fails it falls back to a built-in model table). With modelsOfficialAliases on, official hyphenated aliases are appended (e.g. claude-opus-4.6 → claude-opus-4-6).',
      ),
      parameters: undefined,
      curl: `curl --url ${base}/v1/models \\
  ${authHeader}`,
      responses: [
        { code: 200, label: copy(locale, '响应', 'Response'), body: MODELS_RESP },
      ],
      ...common,
    },
  ]
}
