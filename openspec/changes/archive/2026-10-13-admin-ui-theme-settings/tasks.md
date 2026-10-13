# 任务清单：admin-ui-theme-settings

## 状态：ARCHIVED

## 任务
- [x] 扩展 `use-theme.ts`：新增 ThemeMode（light/dark/system + matchMedia 监听）与 ColorTheme，localStorage 双 key 持久化（迁移旧 key `admin-ui-theme`），html 根元素切换 `dark` + `theme-<id>` class，保留 View Transition 动画（验证：pnpm build 通过 + 手工切换持久化）
- [x] 新增 `src/theme-definitions.ts`：34 个主题 id/名称/分组的 TS 常量，与 codex2api useTheme.tsx 一一对应（验证：数量与分组计数核对）
- [x] 新增 `src/themes.css`：34 个主题的变量映射，覆盖双轨令牌（shadcn HSL 三元组 + --surface/--brand 等原生令牌），含 `.dark.theme-<id>` 暗色变体；index.css 引入（验证：pnpm build + 每主题下抽检 sidebar/dashboard 可读性）
- [x] 新增主题设置页面 `src/components/theme-settings-page.tsx`（含预览子组件）：页头（标题 + 主题数徽章 + 重置）、模式三卡、当前主题信息 + 实时预览、搜索 + 分组筛选 + 主题卡片网格（验证：pnpm build + 手工点击应用主题）
- [x] Dashboard + sidebar 接入：navGroups 新增「主题设置」导航项与 active page 分支（验证：pnpm build + 导航可达）
- [x] settings-panel 主题行改造：跳转主题设置页入口替换旧 light/dark Seg（验证：pnpm build + 入口可用）
- [x] i18n 词条补齐：zh.json + en.json 新增主题页全部文案（验证：pnpm build + 语言切换抽查）
- [x] 全局 grep `use-theme`/`admin-ui-theme` 确认主题状态单一来源，无残留旧入口（验证：grep 无双份状态调用）

## 验收标准
- [x] 主题设置页功能与 codex2api 一致：三档模式（含 system 跟随）、37 色彩主题、搜索、分组筛选、点击应用、实时预览、重置默认
- [x] 主题选择跨会话持久化（localStorage），旧 light/dark 用户升级不丢失模式偏好
- [x] 不新增任何 npm 依赖；pnpm build 通过
- [ ] 37 个主题下 admin-ui 主要页面无不可读样式（文字/边框/背景对比正常）
- [x] 全部新文案有 zh/en 双语词条
