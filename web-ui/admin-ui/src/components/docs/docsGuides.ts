// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 使用文档页开发指南与排障指南内容（自 codex2cc docsGuides 适配为 kiro2cc 端点与机制）

export type DocsSection = 'quick-start' | 'development' | 'model-api' | 'troubleshooting'

export type GuideSpec = {
  id: string
  section: DocsSection
  title: string
  summary: string
  paragraphs?: string[]
  table?: { headers: string[]; rows: string[][] }
  examples?: { label: string; content: string }[]
}

export type DocsLocale = 'zh' | 'en'

export function buildGuides(baseUrl: string, locale: DocsLocale): GuideSpec[] {
  const c = (zh: string, en: string) => (locale === 'zh' ? zh : en)
  const base = baseUrl.replace(/\/+$/, '')
  return [
    {
      id: 'authentication',
      section: 'development',
      title: c('地址与认证', 'URLs & authentication'),
      summary: c(
        '先确认请求发给谁，以及使用哪一种密钥。',
        'Choose the destination and the correct credential first.',
      ),
      paragraphs: [
        c(
          '客户端填写的是本服务的访问地址。账号的代理地址是服务访问上游时使用的出站代理，不是客户端 API Base URL。',
          'Clients connect to this service. An account proxy is the outbound proxy used when reaching Kiro upstream; it is not the client API Base URL.',
        ),
        c(
          '/v1/messages 与 /cc/v1/messages 使用后台创建的子 API Key（sk- 前缀），支持 Authorization: Bearer 或 x-api-key 两种请求头。管理后台（adminPsw）与子 Key 是两套独立凭据，不要混用。',
          '/v1/messages and /cc/v1/messages authenticate with a sub API key (sk- prefix) created in the dashboard, via either Authorization: Bearer or x-api-key. The admin panel (adminPsw) and sub keys are separate credentials; do not mix them.',
        ),
      ],
      table: {
        headers: [c('用途', 'Use'), c('地址 / 请求头', 'URL / header'), c('说明', 'Notes')],
        rows: [
          [
            'Claude Code',
            'ANTHROPIC_BASE_URL',
            c(
              '客户端自动追加 /v1/messages。',
              'The client appends /v1/messages itself.',
            ),
          ],
          [
            'Claude Code（长会话）',
            `${base}/cc`,
            c(
              '300s 全局截止时间，适配长会话不卡死。',
              '300s global deadline for long sessions.',
            ),
          ],
          [
            'OpenAI SDK',
            `${base}/v1`,
            c(
              'SDK 在 Base URL 后追加 /chat/completions 等资源路径。',
              'The SDK appends resource paths such as /chat/completions.',
            ),
          ],
          [
            c('公共 API', 'Public API'),
            'Authorization: Bearer sk-...',
            c(
              'Messages 端点也接受 x-api-key。',
              'The Messages endpoints also accept x-api-key.',
            ),
          ],
        ],
      },
    },
    {
      id: 'model-mapping',
      section: 'development',
      title: c('模型映射规则', 'Model mapping'),
      summary: c(
        '请求模型名按关键词路由到 Kiro 模型。',
        'Requested model names are keyword-routed to Kiro models.',
      ),
      paragraphs: [
        c(
          '模型名含 sonnet / opus / haiku / gpt 等关键词即路由到对应 Kiro 模型；版本号（4.5 / 4.6 / 4.7 / 4.8 / 5）决定具体代际，未明确指定版本时 opus 兜底 4.6。模型名含 thinking 后缀会自动启用扩展思考。',
          'A model name containing sonnet / opus / haiku / gpt keywords routes to the corresponding Kiro model; the version number (4.5 / 4.6 / 4.7 / 4.8 / 5) picks the generation, with opus falling back to 4.6 when unspecified. A thinking suffix enables extended thinking automatically.',
        ),
        c(
          'max_tokens 按模型代际有约束：下限 1024 对全部 Claude 代际生效，上限按代际分 128K / 64K 档；GPT 系模型跳过 additionalModelRequestFields 构建逻辑。/v1/models 返回账号池实际可用的模型列表，含显示名与倍率。',
          'max_tokens is constrained per generation: the 1024 floor applies to every Claude generation with 128K / 64K ceilings by tier; GPT-family models skip the additionalModelRequestFields build. /v1/models lists the models actually available in the account pool with display names and rate multipliers.',
        ),
      ],
      table: {
        headers: [
          c('请求模型名', 'Requested name'),
          c('路由目标', 'Routed to'),
          c('说明', 'Notes'),
        ],
        rows: [
          [
            'claude-sonnet-4-5',
            c('Sonnet 4.5 代际', 'Sonnet 4.5 generation'),
            c('含 sonnet + 4.5', 'contains sonnet + 4.5'),
          ],
          [
            'claude-opus-4-6-thinking',
            c('Opus 4.6 + 扩展思考', 'Opus 4.6 + extended thinking'),
            c('thinking 后缀启用思考', 'thinking suffix enables thinking'),
          ],
          [
            'gpt-5',
            c('GPT 系映射', 'GPT-family mapping'),
            c('跳过代际字段构建', 'skips generation field build'),
          ],
        ],
      },
    },
    {
      id: 'streaming',
      section: 'development',
      title: c('流式响应', 'Streaming responses'),
      summary: c(
        '三个消息端点都支持 SSE 流式输出。',
        'All three message endpoints support SSE streaming.',
      ),
      paragraphs: [
        c(
          'stream=true 时返回 text/event-stream：Anthropic 端点（/v1/messages、/cc/v1/messages）输出 message_start / content_block_delta / message_stop 事件序列；OpenAI 兼容端点输出 chunk 流并以 data: [DONE] 结束（Chat）或 response.completed 结束（Responses）。',
          'With stream=true the response is text/event-stream: the Anthropic endpoints (/v1/messages, /cc/v1/messages) emit message_start / content_block_delta / message_stop events; the OpenAI-compatible endpoints emit chunk streams terminated by data: [DONE] (Chat) or response.completed (Responses).',
        ),
        c(
          '思维链内容以 thinking 块返回；开启 thinkingAsText 设置后，思考块会被改写为逐行流出的引用文本（带灰色 ANSI 样式），便于在 Claude Code 中直接看到思考过程。用量统计在流的末尾事件中校准。',
          'Chain-of-thought is returned as thinking blocks; with the thinkingAsText setting on, thinking blocks are rewritten into line-by-line quoted text (with dim ANSI styling) so Claude Code shows the reasoning inline. Usage is calibrated in the terminal events of the stream.',
        ),
      ],
    },
    {
      id: 'prompt-cache',
      section: 'development',
      title: c('Prompt 缓存机制', 'Prompt caching'),
      summary: c(
        'history[0] 冻结与前缀缓存。',
        'history[0] freeze and prefix caching.',
      ),
      paragraphs: [
        c(
          '服务通过冻结 history[0]（首条消息）保证 Kiro 侧前缀缓存跨请求逐字节稳定：系统提示中的计费哈希规范化、会话连续性 ID 派生等机制都为此服务。客户端侧不要在会话中途改写首条消息，否则上游缓存全量失效、首字延迟显著升高。',
          'The service freezes history[0] (the first message) so the Kiro prefix cache stays byte-stable across requests: billing-hash normalization in the system prompt and conversation-derived continuation IDs all serve this. Do not rewrite the first message mid-session on the client side, or the upstream cache fully invalidates and TTFT rises sharply.',
        ),
        c(
          '用量中的 cache_read / cache_creation 上报有四层降级链：metering 真值 → 前缀估算 → 前缀指纹 → 比例模拟。开启 promptCacheAccounting 后改用 cache_control 断点模拟（按 API Key + Kiro 模型隔离），记账口径更接近 Anthropic 官方，属本地模拟、不代表 Kiro 真实成本。',
          'Reported cache_read / cache_creation goes through a four-tier fallback: metering truth → prefix estimate → prefix fingerprint → ratio simulation. With promptCacheAccounting on, cache_control breakpoint simulation takes over (isolated per API key + Kiro model), which tracks Anthropic accounting closer but is a local simulation, not Kiro real cost.',
        ),
      ],
    },
    {
      id: 'error-429',
      section: 'troubleshooting',
      title: c('429 与限流桶', '429 and rate-limit buckets'),
      summary: c(
        '上游 4 个端点对应 4 个独立限流桶。',
        'The 4 upstream endpoints map to 4 independent rate-limit buckets.',
      ),
      paragraphs: [
        c(
          'Kiro 上游按 ide / runtime / codewhisperer / amazonq 四个端点独立限流，桶级 429 按「凭据 + 端点」二元组隔离，单次 429 触发该桶 30 秒封禁。客户端看到的 429 通常意味着该账号在某端点配额耗尽，服务会自动故障转移到其他账号。',
          'Kiro upstream rate-limits the ide / runtime / codewhisperer / amazonq endpoints independently; a bucket-level 429 is isolated per (credential, endpoint) pair and bans that bucket for 30s. A client-facing 429 usually means the account exhausted quota on one endpoint; the service fails over to other accounts automatically.',
        ),
        c(
          '连续 429 时：在账号管理页确认账号余额与启用状态；调整账号优先级让流量优先走健康账号；或降低客户端并发。',
          'On repeated 429s: check account balance and enabled state on the Accounts page; adjust priorities so traffic prefers healthy accounts; or lower client concurrency.',
        ),
      ],
    },
    {
      id: 'error-401',
      section: 'troubleshooting',
      title: c('401 与认证失败', '401 and auth failures'),
      summary: c(
        '子 Key 无效、过期或额度耗尽。',
        'Invalid, expired or exhausted sub keys.',
      ),
      paragraphs: [
        c(
          '401 表示子 API Key 认证失败：Key 已删除/禁用、有效期已过、或额度（USD / credits 限额）耗尽。到 API Keys 页核对 Key 状态、有效期与消费额；懒激活 Key 首次使用才开始计时。',
          '401 means sub API key authentication failed: the key was deleted/disabled, expired, or hit its spending limit. Check key status, expiry and spend on the API Keys page; lazily-activated keys only start counting on first use.',
        ),
        c(
          '另一个 401 来源是上游账号 token 刷新失败：此时服务会按凭据状态标记账号异常并故障转移。到账号管理页查看失败日志与凭据状态。',
          'Another 401 source is upstream token refresh failure: the service marks the account abnormal and fails over. Check failure logs and credential state on the Accounts page.',
        ),
      ],
    },
    {
      id: 'connection-check',
      section: 'troubleshooting',
      title: c('连接排查步骤', 'Connection checklist'),
      summary: c(
        '从服务在线到生成可用的五步排查。',
        'Five steps from service-up to working generation.',
      ),
      paragraphs: [
        c(
          '按下述顺序排查：① 管理后台能打开且侧栏显示版本号 → 服务在线；② 用快速接入页的 cURL 示例（选一个有效子 Key）发一次 /v1/messages → 看到 message_stop 或完整 JSON → 链路通；③ 仍 401 → 检查 Key；④ 429 → 见限流桶条目；⑤ 200 但内容异常 → 到「查看日志」开 DEBUG 日志复现，再到使用日志核对最终模型与用量。',
          'Troubleshoot in order: ① the admin panel opens and the sidebar shows a version → service up; ② send /v1/messages once with the cURL example from Quick start (pick a valid sub key) → seeing message_stop or full JSON → the pipeline works; ③ still 401 → check the key; ④ 429 → see the rate-limit entry; ⑤ 200 but odd content → turn on DEBUG logs in Log Viewer, reproduce, then verify the final model and usage in Usage.',
        ),
      ],
      examples: [
        {
          label: '排查用最小请求（stream=false）',
          content: `curl --request POST \\
  --url ${base}/v1/messages \\
  --header 'Authorization: Bearer sk-...' \\
  --header 'content-type: application/json' \\
  --data '{"model":"claude-sonnet-4-5","max_tokens":1024,"messages":[{"role":"user","content":"ping"}]}'`,
        },
      ],
    },
  ]
}
