## 上下文

admin-ui 请求日志已有「用时」列（first_token_ms/duration_ms）先例：StreamContext 计时 → UsageRecord 新 Option 字段 → record() 透传 → 分页 DTO → 前端徽章。本变更复制该链路，观测对象从计时换为 thinking token 数。

流式路径现状：thinking 内容经三条来源产生（native reasoning 事件、`<thinking>` 标签解析、流结束 flush 兜底），但全部收敛到 `create_thinking_delta_event(index, thinking)` 单函数发往客户端。现有计数器：`output_chars_*`（计费口径，含 thinking 与 web_search 截获输入）、`visible_chars_*`（上报口径，仅 text/tool_use）。

非流式路径现状：content 块数组中 `{"type":"thinking","thinking":...}` 结构完整，`estimate_output_tokens` 只统计 text/tool_use，不含 thinking。

## 目标 / 非目标

**目标：** thinking token 数单字段落库 + admin-ui 徽章展示，流式/非流式同口径。

**非目标：** 对外 output_tokens 口径调整；record() 参数 struct 化重构；历史数据回填。

## 决策

**D1：独立计数器而非差值推导。**
`output_chars_* - visible_chars_*` 差值混入 echo_guard 剥离内容（计费口径含、客户端不可见）、web_search 截获输入（`process_tool_use` 计入 output_chars_other）、纯 thinking 响应的空格兜底（generate_final_events_raw 补 " " 进 visible）等噪声，推导值系统性偏差。独立计数器在双发射点累计，语义精确：客户端实际收到的 thinking_delta 内容。

**D2：双统计点而非单点收敛。**
thinking 内容有两个互斥的发射路径：(a) native reasoning 主路径走 `native.rs:63` 的 `state_manager.handle_content_block_delta`（thinking_delta 类型），不经过 `create_thinking_delta_event`；(b) `<thinking>` 标签解析与 flush 兜底路径走 `mod.rs::create_thinking_delta_event`。两处均按「state_manager/delta 实际产出」累计，空串与被状态机拒绝的 delta 不计。单点收敛方案（仅改 create_thinking_delta_event）会静默漏计 native 主路径，已否决。

**D3：非流式在 `drop_thinking_blocks` 剔除后统计。**
与流式「客户端实际收到」语义对齐——剔除发生在组装后、上报前，被剔除内容客户端不可见故不计。统计位置与 `estimate_output_tokens` 同层（剔除后遍历 content），`type=="thinking"` 块的 `thinking` 字段用 `count_tokens`（与 estimate_output_tokens 同族，BPE 真值口径优于字符估算）。

**D4：字段命名 `thinking_token_count`（Rust）/ `thinkingTokenCount`（JSON/TS）。**
与既有 effort/first_token_ms 命名风格一致；`Option<i32>` + `skip_serializing_if = "Option::is_none"`，与 first_token_ms 完全同构。

**D5：流式换算用 tokens_from_chars（字符估算），非流式用 count_tokens（BPE）。**
各自复用所在路径的既有口径（流式逐 delta 只能字符累计，收尾一次换算；非流式整块可 BPE）。两侧均为估算值，量级一致（±5% 内），观测用途足够，不入计费。

## 风险 / 权衡

- create_thinking_delta_event 从 `&self` 改 `&mut self`：8 处调用点无冲突；mod.rs:829 参数直接借用 `self.thinking_buffer`（`&self.thinking_buffer` 传参），改 `&mut self` 触发 E0502，处理策略为 `std::mem::take` 后调用（该分支随后本就 `clear()`，语义无损）
- record() 12→13 参：位置参数技术债加深，已知问题（cr-known-issues），本次不重构
- 流式/非流式口径微差（字符估算 vs BPE）：观测字段非计费字段，可接受；spec 不承诺跨路径一致性数值
- 双统计点漏计风险：以单测分别覆盖 native reasoning 与标签解析两条路径的 thinking_token_count > 0 断言锁定
- 统计判定口径：`create_thinking_delta_event` 用 `is_empty()` 判定（与该函数现状发射条件一致，空白内容照计——与 output/visible 口径对齐，不引入第三种语义）

## 待决问题

（无——统计口径已定：客户端实际收到的 thinking 内容；展示条件已定：>0 才渲染）
