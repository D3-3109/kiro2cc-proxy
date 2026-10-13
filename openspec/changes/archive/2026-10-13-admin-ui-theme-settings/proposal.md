# 变更提案：admin-ui-theme-settings

## 背景

当前 admin-ui 仅支持 light/dark 双主题切换（`use-theme.ts`，localStorage key `admin-ui-theme`，入口在 sidebar 与 settings-panel）。codex2api 仓库（`/Users/MacBook/My Files/Code/sourcetree/codex2api`）有一套完整的主题设置页面：三档外观模式（light/dark/system）+ 34 个色彩主题（分 recommended/website/light/dark/editor 六组筛选）+ 搜索 + 实时预览。需要将该页面功能完整移植到本仓库 admin-ui，功能与 codex2api 保持一致。

## 目标范围

**在范围内：**
- 扩展 `use-theme.ts`：ThemeMode（light/dark/system，system 监听 `matchMedia` 变化）+ ColorTheme（34 个主题 id），localStorage 分别持久化，默认 dark
- 新增主题 CSS：34 个色彩主题的变量映射（覆盖 admin-ui 双轨令牌：shadcn HSL 三元组 + 设计稿原生令牌 --surface/--brand 等），随 `html.theme-<id>` class 生效；暗色变体随 `.dark.theme-<id>` 生效
- 新增「主题设置」页面（Dashboard 内页面，sidebar 加导航入口）：
  - 页头：标题 + 主题数量徽章 + 重置默认
  - 当前外观区：三档模式选择（light/dark/system 卡片）+ 当前主题信息 + 实时预览（含迷你 UI 模拟）
  - 主题库区：搜索框 + 分组筛选（推荐/网站/浅色/深色/编辑器等）+ 主题卡片网格（每卡含缩略预览，点击应用）
  - 切换沿用现有 View Transition 圆形扩散动画
- settings-panel 现有主题行对接新主题页（跳转入口），移除旧的 light/dark 两档 Seg
- i18n 词条补齐（zh + en）

**不在范围内：**
- 不引入 framer-motion（codex2api 的 cinematic-theme-switcher 依赖它）；切换动画复用 admin-ui 已有 View Transition 实现，不做 cinematic 轮播组件
- 不移植 user-ui（仅 admin-ui）
- 不改动 Rust 后端
- glass 主题的特殊毛玻璃 UI 规则（glass-theme.css 中针对 sidebar/按钮的覆盖）按 admin-ui 结构适配最小可用集，不做逐像素还原

## 技术方案

- 状态：沿用 Dashboard 持有主题状态的模式（settings-panel 先例：主题状态提升避免多实例不同步），将 `use-theme.ts` 扩展为单一 hook 管理 mode + colorTheme，`document.documentElement` 上切 `dark` 与 `theme-<id>` 两个 class
- CSS：新增 `src/themes.css`，34 个主题按 `.theme-<id>` / `.dark.theme-<id>` 定义，覆盖两轨令牌变量；主题定义（id、名称、分组）收在一个 TS 常量文件供页面与 hook 共用
- 页面组件：`src/components/theme-settings-page.tsx`（+ 预览子组件），样式用 Tailwind 原子类 + 少量新增组件类，遵循现有 index.css 白名单机制
- 导航：sidebar navGroups 新增「主题设置」项，Dashboard 增加对应 active page 分支
- 验证：`pnpm build`（tsc + vite）通过；手工核对主题切换与持久化

## 预期影响

- 纯前端 admin-ui 变更，不影响后端与 user-ui
- 现有 light/dark 切换行为保持兼容（旧 localStorage key `admin-ui-theme` 迁移读取）
- 34 主题 CSS 体积新增约 30-40KB（未压缩），构建产物随之增大
- 全部页面在 34 主题下需可读：新 CSS 同时覆盖两轨令牌，避免出现某些主题下文字/边框不可见

## 风险

- 令牌体系差异：codex2api 只覆盖 shadcn 风格变量，admin-ui 组件大量使用原生令牌（--surface/--ink/--brand 等），若映射遗漏会导致某主题下样式错乱 → 每主题同时映射两轨变量，并以现有 dark 主题值为基准校准
- 主题状态从单 hook 提升为共享状态，settings-panel/sidebar 的既有入口需同步改造，遗漏会出双份状态 → 改造后全局 grep `use-theme` 确认单一来源
- 34 主题逐一手工调色工作量大 → 以 codex2api 的变量值直接搬运并做两轨映射，不重新设计配色
