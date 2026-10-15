# spec 增量：add-cache-observability

## Purpose
为管理员提供缓存命中率聚合可观测能力（按日统计 input/cacheRead），并将 Kiro 请求体落盘与日志文件持久化从环境变量/临时手段升级为配置项，便于运维排查。

## ADDED Requirements

### Requirement: 每日用量聚合携带缓存命中字段
系统 SHALL 在 `GET /api/admin/usage/daily` 的每日聚合结果（DailySummary）与凭证维度聚合结果（CredentialDaySummary）中包含当日累计 input tokens 与 cache read tokens 字段。

#### Scenario: 聚合字段数值正确
- **WHEN** 某日存在用量记录（input=1000, cacheRead=800）与（input=500, cacheRead=0）各一条
- **THEN** 该日 `totalInputTokens` 为 1500，`totalCacheReadTokens` 为 800
- **AND** cacheRead 缺失（None）的记录按 0 计入，不影响其他字段

#### Scenario: 空数据日
- **WHEN** 某日无任何用量记录
- **THEN** 该聚合记录的两个新字段为 0，不影响现有字段行为

### Requirement: Kiro 请求体落盘开关配置化
系统 SHALL 支持 `config.json` 中的 `dump_kiro_body` 布尔配置项（默认 false）控制 Kiro 请求体落盘行为，且保留 `KIRO_DUMP_BODY=1` 环境变量作为兼容开关，两者任一生效即落盘。

#### Scenario: 配置开启触发落盘
- **WHEN** config.json 设置 `"dump_kiro_body": true`
- **AND** 客户端发起一条 /v1/messages 请求
- **THEN** 最终发往 Kiro 的请求 body 写入 `app/config/kiro-body-dump/` 目录

#### Scenario: 默认关闭
- **WHEN** config.json 未设置 `dump_kiro_body`（或为 false）且未设置环境变量
- **THEN** 不落盘，行为与现状一致

#### Scenario: 环境变量兼容
- **WHEN** config.json 未开启，但环境变量 `KIRO_DUMP_BODY=1`
- **THEN** 仍触发落盘（向后兼容）

### Requirement: 日志可选写入文件
系统 SHALL 支持 `config.json` 中的 `log_file` 字符串配置项（可选，默认不设置）；配置后，tracing 日志在输出到控制台与 LogCapture 之外同步追加写入指定文件。

#### Scenario: 配置后写文件
- **WHEN** config.json 设置 `"log_file": "app/config/proxy.log"`
- **THEN** 服务启动后日志行追加写入该文件，且控制台与 LogCapture 输出不受影响

#### Scenario: 未配置零影响
- **WHEN** config.json 未设置 `log_file`
- **THEN** 行为与现状完全一致（仅控制台 + LogCapture）

#### Scenario: 文件不可写降级
- **WHEN** `log_file` 指向不可创建/不可写的路径
- **THEN** 服务启动不失败，仅记录一条警告日志并跳过文件写入
