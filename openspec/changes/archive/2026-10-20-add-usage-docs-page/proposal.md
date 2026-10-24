# 变更提案：add-usage-docs-page

## 背景
Admin 面板当前缺少面向使用者的接入文档：客户端（Claude Code / Codex CLI / OpenAI SDK）如何配置、
服务端点有哪些、认证方式与错误排查，全靠 README.md 承载，使用者必须离开控制台到仓库找文档。
参考 codex2cc 仓库的 Docs 页面（hero + 分区 tabs + 搜索 + 左内容/右 TOC + hash 路由 + 代码块复制），
在 kiro2cc-proxy 的 admin-ui 内实现同等的「使用文档」页面，内容适配本仓库端点与接入方式。

## 目标范围
**在范围内：**
- admin-ui 新增「使用文档」页面（activeTab = `docs`），侧边栏「系统」分组新增入口（BookOpen 图标）；
  页面宿主为 admin-ui（用户已确认）：可复用管理面板登录门保护明文 API Key 填充能力，user-ui 不动
- 四大内容分区（适配 kiro2cc 自身端点与客户端，非照搬 codex2cc 文案）：
  - **快速接入**：三步引导（准备账号 → 选择 API Key → 配置并验证）；客户端配置生成器
    （Claude Code 环境变量 / settings.json、Codex CLI config.toml + auth.json、OpenAI SDK base_url、
    CC Switch 一键导入协议链 `ccswitch://`，macOS/Linux 与 Windows 双 OS 切换）；
    cURL 验证示例（/v1/messages）
  - **开发指南**：地址与认证（ANTHROPIC_BASE_URL vs 上游代理、子 API Key vs adminPsw）、
    双端点差异（/v1/messages 直通 vs /cc/v1/messages 300s deadline）、流式 SSE、模型映射规则
  - **接口参考**：GET /v1/models、POST /v1/messages、POST /cc/v1/messages、
    POST /v1/chat/completions、POST /v1/responses（方法徽章 + 参数表 + 请求/响应示例 + cURL）
  - **排障指南**：常见错误（401 认证、429 限流桶、上游 Reset、额度耗尽）、连接排查步骤
- 动态能力：API Key 下拉选择（读 `useApiKeys()`，明文 key 填充示例）、Base URL 自动取
  `window.location.origin`、每个代码块一键复制
- hash 路由（`#quick-start` / `#development` / `#model-api` / `#troubleshooting` / 具体端点 id）、
  文档搜索（标题/摘要/方法过滤，最多 20 条）、右侧本页 TOC
  （用户已确认剔除移动端目录 Dialog，页面在窄屏下隐藏侧 TOC 即可，保持 admin-ui 既有响应式基线）
- i18n 中英双语文案（docs 命名空间）
- 纯前端实现，不改动任何后端代码；复用已有 `getApiKeys` 端点

**不在范围内：**
- 不引入 shiki 语法高亮（零依赖纯文本代码块 + 复制按钮，样式对齐 log-viewer）
- 不做「试运行」真实发请求按钮
- 不做整页 Markdown 导出
- 不文档化 Admin API（/api/admin/* 内部管理接口）
- 不改动 user-ui（仅 admin-ui）

## 技术方案
- 布局：单文件页面组件 `docs-page.tsx` + `docs/` 子目录（`docsContent.ts` 端点规格、
  `docsGuides.ts` 指南内容、`docsNavigation.ts` hash 解析与搜索、`EndpointDoc.tsx` 端点卡片 +
  `CodeBlock`、`DocsTOC.tsx` 目录、`docs.css` 样式），结构对齐 codex2cc 但按本仓库设计 token
  （ink/surface/brand 色板、PANEL 容器）重写样式，不直接拷贝 docs.css
- 接入 dashboard：`activeTab === 'docs'` 渲染 `<DocsPage />`；侧边栏 navSystem 分组
  「查看日志」上方插入文档入口
- 数据：React Query `useApiKeys()`（选择 key 填充 curl/配置示例）；Base URL 由
  `window.location.origin` 推导，支持手改（用户已确认剔除模型下拉，示例统一用静态兜底模型名）
- hash 路由：监听 hashchange，`resolveDocsTarget` 把 hash 映射到分区/条目；分区切换用 `<a href="#id">`
  锚点，不引入 react-router（admin-ui 无 router 依赖，保持 activeTab 架构）
- 代码块：`<pre><code>` 纯文本 + 右上角复制按钮（复用 `lib/clipboard.ts` 的 `copyToClipboard`），
  label 显示文件路径/语言
- 文案：所有 UI 文案走 i18next `docs.*` 命名空间（zh.json / en.json 同步新增）；
  端点请求/响应示例为协议内容，不进 i18n

## 预期影响
- admin-ui 新增一个页面与侧边栏入口，其余页面零改动；后端零改动；不消费 `getModels` 端点
- 新增前端代码约 2500–3500 行（6 个新文件 + dashboard/zh/en 三处接线）
- 构建产物 dist 体积增加约 60–100KB（纯文本内容无依赖增量）
- 无数据库 / 协议 / 权限变更；`getApiKeys` 为既有端点，页面仅在登录态可访问（Admin 面板本身有登录门）

## 风险
- **明文 key 展示**：示例中填充所选 API Key 明文（codex2cc 同款行为）。缓解：key 选择器默认
  显示脱敏（前 7 后 4），仅填充到示例文本与复制内容中；页面仅在 Admin 登录态可达
- **文档内容与实现漂移**：端点规格硬编码在前端，后端变更需手动同步。缓解：内容取自 README
  既有描述（与实现同步维护），验收时逐项核对
- **hash 路由与 activeTab 冲突**：dashboard 无 router，hash 仅在 docs 页内消费；离开页面时
  hash 残留无副作用（其他页不读 hash）
- **文件行数**：docsContent/docsGuides 内容文件可能超 800 行上限；按数据声明文件处理
  （纯内容无逻辑分支），必要时拆分为两个文件
