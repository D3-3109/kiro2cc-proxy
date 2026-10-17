# 变更提案：usage-log-summary-page-split

## 背景

v3.4.29 将使用日志页改造为「今日明细 + 自定义区间明细 + 7/14/30 汇总视图」三态后，7/14/30 三个 tab 仍展示旧版汇总视图（指标卡 + 折线图 + 日期汇总表），与「点击日期区域按钮选择最近 7/14/30 天」得到的区间明细记录表（CustomRangeRecordsView）行为不一致：同为「最近 7 天」语义，两个入口却呈现两种完全不同的内容形态。

用户诉求：
1. 使用日志页「刷新」按钮左侧新增一个「每日统计」按钮，点击进入 3.4.28 版本的每日统计页面（即现汇总视图：指标卡 + 折线图 + Toolbar + 日期汇总表格 + 下钻明细）。
2. 使用日志页 7/14/30 三个 tab 切换后的内容，改为与「点击日期区域按钮 → 选择最近 7/14/30 天」一致的区间明细记录表。

## 目标范围

**在范围内：**
- `daily-stats-page.tsx`：
  - PageHead actions 区新增「每日统计」按钮（刷新按钮左侧），点击触发 `onOpenSummary` 回调
  - 7/14/30 tab（`range === '7' | '14' | '30'`）主体从汇总视图改为 `CustomRangeRecordsView`，区间取「最近 N 天 CST 闭区间」（`cstDateString(N-1) — cstDateString(0)`）
  - 现汇总视图（MetricsBar + DailyCreditsTrendChart + Toolbar + 日期表）整体搬移至新独立组件 `DailySummaryPage`，带返回按钮（`onBack`）与下钻（`onViewDay`），保留 7/14/30 Segmented 切换
  - 汇总视图内部状态（filter/query）随组件搬移，不再污染使用日志页
- `dashboard.tsx`：`dailyView` 状态机扩展 `'summary'` 值；挂载 `DailySummaryPage`；从汇总页返回回到使用日志页（`'list'`）
- `zh.json` / `en.json`：新增 `dailyStats.navSummaryButton`（每日统计 / Daily Stats）等新键
- 日期范围按钮显示口径：7/14/30 tab 下显示 `最近 N 天起 — 今天`（与明细区间一致）

**不在范围内：**
- 后端 Rust 代码与 API（区间端点 GET /api/admin/usage/records 已存在）
- 今日 tab 与自定义区间 tab 的行为（保持现状）
- 汇总页数据链路与计算口径（buildWindow 等原样搬移）
- 侧边栏「使用日志」导航项行为（保持现状，点击仍进入使用日志页今日态）

## 技术方案

1. **汇总视图搬移而非复制**：将 L220-443 的汇总 JSX 与其依赖的派生计算（rows/prevRows/total/peak/zeroStreak/avg 等）整体提取到新组件 `DailySummaryPage`（同文件内，与 `CustomRangeRecordsView` 同级），内部自持 range Segmented（'7'|'14'|'30'，独立于使用日志页的 range）。理由：两个页面的 range 语义已完全分离（使用日志页 range 含 today/custom，汇总页仅固定区间），共享受控 state 只会互相牵制。
2. **7/14/30 明细区间派生**：`rangeDays` tab 下 `rangeObj = { from: cstDateString(days - 1), to: cstDateString(0) }`，复用 `CustomRangeRecordsView`（已是纯 props 组件）。跨午夜滚动由 60s 轮询重渲染驱动 `cstDateString` 重算，与今日视图同口径。
3. **dashboard 导航**：`dailyView: 'list'（使用日志页）| 'summary'（每日统计页）| 日期串（日明细）`。`DailyStatsPage` 新增 `onOpenSummary: () => void` prop；`DailySummaryPage` 的 onBack 回 `'list'`、onViewDay 仍设日期串；日明细页返回目标动态化（从汇总页下钻返回汇总页，从使用日志页今日 tab 不再有下钻入口）。
4. **i18n**：新增键控制在 3 个以内，复用既有 `rangeDays`/`metricTotalLabel` 等汇总键（随 JSX 搬移自动复用）。

## 预期影响

- 使用日志页 7/14/30 tab 内容形态变化：汇总视图 → 明细记录表（用户显式要求）
- 汇总视图从 tab 内嵌变为独立页面，多一次点击到达（由新增「每日统计」按钮补偿）
- 「每日统计」页内 7/14/30 切换与使用日志页 tab 完全独立，互不影响
- 无后端、无数据口径、无其他页面改动

## 风险

- **汇总视图搬移遗漏依赖**：MetricsBar/Chart/Toolbar 及十余个派生变量耦合在同一函数作用域，搬移时需整块搬移作用域计算，遗漏会导致编译错（tsc 兜底可捕获）
- **dailyView 状态机扩展引入回归**：侧边栏导航重置逻辑、返回按钮目标需覆盖 'summary' 分支；验证点为侧边栏点击、汇总页返回、汇总页下钻→返回三条路径
- **跨午夜区间漂移**：7/14/30 明细区间由渲染期重算，queryKey 随之变化，React Query 自动 refetch（key 变化即新查询），无越界页码问题因 key 含日期，但 page state 需随区间重置（CustomRangeRecordsView 已有 rangeKey 重置模式，直接复用）
