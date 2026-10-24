## 上下文

「每日统计」页（`DailyStatsPage`）现有区间 Segmented（7/14/30 天）驱动纯前端窗口计算（`buildWindow`）+ 按日汇总表格；单日明细由 `DailyDetailPage` 承载，依赖后端 `get_records_paged_by_date`（单日过滤、CST、2000 条上限、page_size 钳制 [1,500]）。本次在该页叠加「今日」「自定义」两个明细视图，需要后端新增跨日区间查询。

## 目标 / 非目标

**目标：**
- 命名统一为「使用日志」，默认视图改为今日明细（零跳达）
- 自定义区间明细查询（前后端全链路）

**非目标：**
- 不改 7/14/30 汇总视图任何行为（零回归）
- 不做区间内分日聚合、不做导出、不做 user-ui 适配

## 决策

1. **区间查询做成独立函数而非改造 `get_records_paged_by_date`**
   - 单日函数签名 `(date: &str)` 语义清晰、已有 3 处调用（含 user 端）；为它增加可选区间参数会把「单日」语义稀释成「单日或区间」双态
   - 新增 `get_records_paged_by_date_range(from, to, ...)` 与单日版并排，共享内部「过滤 → 排序 → 截断 → 分页 → DTO 映射」管线（提取私有公共函数 `paginate_records`，截断上限参数化：单日 2000 / 区间 10000），单日版改为 `range == date == from` 的薄包装
   - 代价：函数数量 +1；收益：单日路径行为零变化、区间语义显式

2. **今日 tab 不复用 `DailyDetailPage` 组件，而在 `DailyStatsPage` 内做分支**
   - `DailyDetailPage` 含独立 PageHead/面包屑/返回逻辑，其「从汇总点行进入」的导航语义与本页 tab 内嵌场景不符
   - 在 `DailyStatsPage` 内按 `range` 分支渲染明细区（明细表格体复用共享的 `UsageLogTable`），PageHead 与 Segmented 常驻，切换视图不丢失区间上下文
   - `DailyDetailPage` 保留原样，继续服务「汇总表格点行下钻」路径

3. **日期输入用原生 `<input type="date">`，不引入日历库**
   - 项目无既有日历组件；引库违反「不新增外部依赖」约定
   - 原生 input 在所有目标浏览器（Chrome/Edge/Safari/Firefox 近代版）可用，格式恒为 YYYY-MM-DD，直接对齐后端校验
   - 弹窗用既有 Radix `Dialog` 组件（batch-import-dialog 同款模式）

4. **「今日」的日期判定用组件内 `cstDateString(0)`（UTC+8），禁用 dashboard 的 `todayLocal`**
   - `todayLocal`（`dashboard.tsx` → `panel-constants.ts::formatLocalDate`）是浏览器本地时区，会让 UTC+8 以西用户在 CST 0–8 点看到昨日明细；组件内 `cstDateString`（`daily-stats-page.tsx:36`）才是 CST 口径
   - todayCst 不 memo 化、`useDailyUsage`（60s 轮询）保持挂载，跨午夜日期随重渲染滚动

5. **端点路径用 `/api/admin/usage/records`（无 daily 前缀）**
   - 语义上是「跨日区间明细」，不隶属单日资源 `/usage/daily/{date}/records`
   - GET + query params（from/to/page/page_size），与既有 records 端点参数风格一致

6. **区间截断上限 10000 条（T6 裁决）**
   - 原始需求为「全部记录」；与单日 2000 同构的口径在 T6 核验被判偏离，用户裁决提高到 10000
   - 10000 与 `MAX_RECORDS_PER_KEY` 单 Key 存量上限同量级，内存/传输可控；单日端点维持 2000 不变

## 风险 / 权衡

- **`DailyStatsPage` 组件膨胀**：从纯汇总页变为「汇总 + 两种明细」多形态。权衡：明细分支抽为子组件（`TodayRecordsView` / `CustomRangeRecordsView` 共用 `UsageLogTable`），主组件只做 tab 路由；控制在 ~150 行增量
- **自定义区间跨度过大**（如选一年）：匹配记录先全量过滤进内存再截断 10000（T6 裁决值）。与既有单日实现同构（records 全量扫描是现有模式），明细记录总量受 `MAX_RECORDS_PER_KEY × API Key 数` 上限约束，实际可控；不做额外防护
- **默认视图变更的肌肉记忆冲击**：老用户首屏从 14 天汇总变今日明细。属产品决策（用户明确要求），7/14/30 一键可达

## 迁移方案

无数据迁移。i18n 键名保持（仅改 zh/en 值），避免引用点连锁改动。

## 待决问题（Open Questions）

无（阶段 0 已澄清三个关键决策点：今日展示明细表、并入 Segmented、自定义后展示区间明细；T5 审查补充澄清：命名保留用户指定、取消回退前一 tab、区间不持久化、移动端 flex-wrap 兜底、CST 口径用 cstDateString；T6 裁决：区间上限 10000 条）。

## 审查响应记录（T5 第 1 轮 → 修订）

| Issue | 处置 |
|---|---|
| 1 命名混淆（vs「请求日志」） | 核实：既有项为「查看日志」（系统分组，实时服务日志流 `LogViewerPage`），非请求明细页；分组不同、语义不同，「使用日志」为用户指定命名保留，风险节显式记录 |
| 2 截断+分页语义 | 明确：降序排序→truncate→对截断结果分页（total=截断后数量），前端提示「仅展示最近 N 条」（上限后经 T6 裁决为 10000） |
| 3 自定义交互边界 | spec 增补 3 个场景：未应用前主区不变、取消回退前一 tab、切回不重弹窗 |
| 4 跨午夜/持久化 | 明确：今日随 60s 轮询滚日期；自定义区间纯组件状态不持久化，刷新回默认 tab |
| 5 UsageLogTable 需核实 | 证伪：`usage-log-table.tsx:151` 已 export，`daily-detail-page.tsx:134` 在用，无需抽取 |
| 6 DailyDetailPage 联动清单 | 任务 4 改为逐项清单（侧边栏/标题/两条面包屑/两个 toast/headNote/rangeGroupLabel） |
| 7 移动端 | PageHead actions 已有 flex-wrap；任务 7 加 ≤768px 验收 + ≥1024px 零回归 |
| 8 测试命名/等价性 | 任务 1/2 约定 `date_range`/`usage_records` 命名关键字；任务 1 加 from==to 等价性验证 |

## T6 需求回溯核验记录（第 1 轮 → 修订）

- 结果：11 一致 / 1 偏离（#10「全部记录」vs 2000 截断）/ 0 缺失 / 7 合理超出
- 裁决：偏离项 #10 经用户裁决「提高上限」→ 区间端点截断上限 2000 → 10000（单日端点维持 2000 不变）；proposal/spec/tasks/design 四产物已同步更新，偏离消除
- 7 条超出项（跨午夜滚动、不持久化、校验、联动文案、UI 显隐、非功能约束、CST 口径）随步骤 5 一并向用户列出勾选保留/剔除
