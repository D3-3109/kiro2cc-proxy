# 任务清单：add-cache-observability

## 状态：DONE

## 任务

### 功能 1：缓存命中率聚合展示
- [x] T1 后端：`src/model/usage/daily.rs` 扩展 `DailySummary`（total_input_tokens / total_cache_read_tokens）与 `CredentialDaySummary`（total_cache_read_tokens），同步修改 `get_daily_summaries`（聚合元组扩为含 cacheRead 的多值元组）与 `get_today_summary_for_credential`（构造点累加 cacheRead）；实现时先核对 `input_tokens` 写入源头确认是否已含 cache 部分，据此校准前端口径必要时 UI 注明（验证：cargo test daily，新增聚合单测断言命中率字段）
- [x] T2 前端：`web-ui/admin-ui/src/types/api.ts` 同步 DailySummary 类型；`dashboard.tsx` 与 `daily-detail-page.tsx` 增加缓存命中率展示，口径 = `cacheRead / input`（input 已含 cache 部分，与 usage-log-table.tsx 的 diff=`input - cacheRead` 口径一致）；`i18n/locales/{zh,en}.json` 加词条（验证：pnpm build 通过）

### 功能 2：dump 开关配置化
- [x] T3 后端：`src/model/config.rs` 新增 `#[serde(default)] pub dump_kiro_body: bool` 并纳入 `apply_env_overrides`（验证：cargo test config 默认值单测）
- [x] T4 后端：`src/anthropic/handlers/post_messages.rs` dump 判定改为 `config OR env`：采用项目现有全局 AtomicBool + setter 模式（同 `client_token_passthrough`，setter 放 calib.rs 或同文件），main.rs 启动时 `set_dump_kiro_body(config.dump_kiro_body)`，判定点改读全局量并保留 env 兼容（验证：cargo test，新增判定逻辑单测）

### 功能 3：日志文件持久化
- [x] T5 后端：`src/model/config.rs` 新增 `#[serde(default)] pub log_file: Option<String>`（验证：cargo test config 单测）
- [x] T6 后端：解决 tracing 初始化先于 config 加载的顺序矛盾（将 config 加载前移至 tracing 初始化之前，加载失败改用 `eprintln!` 输出后 exit(1)），在 `main.rs`（或新建 `src/log_file.rs`）实现 append 模式文件写入层挂入 tracing subscriber；打开失败仅 warn 降级不退出（验证：cargo check + 单测覆盖打开失败降级分支 + 手动启动冒烟确认文件有日志行）

## 验收标准
- [x] `GET /api/admin/usage/daily` 响应含 `totalInputTokens` / `totalCacheReadTokens` 字段，且数值等于当日记录累加
- [x] Admin UI 仪表盘与日明细页展示缓存命中率百分比（口径 = cacheRead / input），zh/en 均正常显示
- [x] config.json 设置 `"dump_kiro_body": true` 后（无需环境变量、无需改代码）请求触发 `app/config/kiro-body-dump/` 落盘；默认 false 不落盘
- [x] config.json 设置 `"log_file": "app/config/proxy.log"` 后日志同步写入该文件；未配置时行为与现状完全一致
- [x] `cargo fmt` + `cargo clippy` clean，`cargo test` 全量通过
