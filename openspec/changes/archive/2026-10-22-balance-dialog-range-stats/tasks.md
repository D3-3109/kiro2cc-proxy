# 任务清单：balance-dialog-range-stats

## 状态：DONE

## 任务
- [x] 任务 1：`src/model/usage/daily.rs` 新增 `get_range_summary_for_credential(credential_id, days)` 聚合函数与 `CredentialRangeSummary` 结构（含 from_date/to_date），内联测试覆盖边界（恰好 N 天前、零记录、时区）；验证：`cargo test get_range_summary` 全绿
- [x] 任务 2：`src/admin/api_keys.rs` 新增 handler（7days/30days 两个薄 handler 调同一聚合函数）+ `src/admin/router.rs` 注册 2 条路由；验证：`cargo check` + curl 实测两端口返回与手工聚合一致
- [x] 任务 3：前端 `api/credentials.ts` 新增请求函数与 `CredentialRangeSummary` 类型，`use-credentials.ts` 新增 `useCredential7DaysSummary` / `useCredential30DaysSummary` hooks（60s 刷新）；验证：`pnpm tsc --noEmit` 无错误
- [x] 任务 4：`balance-dialog.tsx` 使用统计卡片扩展为 累计/今日/7 天/30 天 四组矩阵（复用 UsageStatsRow，credits 沿用 toFixed(4) 口径）+ i18n（zh/en）新增 usageStats7Days / usageStats30Days；验证：`pnpm tsc --noEmit` + `pnpm build` 通过
- [x] 任务 5：浏览器手动验收：弹窗四组统计齐全，7/30 天数值与使用日志页同区间手工核对一致；截图确认 UI 无溢出

## 验收标准
- [x] `GET /credentials/:id/usage/7days` 与 `/30days` 返回区间聚合数据（请求数、输入/输出 tokens、花费、credits、from/to 日期）
- [x] 弹窗使用统计卡片按 累计/今日/7 天/30 天 四组展示，样式与既有分组一致
- [x] 区间聚合边界正确（CST 时区、含今日、恰好 N 天前记录归属正确）
- [x] `cargo fmt` + `cargo clippy` clean；新增聚合函数有内联单元测试
- [x] `pnpm tsc --noEmit` + `pnpm build` 通过
