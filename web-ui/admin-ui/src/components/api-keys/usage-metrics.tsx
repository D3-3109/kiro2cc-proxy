// Copyright (c) 2026 Harllan He. Licensed under MIT.
// API Keys 指标条区块（自 api-keys-panel.tsx 拆出，纯代码搬移）
import { useTranslation } from 'react-i18next'
import type { TFunction } from 'i18next'
import { Delta, FootSep, Metric, MetricAside, MetricFoot, MetricValue, MetricsBar, Ring, Sparkline } from '@/components/metrics'
import { formatTokenCount, localeTag } from '@/lib/locale'
import { quotaTone } from '@/components/api-key-row'
import type { KeyStatusFilter } from '@/components/api-keys/panel-constants'
import type { DailySummary } from '@/types/api'

/** 所有配额型 Key 的合计，credits 与 USD 两种口径并列 */
export interface QuotaTotals {
  usedCredits: number
  usedUsd: number
  remainingCredits: number
  remainingUsd: number
  totalCredits: number
  totalUsd: number
}

type TopQuota = NonNullable<UsageMetricsProps['topQuota']>

/** 悬停详情：占用最高的 Key、credits/USD 两种口径的合计与估算说明（每项一行） */
function quotaTooltip(t: TFunction, topQuota: TopQuota, totals: QuotaTotals): string {
  const topKeyUsage =
    topQuota.unit === 'credits'
      ? t('apiKeys.metricQuotaUsedCredits', { used: topQuota.used.toFixed(1), limit: topQuota.limit })
      : t('apiKeys.metricQuotaUsedUsd', { used: topQuota.used.toFixed(2), limit: topQuota.limit })
  return [
    `Key ${topQuota.name} · ${topKeyUsage}`,
    t('apiKeys.metricQuotaTotalsCredits', {
      used: totals.usedCredits.toFixed(1),
      remaining: totals.remainingCredits.toFixed(1),
      total: totals.totalCredits.toFixed(1),
    }),
    t('apiKeys.metricQuotaTotalsUsd', {
      used: totals.usedUsd.toFixed(2),
      remaining: totals.remainingUsd.toFixed(2),
      total: totals.totalUsd.toFixed(2),
    }),
    t('apiKeys.metricQuotaTotalsHint'),
  ].join('\n')
}

interface UsageMetricsProps {
  statusCounts: Record<KeyStatusFilter, number>
  expiringSoonCount: number
  todayRequests: number | null
  requestsDeltaPercent: number | null
  requestTrend: number[]
  cumulative: { requests: number; tokens: number }
  todayStats: DailySummary | null
  sinceLabel: string | null
  topQuota: { name: string; used: number; limit: number; unit: 'usd' | 'credits'; percent: number } | null
  quotaTotals: QuotaTotals | null
}

export function ApiKeysUsageMetrics({
  statusCounts,
  expiringSoonCount,
  todayRequests,
  requestsDeltaPercent,
  requestTrend,
  cumulative,
  todayStats,
  sinceLabel,
  topQuota,
  quotaTotals,
}: UsageMetricsProps) {
  const { t } = useTranslation()

  return (
    <div className="mt-[15px]">
        <MetricsBar>
          <Metric label={t('apiKeys.metricKeysLabel')}>
            <MetricValue value={String(statusCounts.all)} unit={t('apiKeys.metricKeysUnit')} />
            <MetricFoot>
              <span>
                <b className="font-medium text-ink-2">{statusCounts.active}</b> {t('apiKeys.statusActive')}
              </span>
              {statusCounts.pending > 0 && (
                <>
                  <FootSep />
                  <span>
                    <b className="font-medium text-ink-2">{statusCounts.pending}</b> {t('apiKeys.statusPending')}
                  </span>
                </>
              )}
              <FootSep />
              <span>
                <b className="font-medium text-ink-2">{statusCounts.disabled}</b> {t('apiKeys.statusDisabled')}
              </span>
              {statusCounts.expired > 0 && (
                <>
                  <FootSep />
                  <span>
                    <b className="font-medium text-ink-2">{statusCounts.expired}</b> {t('apiKeys.statusExpired')}
                  </span>
                </>
              )}
              {expiringSoonCount > 0 && (
                <>
                  <FootSep />
                  <span className="font-semibold text-warn">
                    {t('apiKeys.metricExpiringSoon', { count: expiringSoonCount })}
                  </span>
                </>
              )}
            </MetricFoot>
          </Metric>

          <Metric label={t('apiKeys.metricTodayLabel')}>
            <MetricValue
              value={todayRequests === null ? '—' : todayRequests.toLocaleString(localeTag())}
              trailing={requestsDeltaPercent === null ? undefined : <Delta percent={requestsDeltaPercent} />}
            />
            <MetricFoot className="truncate pr-[92px]">
              <span>
                {t('apiKeys.metricTodayCost')}{' '}
                <b className="font-medium text-ink-2">${(todayStats?.totalCost ?? 0).toFixed(2)}</b>
              </span>
              <FootSep />
              <span>
                {t('apiKeys.metricTodayCredits')}{' '}
                <b className="font-medium text-ink-2">{(todayStats?.totalCredits ?? 0).toFixed(1)}</b>
              </span>
            </MetricFoot>
            {requestTrend.length >= 2 && (
              <MetricAside>
                <Sparkline values={requestTrend} />
              </MetricAside>
            )}
          </Metric>

          <Metric label={t('apiKeys.metricTotalLabel')}>
            <MetricValue value={cumulative.requests.toLocaleString(localeTag())} />
            <MetricFoot>
              <span>
                Token <b className="font-medium text-ink-2">{formatTokenCount(cumulative.tokens)}</b>
              </span>
              {sinceLabel && (
                <>
                  <FootSep />
                  <span>{t('apiKeys.metricSince', { date: sinceLabel })}</span>
                </>
              )}
            </MetricFoot>
          </Metric>

          <Metric label={t('apiKeys.metricQuotaLabel')}>
            {/* pr-14 为右侧 Ring 留位，避免长 Key 名与之重叠 */}
            <div className="pr-14">
              <MetricValue
                value={topQuota === null ? '—' : String(Math.round(topQuota.percent))}
                unit={topQuota === null ? undefined : '%'}
                trailing={
                  topQuota === null ? undefined : (
                    <span
                      className="min-w-0 truncate rounded-[6px] bg-surface-3 px-[7px] py-0.5 text-[11px] font-semibold text-ink-3"
                      title={topQuota.name}
                    >
                      {topQuota.name}
                    </span>
                  )
                }
              />
            </div>
            {topQuota === null || quotaTotals === null ? (
              <div className="mt-[3px] text-[11px] text-ink-3">{t('apiKeys.metricQuotaEmpty')}</div>
            ) : (
              <div className="mt-[3px] truncate pr-14 text-[11px] text-ink-3" title={quotaTooltip(t, topQuota, quotaTotals)}>
                {t('apiKeys.metricQuotaTotalsCompact', {
                  used: quotaTotals.usedUsd.toFixed(2),
                  remaining: quotaTotals.remainingUsd.toFixed(2),
                  total: quotaTotals.totalUsd.toFixed(2),
                })}
              </div>
            )}
            <MetricAside>
              <Ring
                percent={topQuota === null ? null : topQuota.percent}
                tone={topQuota === null ? undefined : quotaTone(topQuota.percent).stroke}
                size={42}
              />
            </MetricAside>
          </Metric>
      </MetricsBar>
    </div>
)
}
