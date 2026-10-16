# 变更提案：usage-log-time-range-tabs

## 背景

admin-ui 侧边栏现有「每日统计」页（`DailyStatsPage`）以 7/14/30 天固定区间展示按日汇总，存在三个使用痛点：

1. **命名与内容错位**：页面名为「每日统计」，但用户高频诉求是「看今天/最近用了多少」，日志语义强于统计语义；
2. **看今天要两跳**：先进入区间汇总页，再点击今天的行才能看到今日明细记录，而「今天」是最高频查看目标；
3. **区间不灵活**：只有 7/14/30 天三档，无法查看任意起止日期段（如本月 1 号至今、上周三天）的明细记录。

## 目标范围

**在范围内：**
- 侧边栏入口与页面标题由「每日统计」改为「使用日志」（zh/en 两语言；联动点：侧边栏菜单、页面标题、面包屑、相关 toast 文案，详见任务清单）
- `DailyStatsPage` 的区间 Segmented 新增「今日」tab（默认选中），选中时直接展示当天的请求明细记录表（复用既有共享明细表格组件 `UsageLogTable`：时间/IP/账号/模型/Token 用量/用时/费用/Credits + 汇总卡「当页」口径 + 服务端分页每页 100 条）
- Segmented 新增「自定义」tab，点击弹出起止日期选择对话框（起/止两个原生日期输入 + 应用按钮）；应用后主区展示该起止日期段（含端点）的全部请求明细记录
- 「自定义」tab 交互边界（取消回退 / 空态 / 重开对话框）按 spec 场景执行（见 spec.md「自定义时间段查询」）
- 后端新增跨日区间明细查询能力（CST 时区口径，服务端分页；截断语义：按时间降序排序后保留最新 10000 条，再对截断结果分页——单日明细端点维持既有 2000 条上限不变，仅区间端点为 10000，经用户裁决确认）
- 今日 tab 跨 CST 午夜自动滚动到新日期（随既有 60s 轮询刷新日期计算）；自定义区间为纯组件状态，不持久化到 URL，页面刷新后回到默认「今日」tab（取舍：最小实现，无分享/恢复需求）

**不在范围内：**
- user-ui（用户端无每日统计页，不涉及）
- 按日汇总表格、折线图、指标卡在 7/14/30 天模式下的既有行为（保持不变）
- 旧单日明细页（`DailyDetailPage`）保留为汇总表格点击行进入的路径，不改造其结构；仅 i18n 值变更带来的文案联动（面包屑中的页面名、toast「已刷新每日统计」→「已刷新使用日志」）
- 自定义区间内的按日汇总聚合视图（只做明细记录，不做区间内分日汇总）
- 既有「查看日志」侧边栏项（实时系统日志流 `LogViewerPage`）不做任何改动

## 技术方案

- **前端**（admin-ui）：
  - `DailyStatsPage` 的 `Segmented` options 增加 `今日` 与 `自定义` 两项；`range` 状态扩展为 `'today' | 'custom' | '7' | '14' | '30'`，默认 `'today'`
  - `today` 分支：今日日期用组件内既有 `cstDateString(0)`（UTC+8 口径，`daily-stats-page.tsx:36`；禁用 dashboard 的 `todayLocal`——那是浏览器本地时区，会让 UTC+8 以西用户在 CST 0–8 点看到昨日明细），复用现有 `useDailyUsageRecords(date)` hook + 既有共享组件 `UsageLogTable`（`usage-log-table.tsx` 已 export，无需抽取）渲染当日记录；汇总专属 UI（MetricsBar/折线图/日期搜索 Toolbar）仅在 7/14/30 区间模式渲染，今日/自定义分支不渲染；todayCst 不做 memo 化、`useDailyUsage`（60s 轮询）保持挂载，跨午夜日期随重渲染滚动
  - `custom` 分支：新增起止日期对话框（既有 Radix `Dialog` + 原生 `<input type="date">`×2，不引入日历库）；应用后调用新区间查询 hook 渲染明细
  - i18n：`dashboard.navDailyStats`、`dailyStats.pageTitle` 等 zh/en 文案值改「使用日志 / Usage Log」（键名不变，引用点零改动）
  - 移动端：PageHead actions 区已有 `flex-wrap`，5 项 Segmented 窄屏自动换行；任务清单含 ≤768px 不溢出验收（遵守移动适配对 ≥1024px 桌面零回归的项目约束）
- **后端**（Rust）：
  - `src/model/usage/daily.rs` 提取私有公共管线（过滤 → 降序排序 → truncate(2000) → 分页 → DTO 映射），新增 `get_records_paged_by_date_range(from, to, page, page_size, labels)`（闭区间 `from <= d <= to`，CST 口径），单日版改为 `from == to == date` 的薄包装（行为零变化，测试锁等价性）
  - `src/admin/api_keys.rs` 新增 `GET /api/admin/usage/records?from=YYYY-MM-DD&to=YYYY-MM-DD&page=&page_size=` 端点 + router 注册；参数校验：from ≤ to、格式合法，非法返回 400
  - 单元测试（命名含 `date_range` 关键字便于过滤）：边界日（from==to 与单日版结果等价、from/to 各含记录）、跨 CST 午夜记录归属、空区间、from > to 拒绝、page_size 钳制、超 2000 条截断
- **依赖约束**：不引入新外部 crate；前端不新增依赖（原生 date input）

## 预期影响

- 侧边栏文案与页面标题变化是纯展示层改动，无数据迁移
- 新增端点为纯增量，不影响现有 `/usage/daily` 与 `/usage/daily/{date}/records`
- 默认 tab 从 14 天汇总变为「今日」，改变了该页的初始视图（用户明确要求）
- `DailyStatsPage` 组件职责扩大（汇总 + 明细两种形态），需保持既有 7/14/30 行为零回归

## 风险

- **与既有「查看日志」侧边栏项的命名相似**：侧边栏将同时存在「查看日志」（系统分组，实时服务日志流）与「使用日志」（主要分组，请求用量明细）。二者分组不同、内容性质不同（系统运行日志 vs 请求计费明细），且「使用日志」为用户明确指定的命名，保留；如后续仍混淆可在系统分组项加「实时」前缀（超出本次范围）
- **「今日」明细与「今日汇总」口径不一致风险**：明细按 CST 日期过滤、汇总卡按当页计算（服务端分页下当页≠全天），沿用现有 `DailyDetailPage` 的「当页」标注惯例规避误读
- **自定义区间过大**：按时间降序排序后保留最新 10000 条再分页（单日版维持 2000 不变，区间版经用户裁决提高到 10000，与单 Key 存量上限 MAX_RECORDS_PER_KEY 同量级），UI 在截断时提示「仅展示最近 10000 条」；明细总量受 `MAX_RECORDS_PER_KEY × API Key 数` 上限约束，实际可控
- **日期输入格式**：原生 `type="date"` 浏览器保证 YYYY-MM-DD，后端仍做格式校验兜底
- **默认视图行为变化**：老用户习惯 14 天汇总首屏，改为今日明细后需要通过 tab 一键切回（7/14/30 行为不变）
- **窄屏 Segmented 溢出**：3 项增至 5 项，PageHead actions 已有 flex-wrap 兜底；验收含移动端检查项，且移动适配不得影响 ≥1024px 桌面端
