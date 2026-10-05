// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 响应前缀防护：剔除模型偶发"回声"用户消息中 `<system-reminder>` 块的情况
//!
//! Claude Code 会在用户消息里以独立 content block 附带标准 `<system-reminder>`
//! 文本（如 "If applicable, use the available tools..."），这段文本按设计必须
//! 原样转发给上游模型（见 `converter::prompt` 对 reminder 的处理：它仅对 GPT 系
//! 做 history 位置搬迁，内容本身从不删改）。
//!
//! 个别情况下上游模型会把这段元提示当作要回复的内容直接复述在回答最前面，
//! 导致客户端界面出现一段 `<system-reminder>...</system-reminder>` 原文。这是
//! 模型侧的偶发行为，不受代理控制；本模块在"返回给客户端"的最后一道关口上做
//! 防御性检测与剥离，只处理回复最前面的情况，避免误伤正文中合法出现相似文本。

/// 判断累积文本是否仍可能是正在逐步到达的 `<system-reminder>...</system-reminder>` 块
/// （用于流式场景：标签前缀已匹配但闭合标签尚未到达，需要继续等待更多内容）。
fn is_partial_prefix_of_reminder(buffered: &str) -> bool {
    const TAG: &str = "<system-reminder>";
    if buffered.len() < TAG.len() {
        // 尚不够长，判断已到达部分是否仍是 TAG 的前缀
        TAG.starts_with(buffered)
    } else {
        // 已经超过标签长度，判断开头是否匹配（闭合标签尚未出现）
        buffered.starts_with(TAG)
    }
}

/// 从累积文本开头剥离一个完整的 `<system-reminder>...</system-reminder>` 块
/// （含紧随其后的空白/换行）。未命中时原样返回 `None`。
fn strip_leading_reminder_block(buffered: &str) -> Option<&str> {
    const OPEN: &str = "<system-reminder>";
    const CLOSE: &str = "</system-reminder>";

    let rest = buffered.strip_prefix(OPEN)?;
    let close_pos = rest.find(CLOSE)?;
    let after = &rest[close_pos + CLOSE.len()..];
    Some(after.trim_start_matches(['\n', '\r', ' ']))
}

/// 有状态过滤器：仅在响应最开始、尚无任何内容发出时介入；一旦确认不是
/// reminder 回声（或已剥离完成），立即切换为透传模式，不影响后续性能与正确性。
#[derive(Debug, Default)]
pub struct ResponseEchoGuard {
    /// `None` 表示已经决出结果（命中剥离完成 / 判定不命中），后续全部直接透传
    pending: Option<String>,
    resolved: bool,
    /// 命中剥离但闭合标签后暂无正文内容时置位：下一次非空输出前先修剪前导空白
    /// （闭合标签与正文之间的换行可能跨 chunk 到达）
    trim_leading_whitespace_on_next: bool,
}

/// 防止极端情况下（上游一直不发送结束标签）缓冲区无限增长
const MAX_PENDING_LEN: usize = 4096;

impl ResponseEchoGuard {
    pub fn new() -> Self {
        Self {
            pending: Some(String::new()),
            resolved: false,
            trim_leading_whitespace_on_next: false,
        }
    }

    /// 处理一段新到达的文本增量，返回应当真正下发给客户端的文本（可能为空字符串）。
    pub fn filter(&mut self, chunk: &str) -> String {
        if self.resolved {
            if self.trim_leading_whitespace_on_next {
                let trimmed = chunk.trim_start_matches(['\n', '\r', ' ']);
                if !trimmed.is_empty() || chunk.is_empty() {
                    self.trim_leading_whitespace_on_next = trimmed.is_empty() && !chunk.is_empty();
                }
                return trimmed.to_string();
            }
            return chunk.to_string();
        }

        let buffer = self.pending.get_or_insert_with(String::new);
        buffer.push_str(chunk);

        // 命中完整 reminder 块：剥离后转入透传模式，返回剩余正文（如果有）
        if let Some(remaining) = strip_leading_reminder_block(buffer) {
            let remaining = remaining.to_string();
            self.resolved = true;
            self.pending = None;
            // 闭合标签所在 chunk 内尚未出现非空白正文时，继续在后续 chunk 上修剪前导空白
            self.trim_leading_whitespace_on_next = remaining.is_empty();
            return remaining;
        }

        // 仍可能是部分标签，继续缓冲等待更多内容（有长度上限兜底）
        if is_partial_prefix_of_reminder(buffer) && buffer.len() < MAX_PENDING_LEN {
            return String::new();
        }

        // 不是 reminder 开头（或缓冲已超限仍未闭合）：判定不命中，把已缓冲内容原样放出
        self.resolved = true;
        std::mem::take(&mut self.pending).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_full_reminder_block_delivered_in_one_chunk() {
        let mut guard = ResponseEchoGuard::new();
        let out = guard.filter(
            "<system-reminder>\nIf applicable, use the available tools to fulfill the user's request. If no tools are needed, continue as normal.\n</system-reminder>\n\n正文开始",
        );
        assert_eq!(out, "正文开始");
    }

    #[test]
    fn strips_reminder_split_across_multiple_chunks() {
        let mut guard = ResponseEchoGuard::new();
        let mut out = String::new();
        out.push_str(&guard.filter("<system-reminder>\nIf applicable"));
        out.push_str(&guard.filter(", use the tools.\n</system-reminder>"));
        out.push_str(&guard.filter("\n\n真正的回答"));
        assert_eq!(out, "真正的回答");
    }

    #[test]
    fn passes_through_normal_text_untouched() {
        let mut guard = ResponseEchoGuard::new();
        let mut out = String::new();
        out.push_str(&guard.filter("你好"));
        out.push_str(&guard.filter("，世界"));
        assert_eq!(out, "你好，世界");
    }

    #[test]
    fn passes_through_text_that_merely_mentions_reminder_tag_later() {
        let mut guard = ResponseEchoGuard::new();
        let mut out = String::new();
        out.push_str(&guard.filter("这是关于 "));
        out.push_str(&guard.filter("<system-reminder> 标签的说明"));
        assert_eq!(out, "这是关于 <system-reminder> 标签的说明");
    }

    #[test]
    fn once_resolved_as_no_match_stays_in_passthrough_mode() {
        let mut guard = ResponseEchoGuard::new();
        assert_eq!(guard.filter("hi"), "hi");
        assert_eq!(guard.filter(" there"), " there");
    }

    #[test]
    fn once_resolved_as_match_stays_in_passthrough_mode_for_rest_of_stream() {
        let mut guard = ResponseEchoGuard::new();
        let _ = guard.filter("<system-reminder>x</system-reminder>answer");
        assert_eq!(guard.filter(" more"), " more");
    }

    #[test]
    fn unterminated_reminder_like_prefix_eventually_flushes_without_hanging() {
        let mut guard = ResponseEchoGuard::new();
        // 远超 "<system-reminder>" 长度、但始终不闭合的内容最终必须被放出，不能无限缓冲
        let long_prefix = "<system-reminder>".to_string() + &"a".repeat(MAX_PENDING_LEN);
        let out = guard.filter(&long_prefix);
        assert_eq!(out, long_prefix);
    }

    #[test]
    fn empty_chunk_does_not_resolve_prematurely() {
        let mut guard = ResponseEchoGuard::new();
        assert_eq!(guard.filter(""), "");
        assert_eq!(guard.filter("<system-reminder>"), "");
        assert_eq!(guard.filter("done</system-reminder>answer"), "answer");
    }
}
