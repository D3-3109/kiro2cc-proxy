# 任务清单：summary-page-custom-range

## 状态：ARCHIVED

## 任务
- [x] 任务 1：daily-stats-page.tsx 新增 `buildCustomWindow(data, from, to)`（绝对日期闭区间零填充，跨度 >366 天由调用方拦截），并给 `DailySummaryPage` 扩展 `'custom'` 区间态（sumRange 四态 + customRange state + 日期范围按钮 + 复用 CustomRangeDialog + onApply 内 366 天校验 toast）；custom 态环比 Delta / 上一区间指标条件降级 —— 验证：tsc 无错
- [x] 任务 2：zh.json / en.json 新增 `customSpanTooLong` 提示键 —— 验证：两文件键对齐
- [x] 任务 3：整体验证 `npx tsc --noEmit` + `pnpm build`；手动路径核验（7/14/30 无回归、custom 应用/取消/非法跨度、custom 态指标卡与表格、下钻返回）

## 验收标准
- [ ] 每日统计页 Segmented 右侧存在日期范围按钮（与使用日志页同款样式），点击弹出自定义日期弹窗（默认今天~今天回填）
- [ ] 应用合法区间后页面切换 custom 态：指标卡（除环比 Delta）/折线图/表格/下钻按该区间工作，区间含起止闭区间
- [ ] 应用跨度 >366 天时 toast 提示且不切换区间
- [ ] 取消/遮罩/ESC 关闭弹窗不改变当前区间态
- [ ] 7/14/30 三档现状零回归（滚动窗口、环比、全部指标）
- [ ] `npx tsc --noEmit` 与 `pnpm build` 通过
