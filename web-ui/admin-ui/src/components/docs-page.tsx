// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 使用文档页（自 codex2cc Docs.tsx 适配）：hero + 分区 tabs + 左内容/右 TOC +
// hash 路由 + API Key 填充 + 四客户端配置生成器（Claude Code / Codex CLI / OpenAI SDK / CC Switch）
import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import {
  BookOpen,
  ExternalLink,
  LifeBuoy,
  Server,
  Sparkles,
  Wrench,
} from 'lucide-react'
import { useApiKeys } from '@/hooks/use-credentials'
import { buildCcSwitchDeeplink, importToCcSwitch } from '@/lib/ccswitch'
import { resolveDocsTarget, type DocEntry } from '@/components/docs/docsNavigation'
import { buildEndpointSpecs } from '@/components/docs/docsContent'
import { buildGuides, type DocsSection } from '@/components/docs/docsGuides'
import { CodeBlock, EndpointDoc } from '@/components/docs/EndpointDoc'
import DocsTOC, { type DocsTOCItem } from '@/components/docs/DocsTOC'
import '@/components/docs/docs.css'

type QuickToolTab = 'claude-code' | 'codex-cli' | 'openai-sdk' | 'cc-switch'
type OsTab = 'unix' | 'windows'

/** 静态兜底模型名（用户已确认剔除模型下拉；示例统一用此值） */
const FALLBACK_MODEL = 'claude-sonnet-4-5'

const TOOL_TABS: { value: QuickToolTab; label: string }[] = [
  { value: 'claude-code', label: 'Claude Code' },
  { value: 'codex-cli', label: 'Codex CLI' },
  { value: 'openai-sdk', label: 'OpenAI SDK' },
  { value: 'cc-switch', label: 'CC Switch' },
]

function OsTabs({ active, onChange }: { active: OsTab; onChange: (v: OsTab) => void }) {
  const { t } = useTranslation()
  return (
    <div className="docs-tabs" role="tablist" aria-label={t('docs.osTabsLabel')}>
      <a
        href="#os-unix"
        role="tab"
        aria-selected={active === 'unix'}
        aria-current={active === 'unix' ? 'page' : undefined}
        onClick={(e) => {
          e.preventDefault()
          onChange('unix')
        }}
      >
        macOS / Linux
      </a>
      <a
        href="#os-windows"
        role="tab"
        aria-selected={active === 'windows'}
        aria-current={active === 'windows' ? 'page' : undefined}
        onClick={(e) => {
          e.preventDefault()
          onChange('windows')
        }}
      >
        Windows
      </a>
    </div>
  )
}

export function DocsPage() {
  const { t, i18n } = useTranslation()
  const locale = (i18n.language === 'en' ? 'en' : 'zh') as 'zh' | 'en'
  const { data: apiKeys } = useApiKeys()

  const [baseUrl, setBaseUrl] = useState(window.location.origin)
  const [selectedKeyId, setSelectedKeyId] = useState<number | null>(null)
  const [activeToolTab, setActiveToolTab] = useState<QuickToolTab>('claude-code')
  const [claudeOs, setClaudeOs] = useState<OsTab>('unix')
  const [codexOs, setCodexOs] = useState<OsTab>('unix')
  const [hash, setHash] = useState(window.location.hash)
  useEffect(() => {
    const onHashChange = () => setHash(window.location.hash)
    window.addEventListener('hashchange', onHashChange)
    return () => window.removeEventListener('hashchange', onHashChange)
  }, [])

  // 默认选中第一个启用的 Key；被删的 Key 回落并自动选中下一个可用 Key
  useEffect(() => {
    if (selectedKeyId === null && apiKeys && apiKeys.length > 0) {
      const enabled = apiKeys.find((k) => k.enabled) ?? apiKeys[0]
      setSelectedKeyId(enabled.id)
    }
    if (selectedKeyId !== null && apiKeys && !apiKeys.some((k) => k.id === selectedKeyId)) {
      setSelectedKeyId(null)
    }
  }, [apiKeys, selectedKeyId])

  const selectedKey = apiKeys?.find((k) => k.id === selectedKeyId) ?? null
  const apiKey = selectedKey?.key ?? '<API_KEY>'

  const guides = useMemo(() => buildGuides(baseUrl, locale), [baseUrl, locale])
  const endpoints = useMemo(() => buildEndpointSpecs(baseUrl, locale), [baseUrl, locale])

  const sections = useMemo(
    () => [
      { id: 'quick-start', label: t('docs.sectionQuickStart'), icon: Sparkles },
      { id: 'development', label: t('docs.sectionDevelopment'), icon: Wrench },
      { id: 'model-api', label: t('docs.sectionModelApi'), icon: Server },
      { id: 'troubleshooting', label: t('docs.sectionTroubleshooting'), icon: LifeBuoy },
    ],
    [t],
  )

  const entries = useMemo<DocEntry[]>(
    () => [
      ...sections.map((s) => ({ id: s.id, section: s.id, title: s.label })),
      ...guides.map((g) => ({ id: g.id, section: g.section, title: g.title })),
      ...endpoints.map((e) => ({
        id: e.id,
        section: 'model-api',
        title: e.path,
        method: e.method,
      })),
    ],
    [sections, guides, endpoints],
  )

  const target = useMemo(() => resolveDocsTarget(hash, entries), [hash, entries])
  const activeSection = target.section as DocsSection
  const section = sections.find((s) => s.id === activeSection) ?? sections[0]

  // hash 直达滚动补偿：mount 前浏览器 fragment 滚动已发生（元素尚未渲染），由 effect 补一次；
  // 无 fragment 的普通入口不滚动，避免进入文档页即跳过 hero
  useEffect(() => {
    if (!hash) return
    document.getElementById(target.id)?.scrollIntoView({ block: 'start' })
  }, [target.id, hash])

  const tocItems = useMemo<DocsTOCItem[]>(
    () => [
      {
        id: section.id,
        label: section.label,
        children: [
          ...entries
            .filter((e) => e.section === activeSection && e.id !== activeSection)
            .map((e) => ({ id: e.id, label: e.title, method: e.method })),
        ],
      },
    ],
    [section, entries, activeSection],
  )

  // —— 客户端配置生成（Claude Code / Codex CLI / OpenAI SDK）——
  const claudeEnvUnix = `export ANTHROPIC_BASE_URL="${baseUrl}"
export ANTHROPIC_AUTH_TOKEN="${apiKey}"
export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`
  const claudeEnvWindows = `setx ANTHROPIC_BASE_URL "${baseUrl}"
setx ANTHROPIC_AUTH_TOKEN "${apiKey}"
setx CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC 1`
  const claudeSettings = `{
  "env": {
    "ANTHROPIC_BASE_URL": "${baseUrl}",
    "ANTHROPIC_AUTH_TOKEN": "${apiKey}"
  }
}`

  const codexConfig = `model = "${FALLBACK_MODEL}"
model_provider = "kiro2cc"

[model_providers.kiro2cc]
name = "kiro2cc"
base_url = "${baseUrl}/v1"
wire_api = "responses"`
  const codexAuth = `OPENAI_API_KEY="${apiKey}"`

  const openaiSdk = `from openai import OpenAI

client = OpenAI(
    base_url="${baseUrl}/v1",
    api_key="${apiKey}",
)

resp = client.chat.completions.create(
    model="${FALLBACK_MODEL}",
    messages=[{"role": "user", "content": "Hello!"}],
)
print(resp.choices[0].message.content)`

  const ccSwitchLink = selectedKey
    ? buildCcSwitchDeeplink('claude', { apiKey, keyName: selectedKey.name, baseUrl })
    : ''

  const curlExample = `curl --request POST \\
  --url ${baseUrl}/v1/messages \\
  --header 'Authorization: Bearer ${apiKey}' \\
  --header 'content-type: application/json' \\
  --data '{"model":"${FALLBACK_MODEL}","max_tokens":1024,"messages":[{"role":"user","content":"ping"}]}'`

  const handleCcSwitchImport = () => {
    if (!selectedKey) return
    importToCcSwitch(
      'claude',
      { apiKey, keyName: selectedKey.name, baseUrl },
      { onNotInstalled: () => toast.error(t('docs.ccSwitchNotInstalled')) },
    )
  }

  return (
    <div>
      <header className="docs-hero">
        <div>
          <div className="docs-eyebrow">
            <BookOpen className="size-3.5" />
            KIRO2CC / {t('docs.eyebrow')}
          </div>
          <h2>{t('docs.heroTitle')}</h2>
          <p>{t('docs.heroSubtitle')}</p>
        </div>
      </header>

      <div className="docs-toolbar">
        <nav className="docs-tabs" aria-label={t('docs.tabsLabel')}>
          {sections.map((s) => (
            <a
              key={s.id}
              href={`#${s.id}`}
              aria-current={activeSection === s.id ? 'page' : undefined}
            >
              <s.icon className="size-4" />
              {s.label}
            </a>
          ))}
        </nav>
      </div>

      <div className="docs-layout">
        <div className="docs-content">
          <div id={section.id} className="docs-section-heading scroll-mt-24">
            {activeSection !== 'quick-start' && (
              <>
                <h3>{section.label}</h3>
                <p>{t(`docs.sectionSummaries.${section.id}`)}</p>
              </>
            )}
          </div>

          {activeSection === 'quick-start' && (
            <>
              <div className="docs-steps">
                {(['step1', 'step2', 'step3'] as const).map((key, i) => (
                  <div key={key} className="docs-step">
                    <span>{`0${i + 1}`}</span>
                    <div>
                      <strong>{t(`docs.steps.${key}.title`)}</strong>
                      <p>{t(`docs.steps.${key}.detail`)}</p>
                    </div>
                  </div>
                ))}
              </div>

              <div className="docs-callout" style={{ marginTop: 0 }}>
                <label className="mr-2 font-semibold">{t('docs.selectKey')}</label>
                <select
                  value={selectedKeyId ?? ''}
                  onChange={(e) => setSelectedKeyId(e.target.value ? Number(e.target.value) : null)}
                  className="rounded-[7px] border-[0.5px] border-hairline-2 bg-surface-2 px-2 py-1 text-[12px]"
                  aria-label={t('docs.selectKey')}
                >
                  {apiKeys && apiKeys.length > 0 ? (
                    apiKeys.map((k) => (
                      <option key={k.id} value={k.id}>
                        {k.name}（{k.key.slice(0, 7)}...{k.key.slice(-4)}）
                      </option>
                    ))
                  ) : (
                    <option value="">{t('docs.noKeys')}</option>
                  )}
                </select>
                <span className="ml-2 text-ink-3">{t('docs.baseUrlLabel')}</span>
                <input
                  value={baseUrl}
                  onChange={(e) => setBaseUrl(e.target.value)}
                  className="ml-1 w-[240px] rounded-[7px] border-[0.5px] border-hairline-2 bg-surface-2 px-2 py-1 font-mono text-[12px]"
                  aria-label={t('docs.baseUrlLabel')}
                />
              </div>

              <div className="mt-4">
                <div className="docs-tabs mb-3" role="tablist" aria-label={t('docs.toolsLabel')}>
                  {TOOL_TABS.map((tab) => (
                    <a
                      key={tab.value}
                      href={`#qs-${tab.value}`}
                      role="tab"
                      aria-selected={activeToolTab === tab.value}
                      aria-current={activeToolTab === tab.value ? 'page' : undefined}
                      onClick={(e) => {
                        e.preventDefault()
                        setActiveToolTab(tab.value)
                      }}
                    >
                      {tab.label}
                    </a>
                  ))}
                </div>

                {activeToolTab === 'claude-code' && (
                  <div className="space-y-3">
                    <OsTabs active={claudeOs} onChange={setClaudeOs} />
                    <CodeBlock
                      label={claudeOs === 'unix' ? t('docs.unixTerminal') : t('docs.windowsTerminal')}
                      content={claudeOs === 'unix' ? claudeEnvUnix : claudeEnvWindows}
                    />
                    <CodeBlock label="~/.claude/settings.json" content={claudeSettings} />
                  </div>
                )}
                {activeToolTab === 'codex-cli' && (
                  <div className="space-y-3">
                    <OsTabs active={codexOs} onChange={setCodexOs} />
                    <CodeBlock label="~/.codex/config.toml" content={codexConfig} />
                    <CodeBlock
                      label={codexOs === 'unix' ? '~/.codex/.env' : '%USERPROFILE%\\.codex\\.env'}
                      content={codexAuth}
                    />
                  </div>
                )}
                {activeToolTab === 'openai-sdk' && <CodeBlock label="Python" content={openaiSdk} />}
                {activeToolTab === 'cc-switch' && (
                  <div className="space-y-3">
                    <p className="text-[12.5px] leading-relaxed text-ink-2">
                      {t('docs.ccSwitchDesc')}
                    </p>
                    <CodeBlock label="ccswitch://" content={ccSwitchLink || t('docs.selectKeyFirst')} />
                    <button
                      type="button"
                      disabled={!selectedKey}
                      onClick={handleCcSwitchImport}
                      className="inline-flex h-8 items-center gap-1.5 rounded-[8px] bg-brand px-3 text-[12.5px] font-semibold text-brand-fg transition-colors hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      <ExternalLink className="size-3.5" />
                      {t('docs.ccSwitchLaunch')}
                    </button>
                  </div>
                )}
              </div>

              <div id="qs-curl" className="mt-5 scroll-mt-24">
                <h4 className="docs-group-title">{t('docs.curlTitle')}</h4>
                <CodeBlock label="cURL" content={curlExample} />
              </div>

              <p className="docs-callout">
                {t('docs.verifyNote')}{' '}
                <a href="#connection-check">{t('docs.verifyLink')}</a>
              </p>
            </>
          )}

          {guides
            .filter((g) => g.section === activeSection)
            .map((g) => (
              <article key={g.id} id={g.id} className="docs-guide scroll-mt-24">
                <div className="docs-guide-heading">
                  <div>
                    <h3>
                      <a href={`#${g.id}`}>{g.title}</a>
                    </h3>
                    <p>{g.summary}</p>
                  </div>
                </div>
                <div className="docs-guide-body">
                  {g.paragraphs?.map((paragraph, i) => (
                    <p key={i}>{paragraph}</p>
                  ))}
                  {g.table && (
                    <div className="docs-table-wrap">
                      <table>
                        <thead>
                          <tr>
                            {g.table.headers.map((h) => (
                              <th key={h} scope="col">
                                {h}
                              </th>
                            ))}
                          </tr>
                        </thead>
                        <tbody>
                          {g.table.rows.map((row, i) => (
                            <tr key={i}>
                              {row.map((cell, j) => (
                                <td key={j}>{cell}</td>
                              ))}
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                  {g.examples?.map((example) => (
                    <CodeBlock key={example.label} label={example.label} content={example.content} />
                  ))}
                </div>
              </article>
            ))}

          {activeSection === 'model-api' && (
            <section>
              <h4 className="docs-group-title">
                {t('docs.endpointsGroup')}
                <span>{endpoints.length}</span>
              </h4>
              <div className="space-y-3">
                {endpoints.map((endpoint) => (
                  <EndpointDoc key={endpoint.id} endpoint={endpoint} activeId={target.id} />
                ))}
              </div>
            </section>
          )}
        </div>

        <aside className="docs-sidebar">
          <DocsTOC items={tocItems} title={t('docs.tocTitle')} activeId={target.id} />
        </aside>
      </div>
    </div>
  )
}
