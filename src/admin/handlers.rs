// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! Admin API HTTP 处理器

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use super::{
    middleware::AdminState,
    types::{
        AddCredentialRequest, SetDisabledRequest, SetLoadBalancingModeRequest, SetPriorityRequest,
        SuccessResponse, UpdateCredentialRequest,
    },
};

/// GET /api/admin/credentials
/// 获取所有账号状态
pub async fn get_all_credentials(State(state): State<AdminState>) -> impl IntoResponse {
    let response = state.service.get_all_credentials();
    Json(response)
}

/// POST /api/admin/credentials/:id/disabled
/// 设置账号禁用状态
pub async fn set_credential_disabled(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<SetDisabledRequest>,
) -> impl IntoResponse {
    match state.service.set_disabled(id, payload.disabled).await {
        Ok(_) => {
            let action = if payload.disabled { "禁用" } else { "启用" };
            Json(SuccessResponse::new(format!("账号 #{} 已{}", id, action))).into_response()
        }
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/:id/priority
/// 设置账号优先级
pub async fn set_credential_priority(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<SetPriorityRequest>,
) -> impl IntoResponse {
    match state.service.set_priority(id, payload.priority) {
        Ok(_) => Json(SuccessResponse::new(format!(
            "账号 #{} 优先级已设置为 {}",
            id, payload.priority
        )))
        .into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/:id/reset
/// 重置失败计数并重新启用
pub async fn reset_failure_count(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.reset_and_enable(id).await {
        Ok(_) => Json(SuccessResponse::new(format!(
            "账号 #{} 失败计数已重置并重新启用",
            id
        )))
        .into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// GET /api/admin/credentials/:id/balance
/// 获取指定账号的余额
pub async fn get_credential_balance(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.get_balance(id).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials
/// 添加新账号
pub async fn add_credential(
    State(state): State<AdminState>,
    Json(payload): Json<AddCredentialRequest>,
) -> impl IntoResponse {
    match state.service.add_credential(payload).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// DELETE /api/admin/credentials/:id
/// 删除账号
pub async fn delete_credential(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.delete_credential(id).await {
        Ok(_) => Json(SuccessResponse::new(format!("账号 #{} 已删除", id))).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// PUT /api/admin/credentials/:id
/// 更新账号配置
pub async fn update_credential(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<UpdateCredentialRequest>,
) -> impl IntoResponse {
    match state.service.update_credential(id, payload).await {
        Ok(_) => Json(SuccessResponse::new(format!("账号 #{} 已更新", id))).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// GET /api/admin/config/load-balancing
/// 获取负载均衡模式
pub async fn get_load_balancing_mode(State(state): State<AdminState>) -> impl IntoResponse {
    let response = state.service.get_load_balancing_mode();
    Json(response)
}

/// PUT /api/admin/config/load-balancing
/// 设置负载均衡模式
pub async fn set_load_balancing_mode(
    State(state): State<AdminState>,
    Json(payload): Json<SetLoadBalancingModeRequest>,
) -> impl IntoResponse {
    match state.service.set_load_balancing_mode(payload) {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// 将 API Key 脱敏显示（保留前半部分 + ***）
fn mask_key(key: &str) -> String {
    let visible = key.chars().count() / 2;
    let masked: String = key.chars().take(visible).collect();
    format!("{}***", masked)
}

/// GET /api/admin/config/auth-keys
/// 获取当前认证密钥（脱敏显示）
pub async fn get_auth_keys(State(state): State<AdminState>) -> impl IntoResponse {
    let admin_psw = mask_key(&state.admin_psw.read());

    Json(super::types::AuthKeysResponse { admin_psw })
}

/// PUT /api/admin/config/auth-keys
/// 修改认证密钥（运行时生效并持久化到 config.json）
pub async fn set_auth_keys(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::SetAuthKeysRequest>,
) -> impl IntoResponse {
    // 验证输入
    if let Some(ref key) = payload.admin_psw
        && key.trim().is_empty()
    {
        let error = super::types::AdminErrorResponse::invalid_request(
            "adminPsw 不能为空（Admin Password）",
        );
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!(error)),
        )
            .into_response();
    }

    // 更新运行时值
    if let Some(ref new_admin_psw) = payload.admin_psw {
        *state.admin_psw.write() = new_admin_psw.clone();
    }

    // 持久化到 config.json（persist_lock 防止与其他 PUT 并发覆盖）
    let _guard = state.persist_lock.lock();
    if let Some(ref config_path) = state.config_path
        && let Err(e) = persist_auth_keys(config_path, &payload.admin_psw)
    {
        tracing::error!("持久化认证密钥失败: {}", e);
        let error = super::types::AdminErrorResponse::internal_error("持久化失败，但运行时已生效");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(error)),
        )
            .into_response();
    }

    Json(SuccessResponse::new("认证密钥已更新")).into_response()
}

/// GET /api/admin/config/suggestion-mode
/// 获取 Suggestion Mode 输入建议请求放行开关
pub async fn get_suggestion_mode(State(state): State<AdminState>) -> impl IntoResponse {
    let enabled = state
        .suggestion_mode
        .as_ref()
        .map(|f| f.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false);
    Json(super::types::SuggestionModeResponse { enabled })
}

/// PUT /api/admin/config/suggestion-mode
/// 设置 Suggestion Mode 输入建议请求放行开关（运行时热切换并持久化到 config.json）
pub async fn set_suggestion_mode(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::SetSuggestionModeRequest>,
) -> impl IntoResponse {
    let Some(flag) = &state.suggestion_mode else {
        let error = super::types::AdminErrorResponse::internal_error("Suggestion Mode 开关未启用");
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!(error)),
        )
            .into_response();
    };

    // 运行时热切换（与 anthropic AppState 共享同一 Arc 实例，即时生效）
    flag.store(payload.enabled, std::sync::atomic::Ordering::Relaxed);

    // 持久化到 config.json（重启后保持；persist_lock 防止与其他 PUT 并发覆盖）
    let _guard = state.persist_lock.lock();
    if let Some(ref config_path) = state.config_path
        && let Err(e) = persist_suggestion_mode(config_path, payload.enabled)
    {
        tracing::error!("持久化 Suggestion Mode 开关失败: {}", e);
        let error = super::types::AdminErrorResponse::internal_error("持久化失败，但运行时已生效");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(error)),
        )
            .into_response();
    }

    Json(SuccessResponse::new(if payload.enabled {
        "Suggestion Mode 已放行"
    } else {
        "Suggestion Mode 已拦截"
    }))
    .into_response()
}

/// GET /api/admin/config/client-token-passthrough
/// 获取客户端 token 直通开关
pub async fn get_client_token_passthrough() -> impl IntoResponse {
    Json(super::types::ClientTokenPassthroughResponse {
        enabled: crate::anthropic::client_token_passthrough_enabled(),
    })
}

/// PUT /api/admin/config/client-token-passthrough
/// 设置客户端 token 直通开关（运行时热切换并持久化到 config.json）
pub async fn set_client_token_passthrough(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::SetClientTokenPassthroughRequest>,
) -> impl IntoResponse {
    // 运行时热切换（calib 内 static AtomicBool，全局即时生效）
    crate::anthropic::set_client_token_passthrough(payload.enabled);

    // 持久化到 config.json（重启后保持；persist_lock 防止与其他 PUT 并发覆盖）
    let _guard = state.persist_lock.lock();
    if let Some(ref config_path) = state.config_path
        && let Err(e) = persist_client_token_passthrough(config_path, payload.enabled)
    {
        tracing::error!("持久化客户端 token 直通开关失败: {}", e);
        let error = super::types::AdminErrorResponse::internal_error("持久化失败，但运行时已生效");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(error)),
        )
            .into_response();
    }

    Json(SuccessResponse::new(if payload.enabled {
        "客户端 token 直通已开启"
    } else {
        "客户端 token 直通已关闭"
    }))
    .into_response()
}

/// 将 Suggestion Mode 开关写回 config.json
fn persist_suggestion_mode(config_path: &std::path::Path, enabled: bool) -> anyhow::Result<()> {
    persist_config_field(config_path, |json| {
        json["forwardSuggestionMode"] = serde_json::Value::Bool(enabled);
    })
}

/// 将客户端 token 直通开关写回 config.json
fn persist_client_token_passthrough(
    config_path: &std::path::Path,
    enabled: bool,
) -> anyhow::Result<()> {
    persist_config_field(config_path, |json| {
        json["clientTokenPassthrough"] = serde_json::Value::Bool(enabled);
    })
}

/// config.json 通用读改写：修改字段后经临时文件 + rename 原子替换，
/// 避免进程在写入中途崩溃产生半截文件。调用方须先持有 `AdminState.persist_lock`。
fn persist_config_field(
    config_path: &std::path::Path,
    mutate: impl FnOnce(&mut serde_json::Value),
) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(config_path)?;
    let mut json: serde_json::Value = serde_json::from_str(&content)?;
    mutate(&mut json);

    let output = serde_json::to_string_pretty(&json)?;
    let tmp_path = config_path.with_extension("json.tmp");
    std::fs::write(&tmp_path, output)?;
    std::fs::rename(&tmp_path, config_path)?;
    Ok(())
}

/// 将修改后的密钥写回 config.json
fn persist_auth_keys(
    config_path: &std::path::Path,
    new_admin_psw: &Option<String>,
) -> anyhow::Result<()> {
    persist_config_field(config_path, |json| {
        if let Some(key) = new_admin_psw {
            json["adminPsw"] = serde_json::Value::String(key.clone());
            if let Some(map) = json.as_object_mut() {
                map.remove("adminApiKey");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个测试用独立子目录，避免并行测试共享同一 temp 目录互删
    fn temp_config_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("kiro-admin-test-{}-{}", tag, std::process::id()))
    }

    #[test]
    fn test_persist_suggestion_mode_roundtrip() {
        let dir = temp_config_dir("sugg");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, r#"{"host":"127.0.0.1","port":8080}"#).unwrap();

        persist_suggestion_mode(&path, true).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(json["forwardSuggestionMode"], serde_json::Value::Bool(true));
        // 原有字段不被破坏
        assert_eq!(json["host"], "127.0.0.1");

        persist_suggestion_mode(&path, false).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            json["forwardSuggestionMode"],
            serde_json::Value::Bool(false)
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_persist_client_token_passthrough_roundtrip() {
        let dir = temp_config_dir("ctp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, r#"{"host":"127.0.0.1"}"#).unwrap();

        persist_client_token_passthrough(&path, true).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            json["clientTokenPassthrough"],
            serde_json::Value::Bool(true)
        );
        assert_eq!(json["host"], "127.0.0.1");

        persist_client_token_passthrough(&path, false).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            json["clientTokenPassthrough"],
            serde_json::Value::Bool(false)
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_persist_replaces_tmp_file_atomically() {
        let dir = temp_config_dir("tmp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, r#"{"a":1}"#).unwrap();

        persist_config_field(&path, |json| {
            json["a"] = serde_json::json!(2);
        })
        .unwrap();

        // 原子替换后临时文件应不存在，目标文件内容为新值
        assert!(!path.with_extension("json.tmp").exists());
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(json["a"], serde_json::json!(2));

        std::fs::remove_dir_all(&dir).ok();
    }
}

/// 单批次最多允许查询的 IP 数量
const MAX_GEO_BATCH_IPS: usize = 200;

/// GET /api/admin/geo/batch?ips=ip1,ip2,...
/// 批量查询 IP 归属地
pub async fn get_geo_batch(
    State(state): State<AdminState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let Some(resolver) = &state.geo_resolver else {
        let error = super::types::AdminErrorResponse::internal_error("归属地解析未启用");
        return (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(error)).into_response();
    };
    let ips: Vec<&str> = params
        .get("ips")
        .map(|v| v.split(',').filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    if ips.len() > MAX_GEO_BATCH_IPS {
        let error = super::types::AdminErrorResponse::invalid_request(format!(
            "单批次最多查询 {MAX_GEO_BATCH_IPS} 个 IP"
        ));
        return (axum::http::StatusCode::BAD_REQUEST, Json(error)).into_response();
    }
    let result: std::collections::HashMap<String, Option<crate::model::geo::GeoInfo>> = ips
        .into_iter()
        .map(|ip| (ip.to_string(), resolver.resolve(ip)))
        .collect();
    Json(result).into_response()
}
