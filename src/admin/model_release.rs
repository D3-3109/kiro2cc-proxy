// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 模型发布日期清单
//!
//! 与 `changelog_data.rs`、`build_model_list()` 同模式：编译期硬编码，
//! 随代码发布同步维护。上游 `ListAvailableModels` 不提供发布时间字段，
//! 故新增模型时在 `MODEL_RELEASE_DATES` 顶部追加一条（模型 ID，发布日期）。
//!
//! 仅负责下发日期，「是否最近新增」由前端按当前时间实时判定
//! （避免页面长时间停留后徽章不消失），窗口天数见
//! `web-ui/admin-ui/src/components/model-list-page.tsx` 的 `RECENT_WINDOW_DAYS`。

/// 模型发布日期清单（模型 ID → `YYYY-MM-DD`）
///
/// 日期取该模型在本代理中「可调用」的日期（上线日），非官方发布日期。
const MODEL_RELEASE_DATES: &[(&str, &str)] = &[
    ("claude-opus-5.5", "2026-10-12"),
    ("claude-sonnet-5.5", "2026-10-12"),
];

/// 查询模型发布日期（未登记返回 `None`）
pub(crate) fn release_date(model_id: &str) -> Option<&'static str> {
    MODEL_RELEASE_DATES
        .iter()
        .find(|(id, _)| *id == model_id)
        .map(|(_, date)| *date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_date_returns_registered_date() {
        assert_eq!(release_date("claude-opus-5.5"), Some("2026-10-12"));
        assert_eq!(release_date("claude-sonnet-5.5"), Some("2026-10-12"));
    }

    #[test]
    fn release_date_none_for_unregistered() {
        assert_eq!(release_date("claude-opus-5"), None);
        assert_eq!(release_date("auto"), None);
        assert_eq!(release_date(""), None);
    }

    #[test]
    fn all_entries_use_iso_date_format() {
        // 前端 Date.parse 依赖 `YYYY-MM-DD`，格式错误会静默失效
        for (id, date) in MODEL_RELEASE_DATES {
            assert_eq!(date.len(), 10, "模型 {id} 的日期格式非法: {date}");
            assert!(
                chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok(),
                "模型 {id} 的日期无法解析: {date}"
            );
        }
    }
}
