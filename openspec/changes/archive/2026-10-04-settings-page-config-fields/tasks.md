# 任务清单：settings-page-config-fields

## 状态：DONE

## 任务

### 后端（Rust）

- [x] 任务 1：`MultiTokenManager` 增加 `max_rpm: AtomicU32` 字段：`new()` 时从 config 同步初始化，新增 `set_max_rpm(u32)` 与 maxRpm 读取 getter；`errors.rs::wait_for_rpm_gate` 改读新 getter；为 Admin handler 提供最小访问通道（`AdminService` 增加方法或 getter 暴露 `set_max_rpm`）。验证：cargo check + cargo test（errors/token_manager 相关）
- [x] 任务 2：`src/admin/types.rs` 新增 `RuntimeConfigResponse` / `SetRuntimeConfigRequest` 类型（三字段 Option 可选更新 + apply_effect 标记）。验证：cargo check
- [x] 任务 3：`src/admin/handlers.rs` 新增 `get_runtime_config` / `set_runtime_config`：GET 返回三字段当前值；PUT 校验（port 1–65535、maxRpm ≥ 0、proxyUrl 空 = 清除或 http/https/socks5 前缀）→ maxRpm 热切换 → `persist_config_field` 写回三字段；内联测试覆盖校验分支与持久化 roundtrip。验证：cargo test
- [x] 任务 4：`src/admin/router.rs` 注册 `/config/runtime` GET/PUT 路由。验证：cargo check

### 前端（admin-ui）

- [x] 任务 5：`api/credentials.ts` + `hooks/use-credentials.ts` 新增 `getRuntimeConfig` / `useRuntimeConfig` / `useSetRuntimeConfig`。验证：pnpm build（admin-ui）
- [x] 任务 6：`settings-panel.tsx`「服务配置」分区新增三行（maxRpm 数字输入热生效、port 数字输入标注重启生效、proxyUrl 文本输入标注重启生效，编辑/保存范式同 adminPsw 行）。验证：pnpm build（admin-ui）
- [x] 任务 7：`i18n/locales/zh.json` + `en.json` 同步新增三行键名与说明文案（zh 说明含「重启后生效」标注）。验证：pnpm build（admin-ui）

### 收尾

- [x] 任务 8：全量验证 `cargo fmt` + `cargo clippy`（0 警告）+ `cargo test` + `pnpm build`（admin-ui），确认 clean

## 验收标准

- [ ] GET `/api/admin/config/runtime` 返回 maxRpmPerCredential / port / proxyUrl 当前值
- [ ] PUT maxRpmPerCredential 后无需重启即对 RPM 限流闸门生效（0 = 不限），且写回 config.json
- [ ] PUT port / proxyUrl 校验非法值返回 400，合法值写回 config.json，响应消息标注「重启后生效」
- [ ] 设置页三个字段可查看与编辑，port/proxyUrl 行 UI 标注重启生效，i18n zh/en 均正常显示
- [ ] cargo fmt + clippy clean，全部测试通过，不引入新外部 crate
