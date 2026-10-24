// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 全局日志过滤器 — 支持运行时热切换 DEBUG 级别

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing_subscriber::EnvFilter;
use tracing_subscriber::reload;

type FilterHandle = reload::Handle<EnvFilter, tracing_subscriber::Registry>;

/// 可重载过滤器句柄（install 后可用）
static HANDLE: OnceLock<FilterHandle> = OnceLock::new();
/// 当前 DEBUG 开关状态（GET 接口与状态查询用，避免从过滤器反解）
static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

/// 构建过滤器：优先 RUST_LOG，缺省 info；debug=true 时追加全局 debug 指令
///
/// 追加的 `debug` 是全局默认档，EnvFilter 中更具体的 target 定向指令
/// （如 `kiro2cc_proxy::kiro=debug`）仍优先于它。同 target 指令是替换语义，
/// 故 RUST_LOG 已含裸 trace 指令时跳过追加，保证开关只升不降。
pub fn build_filter(debug: bool) -> EnvFilter {
    let rust_log = std::env::var("RUST_LOG").ok();
    build_filter_from(rust_log.as_deref(), debug)
}

/// `build_filter` 的纯函数核心（rust_log 为 RUST_LOG 原始值，None/空串等价于未设置）
fn build_filter_from(rust_log: Option<&str>, debug: bool) -> EnvFilter {
    // "" 与未设置在 parse_lossy 语义下等价：均回退 default directive（info）
    let mut filter = EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .parse_lossy(rust_log.unwrap_or(""));
    if debug
        && !has_bare_trace_directive(rust_log)
        && let Ok(directive) = "debug".parse()
    {
        filter = filter.add_directive(directive);
    }
    filter
}

/// RUST_LOG 的有效全局档是否已是 trace（无 target 的裸级别指令里，最后一条生效）
///
/// 同 target 的后一条指令替换前一条，所以 `trace,info` 的有效档是 info，
/// 不能因为字符串里出现过 trace 就跳过追加 debug。
fn has_bare_trace_directive(rust_log: Option<&str>) -> bool {
    const LEVELS: [&str; 6] = ["trace", "debug", "info", "warn", "error", "off"];
    rust_log
        .and_then(|v| {
            v.split(',')
                .map(str::trim)
                .rfind(|part| LEVELS.iter().any(|l| part.eq_ignore_ascii_case(l)))
        })
        .is_some_and(|part| part.eq_ignore_ascii_case("trace"))
}

/// 创建可重载过滤器并返回句柄（由 main 装配到 registry）
pub fn new_reload_layer(
    debug: bool,
) -> (
    reload::Layer<EnvFilter, tracing_subscriber::Registry>,
    FilterHandle,
) {
    DEBUG_ENABLED.store(debug, Ordering::Relaxed);
    reload::Layer::new(build_filter(debug))
}

/// 保存句柄（供运行时热切换）
pub fn install(handle: FilterHandle) {
    if HANDLE.set(handle).is_err() {
        tracing::warn!("全局日志过滤器句柄已安装，忽略重复安装");
    }
}

/// 热切换 DEBUG 级别；句柄未安装时返回错误（不得静默失败）
pub fn set_debug_enabled(enabled: bool) -> Result<(), String> {
    let Some(handle) = HANDLE.get() else {
        return Err("日志过滤器句柄尚未安装，无法热切换 DEBUG 级别".to_string());
    };
    let filter = build_filter(enabled);
    handle
        .modify(|current| *current = filter)
        .map_err(|e| format!("热切换日志级别失败: {}", e))?;
    DEBUG_ENABLED.store(enabled, Ordering::Relaxed);
    Ok(())
}

/// 当前 DEBUG 开关状态
pub fn debug_logs_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_filter_default_info_without_debug() {
        let filter = build_filter_from(None, false);
        let text = filter.to_string();
        assert!(text.contains("info"), "filter = {}", text);
    }

    #[test]
    fn test_build_filter_appends_debug() {
        let filter = build_filter_from(None, true);
        let text = filter.to_string();
        assert!(text.contains("debug"), "filter = {}", text);
    }

    #[test]
    fn test_build_filter_keeps_bare_trace_when_debug() {
        // RUST_LOG=trace 时开启开关不得把全局档降为 debug（同 target 替换语义）
        let text = build_filter_from(Some("trace"), true).to_string();
        assert!(text.contains("trace"), "filter = {}", text);
        assert!(!text.contains("debug"), "filter = {}", text);
    }

    #[test]
    fn test_build_filter_appends_debug_with_target_directives() {
        // target 定向指令与追加的全局 debug 共存，互不替换
        let text = build_filter_from(Some("kiro2cc_proxy::kiro=trace"), true).to_string();
        assert!(
            text.contains("debug") && text.contains("trace"),
            "filter = {}",
            text
        );
    }

    #[test]
    fn test_has_bare_trace_directive() {
        // 混合指令 / 大小写 / 首尾空白 → 命中；target 指令 / 前缀反例 / 空值 → 不命中
        assert!(has_bare_trace_directive(Some("trace,kiro=info")));
        assert!(has_bare_trace_directive(Some("TRACE")));
        assert!(has_bare_trace_directive(Some(" trace ")));
        assert!(!has_bare_trace_directive(Some("kiro=trace")));
        assert!(!has_bare_trace_directive(Some("trace_utils=info")));
        assert!(!has_bare_trace_directive(Some("")));
        assert!(!has_bare_trace_directive(None));
        // 同 target 后者覆盖前者：有效全局档以最后一条裸级别指令为准
        assert!(!has_bare_trace_directive(Some("trace,info")));
        assert!(!has_bare_trace_directive(Some("trace,off")));
        assert!(has_bare_trace_directive(Some("info,trace")));
        assert!(has_bare_trace_directive(Some("info,trace,kiro=debug")));
    }

    #[test]
    fn test_build_filter_appends_debug_when_trace_is_overridden() {
        // RUST_LOG=trace,info 的有效档是 info，开关开启后必须能输出 DEBUG
        let text = build_filter_from(Some("trace,info"), true).to_string();
        assert!(text.contains("debug"), "filter = {}", text);
    }

    #[test]
    fn test_set_debug_enabled_handle_lifecycle() {
        // 单测试内先验「句柄未安装报错」再 install 验成功路径：
        // cargo test 默认并行共享进程级 HANDLE（OnceLock），拆成两函数时
        // 成功路径先 install 会让错误路径的 is_none() 守卫恒 false 而空转
        if HANDLE.get().is_none() {
            assert!(set_debug_enabled(true).is_err());
        }
        let (_layer, handle) = new_reload_layer(false);
        install(handle);
        assert!(set_debug_enabled(true).is_ok());
        assert!(debug_logs_enabled());
        assert!(set_debug_enabled(false).is_ok());
        assert!(!debug_logs_enabled());
    }
}
