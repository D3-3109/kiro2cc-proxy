# 任务清单：usage-log-summary-page-split

## 状态：ARCHIVED

## 任务
- [x] 任务 1：daily-stats-page.tsx 新增 `DailySummaryPage` 组件（汇总视图整体搬移：MetricsBar + 折线图 + Toolbar + 日期表 + 内部 7/14/30 Segmented + filter/query 自持），带 onBack/onViewDay props —— 验证：tsc 无错
- [x] 任务 2：`DailyStatsPage` 主组件改造：7/14/30 tab 主体改为 `CustomRangeRecordsView`（区间 `cstDateString(N-1) — cstDateString(0)`）；PageHead 新增 `onOpenSummary` prop 与「每日统计」按钮（刷新按钮左侧）；日期范围按钮 7/14/30 分支改用明细区间 —— 验证：tsc 无错
- [x] 任务 3：dashboard.tsx 状态机扩展 `dailyView: 'summary'` 分支，挂载 `DailySummaryPage`（onBack → 'list'，onViewDay → 日期串）；`DailyDetailPage` 返回目标动态化（汇总页下钻返回汇总页）—— 验证：tsc 无错
- [x] 任务 4：zh.json / en.json 新增 `dailyStats.summaryPageTitle`、`dailyStats.summaryHeadNote`、`dailyStats.navSummaryButton` —— 验证：两文件键对齐
- [x] 任务 5：整体验证 `npx tsc --noEmit` + `pnpm build` 通过；手动路径核验（今日 tab 无回归、7/14/30 明细、每日统计入口→汇总页→下钻→返回、侧边栏导航重置）

## 验收标准
- [ ] 使用日志页「刷新」按钮左侧存在「每日统计」按钮，点击进入 3.4.28 式汇总页面（指标卡 + 折线图 + 日期汇总表 + 下钻）
- [ ] 使用日志页 7/14/30 tab 内容为区间明细记录表，与自定义区间选择最近 7/14/30 天的呈现一致（含分页、截断提示、区间摘要）
- [ ] 日期范围按钮在 7/14/30 tab 下显示「(今天-N+1) — (今天)」（含年份），今日/自定义 tab 显示不变
- [ ] 汇总页内 7/14/30 切换独立工作；汇总页返回回到使用日志页原 tab；汇总页下钻日明细后返回汇总页
- [ ] 侧边栏「使用日志」导航与今日卡入口行为不回归（均回到今日 tab）
- [ ] `npx tsc --noEmit` 与 `pnpm build` 通过
