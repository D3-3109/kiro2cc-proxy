// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { RefreshCw, BarChart3, DollarSign } from 'lucide-react'
import { getUsageRecords } from '@/api/user'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { useIpGeo } from '@/hooks/use-ip-geo'
import { PageHead } from '@/components/page-head'
import { PANEL, PANEL_FOOT, PANEL_TITLE, Pager } from '@/components/table-kit'
import { UsageLogTable, formatCost, formatTokens, getModelTone, recordCredits } from '@/components/usage-log-table'

interface UsageLogPageProps {
  onBack: () => void
}

export function UsageLogPage({ onBack }: UsageLogPageProps) {
  const [page, setPage] = useState(1)
  const pageSize = 100

  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ['usageRecords', page, pageSize],
    queryFn: () => getUsageRecords(page, pageSize),
  })

  const allRecords = data?.records ?? []
  const pageIps = allRecords.map((r) => r.clientIp).filter((ip): ip is string => !!ip)
  const geoMap = useIpGeo(pageIps)

  const byModel = allRecords.reduce<Record<string, { requests: number; inputTokens: number; outputTokens: number; cost: number }>>((acc, r) => {
    const entry = acc[r.model] ?? { requests: 0, inputTokens: 0, outputTokens: 0, cost: 0 }
    entry.requests += 1
    entry.inputTokens += r.inputTokens
    entry.outputTokens += r.outputTokens
    entry.cost += r.estimatedCost
    acc[r.model] = entry
    return acc
  }, {})

  const pageCost = allRecords.reduce((s, r) => s + r.estimatedCost, 0)
  const pageCredits = allRecords.reduce((s, r) => s + recordCredits(r), 0)
  const pageCreditsSaved = allRecords.reduce((s, r) => s + (r.creditsSaved ?? 0), 0)

  return (
    <div className="min-h-screen bg-background">
      <div className="w-[90%] mx-auto px-4 py-6">
        <PageHead
          crumb={['请求日志']}
          title="请求日志"
          note={data ? `共 ${data.total} 条` : undefined}
          onBack={onBack}
          actions={
            <Button
              variant="ghost"
              size="icon"
              onClick={() => refetch()}
              disabled={isFetching}
              aria-label="刷新"
            >
              <RefreshCw className={`h-4 w-4 ${isFetching ? 'animate-spin' : ''}`} />
            </Button>
          }
        />

        <div className="space-y-4">
          {isLoading ? (
            <div className="flex justify-center py-20">
              <RefreshCw className="h-8 w-8 animate-spin text-ink-3" />
            </div>
          ) : !data || data.total === 0 ? (
            <Card>
              <CardContent className="py-12 text-center text-ink-3">
                暂无请求日志
              </CardContent>
            </Card>
          ) : (
            <>
              {/* 汇总卡片 */}
              <div className="grid gap-4 grid-cols-2 md:grid-cols-4">
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-ink-3 flex items-center gap-1">
                      <BarChart3 className="h-3.5 w-3.5" />
                      总请求数
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">{data.total}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-ink-3">本页 Tokens</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-sm font-bold">
                      入 {formatTokens(allRecords.reduce((s, r) => s + r.inputTokens, 0))} /
                      出 {formatTokens(allRecords.reduce((s, r) => s + r.outputTokens, 0))}
                    </div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-ink-3 flex items-center gap-1">
                      <DollarSign className="h-3.5 w-3.5" />
                      本页费用
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold text-warn">
                      {formatCost(pageCost)}
                    </div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-ink-3">本页 Credits</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold text-ok">
                      {pageCredits.toFixed(4)}
                    </div>
                    {pageCreditsSaved > 0 && (
                      <div className="text-xs text-ok mt-0.5">
                        省 {pageCreditsSaved.toFixed(4)}
                      </div>
                    )}
                  </CardContent>
                </Card>
              </div>

              {/* 按模型分组 */}
              {Object.keys(byModel).length > 0 && (
                <div>
                  <h3 className={PANEL_TITLE}>按模型分组（当前页）</h3>
                  <div className="grid gap-2 grid-cols-1 sm:grid-cols-2 md:grid-cols-3">
                    {Object.entries(byModel).map(([model, m]) => (
                      <Card key={model}>
                        <CardContent className="py-3 px-4">
                          <div className={`text-sm font-medium truncate ${getModelTone(model)}`}>{model}</div>
                          <div className="flex flex-wrap gap-x-3 gap-y-0.5 mt-1 text-xs text-ink-3">
                            <span>{m.requests} 次</span>
                            <span>入 {formatTokens(m.inputTokens)}</span>
                            <span>出 {formatTokens(m.outputTokens)}</span>
                            <span className="font-medium text-warn">{formatCost(m.cost)}</span>
                          </div>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </div>
              )}

              {/* 日志表格（.panel + table-kit） */}
              <div>
                <h3 className={PANEL_TITLE}>请求明细</h3>
                <section className={PANEL}>
                  <div className="max-h-[70vh] overflow-auto">
                    <UsageLogTable records={allRecords} geoMap={geoMap} />
                  </div>

                  {/* 面板脚（.panel-foot）：分页 */}
                  {data.totalPages > 1 && (
                    <div className={PANEL_FOOT}>
                      <div className="ml-auto flex flex-none items-center gap-2">
                        <span>第 {data.page} / {data.totalPages} 页</span>
                        <Pager
                          page={data.page}
                          totalPages={data.totalPages}
                          onPage={(p) => setPage(p)}
                        />
                      </div>
                    </div>
                  )}
                </section>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
