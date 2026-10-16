# usage-log-duration Specification

## Purpose
TBD - created by archiving change add-request-duration-column. Update Purpose after archive.
## Requirements
### Requirement: 请求日志耗时字段
UsageRecord SHALL 记录每个请求的 `durationMs`（服务端处理总耗时，毫秒），流式请求 SHALL 额外记录 `firstTokenMs`（从请求开始到首个上游业务数据事件的延迟，毫秒）。两字段均为可选：旧日志数据与非流式请求无 `firstTokenMs`；非流式请求 `durationMs` 必有值。

#### Scenario: 流式请求耗时记录
- **GIVEN** 一次 /v1/messages 或 /cc/v1/messages 流式请求
- **WHEN** 请求完成并落库
- **THEN** 该记录包含 `durationMs`（请求开始 → 落库前的服务端总时长）
- **AND** `firstTokenMs` 等于请求开始到首个上游业务数据事件的延迟（keepalive 与纯控制帧不计入首字）

#### Scenario: 非流式请求耗时记录
- **GIVEN** 一次非流式 Messages 请求
- **WHEN** 请求完成并落库
- **THEN** 该记录包含 `durationMs`
- **AND** 不包含 `firstTokenMs`

#### Scenario: 旧日志数据兼容
- **GIVEN** 升级前落库的历史日志记录
- **WHEN** 通过日志 API 查询
- **THEN** 响应中不含 `firstTokenMs` / `durationMs` 字段，反序列化不报错

### Requirement: 请求日志用时列展示
请求日志表 SHALL 新增「用时」列：流式记录显示「首字 X.Xs」与「耗时 XX.Xs」两个圆角胶囊徽章，非流式记录仅显示「耗时」徽章；缺失字段显示「—」。徽章样式 SHALL 遵循 admin-ui 现有主题 token，不引入新配色。

#### Scenario: 流式记录展示
- **GIVEN** 一条含 `firstTokenMs` / `durationMs` 的流式日志记录
- **WHEN** 请求日志表渲染该记录
- **THEN** 用时列显示「首字 X.Xs」（毫秒格式化为一位小数秒）与「耗时 XX.Xs」两个徽章

#### Scenario: 缺失字段降级展示
- **GIVEN** 一条不含 `firstTokenMs` 或 `durationMs` 的旧记录（或非流式记录）
- **WHEN** 请求日志表渲染该记录
- **THEN** 对应徽章位置显示「—」，不显示为 0s

#### Scenario: 三页面一致生效
- **GIVEN** API Key 详情页、每日明细页、凭据详情页共用同一请求日志表组件
- **WHEN** 任一页面渲染日志表
- **THEN** 三页均出现「用时」列且渲染逻辑一致

