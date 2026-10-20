# 变更提案：redesign-docs-page-ui

## 背景

使用文档页（`web-ui/admin-ui/src/components/docs-page.tsx`）在 v3.4.32 已完成功能补全（hash 路由、四客户端配置生成器、API Key 填充、接口参考折叠卡片），但视觉形态仍是改造前的通用三栏布局：hero 下方紧跟分区 tabs，快速接入的三步引导为三张纵向拉伸的大卡片，客户端配置以裸代码块堆叠呈现，接口参考用统一灰色折叠条，排障指南以纯文本条目罗列。

设计稿目录 `/Users/MacBook/Downloads/Kiro2cc` 给出 6 张目标方案（1440×960 × 5 + 1920×1320 总览），明确对标 Vercel / Supabase / Stripe Docs 的现代开发者文档形态，并写明三条改造主线：消除无意义纵向留白、建立多平台终端工作台、语义化接口与五步排障体系。现有实现与目标形态在信息密度与辨识度上差距明显，需按设计稿重写页面结构与视觉层。

## 目标范围

**在范围内：**
- 重写 `docs-page.tsx` 的页面骨架：sticky hero（eyebrow 含「服务就绪:<port>」实时状态点、大标题「从接入到用好每个接口」、副标题、右上角 GitHub README 外链）、分区 tabs 带分组计数角标、左侧内容 / 右侧「本页目录」sticky 双栏
- 右侧 TOC 补全快速接入子项锚点（01 账号准备 / 02 凭证选择 / 03 服务配置 / 代码工作台 / cURL 链路验证）与滚动跟随高亮（IntersectionObserver，复用 `changelog-page.tsx` 已验证实现）
- 快速接入分区改造：三步引导由纵向卡片改为 STEP 01/02/03 三张联排高内聚流转卡片，每卡带状态徽章（已就绪 / 子凭据已选 / 端口监听中）；API Key 下拉内嵌于 Step 02（选项含名称 + sk 掩码 + 「默认」标记），Base URL 输入与 macOS/Linux / Windows 切换内嵌于 Step 03
- 客户端配置工作台改造：新增 Mac 交通灯窗体容器（固定 `#FF5F57` / `#FEBC2E` / `#28C840`），窗体头部承载客户端名称与一键复制，四客户端 tabs 移入窗体内部
- CC Switch 保持独立卡片形态（标题 + 说明 + 右侧「一键导入到 CC Switch」按钮），不并入窗体
- cURL 验证卡：标题「链路快速验证（cURL）」+ 右上「复制 cURL」
- 接口参考分区改造：语义化 POST/GET 徽标（按方法着色）、端点类型徽章（Anthropic 原生 / Claude Code 专用 / OpenAI 兼容 / 通用）、端点卡片保留折叠抽屉、标题行右侧显示中文用途摘要、「标准请求 Payload」标签 + 复制按钮
- 排障指南分区改造：429 / 401 双卡并排（顶部状态码胶囊 + 说明），五步闭环排查表改为「排查步骤 / 操作与检查点 / 正常预期指标」三列 × 5 行，标题「五步闭环连通性排查流程」
- 开发指南分区改造：地址与认证映射矩阵改为 4 列（调用场景/客户端 | 分发目标地址 | 认证 Header/机制 | 说明与特性），模型映射表头改为「请求模型名 | 路由目标 | 思考逻辑与代际约束」，两表统一表头底色 + 首列 code 化
- 内容数据文件 `docsContent.ts` / `docsGuides.ts` 按设计稿补充列结构、表头文案、类型徽章、状态徽章与排障条目（允许修改内容数据，但不得删减既有技术事实）
- `docs.css` 同步重写为新版结构样式，全部颜色继续走既有语义 token（Mac 交通灯三色为唯一例外）
- 中英双语 i18n 新增/调整键位（步骤标题、分区计数、状态标签、徽章、窗体标签等）

**不在范围内：**
- 不新增任何外部依赖（不使用 shiki / prism / framer-motion 等）
- 不改动 Rust 后端：`/server-info` 现有响应仅含 `version`，端口从 `window.location` 派生
- 不改动 hash 路由与滚动补偿逻辑、API Key 选中/回落逻辑、CC Switch deeplink 生成逻辑
- 不改动其他 7 个页面（账号管理、API Keys、使用日志、支持模型、设置、主题、更新日志）
- 不做移动端目录 Dialog（延续既有决策：窄屏隐藏右侧 TOC）

## 技术方案

**布局骨架**：沿用 `changelog-page.tsx` 已验证的模式 —— 顶部 `sticky top-0 z-20 -mx-9 bg-background px-9 pt-7` 承载 hero，主区 `grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_220px]`，右侧 TOC `lg:sticky lg:top-[84px]`。这保证与更新日志页视觉节奏一致，且粘性偏移量已有先例。

**TOC 滚动跟随**：复用 `changelog-page.tsx` 的 IntersectionObserver 模式（`rootMargin: '-8% 0px -70% 0px'`），对内容区 `[data-toc-id]` 节点观察，取视口内最靠前者高亮；与既有 hash 驱动的 `activeId` 合并为同一高亮状态，避免双源冲突。

**三步流转卡片**：`grid grid-cols-1 gap-3 md:grid-cols-3`，每卡内部为 `STEP 0N` 小标 + 状态徽章 + 图标 + 标题 + 说明 + 内嵌控件槽位。Step 02 的 Key 下拉与 Step 03 的 Base URL 输入 + OS 切换直接放入卡片底部，替代现有独立悬浮工具条。

**Mac 窗体工作台**：新增局部组件 `TerminalWindow`，接收 `title` / `children` / `copyText`，头部渲染三色圆点 + 等宽字体标题 + 复制按钮。颜色为固定字面量（用户已确认），在 CSS 中以 `--traffic-red/yellow/green` 局部变量声明并注明例外理由。

**接口参考**：`MethodBadge` 按方法着色（GET=ok / POST=brand / PUT=warn / DELETE=danger）并加大尺寸；新增 `TypeBadge` 渲染端点类型（数据源为 `EndpointSpec` 新增可选 `kind` 字段）；卡片折叠逻辑沿用 `EndpointDoc` 现有 `activeId` 同步展开机制，不改状态机。

**排障五步表**：在 `docsGuides.ts` 的 `connection-check` 条目新增 `table` 结构（3 列 × 5 行），替代原单段 paragraph；429/401 两条新增 `statusCode` 字段驱动双卡布局；补足第 4 条排障条目以对齐设计稿计数角标。

**约束**：不引入新 crate / npm 包；CSS 零硬编码色值（Mac 三色除外，需注释说明）；保持 `cargo fmt` + `cargo clippy` clean（本变更纯前端，仍需 `pnpm build` + `tsc` 通过）。

## 预期影响

- **正向**：快速接入信息密度提升，配置路径从「三步卡片 → 独立工具条 → tabs → 代码块」四段压缩为「流转卡 → 工作台」两段；接口参考与排障页辨识度显著提升
- **功能等价**：所有既有能力（Key 选择、Base URL 校验、四客户端配置、OS 切换、CC Switch 导入、hash 深链、TOC 高亮、折叠抽屉、返回按钮）全部保留，仅重新排布
- **兼容性**：仅影响 `#docs` tab 内部渲染，`dashboard.tsx` 的 `<DocsPage onBack={...} />` 调用签名不变；i18n 新增键位向后兼容（旧键保留或改名后同步 zh/en 两侧）
- **性能**：新增 DOM 节点约 15%，无新增运行时依赖，无额外网络请求（`useServerInfo` 已有 30s 轮询）

## 风险

| 风险 | 应对 |
|---|---|
| 三步卡片内嵌表单在窄屏（<768px）挤压变形 | 卡片网格 `md:grid-cols-3` 以下回落单列，控件宽度 `w-full` |
| Mac 窗体固定三色在 34 套主题中可能显得突兀 | 已确认为设计意图；三色为高饱和小面积圆点（10px），不参与大面积着色 |
| 重写引入状态丢失（Key 选择 / OS tab / 折叠状态） | 保留全部 `useState`，仅移动 JSX 位置；CR 阶段逐项核对 |
| TOC 滚动跟随与 hash 高亮双源冲突 | 统一为单一 `activeId` 状态：hash 变化时立即写入，滚动观察在用户手动滚动后接管 |
| 现有 `docs.css` 类名被其他文件引用 | 先 `grep` 确认 `.docs-*` 类仅在本页使用，重写时保持类名或同步清理 |
| 内容数据补充后中英双语不同步 | `buildGuides` / `buildEndpointSpecs` 的 `copy()` 双分支必须同时更新，CR 阶段逐条核对 |
