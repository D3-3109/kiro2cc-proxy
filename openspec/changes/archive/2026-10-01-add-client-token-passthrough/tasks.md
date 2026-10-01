# 任务清单：add-client-token-passthrough

## 状态：ARCHIVED

## 任务
- [x] 任务 1：`src/model/config.rs` 新增 `client_token_passthrough: bool` 字段（serde default false）、Default 同步、`apply_env_overrides` 支持 `CLIENT_TOKEN_PASSTHROUGH`；附 config 解析单元测试（缺省 false / 显式 true / env 覆盖）
- [x] 任务 2：`src/anthropic/stream/calib.rs` 新增 `scale_for_client_with(n, model, passthrough)` 纯函数（true 时 1:1，false 走现有缩放）+ `CLIENT_TOKEN_PASSTHROUGH` AtomicBool 与 setter；`scale_for_client` 委托纯函数；附两种模式断言测试（含 0/负数/大值边界）
- [x] 任务 3：`src/main.rs` 在 config 加载后调用 setter 并在开启时输出 info 日志；`cargo fmt` + `cargo clippy` clean + `cargo test` 全绿
- [x] 任务 4：更新 `config.example.json` 与 `docs/使用指南.md` 配置表（含 auto-compact 提前触发的取舍说明）

## 验收标准
- [ ] 不设置该字段时所有现有测试不变绿→绿，行为与当前完全一致（默认 0.6657 缩放）
- [ ] 设置 `clientTokenPassthrough: true`（或 env `CLIENT_TOKEN_PASSTHROUGH=true`）时，usage.input_tokens / cache_* 字段按真实值 1:1 上报（由 scale_for_client_with 单元测试验证）
- [ ] 内部入库值（usage_tracker）不受该开关影响（现有入库测试保持通过）
- [ ] `cargo fmt --check` 与 `cargo clippy` 无新增告警，`cargo test` 全部通过
