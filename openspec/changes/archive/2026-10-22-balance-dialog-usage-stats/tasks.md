# 任务清单：balance-dialog-usage-stats

## 状态：ARCHIVED

## 任务
- [x] 任务 1：`web-ui/admin-ui/src/i18n/locales/zh.json` 与 `en.json` 在 credentials 命名空间新增使用统计文案 key（usageStatsTitle / usageTotal / usageToday / usageRequests / usageInputTokens / usageOutputTokens / usageCost）；验证：key 齐全且两语言结构一致
- [x] 任务 2：`web-ui/admin-ui/src/components/balance-dialog.tsx` 阶梯四之后接入已有 `useCredentialUsageSummary` / `useCredentialTodaySummary` hooks，新增「使用统计」卡片（CardHeader + 累计/今日两组 4 列指标矩阵，加载失败显示 `--` 占位，千分位格式化）；验证：`pnpm tsc --noEmit` + `pnpm build` 通过
- [x] 任务 3：浏览器手动验收：打开账号余额弹窗，末尾展示使用统计卡片，累计与今日数据与 usage-log 页一致；验证：截图确认 UI 美观简洁、无布局溢出

## 验收标准
- [x] 弹窗最后（运行健康卡片之后）出现「使用统计」卡片，含累计与今日两组指标
- [x] 每组指标含请求数、输入 tokens、输出 tokens、花费（credits）四项
- [x] usage tracker 未启用 / 接口失败时卡片渲染占位符，不影响弹窗其余区块
- [x] UI 与弹窗现有卡片风格一致（bg-surface-2 + CardHeader + divide-x 指标矩阵），美观简洁
- [x] `pnpm tsc --noEmit` + `pnpm build` 通过；不改后端，`cargo check` 不受影响
