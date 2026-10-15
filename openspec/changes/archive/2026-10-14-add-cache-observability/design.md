# design：add-cache-observability

## 上下文

三个观测增强均为低风险外围改动。现有事实：
- `DailySummary`（src/model/usage/daily.rs）仅 5 个字段，`get_daily_summaries()` 已遍历全部 UsageRecord 聚合，`UsageRecord.input_tokens` / `cache_read_input_tokens: Option<u64>` 可直接累加
- dump 判定点在 `post_messages.rs:183`（`std::env::var_os("KIRO_DUMP_BODY")`），handler 链已有 Config 访问路径（AppState 持有 Arc<Config>，若 handler 上下文无现成引用则沿 State 传入）
- 日志层在 `main.rs:40-51`（LogCapture + tracing_subscriber layers），LogCapture 的 MakeWriter 模式可复用于文件写入

## 目标 / 非目标

目标：最小改动达成三项可观测增强，零核心链路改动。
非目标：日志轮转、dump 清理、SQLite、缓存预热。

## 决策

1. **聚合字段直接扩 struct**，不做版本化 API：serde camelCase 增量字段对旧前端无破坏（TS 类型可选/新增均兼容）。**命中率口径 = `cacheRead / input`**（`input_tokens` 已含 cache 命中部分，与 usage-log-table.tsx 的 diff=`input - cacheRead` 展示口径一致）；后端只透出 tokens 原值，前端自行算比值，避免除零与四舍五入口径争议。注意 `get_daily_summaries` 内部用 `(u64, f64, f64, f64)` 四元组聚合、`get_today_summary_for_credential` 是另一个构造点，两处都要同步扩展。
2. **dump 开关采用项目现有全局 AtomicBool + setter 模式**（同 `CLIENT_TOKEN_PASSTHROUGH`，`src/anthropic/stream/calib.rs:58`）：`main.rs` 启动时 `set_dump_kiro_body(config.dump_kiro_body)`，判定点 `config 全局量 OR env KIRO_DUMP_BODY=1`。不删除 env 判定（兼容已在跑的部署脚本）；不做热更新（config 本就启动时读取）。handlers 层拿不到 Config 实例是选择此模式的原因。
3. **日志文件用自定义 MakeWriter + `std::fs::File` append**，不引入 tracing-appender crate：LogCapture 已有同款模式，实现一个 `Arc<File>` 包一层 `MakeWriter` 即可；打开失败仅 warn 不 panic（spec 要求降级），并为此分支补一条单测。同步写（每行一次 write），量级与 stdout 相同，不做 spawn_blocking。
4. **初始化顺序约束**：现有 `main.rs` 中 tracing 层链初始化（40-51 行）先于 config 加载（56 行），`log_file` 依赖 config。决策：**将 config 加载前移至 tracing 初始化之前**（配置加载失败 exit(1) 语义不变，早于任何日志输出可接受）。
5. **config 字段命名**：`dump_kiro_body: bool` 与 `log_file: Option<String>`，均 `#[serde(default)]` + `apply_env_overrides` 补 DUMP_KIRO_BODY env 覆盖，与现有字段风格一致。

## 风险 / 权衡

- 前端 dashboard 布局改动有 UI 回归风险 → 命中率只加展示位（badge/列），不动现有布局结构
- 日志文件句柄为进程生命周期持有，用户需自行处理轮转（与 failure_log.json 策略一致，文档注明）
- 无

## 迁移方案

无需迁移：旧 config.json 缺字段走 serde default；旧前端忽略新增 JSON 字段。

## 待决问题

无。（第 1 轮 sub-agent 审查提出的 4 项 Issue 已全部落入本 design：初始化顺序 → 决策 4、命中率口径 → 决策 1、dump 传递机制 → 决策 2、构造点遗漏 → 决策 1 括注；降级分支单测 → 决策 3。）
