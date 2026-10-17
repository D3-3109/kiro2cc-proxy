// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { CalendarDays, Loader2, RotateCw, SearchX } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { DailyCreditsTrendChart } from '@/components/daily-credits-trend-chart'
import { Delta, FootSep, Metric, MetricAside, MetricFoot, MetricValue, MetricsBar, Ring } from '@/components/metrics'
import { PageHead } from '@/components/page-head'
import { SearchBox, Segmented, Toolbar, UpdatedAgo } from '@/components/toolbar'
import { CELL, PANEL, PANEL_FOOT, PANEL_TITLE, Pager, TH_BASE } from '@/components/table-kit'
import { UsageLogTable, formatCost, recordCredits } from '@/components/usage-log-table'
import { useDailyUsage, useDailyUsageRecords, useUsageRecordsByRange } from '@/hooks/use-credentials'
import { useIpGeo } from '@/hooks/use-ip-geo'
import { extractErrorMessage } from '@/lib/utils'
import { formatTokenCount, localeTag } from '@/lib/locale'
import type { DailySummary } from '@/types/api'

interface DailyStatsPageProps {
  onBack: () => void
  onViewDay: (date: string) => void
  showBack?: boolean
  range: RangeKey
  onRangeChange: (r: RangeKey) => void
  customRange: CustomRange | null
  onCustomRangeChange: (r: CustomRange | null) => void
}

export type RangeKey = 'today' | '7' | '14' | '30' | 'custom'
const RANGE_DAYS: Record<'7' | '14' | '30', number> = { '7': 7, '14': 14, '30': 30 }

/** 已应用的自定义区间（CST 日历日，闭区间含两端） */
export interface CustomRange {
  from: string
  to: string
}

type UsageFilter = 'active' | 'all'

const DAY_MS = 86_400_000
/**
 * 后端 `get_daily_summaries()`（src/model/usage.rs:738）按 CST(UTC+8) 聚合，
 * 前端日期窗口必须同口径 —— 按 UTC 推算会让 CST 00:00–08:00 的当日数据落在窗口外。
 */
const CST_OFFSET_MS = 8 * 3_600_000

/** daysAgo 天前的 CST 日历日（YYYY-MM-DD） */
function cstDateString(daysAgo: number): string {
  const cstMidnightMs = Math.floor((Date.now() + CST_OFFSET_MS) / DAY_MS) * DAY_MS
  return new Date(cstMidnightMs - daysAgo * DAY_MS).toISOString().slice(0, 10)
}

interface DayRow {
  date: string
  credits: number
  cost: number
  requests: number
  saved: number
}

/**
 * 区间内每一天的零填充序列（升序）。
 *
 * 有意不做 useMemo —— 结果依赖「当前时刻」，memo 会把窗口冻结在首次渲染，
 * 跨 CST 午夜后滞后一天。`useDailyUsage` 的 refetchInterval 为 60s，重算成本可忽略。
 */
function buildWindow(data: DailySummary[] | undefined, days: number, offset: number): DayRow[] {
  const byDate = new Map((data ?? []).map((d) => [d.date, d]))
  return Array.from({ length: days }, (_, i) => {
    const date = cstDateString(offset + days - 1 - i)
    const row = byDate.get(date)
    return {
      date,
      credits: row?.totalCredits ?? 0,
      cost: row?.totalCost ?? 0,
      requests: row?.totalRequests ?? 0,
      saved: row?.totalCreditsSaved ?? 0,
    }
  })
}

/** 按 UTC 解析 + 按 UTC 取星期，避免浏览器本地时区把 CST 日历日挪一天 */
function weekdayLabel(date: string): string {
  return new Date(`${date}T00:00:00Z`).toLocaleDateString(localeTag(), { weekday: 'short', timeZone: 'UTC' })
}

function median(values: number[]): number {
  if (values.length === 0) return 0
  const sorted = [...values].sort((a, b) => a - b)
  const mid = Math.floor(sorted.length / 2)
  return sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid]
}

/** 占比 ≥ 此值视为区间主导日（设计稿 56.8% 用 accent 高亮） */
const SHARE_TONE_STRONG = 50
/** 占比 < 此值视为可忽略量（设计稿 0.2% 用 text-3 弱化） */
const SHARE_TONE_WEAK = 1

/** 占区间百分比着色：设计稿 56.8% 用 accent、20% 档用 text-2、0.2% 用 text-3 */
function shareTone(share: number): string {
  if (share >= SHARE_TONE_STRONG) return 'font-semibold text-brand'
  return share >= SHARE_TONE_WEAK ? 'text-ink-2' : 'text-ink-3'
}

const COLUMN_WIDTHS: (number | undefined)[] = [130, 78, undefined, 120]

export function DailyStatsPage({
  onBack,
  onViewDay,
  showBack = true,
  range,
  onRangeChange,
  customRange,
  onCustomRangeChange,
}: DailyStatsPageProps) {
  const { t } = useTranslation()
  // 保持无条件挂载：60s 轮询触发重渲染驱动今日视图跨午夜滚动日期（不依赖区间模式被选中）
  const { data, isLoading, refetch, dataUpdatedAt } = useDailyUsage()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [filter, setFilter] = useState<UsageFilter>('active')
  const [query, setQuery] = useState('')

  const isSummary = range !== 'today' && range !== 'custom'
  const days = isSummary ? RANGE_DAYS[range] : undefined
  const rows = days !== undefined ? buildWindow(data, days, 0) : []
  const prevRows = days !== undefined ? buildWindow(data, days, days) : []

  const total = rows.reduce((s, r) => s + r.credits, 0)
  const prevTotal = prevRows.reduce((s, r) => s + r.credits, 0)
  const totalRequests = rows.reduce((s, r) => s + r.requests, 0)
  const activeRows = rows.filter((r) => r.credits > 0)
  const peak = activeRows.reduce<DayRow | null>(
    (best, r) => (best === null || r.credits > best.credits ? r : best),
    null,
  )

  // 从区间末尾往前数的连续零消耗天数
  let zeroStreak = 0
  for (let i = rows.length - 1; i >= 0 && rows[i].credits === 0; i--) zeroStreak++
  const zeroStreakSince = zeroStreak > 0 ? rows[rows.length - zeroStreak].date : null

  const avgActive = activeRows.length > 0 ? total / activeRows.length : 0
  const avgAll = days !== undefined ? total / days : 0
  const medianActive = median(activeRows.map((r) => r.credits))
  const from = rows.length > 0 ? rows[0].date : ''
  const to = rows.length > 0 ? rows[rows.length - 1].date : ''

  const q = query.trim()
  const tableRows = (filter === 'active' ? activeRows : rows)
    .filter((r) => q === '' || r.date.includes(q))
    // 日期降序（最新在前）：设计稿原为积分降序，但日期乱序不利于按天核对，故改为时间轴倒序
    .sort((a, b) => b.date.localeCompare(a.date))

  // refetch 默认不抛异常，失败只体现在返回结果上，须显式检查（与其余页刷新同口径）
  const handleRefresh = async () => {
    const result = await refetch()
    if (result.isError) {
      toast.error(extractErrorMessage(result.error))
      return
    }
    toast.success(t('dailyStats.toastRefreshed'))
  }

  return (
    <div className="flex min-h-0 flex-col">
      <PageHead
        crumb={[t('dashboard.navMain'), t('dailyStats.pageTitle')]}
        title={t('dailyStats.pageTitle')}
        note={t('dailyStats.headNote')}
        onBack={showBack ? onBack : undefined}
        actions={
          <>
            <Segmented
              value={range}
              onChange={onRangeChange}
              groupLabel={t('dailyStats.rangeGroupLabel')}
              options={[
                { key: 'today', label: t('dailyStats.rangeToday') },
                { key: '7', label: t('dailyStats.rangeDays', { days: 7 }) },
                { key: '14', label: t('dailyStats.rangeDays', { days: 14 }) },
                { key: '30', label: t('dailyStats.rangeDays', { days: 30 }) },
              ]}
            />
            {/* 日期范围按钮：点击弹出自定义日期对话框（应用成功才切到 custom 视图） */}
            <Button
              variant="outline"
              className="gap-[7px] px-2.5 font-mono text-[11.5px] tabular-nums"
              title={t('dailyStats.rangeSpanLabel')}
              onClick={() => setDialogOpen(true)}
            >
              <CalendarDays strokeWidth={1.7} aria-hidden="true" />
              {range === 'custom' && customRange
                ? `${customRange.from.slice(5)} — ${customRange.to.slice(5)}`
                : isSummary
                  ? `${from.slice(5)} — ${to.slice(5)}`
                  : cstDateString(0)}
            </Button>
            <Button variant="outline" onClick={handleRefresh} disabled={isLoading}>
              <RotateCw className={isLoading ? 'animate-spin' : ''} aria-hidden="true" />
              {t('common.refresh')}
            </Button>
          </>
        }
      />

      {/* 明细视图（今日默认/自定义区间）：请求记录表 + 当页口径指标卡；汇总专属 UI 仅固定区间模式渲染 */}
      {range === 'today' ? (
        <TodayRecordsView />
      ) : range === 'custom' && customRange ? (
        <CustomRangeRecordsView range={customRange} />
      ) : (
        <>
      <MetricsBar>
        <Metric label={t('dailyStats.metricTotalLabel')}>
          <MetricValue value={total.toFixed(2)} unit={t('dailyStats.unitCredits')} />
          <MetricFoot className="truncate">
            <span>{t('dailyStats.rangeDays', { days })}</span>
            <FootSep />
            <span className="flex items-center gap-1">
              {t('dailyStats.metricVsPrev')}
              {/* 上一区间为 0 时百分比无意义（除零），不渲染 */}
              {prevTotal > 0 && (
                <span title={t('dailyStats.metricVsPrevHint')}>
                  <Delta percent={((total - prevTotal) / prevTotal) * 100} />
                </span>
              )}
            </span>
          </MetricFoot>
        </Metric>

        <Metric label={t('dailyStats.metricPeakLabel')}>
          <MetricValue value={peak ? peak.credits.toFixed(2) : '—'} unit={peak ? peak.date.slice(5) : undefined} />
          <MetricFoot className="truncate">
            {peak && total > 0 ? (
              <>
                <span title={t('dailyStats.metricPeakShareHint')}>
                  {t('dailyStats.metricPeakShare')}{' '}
                  <b className="font-semibold text-ink-2">{((peak.credits / total) * 100).toFixed(1)}%</b>
                </span>
                <FootSep />
                <span>
                  {t('dailyStats.metricPeakRequests')}{' '}
                  <b className="font-semibold text-ink-2">{peak.requests.toLocaleString(localeTag())}</b>
                </span>
              </>
            ) : (
              <span>{t('dailyStats.metricNoUsage')}</span>
            )}
          </MetricFoot>
        </Metric>

        <Metric label={t('dailyStats.metricActiveDaysLabel')}>
          <MetricValue value={String(activeRows.length)} unit={t('dailyStats.metricOfDays', { days })} />
          {/* pr 给 .m-aside 的 42px 环形图让位 */}
          <MetricFoot className="truncate pr-[62px]">
            {zeroStreakSince ? (
              <>
                <span>
                  {t('dailyStats.metricZeroStreak')}{' '}
                  <b className="font-semibold text-warn">{t('dailyStats.rangeDays', { days: zeroStreak })}</b>
                </span>
                <FootSep />
                <span>{t('dailyStats.metricZeroStreakSince', { date: zeroStreakSince.slice(5) })}</span>
              </>
            ) : (
              <span>{t('dailyStats.metricNoZeroStreak')}</span>
            )}
          </MetricFoot>
          <MetricAside>
            <Ring percent={days !== undefined ? (activeRows.length / days) * 100 : 0} tone="stroke-warn" size={42} />
          </MetricAside>
        </Metric>

        <Metric label={t('dailyStats.metricAvgActiveLabel')}>
          <MetricValue value={avgActive.toFixed(2)} unit={t('dailyStats.unitCreditsPerDay')} />
          <MetricFoot className="truncate">
            <span>
              {t('dailyStats.metricAvgAll')} <b className="font-semibold text-ink-2">{avgAll.toFixed(2)}</b>
            </span>
            <FootSep />
            <span>
              {t('dailyStats.metricMedian')} <b className="font-semibold text-ink-2">{medianActive.toFixed(2)}</b>
            </span>
          </MetricFoot>
        </Metric>
      </MetricsBar>

      <div className="mt-4">
        <DailyCreditsTrendChart points={rows} />
      </div>

      <Toolbar>
        <SearchBox
          value={query}
          onChange={setQuery}
          placeholder={t('dailyStats.searchPlaceholder')}
          clearLabel={t('dailyStats.searchClear')}
        />
        <Segmented
          value={filter}
          onChange={setFilter}
          groupLabel={t('dailyStats.segGroupLabel')}
          options={[
            { key: 'active', label: t('dailyStats.segActive'), count: activeRows.length },
            { key: 'all', label: t('dailyStats.segAll'), count: rows.length },
          ]}
        />
        <div className="ml-auto flex flex-wrap items-center gap-2.5">
          <span className="text-[11px] text-ink-3">{t('dailyStats.toolNote')}</span>
          <UpdatedAgo dataUpdatedAt={dataUpdatedAt} />
        </div>
      </Toolbar>

      {/* 设计稿 .panel.is-static：不吃剩余高度、不内滚，行数受区间上限 30 约束 */}
      <section className="flex-none overflow-hidden rounded-[11px] border border-transparent bg-surface shadow-panel">
        <table className="w-full border-collapse text-[12.5px]">
          <colgroup>
            {COLUMN_WIDTHS.map((w, i) => (
              <col key={i} style={w ? { width: w } : undefined} />
            ))}
          </colgroup>
          <thead>
            <tr>
              <th className={TH_BASE}>{t('dailyStats.colDate')}</th>
              <th className={TH_BASE}>{t('dailyStats.colWeekday')}</th>
              <th className={TH_BASE}>
                <div className="flex items-baseline gap-1.5">
                  <span>{t('dailyStats.colCreditsUsage')}</span>
                  <span className="font-normal normal-case tracking-normal text-ink-3">
                    {t('dailyStats.colCreditsUsageShare', { days })}
                  </span>
                </div>
              </th>
              <th className={`${TH_BASE} text-right`}>{t('dailyStats.colRequests')}</th>
            </tr>
          </thead>
          <tbody className="[&_tr:last-child>td]:border-b-0">
            {tableRows.length === 0 ? (
              <tr>
                <td colSpan={COLUMN_WIDTHS.length} className="px-3 py-14">
                  <div className="flex flex-col items-center gap-2.5 text-ink-3">
                    {isLoading ? (
                      <Loader2 className="size-6 animate-spin" aria-hidden="true" />
                    ) : (
                      <SearchX className="size-6" aria-hidden="true" />
                    )}
                    <span className="text-[12.5px]">
                      {isLoading
                        ? t('common.loading')
                        : q !== '' || filter === 'active'
                          ? t('dailyStats.emptyNoMatch')
                          : t('dailyStats.emptyNoRecords')}
                    </span>
                  </div>
                </td>
              </tr>
            ) : (
              tableRows.map((row) => {
                const share = total > 0 ? (row.credits / total) * 100 : 0
                // .bar 长度相对峰值日（设计稿 25.14 → 100%、10.57 → 42%）
                const barWidth = peak && peak.credits > 0 ? (row.credits / peak.credits) * 100 : 0
                return (
                  <tr
                    key={row.date}
                    tabIndex={0}
                    onClick={() => onViewDay(row.date)}
                    onKeyDown={(e) => {
                      if (e.key !== 'Enter' && e.key !== ' ') return
                      e.preventDefault()
                      onViewDay(row.date)
                    }}
                    title={t('dailyStats.viewDetailTitle')}
                    className="cursor-pointer transition-colors hover:bg-surface-2 focus-visible:outline focus-visible:-outline-offset-2 focus-visible:outline-brand"
                  >
                    <td className={CELL}>
                      <span className="whitespace-nowrap font-mono text-[12px] tabular-nums text-ink">{row.date}</span>
                    </td>
                    <td className={CELL}>
                      <span className="whitespace-nowrap text-[11.5px] text-ink-3">{weekdayLabel(row.date)}</span>
                    </td>
                    {/* 设计稿 .quota：绝对值 + 占区间百分比 + .bar；USD 与省额降为副行，保证费用信息不丢 */}
                    <td className={CELL}>
                      <div className="flex min-w-[190px] flex-col gap-[5px]">
                        <div className="flex items-baseline justify-between gap-2 font-mono text-[11.5px] tabular-nums">
                          <span className="text-ink-2">
                            <b className="font-semibold text-ink">{row.credits.toFixed(4)}</b>{' '}
                            <span className="text-ink-3">{t('dailyStats.unitCredits')}</span>
                          </span>
                          <span className={shareTone(share)} title={t('dailyStats.colCreditsUsageShareTitle')}>
                            {share.toFixed(1)}%
                          </span>
                        </div>
                        <span className="block h-1 overflow-hidden rounded-[3px] bg-track">
                          <span
                            className="block h-full rounded-[3px] bg-brand"
                            style={{ width: `${Math.min(100, Math.max(0, barWidth))}%` }}
                          />
                        </span>
                        <span className="font-mono text-[10px] tabular-nums text-ink-3">
                          ${row.cost.toFixed(4)}
                          {row.saved > 0 && (
                            <span className="ml-1.5 text-ok">
                              {t('common.savedPrefix', { amount: row.saved.toFixed(4) })}
                            </span>
                          )}
                        </span>
                      </div>
                    </td>
                    <td className={`${CELL} text-right`}>
                      <span className="font-mono text-[12px] tabular-nums text-ink-2">
                        {row.requests.toLocaleString(localeTag())}
                      </span>
                    </td>
                  </tr>
                )
              })
            )}
          </tbody>
        </table>

        <div className={PANEL_FOOT}>
          <span>
            {t('dailyStats.footRange')} <b className="font-semibold text-ink-2">{from}</b> —{' '}
            <b className="font-semibold text-ink-2">{to}</b>
          </span>
          <span className="ml-auto">
            {t('dailyStats.footTotals', {
              credits: total.toFixed(4),
              requests: totalRequests.toLocaleString(localeTag()),
            })}
          </span>
        </div>
      </section>
        </>
      )}

      <CustomRangeDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onApply={(r) => {
          onCustomRangeChange(r)
          onRangeChange('custom')
          setDialogOpen(false)
        }}
        initial={customRange}
      />
    </div>
  )
}

/** 今日明细视图的分页大小（与单日明细页一致） */
const TODAY_PAGE_SIZE = 100

/**
 * 「今日」tab：当天（CST）请求明细记录表。
 *
 * - 日期取 cstDateString(0)（UTC+8 口径，每次渲染重算不 memo，跨 CST 午夜
 *   随 60s 轮询重渲染滚动到新日期）；禁用 dashboard 的 todayLocal（浏览器
 *   本地时区会让 UTC+8 以西用户在 CST 0–8 点看到昨日明细）
 * - 服务端分页下指标卡为「当页」口径（与 DailyDetailPage 同惯例）
 * - 与 DailyDetailPage 的差异：无独立 PageHead/面包屑（tab 内嵌在本页），
 *   复用其明细表体 UsageLogTable
 */
function TodayRecordsView() {
  const { t } = useTranslation()
  const [page, setPage] = useState(1)
  const todayCst = cstDateString(0)
  const { data: recordsData, isLoading, isError } = useDailyUsageRecords(todayCst, page, TODAY_PAGE_SIZE)

  // 跨 CST 午夜日期滚动后重置回第一页，避免停留在已越界的旧页码
  const [lastTodayCst, setLastTodayCst] = useState(todayCst)
  if (lastTodayCst !== todayCst) {
    setLastTodayCst(todayCst)
    setPage(1)
  }

  const records = recordsData?.records ?? []
  const totalRecords = recordsData?.total ?? 0
  const totalPages = Math.max(1, recordsData?.totalPages ?? 1)

  const pageIps = records.map((r) => r.clientIp).filter((ip): ip is string => !!ip)
  const geoMap = useIpGeo(pageIps)

  const pageCost = records.reduce((s, r) => s + r.estimatedCost, 0)
  const pageCredits = records.reduce((s, r) => s + recordCredits(r), 0)
  const pageSaved = records.reduce((s, r) => s + (r.creditsSaved ?? 0), 0)
  const pageInput = records.reduce((s, r) => s + r.inputTokens, 0)
  const pageOutput = records.reduce((s, r) => s + r.outputTokens, 0)
  const pageCacheRead = records.reduce((s, r) => s + (r.cacheReadInputTokens ?? 0), 0)
  const cacheShare = pageInput > 0 ? (pageCacheRead / pageInput) * 100 : 0
  const avgCost = records.length > 0 ? pageCost / records.length : 0

  return (
    <div className="flex min-h-[calc(100vh-56px)] flex-col">
      <MetricsBar>
        <Metric label={t('common.statTotalRequests')}>
          <MetricValue value={totalRecords.toLocaleString(localeTag())} />
          <MetricFoot className="truncate">
            <span>{t('dailyStats.metricPageOf', { shown: records.length, total: totalRecords })}</span>
            <FootSep />
            <span>{t('common.pageInfoSimple', { current: page, total: totalPages })}</span>
          </MetricFoot>
        </Metric>

        <Metric label={t('common.pageCost')}>
          <MetricValue value={formatCost(pageCost)} />
          <MetricFoot className="truncate">
            <span>
              {t('dailyStats.detailAvgCost')} <b className="font-semibold text-ink-2">{formatCost(avgCost)}</b>
            </span>
          </MetricFoot>
        </Metric>

        <Metric label={t('common.pageCredits')}>
          <MetricValue value={pageCredits.toFixed(4)} unit={t('dailyStats.unitCredits')} />
          <MetricFoot className="truncate">
            {pageSaved > 0 ? (
              <span className="text-ok">{t('common.savedPrefix', { amount: pageSaved.toFixed(4) })}</span>
            ) : (
              <span>{t('dailyStats.detailNoSaving')}</span>
            )}
          </MetricFoot>
        </Metric>

        <Metric label={t('dailyStats.metricPageTokensLabel')}>
          <MetricValue value={formatTokenCount(pageInput + pageOutput)} />
          {/* pr 给 .m-aside 的 42px 环形图让位 */}
          <MetricFoot className="truncate pr-[62px]">
            <span>
              {t('dailyStats.metricCacheRead')}{' '}
              <b className="font-semibold text-ok">{formatTokenCount(pageCacheRead)}</b>
            </span>
            <FootSep />
            <span>{cacheShare.toFixed(1)}%</span>
          </MetricFoot>
          <MetricAside>
            <Ring percent={cacheShare} tone="stroke-ok" size={42} />
          </MetricAside>
        </Metric>
      </MetricsBar>

      {/* 请求日志（.panel + .tscroll + .panel-foot），与单日明细页共用同一张表 */}
      <div className="mt-[15px] flex min-h-0 flex-1 flex-col">
        <h3 className={PANEL_TITLE}>{t('common.requestLog')}</h3>
        <section className={`${PANEL} flex min-h-0 flex-1 flex-col`}>
          <div className="min-h-0 flex-1 overflow-y-auto">
            {isError ? (
              <div className="px-4 py-12 text-center text-[12.5px] text-ink-3">
                {t('common.loadFailed')}
              </div>
            ) : (
              <UsageLogTable records={records} geoMap={geoMap} isLoading={isLoading} />
            )}
          </div>

          <div className={PANEL_FOOT}>
            <div className="ml-auto flex flex-none items-center gap-2">
              <span>{t('apiKeys.footRecords', { count: totalRecords })}</span>
              {totalRecords > 0 && (
                <>
                  <Pager page={page} totalPages={totalPages} onPage={setPage} />
                  <span>{t('credentials.footPerPage', { size: TODAY_PAGE_SIZE })}</span>
                </>
              )}
            </div>
          </div>
        </section>
      </div>

      </div>
  )
}

/**
 * 自定义区间日期对话框：两个原生 date input（YYYY-MM-DD 由浏览器保证）+ 应用/取消。
 * 非法（任一为空或起始>结束）时点应用显示红字提示、不关弹窗不发请求；
 * 取消/遮罩/ESC 由 onOpenChange(false) 上抛，调用方 range 不变即回退前一 tab。
 * 重开时用上次已应用区间回填（切走再切回不重弹窗，但重开保留上一次选择）。
 */
function CustomRangeDialog({
  open,
  onOpenChange,
  onApply,
  initial,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  onApply: (range: CustomRange) => void
  initial: CustomRange | null
}) {
  const { t } = useTranslation()
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [touched, setTouched] = useState(false)

  // 每次打开时回填上次已应用区间（关闭不清，避免应用前的输入在轮询重渲染间丢失）
  const [wasOpen, setWasOpen] = useState(false)
  if (open && !wasOpen) {
    setWasOpen(true)
    setFrom(initial?.from ?? cstDateString(0))
    setTo(initial?.to ?? cstDateString(0))
    setTouched(false)
  } else if (!open && wasOpen) {
    setWasOpen(false)
  }

  const invalid = from === '' || to === '' || from > to

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t('dailyStats.customDialogTitle')}</DialogTitle>
        </DialogHeader>

        <div className="grid grid-cols-1 gap-4 py-2 sm:grid-cols-2">
          <label className="flex flex-col gap-1.5">
            <span className="text-[12px] font-medium text-ink-2">{t('dailyStats.customFromLabel')}</span>
            <input
              type="date"
              value={from}
              onChange={(e) => {
                setFrom(e.target.value)
                setTouched(true)
              }}
              className="h-9 rounded-[7px] border border-transparent bg-surface-2 px-3 font-mono text-[12.5px] text-ink shadow-hair outline-none focus-visible:border-brand"
            />
          </label>
          <label className="flex flex-col gap-1.5">
            <span className="text-[12px] font-medium text-ink-2">{t('dailyStats.customToLabel')}</span>
            <input
              type="date"
              value={to}
              onChange={(e) => {
                setTo(e.target.value)
                setTouched(true)
              }}
              className="h-9 rounded-[7px] border border-transparent bg-surface-2 px-3 font-mono text-[12.5px] text-ink shadow-hair outline-none focus-visible:border-brand"
            />
          </label>
        </div>

        {/* 仅在用户改过且非法时提示（初次打开的空输入不红字轰炸） */}
        {touched && invalid && (
          <p className="text-[11.5px] text-danger" role="alert">
            {t('dailyStats.customInvalidRange')}
          </p>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t('common.cancel')}
          </Button>
          <Button
            disabled={invalid}
            onClick={() => {
              if (invalid) return
              onApply({ from, to })
            }}
          >
            {t('dailyStats.customApply')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

/** 区间明细的截断上限（与后端 MAX_RANGE_TOTAL 对齐），用于截断提示判定 */
const RANGE_MAX_TOTAL = 10_000

/** 自定义区间视图的每页大小（与今日视图一致） */
const RANGE_PAGE_SIZE = 100

/** 「自定义」tab 主体：已应用区间的请求明细记录表（结构与今日视图同构） */
function CustomRangeRecordsView({ range }: { range: CustomRange }) {
  const { t } = useTranslation()
  const [page, setPage] = useState(1)

  // 重新应用新区间后重置回第一页，避免残留页码越界（与今日视图跨午夜重置同模式）
  const rangeKey = `${range.from}~${range.to}`
  const [lastRangeKey, setLastRangeKey] = useState(rangeKey)
  if (lastRangeKey !== rangeKey) {
    setLastRangeKey(rangeKey)
    setPage(1)
  }

  const { data: recordsData, isLoading, isError } = useUsageRecordsByRange(
    range.from,
    range.to,
    page,
    RANGE_PAGE_SIZE,
  )

  const records = recordsData?.records ?? []
  const totalRecords = recordsData?.total ?? 0
  const totalPages = Math.max(1, recordsData?.totalPages ?? 1)
  const truncated = totalRecords > RANGE_MAX_TOTAL

  const pageIps = records.map((r) => r.clientIp).filter((ip): ip is string => !!ip)
  const geoMap = useIpGeo(pageIps)

  const pageCost = records.reduce((s, r) => s + r.estimatedCost, 0)
  const pageCredits = records.reduce((s, r) => s + recordCredits(r), 0)
  const pageSaved = records.reduce((s, r) => s + (r.creditsSaved ?? 0), 0)
  const pageInput = records.reduce((s, r) => s + r.inputTokens, 0)
  const pageOutput = records.reduce((s, r) => s + r.outputTokens, 0)
  const pageCacheRead = records.reduce((s, r) => s + (r.cacheReadInputTokens ?? 0), 0)
  const cacheShare = pageInput > 0 ? (pageCacheRead / pageInput) * 100 : 0
  const avgCost = records.length > 0 ? pageCost / records.length : 0

  return (
    <div className="flex min-h-[calc(100vh-56px)] flex-col">
      <MetricsBar>
        <Metric label={t('common.statTotalRequests')}>
          <MetricValue value={totalRecords.toLocaleString(localeTag())} />
          <MetricFoot className="truncate">
            <span>{t('dailyStats.metricPageOf', { shown: records.length, total: totalRecords })}</span>
            <FootSep />
            <span>{t('common.pageInfoSimple', { current: page, total: totalPages })}</span>
          </MetricFoot>
        </Metric>

        <Metric label={t('common.pageCost')}>
          <MetricValue value={formatCost(pageCost)} />
          <MetricFoot className="truncate">
            <span>
              {t('dailyStats.detailAvgCost')} <b className="font-semibold text-ink-2">{formatCost(avgCost)}</b>
            </span>
          </MetricFoot>
        </Metric>

        <Metric label={t('common.pageCredits')}>
          <MetricValue value={pageCredits.toFixed(4)} unit={t('dailyStats.unitCredits')} />
          <MetricFoot className="truncate">
            {pageSaved > 0 ? (
              <span className="text-ok">{t('common.savedPrefix', { amount: pageSaved.toFixed(4) })}</span>
            ) : (
              <span>{t('dailyStats.detailNoSaving')}</span>
            )}
          </MetricFoot>
        </Metric>

        <Metric label={t('dailyStats.metricPageTokensLabel')}>
          <MetricValue value={formatTokenCount(pageInput + pageOutput)} />
          {/* pr 给 .m-aside 的 42px 环形图让位 */}
          <MetricFoot className="truncate pr-[62px]">
            <span>
              {t('dailyStats.metricCacheRead')}{' '}
              <b className="font-semibold text-ok">{formatTokenCount(pageCacheRead)}</b>
            </span>
            <FootSep />
            <span>{cacheShare.toFixed(1)}%</span>
          </MetricFoot>
          <MetricAside>
            <Ring percent={cacheShare} tone="stroke-ok" size={42} />
          </MetricAside>
        </Metric>
      </MetricsBar>

      {/* 请求日志（.panel + .tscroll + .panel-foot），与今日视图共用同一张表 */}
      <div className="mt-[15px] flex min-h-0 flex-1 flex-col">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h3 className={PANEL_TITLE}>{t('common.requestLog')}</h3>
          {/* 区间摘要 + 截断提示（匹配超上限时后端已降序截断到最新 10000 条） */}
          <div className="flex flex-wrap items-baseline gap-2 text-[11px] text-ink-3">
            <span className="font-mono tabular-nums">
              {range.from} — {range.to}
            </span>
            {truncated && <span className="text-warn">{t('dailyStats.customTruncated')}</span>}
          </div>
        </div>
        <section className={`${PANEL} flex min-h-0 flex-1 flex-col`}>
          <div className="min-h-0 flex-1 overflow-y-auto">
            {isError ? (
              <div className="px-4 py-12 text-center text-[12.5px] text-ink-3">
                {t('common.loadFailed')}
              </div>
            ) : (
              <UsageLogTable records={records} geoMap={geoMap} isLoading={isLoading} />
            )}
          </div>

          <div className={PANEL_FOOT}>
            <div className="ml-auto flex flex-none items-center gap-2">
              <span>{t('apiKeys.footRecords', { count: totalRecords })}</span>
              {totalRecords > 0 && (
                <>
                  <Pager page={page} totalPages={totalPages} onPage={setPage} />
                  <span>{t('credentials.footPerPage', { size: RANGE_PAGE_SIZE })}</span>
                </>
              )}
            </div>
          </div>
        </section>
      </div>
    </div>
  )
}
