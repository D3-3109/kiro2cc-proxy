# 任务清单：redesign-docs-page-ui

## 状态：DONE

## 任务

- [x] **T1 — 页面骨架、hero 与 i18n 键位**
  重写 `docs-page.tsx` 顶部结构：sticky hero（eyebrow 含「服务就绪:<port>」状态点、大标题、副标题、右上 GitHub README 外链）+ 分区 tabs 计数角标 + 双栏 grid（左内容 / 右 TOC），移除原 `.docs-toolbar` 独立工具条与 `.docs-layout`。同步在 `zh.json` / `en.json` 的 `docs` 命名空间新增/调整键位（`readyLabel`、`stepLabel`、`sectionCount`、`windowCopy`、`statusReady`、`statusKeySelected`、`statusPortListening`、`copyCurl`、`copyPayload` 等）。
  验证：`tsc --noEmit` 通过；页面可见 eyebrow 状态点与 tabs 角标；切换 zh/en 无缺失键；GitHub README 外链可点击。

- [x] **T2 — 快速接入：三步联排流转卡片**
  实现三步卡：`md:grid-cols-3` 三卡，每卡 `STEP 0N` + 状态徽章 + 图标 + 标题 + 说明。Step 02 底部内嵌 API Key 下拉（复用 `selectedKeyId` 与回落逻辑，选项含名称 + sk 掩码 + 「默认」标记）；Step 03 底部内嵌 Base URL 输入（复用 `baseUrlInput` / `parseBaseUrl` / 校验错误提示）与 macOS/Linux / Windows 切换。
  验证：三卡在 1440px 宽度并排、<768px 单列；下拉切换后下方配置同步更新；非法 Base URL 显示错误且示例回退 origin；三个状态徽章随状态变化（无 Key → 已选 Key → 端口监听中）。

- [x] **T3 — 客户端配置工作台（Mac 窗体）+ CC Switch 独立卡**
  新增 `TerminalWindow` 局部组件（三色圆点 `#FF5F57`/`#FEBC2E`/`#28C840` + 等宽标题 + 一键复制），四客户端 tabs 移入窗体头部下方，`CodeBlock` 渲染于窗体体内；CC Switch 保持独立卡片（标题「支持 CC Switch 专属协议一键导入」+ 说明 + 右侧「一键导入到 CC Switch」按钮）；cURL 卡标题改为「链路快速验证（cURL）」+ 右上「复制 cURL」。
  验证：四客户端切换正常；OS 切换仅影响 Claude Code / Codex CLI 分支；复制按钮复制对应完整内容；CC Switch 导入仍可唤起；cURL 卡复制按钮复制完整命令。

- [x] **T4 — 接口参考：语义化徽标、类型徽章与折叠抽屉**
  `docsContent.ts` 的 `EndpointSpec` 新增可选 `kind` 字段（Anthropic 原生 / Claude Code 专用 / OpenAI 兼容 / 通用）并中英双语补齐；改造 `EndpointDoc.tsx`：`MethodBadge` 提升至 `h-[22px] px-2.5 text-[11px]`，新增 `TypeBadge`，标题行左侧方法徽标 + 类型徽章 + 等宽路径 + 右侧用途摘要；「请求体」标签改为「标准请求 Payload」+ 复制按钮；折叠抽屉保留 `activeId` 同步展开；表头统一 `bg-surface-2`。`DocsTOC` 子项方法徽标同步放大。
  验证：五端点徽标与类型徽章均正确渲染；POST=brand / GET=ok；hash 直达 `#api-chat-completions` 自动展开；折叠/展开手动切换正常；Payload 复制可用。

- [x] **T5 — 排障指南：429/401 双卡与五步闭环表**
  `docsGuides.ts`：`error-429` / `error-401` 新增 `statusCode` 字段驱动双卡（状态码胶囊 + 标题 + 说明）；`connection-check` 标题改为「五步闭环连通性排查流程」，`paragraphs` 替换为 3 列 × 5 行 `table`（排查步骤 / 操作与检查点 / 正常预期指标）；补足第 4 条排障条目以对齐设计稿计数角标 4。同步调整 `docs-page.tsx` 渲染分支。
  验证：zh/en 两语言双卡与表格均完整；tabs 角标显示 4；窄屏表格横向滚动（`overflow-x-auto`）不撑破布局。

- [x] **T6 — 开发指南：映射矩阵表 4 列改造**
  `docsGuides.ts`：`authentication` 表改为 4 列（调用场景/客户端 | 分发目标地址 | 认证 Header/机制 | 说明与特性），`model-mapping` 表头第三列改为「思考逻辑与代际约束」；两表统一表头 `bg-surface-2` + 首列等宽 code 化；确认 `.docs-table-wrap` 新样式在全部分区生效且不回归。
  验证：两张表格列数与设计稿一致；首列 code 样式正确；表格在 1024px / 1440px 下不溢出。

- [x] **T7 — TOC 快速接入子项与滚动跟随**
  为快速接入分区补 5 个 TOC 锚点（01 账号准备 / 02 凭证选择 / 03 服务配置 / 代码工作台 / cURL 链路验证），对应 DOM 加 `data-toc-id`；引入 IntersectionObserver（复用 `changelog-page.tsx` 的 `rootMargin: '-8% 0px -70% 0px'`）跟随阅读进度高亮，与 hash 驱动的 `activeId` 合并为单一状态。
  验证：滚动时 TOC 高亮随视口变化；点击 TOC 子项正确跳转；hash 直达后高亮正确；无重复高亮或抖动。

- [x] **T8 — styles 重写与全量验证**
  重写 `docs.css` 为新版结构样式（hero / tabs 角标 / 三步卡 / 窗体 / 类型徽章 / 数据表），所有色值走既有 token（Mac 三色例外并注释）；`grep` 确认 `.docs-*` 类无跨文件引用后清理废弃类。
  验证：`pnpm build` 成功；1440×960 与 1024×768 两档宽度下无横向滚动；暗色 / 亮色 / theme-mono 三套主题下文字对比度可读。

## 验收标准

- [x] 页面结构与 `/Users/MacBook/Downloads/Kiro2cc` 下 1–5 号设计稿一致：sticky hero（含状态点与 GitHub README）、计数 tabs、三步流转卡（含状态徽章）、Mac 窗体工作台、CC Switch 独立卡、cURL 验证卡、语义化接口徽标与类型徽章、429/401 双卡、五步闭环表、TOC 快速接入子项与滚动跟随
- [x] 所有既有功能零回归：Key 选择与回落、Base URL 校验与回退、四客户端 × 两 OS 配置生成、CC Switch deeplink 与唤起、hash 深链与滚动补偿、TOC 高亮、折叠抽屉、返回按钮
- [x] 明暗双主题与至少一套非默认主题（theme-mono）下排版正常、文字对比度 ≥ 4.5:1
- [x] `pnpm build` + `tsc --noEmit` 通过，无 TypeScript 错误与未使用导入
- [x] 不新增任何外部依赖（`package.json` 无变更）
- [x] `cargo fmt` + `cargo clippy` 保持 clean（无 Rust 改动，仅确认未破坏）
