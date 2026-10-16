# 任务清单：add-thinking-token-column

## 状态：COMPLETED

## 任务
- [x] 任务 0（集成前置，先于 1/2 执行）：`UsageRecord`/`UsageRecordItem` 新增 `thinking_token_count: Option<i32>`（skip_serializing_if），record() 签名 12→13 参扩展，全调用点先以 `None` 占位透传（daily/pagination 三处构建点同步映射，stream/nonstream/测试调用点 None 占位；验证：`cargo check --tests` 零错误 + serde 兼容单测）
- [x] 任务 1：StreamContext 新增 `thinking_chars_cn`/`thinking_chars_other` 计数器与 `thinking_token_count()` 派生方法，在 native.rs thinking_delta 分支与 mod.rs `create_thinking_delta_event`（改 `&mut self`）非空路径双点累计；mod.rs:829 借用冲突点用 `std::mem::take` 处理（验证：单测分别覆盖 native reasoning 与标签解析两路径断言 >0，`cargo test thinking_token` 通过）
- [x] 任务 2：非流式路径在 `drop_thinking_blocks` 剔除后统计 content 中 `type=="thinking"` 块 token（`count_tokens`，验证：新增单测 `cargo test nonstream_thinking` 通过）
- [x] 任务 3（集成）：流式 record 传入 `Some(thinking_token_count())`、非流式传入统计值，全链路编译通过（验证：`cargo check --tests` 零错误）
- [x] 任务 4：admin-ui Token 用量列追加 🧠 徽章（Brain 图标 + 数字，text-warn 色，千分位格式化与现有 token 数字同源，>0 渲染），i18n 中英 key，types/api.ts 类型（验证：`npx tsc --noEmit` + `pnpm build` 通过）
- [x] 任务 4b：user-ui 请求日志页同步追加同款徽章（复用 thinkingTokenCount 字段，样式遵循 user-ui 主题，验证：user-ui 构建通过 + 徽章条件渲染逻辑与 admin-ui 一致）
- [x] 任务 5：全量验证 `cargo fmt --check` + `cargo clippy` 零警告 + `cargo test` 全绿 + 前端构建（验证：命令全部通过；clippy 警告数 96 与基线 stash 对照一致，本次改动零新增）

## 验收标准
- [x] 开启深度思考的流式请求，日志 Token 列出现 🧠 徽章且数值 > 0；未开启 thinking 的请求不出现徽章
- [x] 非流式（stream=false）请求同样正确统计 thinking token 并落库
- [x] 旧持久化记录（无新字段）加载与分页展示不报错，徽章自然隐藏
- [x] 对外 SSE/JSON 协议响应体不含新字段（output_tokens 口径不变）
- [x] 徽章样式符合 admin-ui 主题（设计令牌取色，数值千分位格式化与现有 token 数字一致，字号字体同源）
- [x] user-ui 请求日志页同步追加同款徽章（复用同字段，样式遵循 user-ui 主题）
