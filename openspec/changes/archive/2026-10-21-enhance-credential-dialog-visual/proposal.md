# 变更提案：enhance-credential-dialog-visual

## 背景

上一变更 `redesign-credential-dialogs`（已归档，commit 2fc7e861）完成了凭据弹窗的三组分区重排与基元统一，但用户实机查看后反馈「UI 没啥变化」——该轮设计刻意保守：分区卡片用 `bg-surface` 与弹窗同色 + 0.5px hairline 描边，默认浅色主题下 `#E6E8EA` 描边几乎不可见，扫一眼与旧表单无异。

用户已勾选四项视觉加强（A/B/C/D 全选），目标是让重设计「一眼可见」，同时仍严格沿用仓库令牌体系。

## 目标范围

**在范围内：**

- **A 卡片底色 + 图标**：`credential-form-kit.tsx` 的 `SECTION_CARD` 从 `bg-surface` 改为 `bg-surface-2`，与弹窗 `bg-surface` 拉开色差层次；分区标题左侧加 12px 语义图标（核心凭据=KeyRound、身份与区域=Globe、运行与网络=Settings2，均来自已有依赖 lucide-react）。`Section` 组件新增可选 `icon` prop。
- **B 认证方式分段器**：`add-credential-dialog.tsx` 的「认证方式」下拉改为双档分段器（Social / IdC/Builder-ID/IAM 两个可点击卡片块），选中态 `border-brand + bg-brand-soft + 文字 brand`，未选中 `border-hairline-2 + bg-surface`。风格对齐 `settings-panel.tsx` 的 `.seg` 分段器但为块状双列。编辑弹窗无认证方式字段，不涉及。
- **C 头部图标 + 副标题**：两个弹窗标题左侧加品牌色圆角方块图标（添加=UserRoundPlus、编辑=Pencil，`bg-brand-soft text-brand` 底）；标题下加一行 DialogDescription 副标题（i18n 新键：添加=「填入 Kiro 账号的 Refresh Token 即可接入」、编辑=「只修改需要变更的字段」，编辑的副标题复用/替代现有 editHintOnlyChanged 提示行，避免重复）。
- **D 输入框前缀图标**：Refresh Token 输入框左侧加 KeyRound 前缀图标（`size-[13px] text-ink-3`）。仅此一处（最高频、最易定位错位的字段）；`Input` 原语不动，用现有 `unit` 同款外壳思路在弹窗内局部包一层前缀外壳，或为 `Input` 新增可选 `prefix` prop（实现期按最小改动定夺：倾向给 `Input` 加 `prefix`，与既有 `unit` 对称，`input.tsx` 已有两个分支的先例）。
- i18n 中英各新增：`addDialogSubtitle`、`editDialogSubtitle`、分段器两档文案复用现有 `authMethod_social`（「社交登录」）与 `authMethod_idc`（"BuilderId"）键，不新增。
- 同步微调：分区标题保持 `.set-cap` 语义不变；hint/标签体系不动。

**不在范围内：**

- 不改任何字段集、显隐条件、提交逻辑、校验、toast。
- 不改 `Select` 原语、`Dialog` 原语、其他弹窗、其他页面。
- 不引入新 npm 依赖（图标全部来自已装 lucide-react）。
- 不做暗色以外的主题特判（全部走令牌，36 套主题自动适配）。

## 技术方案

- `Section` 加 `icon?: LucideIcon`，标题行渲染 `<Icon className="size-3" />`，与 `settings-panel.tsx` 的 `Section` 模式同构（该文件已有完全相同的实现先例）。
- 分段器：两个 `<button type="button">`，`aria-pressed` 标记选中态，点击切换 `authMethod`；键盘可达（原生 button）；不使用 radio 语义以避免改 DOM 结构过大。
- `Input` 新增 `prefix?: ReactNode`：与 `unit` 分支合并为同一外壳分支（外壳 = FIELD_BOX + focus-within + has-[:disabled]，前缀图标绝对定位或 flex 首位，`pl-8` 留槽位）。该改动对 `Input` 既有调用零影响（不传 prefix 走原路径）。
- 副标题用既有 `DialogDescription` 组件（`dialog.tsx` 已导出，样式 `text-[11.5px] text-ink-3` 已就绪）。
- 头部图标：`DialogTitle` 行改为 flex 布局：`size-[34px] rounded-[9px] bg-brand-soft text-brand` 方块 + 标题。仅两个弹窗内联实现，不改 `DialogTitle` 原语。

## 预期影响

- 纯展示层。`Input` 加 `prefix` prop 属向后兼容扩展，不传即零变化。
- 浅色主题下分区层次从「几乎不可见的描边」变为「可见的浅灰底色块」，视觉变化显著。

## 风险

| 风险 | 应对 |
|---|---|
| `bg-surface-2` 底色卡片上的 `surface-2` 输入框同色失去边界 | 输入框自身有 `hairline-2` 描边，仍可见；实机验收确认 |
| 分段器替代下拉后，登录来源显隐切换（social↔idc）时字段跳动 | 切换逻辑不变，仅控件形态变化；表单高度变化与原下拉等价 |
| `Input` 动 `input.tsx` 牵连全局 | 仅新增可选分支，不触碰既有两个分支的类名；`tsc` + 全局 build 验证 |
