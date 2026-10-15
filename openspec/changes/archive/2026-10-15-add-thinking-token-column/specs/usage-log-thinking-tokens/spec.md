## Purpose

请求日志的深度思考 token 观测能力：每条使用日志记录模型生成的 thinking 文案 token 数（thinking_token_count），供管理员评估深度思考的输出开销。对外协议零变化，仅入库存观测与 admin-ui / user-ui 展示。

## ADDED Requirements

### Requirement: 思考 token 统计落库
The system SHALL 在每条请求日志中记录 `thinking_token_count`（深度思考文案的 token 估算值，`Option<i32>`），流式与非流式路径同统计语义（客户端实际收到的 thinking 内容；换算算法按路径既有口径），旧数据缺字段时反序列化为 `None`。

#### Scenario: 流式请求含 thinking 内容
- **GIVEN** 客户端请求启用 extended thinking（或账号级强制思考）
- **WHEN** 上游流返回 thinking 内容且流结束落库
- **THEN** 该条日志 `thinking_token_count` 为大于 0 的整数
- **AND** 客户端收到的 `output_tokens` 上报值与本变更前完全一致（口径不变）

#### Scenario: 请求未产生 thinking 内容
- **GIVEN** 请求未启用 thinking 或模型未输出思考文案
- **WHEN** 请求结束落库
- **THEN** 该条日志 `thinking_token_count` 为 `Some(0)` 或 `None`，前端不渲染徽章

#### Scenario: 非流式请求含 thinking 块
- **GIVEN** 客户端请求 `stream=false` 且响应含 thinking 块
- **WHEN** 响应组装完成且未被 thinking_suppressed 剔除
- **THEN** 该条日志 `thinking_token_count` 为大于 0 的整数

#### Scenario: 账号关闭 thinking 剔除推理块
- **GIVEN** 响应含 thinking 块但账号级开关裁决为禁止思考（thinking_suppressed）
- **WHEN** `drop_thinking_blocks` 剔除推理块后落库
- **THEN** `thinking_token_count` 不统计被剔除的内容（仅统计剔除后 content 中存留的 thinking 块，通常为 0）

### Requirement: 请求日志展示思考 token 徽章
The admin-ui 与 user-ui SHALL 在各自请求日志 Token 用量列中、当 `thinkingTokenCount` 存在且大于 0 时，渲染 🧠 图标加数值的徽章，数值格式化与所在前端现有 token 数字一致（千分位、字号、字体同源），样式遵循各自前端主题（设计令牌取色），无思考数据时自然隐藏。

#### Scenario: 旧记录无新字段
- **GIVEN** 历史日志记录不含 `thinkingTokenCount` 字段
- **WHEN** 请求日志表渲染该记录
- **THEN** 不渲染 🧠 徽章，其余列展示不变

#### Scenario: user-ui 同步展示
- **GIVEN** 记录含 `thinkingTokenCount > 0`
- **WHEN** user-ui 请求日志页渲染该记录
- **THEN** 同样渲染 🧠 徽章，样式遵循 user-ui 主题（与 admin-ui 同字段同条件，仅视觉令牌不同）
