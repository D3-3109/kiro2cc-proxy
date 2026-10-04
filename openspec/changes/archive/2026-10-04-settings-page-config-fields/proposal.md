# 变更提案：settings-page-config-fields

## 背景

设置页目前已迁移 Suggestion Mode 与 clientTokenPassthrough 两个开关，但 `maxRpmPerCredential`、`port`、`proxyUrl` 三个字段仍只能手工编辑 config.json 后重启才能调整。需求是将这三个字段也迁移到 Admin 设置页可视化管理。

三个字段的运行时生效机制不同：
- `maxRpmPerCredential`：消费点 `src/kiro/provider/errors.rs::wait_for_rpm_gate` 每次请求时从 `token_manager.config().max_rpm_per_credential` 读取，具备热切换条件——但 `MultiTokenManager::config()` 返回不可变引用 `&Config`，需引入内部可变性
- `port`：`src/main.rs` 启动时 `format!("{}:{}", config.host, config.port)` 绑定监听地址，改后只能重启生效
- `proxyUrl`：`src/main.rs` 启动时构建 `ProxyConfig` 固化进 reqwest Client（`MultiTokenManager::new`），改后只能重启生效

用户已确认采用「热切换 + 重启标注」方案：maxRpm 热切换，port/proxyUrl 可编辑保存但 UI 与 API 标注「重启后生效」。

## 目标范围

**在范围内：**
- 新增 Admin API：GET `/api/admin/config/runtime` 返回三字段当前值（含 `config.json` 持久化值）；PUT `/api/admin/config/runtime` 保存三字段（校验 + 运行时热切换 maxRpm + 写回 config.json）
- maxRpm 热切换：`MultiTokenManager` 内部增加该字段的可变性通道，`wait_for_rpm_gate` 读到最新值，`0` 表示不限流语义保持
- PUT 校验：port 范围 1–65535；maxRpm 允许 0（不限）及正整数；proxyUrl 允许空串/缺省（清除代理）或 http/https/socks5 URL
- 持久化复用 `persist_config_field`（tmp + rename 原子替换，`persist_lock` 防并发）
- 前端设置页「服务配置」分区新增三行：maxRpm 数字输入（热生效）、port 数字输入（标注重启后生效）、proxyUrl 文本输入（标注重启后生效）
- i18n zh/en 同步新增文案
- 内联 `#[cfg(test)]` 单元测试覆盖持久化与校验逻辑

**不在范围内：**
- port/proxyUrl 的运行时热生效（需重建 reqwest Client / 重新绑定监听端口，风险大，明确排除）
- host 字段迁移
- 修改 OpenAI 兼容层、协议转换层任何逻辑

## 技术方案

- **API 形态**：单端点 `/config/runtime`（GET/PUT）一次承载三字段，避免三个独立端点碎片化；PUT 请求体三字段均为 `Option`，只更新显式传入的字段
- **热切换实现**：`MultiTokenManager` 新增 `max_rpm: AtomicU32` 独立字段（最小改动，不改动 Config 的 serde 结构），`set_max_rpm()` 写入；`wait_for_rpm_gate` 改为通过新 getter 读取；PUT handler 持 token_manager Arc 调用 setter
- **值获取**：GET 从 token_manager 读 maxRpm 当前值（含热切换后的值），port/proxyUrl 从 AdminState 已有的 config_path 读 config.json（port 无运行时值概念，直接读文件）。已知 `Config::apply_env_overrides` 支持环境变量覆盖 PORT/PROXY_URL（容器化部署），GET 返回的 port/proxyUrl 是 config.json 持久化值而非运行值——明确接受此差异，前端文案统一注明「修改后需重启生效」，不做运行值比对
- **前端**：沿用 settings-panel.tsx 现有 `Row` + `Input` + 编辑/保存按钮范式（同 adminPsw 行），port/proxyUrl 行 desc 标注「重启后生效」，保存成功 toast 提示重启提示

## 预期影响

- 新增 2 个 Admin API 端点，不影响现有端点与 `/v1/messages`、`/cc/v1/messages` 链路
- maxRpm 热切换改变限流闸门取值来源（Config 直读 → AtomicU32），行为语义不变（0 = 不限）
- config.json 新增/更新三个既有字段，老配置文件不含这些字段时 serde 默认值兜底，兼容无损

## 风险

- **maxRpm 取值来源分叉**：Config 内字段与 AtomicU32 可能不一致 → 统一入口：初始化时从 config 同步一次，此后运行时只读 AtomicU32，持久化写文件以 PUT 请求值为准
- **PUT 并发覆盖**：与其他 PUT 共用 `persist_lock`，已由现有机制兜底
- **port 改为非法值导致重启失败**：PUT 侧范围校验（1–65535）+ 前端输入约束，降低风险
- **持久化失败后的 maxRpm 分叉**：沿用现有「持久化失败但运行时已生效」500 响应语义（与 suggestion-mode 一致），此时 AtomicU32 与 config.json 暂时分叉，重启后回退旧值——已知取舍，不做额外处理
- **访问通道**：`AdminService.token_manager` 为 private 字段，需经 `AdminState`/service 增加最小访问通道（getter 或 service 方法）供 handler 调用 `set_max_rpm`
