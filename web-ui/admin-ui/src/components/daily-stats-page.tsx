// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { BarChart3, CalendarDays, RotateCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { CustomRangeDialog, RANGE_DAYS, cstDateString, type CustomRange } from '@/components/daily-shared'
import { FootSep, Metric, MetricAside, MetricFoot, MetricValue, MetricsBar, Ring } from '@/components/metrics'
import { ModelAggPanel } from '@/components/model-agg-panel'
import { PageHead } from '@/components/page-head'
import { PANEL, PANEL_FOOT, PANEL_TITLE, Pager } from '@/components/table-kit'
import { Segmented } from '@/components/toolbar'
import { UsageLogTable, formatCost, recordCredits } from '@/components/usage-log-table'
import { useDailyUsage, useDailyUsageRecords, useUsageRecordsByRange } from '@/hooks/use-credentials'
import { useIpGeo } from '@/hooks/use-ip-geo'
import { formatTokenCount, localeTag } from '@/lib/locale'
import { extractErrorMessage } from '@/lib/utils'

interface DailyStatsPageProps {
  onBack: () => void
  onOpenSummary: () => void
  showBack?: boolean
  range: RangeKey
  onRangeChange: (r: RangeKey) => void
  customRange: CustomRange | null
  onCustomRangeChange: (r: CustomRange | null) => void
}

export type RangeKey = 'today' | '7' | '14' | '30' | 'custom'

/** 刷新按钮管辖的查询键前缀（失效与失败判定共用一份定义） */
const REFRESH_QUERY_ROOTS = ['dailyUsage', 'dailyUsageRecords', 'usageRecordsByRange'] as const

export function DailyStatsPage({
  onBack,
  onOpenSummary,
  showBack = true,
  range,
  onRangeChange,
  customRange,
  onCustomRangeChange,
}: DailyStatsPageProps) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  // 保持无条件挂载：60s 轮询触发重渲染驱动今日视图跨午夜滚动日期（不依赖区间模式被选中）
  const { isLoading } = useDailyUsage()
  const [dialogOpen, setDialogOpen] = useState(false)

  const isFixedRange = range === '7' || range === '14' || range === '30'
  const todayCst = cstDateString(0)
  // 7/14/30 tab 的明细区间：最近 N 天 CST 闭区间（含今天），与自定义区间选最近 N 天同口径
  const fixedRangeObj = isFixedRange
    ? { from: cstDateString(RANGE_DAYS[range] - 1), to: todayCst }
    : null

  // 使全部用量查询失效并等待重拉：当前 tab 的明细（dailyUsageRecords /
  // usageRecordsByRange）与汇总数据（dailyUsage）一并刷新，避免刷新按钮与可见内容脱钩。
  // invalidateQueries 的 promise 不反映查询失败（错误入 query state、promise 不 reject），
  // 须在失效完成后读缓存状态判定。不传 refetchType 时只重拉活跃查询：非活跃缓存页
  // 已被标记 stale，切回该 tab 时自动重拉，无需此刻连带给不可见分页加压。
  const handleRefresh = async () => {
    try {
      await Promise.all(
        REFRESH_QUERY_ROOTS.map((root) => queryClient.invalidateQueries({ queryKey: [root] })),
      )
    } catch (err) {
      // throwOnError 场景下 invalidate 会 reject，此时缓存状态判定不可达，直接报错兜底
      toast.error(extractErrorMessage(err))
      return
    }
    // 失败判定只看有 observer 的活跃查询，避免用户看不见的分页失败掩盖可见数据刷新成功
    const failed = queryClient
      .getQueryCache()
      .findAll({
        predicate: (q) =>
          q.observers.length > 0 && REFRESH_QUERY_ROOTS.some((root) => root === q.queryKey[0]),
      })
      .find((q) => q.state.status === 'error')
    if (failed) {
      toast.error(extractErrorMessage(failed.state.error))
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
                ? `${customRange.from} — ${customRange.to}`
                : fixedRangeObj
                  ? `${fixedRangeObj.from} — ${fixedRangeObj.to}`
                  : `${todayCst} — ${todayCst}`}
            </Button>
            {/* 每日统计入口：进入独立汇总页（指标卡 + 趋势图 + 日期汇总表） */}
            <Button variant="outline" onClick={onOpenSummary}>
              <BarChart3 aria-hidden="true" />
              {t('dailyStats.navSummaryButton')}
            </Button>
            <Button variant="outline" onClick={handleRefresh} disabled={isLoading}>
              <RotateCw className={isLoading ? 'animate-spin' : ''} aria-hidden="true" />
              {t('common.refresh')}
            </Button>
          </>
        }
      />

      {/* 三态均为明细记录表：今日 / 自定义区间 / 最近 N 天固定区间（与自定义区间呈现一致） */}
      {range === 'today' ? (
        <TodayRecordsView />
      ) : range === 'custom' && customRange ? (
        <CustomRangeRecordsView range={customRange} />
      ) : fixedRangeObj ? (
        <CustomRangeRecordsView range={fixedRangeObj} />
      ) : (
        <TodayRecordsView />
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

      {/* 按模型分组（当前页）：与详情页同款可折叠面板 */}
      <ModelAggPanel records={records} />

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

      {/* 按模型分组（当前页）：与详情页同款可折叠面板 */}
      <ModelAggPanel records={records} />

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
