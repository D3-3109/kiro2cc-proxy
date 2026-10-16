# 任务清单：usage-log-time-range-tabs

## 状态：DONE

## 任务

- [x] 任务 1：后端区间明细查询函数 —— `src/model/usage/daily.rs` 提取私有公共管线（过滤→降序排序→截断→分页→DTO 映射，截断上限参数化：单日 2000 / 区间 10000），新增 `get_records_paged_by_date_range(from, to, ...)`（CST 闭区间、上限 10000），单日版改薄包装（维持 2000 不变）；单测（命名含 `date_range`）：from==to 与单日版结果等价、边界日各含记录、跨 CST 午夜归属、空区间、page_size 钳制、超 10000 条截断后分页；`cargo test date_range` 全绿
- [x] 任务 2：后端区间端点与路由 —— `src/admin/api_keys.rs` 新增 `GET /api/admin/usage/records?from&to&page&page_size` handler（from>to 或非 YYYY-MM-DD 返回 400），`router.rs` 注册；单测（命名含 `usage_records`）验证参数校验与响应结构（含 credential_label/user_agent 透传）；`cargo test usage_records` 通过
- [x] 任务 3：前端 API hook 与类型 —— `web-ui/admin-ui` 新增 `getUsageRecordsByRange(from, to, page, pageSize)` API 调用与 `useUsageRecordsByRange` hook（react-query，queryKey 含 from/to/page）；`npx tsc --noEmit` 通过
- [x] 任务 4：使用日志命名统一 —— zh/en i18n 值改「使用日志 / Usage Log」（键名不变），联动点清单逐项核对：`dashboard.navDailyStats`（侧边栏）、`dailyStats.pageTitle`（页面标题 + 两条面包屑：汇总页/单日明细页）、`dailyStats.toastRefreshed`/`toastDetailRefreshed`（toast）、`dailyStats.headNote`（页顶说明，改文案适配今日明细默认视图，如「按时间维度查看用量与请求明细」）、`dailyStats.rangeGroupLabel`（「统计区间」→「时间范围」）；`DailyDetailPage` 组件结构不动；`npx tsc --noEmit` 通过
- [x] 任务 5：「今日」tab —— `DailyStatsPage` range 状态扩展 `'today'|'custom'|'7'|'14'|'30'`（默认 today），Segmented 加「今日」项；today 分支用既有 `cstDateString(0)`（UTC+8，禁用本地时区的 todayLocal）取今日日期，复用 `useDailyUsageRecords(todayCst)` + 既有共享组件 `UsageLogTable`（已存在于 `usage-log-table.tsx`）渲染当日明细（汇总卡「当页」口径、每页 100 条、空态）；汇总专属 UI（MetricsBar/折线图/日期搜索 Toolbar）仅区间模式渲染；todayCst 不 memo 化、`useDailyUsage` 60s 轮询保持挂载以跨午夜滚动日期；`npx tsc --noEmit` 通过
- [x] 任务 6：「自定义」tab 与日期对话框 —— Segmented 加「自定义」项；非自定义态点击弹 Radix Dialog（两个原生 `<input type="date">` + 应用/取消）；起始>结束或任一为空时前端拦截不关弹窗；取消/遮罩/ESC 关闭回退前一 tab 不发请求；应用后用任务 3 hook 渲染区间明细（区间摘要 + 分页 + 截断提示「仅展示最近 10000 条」），切走再切回不重弹窗；区间明细模式下同样不渲染汇总 MetricsBar/折线图/日期搜索 Toolbar；`npx tsc --noEmit` 通过
- [x] 任务 7：回归验证 —— `cargo fmt --check` + `cargo clippy` clean、`cargo test` 全绿；`cd web-ui/admin-ui && npx tsc --noEmit && pnpm build` 成功；核对 7/14/30 汇总视图与单日明细页行为不变；≤768px 视口下 Segmented 5 项与对话框不横向溢出（PageHead actions flex-wrap 生效）且 ≥1024px 桌面布局零回归

## 验收标准

- [x] 侧边栏与页面标题显示「使用日志」（英文 Usage Log），面包屑与 toast 联动更新
- [x] 进入页面默认「今日」tab，展示当天（CST）请求明细记录表，分页每页 100 条，空态正常
- [x] 「自定义」tab 弹出起止日期对话框，选择 2026-10-01~2026-10-07 应用后展示含两端日期的明细记录，tab 保持选中并显示区间摘要；切走再切回不重弹窗，刷新页面回默认「今日」
- [x] 对话框取消/关闭时回退前一 tab 且不发请求；起始>结束或日期为空时前端提示；后端对非法参数返回 400
- [x] 区间匹配超 10000 条时按降序保留最新 10000 条（单日明细端点维持 2000 不变），total/totalPages 基于截断后数量，前端展示截断提示
- [x] 7/14/30 天汇总视图（指标卡/折线图/表格/下钻明细页）行为与改动前一致；from==to 时区间接口与既有单日接口返回一致
- [x] `cargo fmt`/`cargo clippy` clean、`cargo test` 全绿、admin-ui 构建成功；≤768px 无横向溢出、≥1024px 桌面零回归
