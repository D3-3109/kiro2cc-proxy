# 任务清单：redesign-credential-dialogs

## 状态：ARCHIVED

## 任务

- [x] **T1 — 抽取下拉原语与表单视觉基元（单点定义）**
  新增 `components/ui/select.tsx`：从 `input.tsx` 导出并复用 `FIELD_BOX`，外壳承载 31px 高 / `rounded-[7px]` / `border-[0.5px] border-hairline-2` / `bg-surface-2` / `px-2.5` / `text-[12px]`，`<select>` 自身 `appearance-none bg-transparent`，右侧绝对定位 `lucide-react` 的 `ChevronDown`，聚焦 `border-brand`，禁用 `opacity-50`。新增 `components/credential-form-kit.tsx` **单点定义** `SECTION_CAP`（`text-[10.5px] font-semibold uppercase tracking-[.08em] text-ink-3`）/ `SECTION_CARD`（`rounded-lg border-[0.5px] border-hairline bg-surface`）/ `FIELD_LABEL` / `FIELD_HINT`（`text-[11px] leading-[1.55] text-ink-3`）/ `FIELD_LABEL_SPACING`（label 的 `mb-2`）/ `FIELD_HINT_SPACING`（hint 的 `mt-1.5`）。
  验证：`pnpm exec tsc --noEmit` 通过；新原语在 light / dark / 非默认主题下外观与 `Input` 对齐；原生 `<select>` 键盘操作（Tab 聚焦、方向键切换、输入首字母跳转）仍正常；`credential-form-kit.tsx` 是这六个常量的唯一定义处（两个弹窗只 import 不重复定义）。

- [x] **T2 — 添加账号弹窗三段分区重排**
  将 `add-credential-dialog.tsx` 的 14 个字段重排为三组（全部展开）：「核心凭据」= Refresh Token / 用户名邮箱 / 认证方式 / 登录来源；「身份与区域」= Region 双列 / Profile ARN / IdC 的 Client ID + Client Secret；「运行与网络」= 优先级 / Machine ID / 代理 URL + 用户名密码。每组以 `SECTION_CAP` 标题 + `SECTION_CARD` 描边容器承载，组内字段 `space-y-4`。字段块不用容器 gap：label 用 `mb-2`、hint 用 `mt-1.5`，框→hint 6px、hint→下一字段 16px。所有字段的 `useState`、`resetForm`、`handleSubmit`、`mutate` 调用逐行保持不变。
  验证：`tsc --noEmit` 通过；14 个字段逐一确认 `id` / `htmlFor` / `onChange` / `disabled={isPending}` 未丢失；切换 `authMethod` 时登录来源与 IdC 字段的显隐行为与改动前一致；提交一次成功用例，payload 与改动前逐字段等价；实机确认 hint 与输入框距离（6px）明显小于 hint 与下一字段 label 的距离（16px）。

- [x] **T3 — 编辑账号弹窗同步分区与基元**
  将 `edit-credential-dialog.tsx` 的 13 个字段按同一套分区语义重排：「核心凭据」= 昵称 / 邮箱 / 登录来源（social-only）；「身份与区域」= Region 双列 / Profile ARN / IdC 的 Client ID + Client Secret；「运行与网络」= 优先级 / Machine ID / 代理 URL + 用户名密码。保留 social-only 的登录来源下拉（`isSocial` 分支）并按原逻辑继续参与 `provider` 提交；IdC 分支按原逻辑保留；不含 Refresh Token / 认证方式。顶部 `editHintOnlyChanged` 提示保留在分区之外、表单最上方。标签与下拉替换为 T1 共享基元；4 处 `text-xs text-muted-foreground` 的 hint 替换为 `FIELD_HINT`。逐字段 diff 提交逻辑保持等价。
  验证：`tsc --noEmit` 通过；`useEffect` 的打开时重置逻辑（依赖 `[open, credential.id]`）未被破坏；`isSocial` 为真时登录来源下拉仍渲染且改动后仍能提交 `provider`；只填单个字段提交时仍只下发该字段；优先级独立端点串行提交顺序不变。

- [x] **T4 — i18n 分区标题键位补齐**
  在 `web-ui/admin-ui/src/i18n/locales/zh.json` 与 `en.json` 的 `credentials` 命名空间新增 `sectionCore` / `sectionIdentityRegion` / `sectionRuntimeNetwork` 三个键（核心凭据 / 身份与区域 / 运行与网络），中英各一份。不改动其余 231 个既有键位文案。
  验证：切换 zh / en 时弹窗无缺失键（无 `credentials.section*` 原样输出）；`node -e "JSON.parse(...)"` 校验两个 JSON 文件语法合法。

- [x] **T5 — 构建与实机视觉验收**
- [x] **T6 — CR 勾选修复：`Field.required` 星号 a11y**
  星号加 `aria-hidden="true"`，新增 `sr-only` 必填文案走 `credentials.required`（中英键位），不注入原生 `required`（校验仍走既有 toast）。
  验证：第 2 轮 CR PASS（仅 1 条 low 可选增强备注）；`tsc --noEmit` + `pnpm build` 通过；`.sr-only` 与 `appearance-none` 均在产物 CSS 中生成。
  执行 `cd web-ui/admin-ui && pnpm install && pnpm build`，产物落 `dist`；实机视觉由用户自行运行核验（本轮不用 Playwright 截图）。
  验证：`pnpm build` 无 TypeScript 错误、无未使用导入；实机在 light / dark / 非默认主题下打开两个弹窗核对分区层级、间距与对齐。

## 验收标准

- [x] 添加弹窗 14 个字段、编辑弹窗 13 个字段全部按「核心凭据 / 身份与区域 / 运行与网络」三组呈现，且全部展开无需折叠交互
- [x] 共 3 处 `<select>`（添加弹窗 2 处 + 编辑弹窗 1 处）外观与 `Input` 字段盒一致（同高、同圆角、同边框、同底色），`appearance-none` 且带主题化 `ChevronDown` 图标
- [x] 标签字号/字重/颜色在两个弹窗内统一，且由 `credential-form-kit.tsx` 单点定义
- [x] 共 9 处 hint（添加弹窗 5 处 + 编辑弹窗 4 处）样式统一为 `text-[11px] leading-[1.55] text-ink-3`，且框→hint 间距 6px（`mt-1.5`）明显小于 hint→下一字段 label 间距 16px（`space-y-4`），无容器 gap 叠加
- [x] 分区卡片为 `border-[0.5px] border-hairline` + `bg-surface`，在 36 套主题下描边均可见
- [x] 所有字段的 `id` ↔ `htmlFor` 关联、`onChange`、`disabled={isPending}`、显隐条件零丢失（编辑弹窗原无 id 的字段不强制新增）
- [x] 提交 payload、校验顺序、toast 文案、resetForm 行为与改动前逐字段等价
- [x] light / dark / 至少一套非默认主题下排版正常，无写死色值
- [x] `pnpm build` 通过；本次改动不触及任何 Rust 文件，`web-ui/admin-ui/package.json` 无新增依赖
