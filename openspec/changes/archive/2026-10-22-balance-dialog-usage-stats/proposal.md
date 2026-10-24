# 变更提案：balance-dialog-usage-stats

## 背景
账号余额弹窗（BalanceDialog）目前仅展示订阅额度、身份认证信息与调用健康指标（成功/失败/限流次数），缺少该账号的请求用量维度数据。后端已有现成 API：
- `GET /api/admin/credentials/:id/usage/summary` — 全部留存记录的累计用量汇总
- `GET /api/admin/credentials/:id/usage/today` — CST 今日用量汇总

前端 `api/credentials.ts` 已封装对应请求函数与类型，hooks（`useCredentialUsageSummary` / `useCredentialTodaySummary`）也已存在并被 credential-detail-page 使用，但 BalanceDialog 未接入，用户无法在账号维度快速了解「这个号一共用了多少、今天用了多少」。

## 目标范围
**在范围内：**
- BalanceDialog 弹窗末尾（阶梯四「状态与健康 & 运行配置与统计」卡片之后）新增「使用统计」卡片，含：
  - 累计使用：请求数、输入 tokens、输出 tokens、花费（credits）
  - 今日使用：请求数、输入 tokens、输出 tokens、花费（credits）
- 复用已有 React Query hooks（`useCredentialUsageSummary` / `useCredentialTodaySummary`，已存在于 `use-credentials.ts`，60s 自动刷新）
- i18n 文案补充（zh / en）
- UI 风格与弹窗现有卡片体系一致（bg-surface-2 卡片 + CardHeader + 指标矩阵），美观简洁

**不在范围内：**
- 不改动后端任何 Rust 代码（API 已存在且满足需求）
- 不改动 usage summary/today API 的返回结构
- 不改动已有 hooks 的实现（`credential-detail-page.tsx` 正在使用，保持行为不变）
- 不新增图表或趋势可视化
- 不改动弹窗其他区块布局

## 技术方案
- 数据获取：在 `balance-dialog.tsx` 直接调用已有 `useCredentialUsageSummary(id)` / `useCredentialTodaySummary(id)`，不新增 hooks
- UI：在 `balance-dialog.tsx` 阶梯四之后新增一个 `bg-surface-2` 卡片：
  - 头部复用 `CardHeader`（label 使用「使用统计」）
  - 内容为累计 / 今日两组 4 列指标矩阵（请求数、输入 tokens、输出 tokens、花费），复用阶梯四 `grid grid-cols-N divide-x` 的样式语言，保持简洁
- tokens 数字做千分位格式化；花费保留 2 位小数
- 两个接口任一失败时不阻塞弹窗其余内容，仅统计卡片内显示占位（`--`）
- i18n：`zh.json` / `en.json` 在 credentials 命名空间下新增统计相关 key

## 预期影响
- 仅前端 admin-ui 改动，1 个代码文件（`balance-dialog.tsx`）+ 2 个 i18n 资源文件（`zh.json` / `en.json`），不改动 hooks
- 后端零改动，接口调用为已有轻量聚合查询，无性能风险
- 弹窗打开时多 2 个 GET 请求（hooks 自带 60s 轮询），均为内存聚合查询，开销可忽略
- 弹窗高度增加约 120px，已有 `max-h-[90vh] overflow-y-auto` 兜底

## 风险
- 风险：usage tracker 未启用时接口返回 503 → 应对：hooks 查询失败静默降级为占位符，卡片仍渲染
- 风险：tokens 数值过大导致溢出换行 → 应对：使用 `font-mono` + 千分位 + `tabular-nums`，超出时 truncate
