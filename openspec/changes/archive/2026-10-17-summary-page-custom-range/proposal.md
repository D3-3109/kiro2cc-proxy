# 变更提案：summary-page-custom-range

## 背景

usage-log-summary-page-split 将汇总视图独立为「每日统计」页（DailySummaryPage）后，该页仅支持 7/14/30 三个固定区间切换。用户在使用日志页可自定义任意日期区间查看明细，每日统计页却没有同能力——查看任意历史区间（如跨月、某周）的汇总趋势必须改用固定档近似，能力不对等。

用户诉求：在「每日统计」页面也新增用户能自定义选择日期区间的按钮和功能（与使用日志页同款交互）。

## 目标范围

**在范围内：**
- `daily-stats-page.tsx`：
  - 新增绝对区间窗口函数 `buildCustomWindow(data, from, to)`：按 CST 日期闭区间零填充（升序），与 `buildWindow` 同口径但接受绝对日期字符串；跨度上限约束（与后端明细区间 10000 条无对应限制，但前端图表与表格需要上限——取 366 天，超出弹窗提示不支持）
  - `DailySummaryPage`：区间状态从 `sumRange: '7'|'14'|'30'` 扩展为 `sumRange: '7'|'14'|'30'|'custom'`，custom 态由 `customRange: CustomRange | null` 携带；Segmented 右侧复用使用日志页同款日期范围按钮（CalendarDays 图标 + 区间文本），点击打开复用的 `CustomRangeDialog`；应用成功切 custom 态，取消保持原态
  - custom 态派生计算：rows 用 buildCustomWindow；prevRows（环比上一区间）无定义（区间长度任意），不渲染环比 Delta 与「N 天」foot，峰值占比/有消耗天数/日均等指标照常（分母为区间实际天数）
  - i18n 复用既有 custom* 键，新增 1-2 键（如 customRangeTooLong 上限提示）
- `zh.json` / `en.json`：新增上限提示键

**不在范围内：**
- 后端 Rust（/usage/daily 已返回全量汇总，前端过滤足够）
- 使用日志页（DailyStatsPage）行为
- CustomRangeDialog 组件本体
- 汇总图表/表格组件

## 技术方案

1. **前端区间过滤而非新增后端端点**：`/usage/daily` 数据量 = 有记录的日期数（≤ 记录保留期），全量拉取 + 前端 buildCustomWindow 过滤零成本；后端汇总聚合逻辑（get_daily_summaries）不感知区间。
2. **跨度上限 366 天**：与明细区间不同，汇总无 10000 条截断，但折线图（DailyCreditsTrendChart）与表格行数在超长区间下不可用；366 天覆盖「整年对比」场景。CustomRangeDialog 复用时由调用方校验跨度（应用回调中 from/to 差 > 366 时弹 toast 提示并保持弹窗打开——需给 Dialog 增加可选校验回调或在使用处包一层）。
3. **环比指标降级**：custom 态 prevRows 语义为「等长前一区间」，对任意区间仍可定义（to=N 天前 to+1 开始的等长区间）但用户预期模糊，v1 不渲染（MetricsBar 的 Delta 与 foot 文案条件跳过），避免误导。
4. **状态自持**：sumRange/customRange 均为 DailySummaryPage 内部 state（与使用日志页 range 隔离），下钻/离开重置（known issue 已记录该行为，保持一致）。

## 预期影响

- 每日统计页新增第四种区间态（custom），7/14/30 现状不变
- custom 态下指标卡少两项派生展示（环比 Delta、区间天数 foot 的「N 天」标签改为实际天数），其余功能全量可用
- 无后端、无使用日志页改动

## 风险

- **buildCustomWindow 与 buildWindow 口径漂移**：两函数同源不同参，新增字段（DayRow 扩展）需双改；以共享 DayRow 类型 + 相同 map 取值逻辑收敛
- **跨午夜**：custom 区间为用户固定日期，不随午夜滚动（与使用日志页 custom 一致）；7/14/30 态保持滚动
- **弹窗复用校验**：CustomRangeDialog 的 onApply 无校验钩子，366 天上限若在应用后提示会导致区间已切换的中间态；方案是在 onApply 回调内校验，非法时仅 toast 不切态（弹窗已关闭属可接受，用户重开重选）
