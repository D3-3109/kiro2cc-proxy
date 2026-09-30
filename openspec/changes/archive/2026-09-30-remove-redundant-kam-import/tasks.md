# 任务清单：remove-redundant-kam-import

## 状态：ARCHIVED

## 任务
- [x] 从账号操作栏及 `Dashboard` → `CredentialList` → `CredentialActionBar` 属性链删除独立 KAM 导入入口；使用引用扫描确认 `openKamImport` 和 `dashboard.kamImport` 不再存在。
- [x] 从 `Dashboard` 和 `DashboardDialogs` 删除独立 KAM 导入弹窗状态、属性、导入与挂载，并删除 `kam-import-dialog.tsx`；使用引用扫描确认 `KamImportDialog`、`kamImportDialogOpen` 和组件路径无残留。
- [x] 删除仅由独立 KAM 导入功能使用的中英文翻译键，保留批量导入共享键；通过 JSON 解析和翻译键引用扫描验证。
- [x] 运行 `pnpm --dir admin-ui build`，确认 TypeScript 与 Vite 生产构建通过，并核对“KAM 批量导入”入口仍存在。

## 验收标准
- [x] 账号管理操作栏不再显示“KAM 导入”按钮。
- [x] 独立 KAM 导入弹窗、组件文件、状态和属性链已删除。
- [x] “KAM 批量导入”按钮及其单账号/多账号兼容逻辑保持不变。
- [x] 中英文翻译文件无失效的独立 KAM 导入键，且 JSON 语法有效。
- [x] Admin UI 生产构建通过。
