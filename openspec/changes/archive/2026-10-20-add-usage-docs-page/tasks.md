# 任务清单：add-usage-docs-page

## 状态：DONE

## 任务
- [x] 1. `web-ui/admin-ui/src/components/docs/docsNavigation.ts`：DocEntry 类型、
      `resolveDocsTarget`（hash → 分区/条目）与 `searchDocEntries`（多词 AND 过滤，限 20 条）
      （验证：`npx tsc --noEmit` 通过）
- [x] 2. `web-ui/admin-ui/src/components/docs/EndpointDoc.tsx`：`CodeBlock`（纯 pre/code +
      复制按钮，复用 lib/clipboard）、`MethodBadge`（GET/POST/PUT/DELETE 色板）、
      `EndpointDoc`（折叠端点卡片：方法徽章 + 路径 + 参数表 + 请求/响应示例 + cURL）
      （验证：`npx tsc --noEmit` 通过）
- [x] 3. `web-ui/admin-ui/src/components/docs/docsContent.ts`：5 个端点规格
      （/v1/models、/v1/messages、/cc/v1/messages、/v1/chat/completions、/v1/responses），
      中英双语文案、参数表、curl 与响应示例（从 README 既有描述适配）
      （验证：`npx tsc --noEmit` 通过 + 内容与 README/路由核对一致）
- [x] 4. `web-ui/admin-ui/src/components/docs/docsGuides.ts`：开发指南（地址与认证、双端点差异、
      流式 SSE、模型映射）与排障指南（401/429/Reset/额度）指南条目，
      GuideSpec 结构（paragraphs/table/examples/links），中英双语
      （验证：`npx tsc --noEmit` 通过）
- [x] 5. `web-ui/admin-ui/src/components/docs/DocsTOC.tsx` + `docs.css`：本页目录组件
      （父子两级 + aria-current）与页面样式（hero/tabs/搜索/折叠/表格/步骤条，
      用 ink/surface/brand token，支持暗色）
      （验证：`npx tsc --noEmit` 通过 + 浏览器目视两主题）
- [x] 6. `web-ui/admin-ui/src/components/docs-page.tsx`：主页面（hero + 分区 tabs + DocsSearch +
      左内容/右 TOC + hash 路由监听 + API Key 选择器 + Base URL 推导 +
      客户端配置生成器 Claude Code/Codex CLI/OpenAI SDK/CC Switch 协议链导入 双 OS + cURL 验证块）
      （验证：`npx tsc --noEmit` 通过）
- [x] 7. 接线：`dashboard.tsx` 新增 activeTab 'docs' 渲染 DocsPage + 侧边栏 navSystem 插入
      「使用文档」入口；`i18n/locales/{zh,en}.json` 新增 docs 命名空间全部文案
      （验证：`npx tsc --noEmit` 通过 + 侧边栏入口可见、中英切换文案正确）
- [x] 8. `pnpm build` 构建通过 + README 设置页/功能清单提及新增文档页（如适用）
      （验证：build 产物生成、无 TS/构建错误）

## 验收标准
- [ ] 侧边栏「系统」分组出现「使用文档」入口，点击进入文档页，其他页面不受影响
- [ ] 四大分区均可切换：快速接入 / 开发指南 / 接口参考 / 排障指南，hash 路由直达
      （#quick-start / #development / #model-api / #troubleshooting）
- [ ] 快速接入：选择 API Key 后，Claude Code / Codex CLI / OpenAI SDK / CC Switch 配置块自动填充
      当前访问地址与所选 key，复制按钮可用；macOS/Linux 与 Windows 切换正常
- [ ] 接口参考：5 个端点卡片展示方法徽章、参数、请求/响应示例与可复制的 cURL
- [ ] 搜索框可按标题/摘要/方法过滤条目（中英文均生效）
- [ ] 中英双语切换后所有文案正确；暗色主题样式正常
- [ ] `npx tsc --noEmit` 与 `pnpm build` 通过
