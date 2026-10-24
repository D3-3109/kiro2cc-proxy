# 变更提案：balance-dialog-range-stats

## 背景
余额弹窗已展示「累计 / 今日」两组使用统计。用户需要更长周期的使用趋势感知：7 天、30 天窗口内的账号用量。现有后端 `UsageTracker` 仅有 `get_summary_for_credential`（全部留存记录）与 `get_today_summary_for_credential`（CST 今日）两个单账号聚合函数，无 N 天区间聚合能力，需扩展。

## 目标范围
**在范围内：**
- 后端：`src/model/usage/daily.rs` 新增通用 N 天区间单账号聚合函数 `get_range_summary_for_credential(credential_id, days)`，复用今日汇总的聚合口径（请求数、输入/输出 tokens、花费、credits）
- 后端：新增 2 个 admin API 端点与 handler：
  - `GET /api/admin/credentials/:id/usage/7days`
  - `GET /api/admin/credentials/:id/usage/30days`
- 前端：`api/credentials.ts` 新增 2 个请求函数与类型（复用 `CredentialDaySummary` 结构改名为区间语义或新增 `CredentialRangeSummary`）
- 前端：`use-credentials.ts` 新增 2 个 hooks（60s 自动刷新，与既有 hooks 一致）
- 前端：`balance-dialog.tsx` 使用统计卡片扩展为四组：累计 / 今日 / 7 天 / 30 天（5 列指标矩阵 × 4 组）
- i18n：zh / en 新增 2 个 key（usageStats7Days / usageStats30Days）

**不在范围内：**
- 不改动既有累计/今日接口的行为与返回结构
- 不新增图表或趋势可视化
- 不改动弹窗其他区块布局
- 不改动 credential-detail-page（其可后续自行接入新 hooks，非本次必须）

## 技术方案
- 聚合函数：`get_range_summary_for_credential` 按 CST 当前时刻往前推 `days` 天（含今日）过滤记录，聚合字段与今日汇总完全一致；返回结构新增 `from_date` / `to_date` 字段便于前端展示语义
- 路由复用：两个端点共用一个 handler，路径参数化为 `usage/:range`（range ∈ {7days, 30days}），或两个独立薄 handler 调同一聚合函数（倾向后者，与既有 today/summary handler 风格一致）
- 前端：使用统计卡片四组行布局保持既有「分组标题 + 5 列矩阵」样式；顺序为 累计 / 今日 / 7 天 / 30 天（今日紧邻累计，便于短期与长期对照）
- 前端：新增组 credits 沿用 `UsageStatsRow` 既有 `toFixed(4)` 口径（复用该组件，口径自动继承，不另起格式）
- 单元测试：新增区间聚合函数的内联测试（边界：恰好 7 天前记录、时区跨日、零记录）

## 预期影响
- 后端 2 个文件（`src/model/usage/daily.rs` + `src/admin/api_keys.rs` + `src/admin/router.rs` 共 3 个）+ 前端 4 个文件（api / hooks / balance-dialog / i18n ×2）
- 区间聚合为内存遍历（与今日汇总同复杂度 O(n)），单请求开销可忽略；弹窗打开多 2 个 GET
- 记录留存受 `MAX_RECORDS_PER_KEY` 裁剪约束：30 天统计的实际覆盖范围以留存记录为准（与「累计」同口径限制，UI 不做额外标注，与现状一致）

## 风险
- 风险：区间起点时区计算错误导致边界日期记录漏算/重算 → 应对：聚合函数内联测试覆盖「恰好 N 天前」边界
- 风险：弹窗四组统计垂直堆叠过高 → 应对：四组行紧凑排列（每组约 48px），弹窗已有 max-h + 滚动兜底
