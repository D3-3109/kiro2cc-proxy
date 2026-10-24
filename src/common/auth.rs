// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 通用认证工具：从请求头提取 API Key，constant_time_eq 恒定时间比较防时序攻击。

use axum::{
    body::Body,
    http::{HeaderMap, Request, header},
};
use subtle::ConstantTimeEq;

/// 从请求中提取 API Key
///
/// 支持两种认证方式：
/// - `x-api-key` header
/// - `Authorization: Bearer <token>` header
pub fn extract_api_key(request: &Request<Body>) -> Option<String> {
    extract_api_key_from_headers(request.headers()).map(str::to_string)
}

/// 从请求头提取 API Key（`x-api-key` 优先，其次 `Authorization: Bearer`）。
/// 供拿不到完整 `Request` 的 handler（只有 `HeaderMap` 提取器）复用同一口径。
pub fn extract_api_key_from_headers(headers: &HeaderMap) -> Option<&str> {
    // 优先检查 x-api-key
    if let Some(key) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        return Some(key);
    }

    // 其次检查 Authorization: Bearer
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// 常量时间字符串比较，防止时序攻击
///
/// 无论字符串内容如何，比较所需的时间都是恒定的，
/// 这可以防止攻击者通过测量响应时间来猜测 API Key。
///
/// 使用经过安全审计的 `subtle` crate 实现
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}
