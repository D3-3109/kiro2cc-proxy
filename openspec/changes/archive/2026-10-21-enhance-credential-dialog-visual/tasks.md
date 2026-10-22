# 任务清单：enhance-credential-dialog-visual

## 状态：IN_PROGRESS

## 任务

- [x] **T1 — 基元扩展：Section 图标、Input prefix、卡片底色**
  `credential-form-kit.tsx`：`SECTION_CARD` 改 `bg-surface-2`；`Section` 加 `icon?: LucideIcon`（渲染 `size-3` 图标于标题前，对齐 settings-panel 同名模式）。`ui/input.tsx`：新增可选 `prefix` prop，与 `unit` 共用外壳分支（外壳 FIELD_BOX + 前缀图标 + `pl-8` 槽位），不传 prefix 行为与现状逐字节一致。
  验证：`tsc --noEmit` 通过；不传 prefix 的既有调用（登录页、设置页、docs 页等）产物 CSS/类名无变化；传 prefix 时图标与占位文字不重叠。

- [x] **T2 — 添加弹窗：分段器 + 头部 + 前缀图标落地**
  认证方式改双档块状分段器（Social / IdC，选中 `border-brand bg-brand-soft text-brand`，`aria-pressed`，原生 button）；弹窗头部加 `UserRoundPlus` 品牌色方块图标 + `DialogDescription` 副标题；Refresh Token 加 `KeyRound` 前缀。三组分区分别传 `KeyRound` / `Globe` / `Settings2` 图标。提交逻辑零改动。
  验证：`tsc --noEmit` 通过；分段器切换 social↔idc 时登录来源/IdC 字段显隐与原下拉一致；`authMethod` 提交值仍为 'social'|'idc'；键盘 Tab 可达分段器两个按钮。

- [x] **T3 — 编辑弹窗：头部 + 图标落地**
  头部加 `Pencil` 品牌色方块图标 + 副标题（替代现有 `editHintOnlyChanged` 裸提示行，文案复用该键或新增 `editDialogSubtitle`，二选一以不重复为原则）；三组分区同款图标。字段与提交逻辑零改动。
  验证：`tsc --noEmit` 通过；`editHintOnlyChanged` 语义不丢失（副标题承载）；IdC / social 分支不受影响。

- [x] **T4 — i18n 键位**
  中英各新增 `credentials.addDialogSubtitle` / `editDialogSubtitle`（若 T3 复用既有键则只加前者）。分段器文案复用 `authMethod_social` / `authMethod_idc`。
  验证：zh/en 切换无缺失键；JSON 语法合法。

- [x] **T5 — 构建验收**
  `pnpm build`；浅色主题下分区底色层次、选中态分段器、头部图标肉眼显著可辨；dark 与一套非默认主题复核。
  验证：构建零错误；`bg-surface-2` 卡片上输入框边界仍可见。

## 验收标准

- [ ] 分区卡片 `bg-surface-2` 与弹窗底色形成可见层次，浅色主题下一眼可辨三组
- [ ] 三个分区标题各带语义图标，与 settings-panel 的 Section 模式同构
- [ ] 认证方式为双档分段器，选中态品牌色，键盘可达，提交值与原下拉一致
- [ ] 两弹窗头部有品牌色图标 + 副标题；编辑弹窗原「只改需要修改的字段」提示不重复出现
- [ ] Refresh Token 输入框带 KeyRound 前缀图标；其余输入框不受影响
- [ ] 字段集、显隐、提交、校验、toast 零回归；`Input` 不传 prefix 时全局零变化
- [ ] 无新增依赖；`pnpm build` 通过；全部颜色走令牌
