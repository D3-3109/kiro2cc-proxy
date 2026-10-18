// Copyright (c) 2026 Harllan He. Licensed under MIT.
// 「按模型分组（当前页）」可折叠面板：账号详情页与 API Key 详情页共用，
// 聚合逻辑与表格结构自 credential-detail-page.tsx 搬移复用。
import { useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronDown } from 'lucide-react'
import { CELL, PANEL, TH_BASE } from '@/components/table-kit'
import { formatCost, getModelColor, recordCredits } from '@/components/usage-log-table'
import { formatTokenCount, localeTag } from '@/lib/locale'
import type { UsageRecord } from '@/types/api'

/** 逐模型用量表头：模型 + 5 个右对齐数值列 */
const MODEL_COLS: { labelKey: string; num?: boolean }[] = [
  { labelKey: 'common.colModel' },
  { labelKey: 'apiKeys.colRequests', num: true },
  { labelKey: 'apiKeys.colInputTokens', num: true },
  { labelKey: 'apiKeys.colOutputTokens', num: true },
  { labelKey: 'common.colCost', num: true },
  { labelKey: 'apiKeys.colCredits', num: true },
]

interface ModelAgg {
  requests: number
  inputTokens: number
  outputTokens: number
  cost: number
  credits: number
  creditsSaved: number
}

// 逐模型聚合仅覆盖当前页（全量聚合需后端新接口），标题文案已含「（当前页）」限定
function aggregateByModel(records: UsageRecord[]): Record<string, ModelAgg> {
  return records.reduce<Record<string, ModelAgg>>((acc, r) => {
    const entry =
      acc[r.model] ?? { requests: 0, inputTokens: 0, outputTokens: 0, cost: 0, credits: 0, creditsSaved: 0 }
    entry.requests += 1
    entry.inputTokens += r.inputTokens
    entry.outputTokens += r.outputTokens
    entry.cost += r.estimatedCost
    entry.credits += recordCredits(r)
    entry.creditsSaved += r.creditsSaved ?? 0
    acc[r.model] = entry
    return acc
  }, {})
}

export function ModelAggPanel({ records }: { records: UsageRecord[] }) {
  const { t } = useTranslation()
  // 默认展开；两页折叠态各自独立（组件实例级 state）：切换页面卸载重挂后回到展开，
  // 同页翻页/查询刷新仅更新 records，折叠态保持
  const [open, setOpen] = useState(true)
  const bodyId = useId()

  const byModel = useMemo(() => aggregateByModel(records), [records])
  if (Object.keys(byModel).length === 0) return null

  return (
    <div className="mt-[15px]">
      {/* 标题行整行可点击折叠；保留 panel-title 视觉，chevron 指示折叠方向 */}
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        aria-controls={bodyId}
        className="mb-[7px] flex w-full items-center gap-[5px] text-left text-[10.5px] font-semibold uppercase tracking-[.07em] text-ink-3 transition-colors hover:text-ink focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
      >
        <ChevronDown
          className={`size-[13px] flex-none transition-transform ${open ? '' : '-rotate-90'}`}
          strokeWidth={1.8}
          aria-hidden="true"
        />
        {t('credentials.groupByModel')}
      </button>
      {open && (
        <section id={bodyId} aria-label={t('credentials.groupByModel')} className={PANEL}>
          <div className="max-h-[42vh] overflow-y-auto">
            <table className="w-full border-separate border-spacing-0">
              <thead>
                <tr>
                  {MODEL_COLS.map((col) => (
                    <th key={col.labelKey} className={`${TH_BASE} ${col.num ? 'text-right' : ''}`}>
                      {t(col.labelKey)}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="[&_tr:last-child>td]:border-b-0">
                {Object.entries(byModel).map(([model, m]) => (
                  <tr key={model} className="transition-colors hover:bg-surface-2">
                    <td className={CELL}>
                      <span className={`font-mono text-[12px] ${getModelColor(model)}`}>{model}</span>
                    </td>
                    <td className={`${CELL} text-right font-mono text-[12px] tabular-nums text-ink-2`}>
                      {m.requests.toLocaleString(localeTag())}
                    </td>
                    <td className={`${CELL} text-right font-mono text-[12px] tabular-nums text-ink-2`}>
                      {formatTokenCount(m.inputTokens)}
                    </td>
                    <td className={`${CELL} text-right font-mono text-[12px] tabular-nums text-ink-2`}>
                      {formatTokenCount(m.outputTokens)}
                    </td>
                    <td className={`${CELL} text-right font-mono text-[12px] font-medium tabular-nums text-warn`}>
                      {formatCost(m.cost)}
                    </td>
                    <td className={`${CELL} text-right font-mono text-[12px] font-medium tabular-nums text-brand`}>
                      {m.credits.toFixed(4)}
                      {m.creditsSaved > 0 && (
                        <small className="block text-[10px] font-normal text-ok">
                          {t('common.savedPrefix', { amount: m.creditsSaved.toFixed(4) })}
                        </small>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      )}
    </div>
  )
}
