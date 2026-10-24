# 变更提案：redesign-credential-dialogs

## 背景

admin-ui 的「添加账号」弹窗（`web-ui/admin-ui/src/components/add-credential-dialog.tsx`）与「编辑账号」弹窗（`edit-credential-dialog.tsx`）承载 10~14 个表单字段，目前存在四个可直接观察的问题：

1. **无信息分组**：14 个字段以单一 `space-y-4` 平铺，必填项（Refresh Token）、身份项（邮箱 / 认证方式 / 登录来源）、区域项（Auth / API Region）、运行项（优先级 / Machine ID）、网络项（代理 URL / 用户名 / 密码）视觉权重完全相同，用户无法快速定位。
2. **下拉控件是浏览器原生样式**：两个 `<select>` 虽已带有 `px-2.5`、`disabled:opacity-50` 及与 `Input` 同款的边框/底色，但未做 `appearance-none`，右侧箭头仍由浏览器 UA 绘制——箭头字形、点击区域、与字段盒的垂直对齐均不随主题变化，是截图里最突兀的视觉噪点。
3. **标签层级混乱**：添加弹窗用 `text-[11.5px] font-medium text-ink-2`，编辑弹窗用 `text-sm font-medium`（默认继承 `text-foreground`），两个高度重叠的弹窗在同一后台里字号、颜色、字重都不一致。
4. **说明文字与交互元素混杂**：Region、Profile ARN、优先级、Machine ID、代理五处 hint 都是裸 `<p class="text-[11px] text-ink-3">`；字号色值本身已达标，问题在于外层 `space-y-2` 让 label / 输入框 / hint 三者两两间距相同（均为 8px），hint 与输入框的距离和 label 与输入框的距离一样大，视觉上读不出「这条说明属于上面那个框」，长表单读起来像一堵墙。

仓库已有成熟的设计令牌体系（`index.css` 的 `--surface/-2/-3`、`--ink/-2/-3`、`--hairline/-2`、`--brand*`）与既有的分区卡片模式（`settings-panel.tsx` 的 `SET_CAP` + `SET_CARD`），本变更将这两套已有资产复用到弹窗上，让账号表单与后台其余页面同源。

## 目标范围

**在范围内：**

- `add-credential-dialog.tsx`：字段按「核心凭据 / 身份与区域 / 运行与网络」三组分区（全部展开，不做折叠），每组使用分区标题 + 卡片承载。
- `edit-credential-dialog.tsx`：同步采用与添加弹窗一致的分区结构与标签/控件样式，字段集保持各自原有语义不变（共 13 个字段：昵称、邮箱、优先级、social-only 的登录来源、Region 双列、IdC 分支的 Client ID / Client Secret、Profile ARN、Machine ID、代理 URL + 用户名 + 密码；无 Refresh Token / 认证方式）。
- 新增 admin-ui 内部共享的 `select` 外观组件（或等价共享样式常量），让添加 / 编辑两个弹窗的下拉外观一致，并与 `Input` 的字段盒体系对齐（31px 高、7px 圆角、`surface-2` 底、`hairline-2` 边、`ChevronDown` 图标、聚焦 `border-brand`、禁用 `opacity-50`）。
- 标签统一为单一共享常量；说明文字统一为单一共享常量并归入对应字段的视觉块内。
- **hint 间距收紧为显式规格**：字段块由 `space-y-2`（label / 框 / hint 两两 8px）改为**不用容器 gap** —— label 用 `mb-2`（8px）贴在框上，hint 用 `mt-1.5`（6px）贴在框下，hint 之后不再有容器间距。这样框→hint 为 6px，hint→下一字段 label 由分组内的字段间距（`space-y-4` = 16px）承载，两者拉开 10px，hint 的从属关系一眼可辨。**不使用「容器 `gap-2` + hint `mt-1.5`」的组合**，否则实际间距叠加为 14px，比现状更松，与目标相反。
- 必填标记（`*`）、占位符、hint 文案内容本身保持不变（仅调整呈现方式，不新写业务语义）。必填星号对辅助技术隐藏，由 `sr-only` 文案（`credentials.required`）提供必填语义 —— CR 勾选修复项。
- `web-ui/admin-ui/src/i18n/locales/zh.json` / `en.json`：仅在三组分区标题需要新增键位时才新增；已有键位不改文案。

**不在范围内：**

- 不改动后端任何 Rust 代码、API 契约、字段校验规则、提交 payload 结构。
- 不改动表单字段的显示/隐藏条件（`authMethod === 'social'` 才显示登录来源、`authMethod === 'idc'` 才显示 Client ID/Secret 的逻辑保持原样）。
- 不改动 `Dialog` / `Input` / `Button` 基础原语的既有样式（避免波及其他页面），仅在弹窗层组合使用。
- 不改动 `batch-import-dialog.tsx`、`balance-dialog.tsx`、`models-dialog.tsx` 等其他弹窗。
- 不改动添加账号的校验逻辑、错误 toast 文案、提交顺序。
- 不做移动端布局改造（本次仅桌面端视觉重排，弹窗宽度与 `max-h-[85vh]` 滚动容器保持现有行为）。

## 技术方案

**分区承载**：新增 `web-ui/admin-ui/src/components/credential-form-kit.tsx`，**单点定义**两个弹窗共用的常量与轻量组件。分区标题沿用 `settings-panel.tsx` 的 `.set-cap` 语义（`text-[10.5px] font-semibold uppercase tracking-[.08em] text-ink-3`）。

分区卡片**定稿为 `hairline` 描边形态**：`rounded-lg border-[0.5px] border-hairline bg-surface`（`bg-surface` 与弹窗同色，边界完全由描边提供）。**不照搬 `settings-panel.tsx` 的 `.set-card`**（`border-transparent bg-surface shadow-hair`）——那一套依赖页面底色 `--background` 与卡片 `--surface` 的色差 + 投影来区分层级，而弹窗背景本身就是 `bg-surface`，照搬会导致卡片边界完全消失。因此这里保留 `.set-card` 的圆角与内边距语义，仅把「靠色差 + 投影」换成「靠描边」。**两个弹窗文件都不再各自定义常量**，只 import。

**字段块间距**：`FIELD_STACK` 不引入容器级 `gap`。label 用 `mb-2`、hint 用 `mt-1.5`，字段之间由分组内的 `space-y-4` 提供间距。框→hint 6px 明显小于 hint→下一字段 16px，从属关系清晰。

**下拉控件**：不引入新依赖（仓库约定不新增 crate / npm 包），用原生 `<select>` + 包裹层实现：外壳承载字段盒视觉（与 `Input` 的 `FIELD_BOX` 一致），`<select>` 自身 `appearance-none bg-transparent`，右侧绝对定位 `ChevronDown`（`lucide-react` 已在依赖中且其他组件已在用）。**确定新增 `components/ui/select.tsx`** 作为与 `input.tsx` 平级的原语，与 `Input` 共享同一组字段盒样式心智（实现时从 `input.tsx` 导出 `FIELD_BOX` 复用，避免复制常量）。本次改动范围限定在这两个凭据弹窗，不迁移其他页面的 `<select>`。

**i18n**：三组分区标题需要新增三个键：`credentials.sectionCore`（核心凭据）、`credentials.sectionIdentityRegion`（身份与区域）、`credentials.sectionRuntimeNetwork`（运行与网络），中英各一份。`credentials` 命名空间其余 231 个已有键位文案零改动。

**状态与提交逻辑零改动**：所有 `useState`、`resetForm`、`handleSubmit`、`mutate` 调用保持逐行等价，仅在 JSX 结构上重排。

## 预期影响

- 纯前端展示层变更，`cargo` 构建不受影响（仅 `web-ui/admin-ui` 需重新 `pnpm build`）。
- 两个弹窗的所有字段、校验、提交行为必须零回归：字段 id（`htmlFor` 关联）、`onChange`、`disabled={isPending}` 全部保留。
- 暗色模式与 36 套主题下均需正常：只使用已令牌化的颜色（`surface*` / `ink*` / `hairline*` / `brand*`），不写死色值，因此主题覆盖自动生效。
- 弹窗纵向高度会因分区标题、分组间距、卡片内边距略有增加，仍在 `max-h-[85vh]` + `overflow-y-auto` 容器内滚动，不改变既有滚动行为。

## 风险

| 风险 | 应对 |
|---|---|
| 重排 JSX 时漏掉某个字段的 `id` / `htmlFor` 关联，导致点击标签不聚焦 | 逐字段对照原文件迁移，任务验收里显式检查 label-input 关联 |
| 分区卡片用 `bg-surface` 与弹窗 `bg-surface` 同色导致层级消失 | 已定稿：分组容器用 `border-[0.5px] border-hairline` 描边 + `bg-surface`，层级完全由描边提供；实机截图确认描边在 36 套主题下均可见 |
| `<select>` 加 `appearance-none` 后丢失原生可访问性（键盘/读屏） | 保留原生 `<select>` 元素与 `id`/`aria`，仅覆盖视觉；`appearance-none` 不影响原生键盘交互 |
| 编辑弹窗字段集与添加弹窗不同，强行统一可能引入不存在的字段 | 分区只按语义归类，不新增/删除任何字段；编辑弹窗保留昵称、排除 Refresh Token |
| 新增 `components/ui/select.tsx` 原语可能被误认为通用改造入口，牵引出对其他页面下拉的重构 | 在提案与任务中明确限定仅两个凭据弹窗使用；不在本次改动其他调用点 |
