# 变更提案：add-cache-observability

## 背景

前缀缓存命中率直接决定 Kiro credits 消耗（实测：TTL 过期后首轮 diff 可达 4 万+ tokens 全价计费，稳态轮间 diff 仅 24-36）。当前用户无法在 Admin UI 中看到缓存命中率聚合趋势，只能手工调用 Admin API 逐条计算；排查协议问题时需依赖 `KIRO_DUMP_BODY=1` 环境变量并重启服务；日志仅有 1000 条内存环形缓冲（LogCapture），重启即丢，多账号并发下几分钟即被冲掉，故障复盘困难。

## 目标范围

**在范围内：**
1. **缓存命中率聚合展示**：
   - 后端 `DailySummary` 增加 `total_input_tokens` / `total_cache_read_tokens` 字段；`CredentialDaySummary` 增加 `total_cache_read_tokens`
   - Admin UI 仪表盘与日明细页展示缓存命中率（cacheRead / input 百分比）
   - `GET /api/admin/usage/daily` 与 `GET /api/admin/usage/daily/{date}/records` 响应随之携带新字段
2. **Kiro 请求体落盘开关配置化**：`Config` 新增 `dump_kiro_body: bool`（默认 false），`post_messages.rs` 的 dump 判定从 `KIRO_DUMP_BODY` 环境变量改为读配置（环境变量仍保留兼容，OR 语义任一生效即落盘）。传递采用项目现有全局 AtomicBool + setter 模式（同 `client_token_passthrough`，见 `src/anthropic/stream/calib.rs:58`），不通过 handler State 传 Config
3. **日志文件持久化（可选）**：`Config` 新增 `log_file: Option<String>`（默认 None），配置后日志在输出到控制台 + LogCapture 之外同步追加写入该文件。**实现顺序约束**：现有 `main.rs` 中 tracing 层链初始化（40-51 行）先于 config 加载（56 行），`log_file` 依赖 config，需将「读取 config 中的 log_file」提前到 tracing 初始化之前（config 主体加载保持在原位，仅 log_file 相关初始化前移或采用二次初始化方案，实施时二选一并在 PR 中注明）

**不在范围内：**
- dump 文件的自动滚动清理（保持现状：手动清理 `app/config/kiro-body-dump/`）
- 缓存 TTL 感知预热、用量预算限额（本次明确不做）
- LogCapture 的环形缓冲容量调整、日志按级别过滤配置
- 数据库/SQLite 持久化（日志写文件用最简追加方式）

## 技术方案

1. **命中率聚合**：`src/model/usage/daily.rs` 的 `get_daily_summaries` 聚合循环中累加 `input_tokens` 与 `cache_read_input_tokens`（None 视为 0），扩展 `DailySummary` / `CredentialDaySummary` 结构体（serde camelCase 序列化自动透出）。前端 `dashboard.tsx` / `daily-detail-page.tsx` 增加命中率展示（复用现有 i18n 机制，zh/en 各加词条）。
2. **dump 开关**：`Config` 加 `#[serde(default)] pub dump_kiro_body: bool`；`post_messages.rs` 判定改为 `config.dump_kiro_body || env KIRO_DUMP_BODY=1`（通过现有 AppState/Config 引用传递，不引入全局 static）。
3. **日志文件**：`Config` 加 `#[serde(default)] pub log_file: Option<String>`；`main.rs` 初始化时若配置了 `log_file`，以 append 模式打开文件，用 `tracing_appender` 风格的自定义 `MakeWriter`（基于现有依赖实现，不新增 crate）挂入 `tracing_subscriber` 层链。

不改动核心转发链路（converter/provider/stream 零改动），无协议行为变化。

## 预期影响

- **兼容性**：新配置字段均有 serde 默认值，旧 config.json 无需修改即可继续使用；Admin API 新增字段为增量，旧前端读取不受影响
- **性能**：命中率聚合在已有遍历中完成，O(1) 额外开销；日志写文件为同步追加（每行一次 write），量级与现有 stdout 输出相同
- **测试**：现有 600 个测试全部不受影响；新增 daily 聚合字段、config 默认值、dump 判定逻辑的内联单测

## 风险

- **风险**：`DailySummary` 字段扩展可能影响前端类型定义 → 应对：同步更新 `web-ui/admin-ui/src/types/api.ts`
- **风险**：tracing 初始化先于 config 加载，`log_file` 存在初始化顺序矛盾 → 应对：将 config 加载（或仅 log_file 读取）前移至 tracing 初始化之前，实施时保持「配置加载失败 exit(1)」语义不变
- **风险**：日志文件句柄长期占用 / 文件无限增长 → 应对：仅 append 写入、不做轮转，文档注明由外部 logrotate 处理（与 failure_log.json 等现有文件策略一致）
- **风险**：dump 开关误开导致磁盘增长 → 应对：默认 false，且 dump 目录仅写 0600 权限文件（现状保留）
