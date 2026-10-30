//! 重试与故障转移：端点选择（select_endpoint）与 call_api_with_retry 多账号多轮重试。
use tokio::time::sleep;

use crate::kiro::endpoint::{BUCKET_THROTTLE_DURATION, Endpoint, EndpointName};
use crate::kiro::model::credentials::KiroCredentials;

use super::SignatureRepair;
use super::core::{KiroProvider, MAX_RETRIES_PER_CREDENTIAL, MAX_TOTAL_RETRIES};

/// 一次请求内逐个删除坏签名的上限；仍被拒则删除全部历史推理
///
/// 坏签名通常成片出现（会话中途从别的服务切过来），单次请求只认领这么多，
/// 余下的由后续请求继续认领（认领结果由 handler 写入拒绝缓存）。
const MAX_TARGETED_SIGNATURE_STRIPS: usize = 4;

/// 签名被拒后逐个删除坏签名的进度
///
/// 上游每次只点名一个坏签名（`messages.N`）。删掉 `history[N]` 的推理后，下一次
/// 响应成功或点名别的位置，才能确认 N 确实是坏签名；仍点名 N 说明下标映射不符、
/// 删错了，此时放弃逐个删除，改为全部删除，且删错的那个不计入坏签名。
#[derive(Default)]
struct SignatureRepairState {
    /// 删改后的请求体（None = 原始请求体）
    body: Option<String>,
    /// 最近一次删除的 (history 下标, 签名)，待下一次响应确认
    unconfirmed: Option<(usize, String)>,
    outcome: SignatureRepair,
}

impl SignatureRepairState {
    /// 又一次被拒：确认上一次删除，再删下一个；返回 false 表示已无可删
    ///
    /// `history_offset`：本次发出前 rewrite 从 history 头部删掉的条数，用于把上游
    /// 点名的下标换算回 `self.body` 的下标。
    fn advance(&mut self, original_body: &str, error_body: &str, history_offset: usize) -> bool {
        if self.outcome.stripped_all {
            return false;
        }

        let rejected_index =
            KiroProvider::rejected_history_index(error_body).map(|index| index + history_offset);
        let mapping_holds = match self.unconfirmed.take() {
            Some((index, _)) if rejected_index == Some(index) => false,
            Some((_, signature)) => {
                self.outcome.rejected.push(signature);
                true
            }
            None => true,
        };

        let current = self.body.as_deref().unwrap_or(original_body);
        let within_budget = self.outcome.rejected.len() < MAX_TARGETED_SIGNATURE_STRIPS;
        let targeted = rejected_index
            .filter(|_| mapping_holds && within_budget)
            .and_then(|index| {
                KiroProvider::strip_history_reasoning_at(current, index)
                    .map(|(body, signature)| (index, body, signature))
            });
        if let Some((index, body, signature)) = targeted {
            self.body = Some(body);
            self.unconfirmed = Some((index, signature));
            return true;
        }

        let Some(body) = KiroProvider::strip_history_reasoning(current) else {
            return false;
        };
        self.body = Some(body);
        self.outcome.stripped_all = true;
        true
    }

    /// 请求成功：最近一次删除随之确认
    fn finish(mut self) -> SignatureRepair {
        if let Some((_, signature)) = self.unconfirmed.take() {
            self.outcome.rejected.push(signature);
        }
        self.outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 第 1..=n 轮 assistant 各带签名 s1..sn，位于 history 的奇数下标
    fn body_with_signatures(n: usize) -> String {
        let history: Vec<serde_json::Value> = (1..=n)
            .flat_map(|i| {
                [
                    serde_json::json!({"userInputMessage": {"content": format!("q{i}")}}),
                    serde_json::json!({"assistantResponseMessage": {"content": format!("a{i}"),
                        "reasoningContent": {"reasoningText": {"text": "", "signature": format!("s{i}")}}}}),
                ]
            })
            .collect();
        serde_json::json!({"conversationState": {"history": history}}).to_string()
    }

    fn rejection_at(index: usize) -> String {
        format!(
            r#"{{"message":"messages.{index}.content.0: Invalid `signature` in `thinking` block","reason":"THINKING_SIGNATURE_INVALID"}}"#
        )
    }

    fn remaining_signatures(state: &SignatureRepairState) -> Vec<String> {
        let body: serde_json::Value = serde_json::from_str(state.body.as_ref().unwrap()).unwrap();
        body["conversationState"]["history"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|m| {
                m.pointer("/assistantResponseMessage/reasoningContent/reasoningText/signature")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .collect()
    }

    #[test]
    fn named_signatures_are_stripped_one_by_one_and_confirmed() {
        let original = body_with_signatures(3);
        let mut state = SignatureRepairState::default();

        assert!(state.advance(&original, &rejection_at(1), 0));
        assert_eq!(remaining_signatures(&state), ["s2", "s3"]);
        // 上游转而点名下一处：s1 确认是坏签名
        assert!(state.advance(&original, &rejection_at(5), 0));
        assert_eq!(remaining_signatures(&state), ["s2"]);

        let outcome = state.finish();
        assert_eq!(outcome.rejected, ["s1", "s3"]);
        assert!(!outcome.stripped_all);
    }

    #[test]
    fn same_index_named_again_falls_back_to_stripping_all() {
        // 删掉 history[1] 后上游仍点名 1：映射不符，删错的 s1 不算坏签名
        let original = body_with_signatures(2);
        let mut state = SignatureRepairState::default();
        assert!(state.advance(&original, &rejection_at(1), 0));
        assert!(state.advance(&original, &rejection_at(1), 0));
        assert!(remaining_signatures(&state).is_empty());

        let outcome = state.finish();
        assert!(outcome.rejected.is_empty());
        assert!(outcome.stripped_all);
    }

    #[test]
    fn unparsable_rejection_strips_all_and_then_gives_up() {
        let original = body_with_signatures(2);
        let mut state = SignatureRepairState::default();
        assert!(state.advance(&original, r#"{"reason":"THINKING_SIGNATURE_INVALID"}"#, 0));
        assert!(state.outcome.stripped_all);
        // 全部删除后仍被拒：无可再删，交给普通 400 处理
        assert!(!state.advance(&original, &rejection_at(1), 0));
    }

    #[test]
    fn upstream_index_is_shifted_by_rewrite_offset() {
        // 发出前删掉了 2 条前缀：上游点名 1，对应改写前的 history[3]
        let original = body_with_signatures(2);
        let mut state = SignatureRepairState::default();
        assert!(state.advance(&original, &rejection_at(1), 2));
        assert_eq!(remaining_signatures(&state), ["s1"]);
        assert_eq!(state.finish().rejected, ["s2"]);
    }

    #[test]
    fn targeted_strips_stop_at_budget() {
        let original = body_with_signatures(MAX_TARGETED_SIGNATURE_STRIPS + 2);
        let mut state = SignatureRepairState::default();
        for round in 0..=MAX_TARGETED_SIGNATURE_STRIPS {
            assert!(state.advance(&original, &rejection_at(2 * round + 1), 0));
        }
        let outcome = state.finish();
        assert_eq!(outcome.rejected.len(), MAX_TARGETED_SIGNATURE_STRIPS);
        assert!(outcome.stripped_all);
    }
}

impl KiroProvider {
    pub(crate) fn select_endpoint(
        &self,
        credentials: &KiroCredentials,
        attempt: usize,
    ) -> Option<Endpoint> {
        let region = credentials.effective_api_region(self.token_manager.config());
        let endpoints = credentials.effective_endpoints(region);
        if endpoints.is_empty() {
            return None;
        }
        let len = endpoints.len();
        // 起点 attempt 偏移（attempt % len），轮询保证单账号内多次 attempt 走不同端点
        let start = attempt % len;
        for i in 0..len {
            let candidate = &endpoints[(start + i) % len];
            if !self
                .endpoint_registry
                .is_throttled(credentials.id.unwrap_or(0), candidate.name)
            {
                return Some(candidate.clone());
            }
        }
        None
    }

    /// 内部方法：带重试逻辑的 API 调用
    ///
    /// 重试策略：
    /// - 每个账号最多重试 MAX_RETRIES_PER_CREDENTIAL 次
    /// - 总重试次数 = min(可用池大小 × 每账号重试次数, MAX_TOTAL_RETRIES)
    /// - 硬上限 9 次，避免无限重试
    /// - 当可用池 ≤ 1 时，仅重试 3 次并使用更长退避间隔
    /// - 单账号内 3 次 attempts 之间切换多端点（不消耗切账号配额）
    pub(crate) async fn call_api_with_retry(
        &self,
        request_body: &str,
        is_stream: bool,
        is_compact: bool,
        thinking_adaptive_requested: bool,
        bound_ids: &[u64],
    ) -> anyhow::Result<super::ApiResponse> {
        // 模型感知路由：确保各账号的实际可用模型探测结果是新的。
        // 探测是全局共享的（缓存 + 串行锁），放在并发 permit 之前，
        // 避免冷启动时大量请求占着额度排队等探测
        let model = Self::extract_model_from_request(request_body);
        self.token_manager
            .refresh_model_support_if_stale(model.as_deref(), bound_ids)
            .await;

        let _permit = self.concurrency_limit.acquire().await?;
        // 流式请求的响应体在上游 200 headers 返回后仍持续生成，受 reqwest
        // 总超时约束，统一使用 `UPSTREAM_TIMEOUT_SECS` 长超时档；若使用较短
        // 档位会在长生成中途被掐断（历史分档背景见常量注释）
        let use_long_timeout = is_stream || is_compact;
        let effective_pool = if bound_ids.is_empty() {
            self.token_manager.total_count()
        } else {
            bound_ids.len()
        };
        let max_retries = (effective_pool * MAX_RETRIES_PER_CREDENTIAL).min(MAX_TOTAL_RETRIES);
        let small_pool = effective_pool <= 1;
        let mut last_error: Option<anyhow::Error> = None;
        let api_type = if is_stream { "流式" } else { "非流式" };

        // 尝试从请求体中提取会话 ID（模型信息已在上方提取）
        let continuation_id = Self::extract_continuation_id_from_request(request_body);
        // 本次请求内已限流的账号：重试时避开，但不销毁 sticky 绑定
        let mut throttled_in_request: Vec<u64> = Vec::new();
        // 历史推理签名被拒后的请求体删改
        let mut signature_repair = SignatureRepairState::default();
        // 删改签名后的重试属请求内容问题，不占故障转移配额：每删改一次放宽一次上限
        let mut attempt_limit = max_retries;

        for attempt in 0.. {
            if attempt >= attempt_limit {
                break;
            }
            // 获取调用上下文（优先路由到同一会话的缓存账号）
            let ctx = match self
                .token_manager
                .acquire_context_sticky(
                    model.as_deref(),
                    bound_ids,
                    continuation_id.as_deref(),
                    &throttled_in_request,
                )
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            };

            // RPM 硬限制：精确等待或多账号时 skip（先于并发 permit 获取，避免等待期间占用账号级并发槛位）
            let rpm_ok = self.wait_for_rpm_gate(ctx.id, "").await;
            if !rpm_ok && !small_pool {
                tracing::info!("[RPM-GATE] credential={} RPM 满，跳过切换下一账号", ctx.id);
                self.token_manager.report_throttled_for_rotation(ctx.id);
                if let Some(cid) = continuation_id.as_deref() {
                    self.token_manager.report_sticky_throttled(cid, ctx.id);
                }
                if !throttled_in_request.contains(&ctx.id) {
                    throttled_in_request.push(ctx.id);
                }
                last_error = Some(anyhow::anyhow!(
                    "RPM limit exceeded for credential {}",
                    ctx.id
                ));
                continue;
            }

            // 获取单账号并发 permit（非 sleep 的 continue/Err 分支依赖 _cred_permit 随作用域结束隐式释放；
            // 仅进入 sleep 退避的分支需要显式 drop，以便退避等待期间归还槛位）
            let _cred_permit = self.semaphore_for(ctx.id).acquire_owned().await?;

            // 选择下一个可用端点（单账号内轮询，跳过被封禁桶）
            let endpoint = match self.select_endpoint(&ctx.credentials, attempt) {
                Some(e) => e,
                None => {
                    // 4 桶全封：该账号本次请求内已无可用端点。
                    // 登记避让后继续重试其它账号，仅在所有账号都走到这一步时才失败，
                    // 避免多账号池里因单账号端点全封而直接返回 502。
                    let endpoints: Vec<EndpointName> = ctx
                        .credentials
                        .effective_endpoints(
                            ctx.credentials
                                .effective_api_region(self.token_manager.config()),
                        )
                        .iter()
                        .map(|e| e.name)
                        .collect();
                    let ids = endpoints
                        .iter()
                        .map(|n| n.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    tracing::info!(
                        "[ENDPOINT] credential={} 端点全封（{}），避让并尝试其它账号",
                        ctx.id,
                        ids
                    );
                    if !throttled_in_request.contains(&ctx.id) {
                        throttled_in_request.push(ctx.id);
                    }
                    last_error = Some(anyhow::anyhow!(
                        "All endpoints throttled for credential {} (tried: [{}])",
                        ctx.id,
                        ids
                    ));
                    // 所有候选账号都已端点全封时立刻换号是空转，需要退避等待桶解封。
                    // 单账号池同理。桶封禁窗口固定，退避比连续空转更快拿到可用端点。
                    let all_avoided = self
                        .token_manager
                        .credential_ids()
                        .iter()
                        .all(|id| throttled_in_request.contains(id));
                    if (small_pool || all_avoided) && attempt + 1 < max_retries {
                        drop(_cred_permit);
                        sleep(Self::throttle_delay(attempt)).await;
                    }
                    continue;
                }
            };
            tracing::debug!(
                "[ENDPOINT] credential={} attempt={} selected={:?} host={}",
                ctx.id,
                attempt + 1,
                endpoint.name,
                endpoint.host
            );

            let url = self.base_url_for(&ctx.credentials, &endpoint);
            let current_body = signature_repair.body.as_deref().unwrap_or(request_body);
            let history_offset = Self::rewritten_history_offset(current_body, &ctx.credentials);
            let effective_body = Self::rewrite_request_body(
                current_body,
                &ctx.credentials,
                thinking_adaptive_requested,
            );
            let headers = match self.build_headers(&ctx, &effective_body, attempt, &endpoint) {
                Ok(h) => h,
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            };

            // 发送请求
            let client = match self.client_for(&ctx.credentials, use_long_timeout) {
                Ok(c) => c,
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            };
            // 请求体可能含敏感上下文，禁止整包入日志，只记录 URL 与长度（cr-result C6）
            tracing::debug!(
                "[KIRO-REQUEST] url={} body_len={}",
                url,
                effective_body.len()
            );
            let response = match client
                .post(&url)
                .headers(headers)
                .body(effective_body)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::warn!(
                        "API 请求发送失败（尝试 {}/{}）: {}",
                        attempt + 1,
                        max_retries,
                        e
                    );
                    last_error = Some(e.into());
                    if attempt + 1 < max_retries {
                        drop(_cred_permit);
                        sleep(Self::retry_delay(attempt)).await;
                    }
                    continue;
                }
            };

            let status = response.status();

            // 成功响应
            if status.is_success() {
                self.token_manager.report_success(ctx.id);
                let credential_in_flight = self
                    .rpm_tracker
                    .as_ref()
                    .map(|rpm| rpm.begin_credential_request(ctx.id));
                return Ok(super::ApiResponse {
                    response,
                    credential_id: ctx.id,
                    signature_repair: signature_repair.finish(),
                    credential_in_flight,
                });
            }

            // 失败响应：读取 body 用于日志/错误信息
            let body = response.text().await.unwrap_or_default();

            // 402 Payment Required 且额度用尽：禁用账号并故障转移
            if status.as_u16() == 402 && Self::is_monthly_request_limit(&body) {
                tracing::warn!(
                    "API 请求失败（额度已用尽，禁用账号并切换，尝试 {}/{}）: {} {}",
                    attempt + 1,
                    max_retries,
                    status,
                    body
                );

                let has_available = self.token_manager.report_quota_exhausted(ctx.id);
                if !has_available {
                    // 全局无可用账号：必须走 describe_unavailable 产出带
                    // QUOTA_EXHAUSTED_ALL_MARKER 的文案，否则这条错误串在
                    // handlers.rs 会被误判进 502 兜底，而非正确的 402。
                    anyhow::bail!(
                        "{}",
                        self.token_manager
                            .describe_unavailable(model.as_deref(), bound_ids)
                    );
                }

                last_error = Some(anyhow::anyhow!(
                    "{} API 请求失败: {} {}",
                    api_type,
                    status,
                    body
                ));
                continue;
            }

            // 400 Bad Request - 请求问题，重试/切换账号无意义；
            // 例外：企业 IdC 账号缺失 profileArn 属确定性配置缺陷——首即禁用
            // （ProfileArnMissing，不做失败计数）并故障转移到其他账号
            if status.as_u16() == 400 {
                if Self::is_profile_arn_required_error(&body) {
                    tracing::warn!(
                        "API 请求失败（账号缺少 profileArn，尝试 {}/{}）: {} {}",
                        attempt + 1,
                        max_retries,
                        status,
                        body
                    );
                    // 确定性配置缺陷：首即禁用（ProfileArnMissing），不做失败计数
                    let has_available = self.token_manager.report_profile_arn_missing(ctx.id);
                    if let Some(ref store) = self.failure_log_store {
                        store.record(ctx.id, "api", status.as_u16(), &body);
                    }
                    if !has_available {
                        anyhow::bail!(
                            "{} API 请求失败（所有账号已用尽）: {} {}",
                            api_type,
                            status,
                            body
                        );
                    }
                    last_error = Some(anyhow::anyhow!(
                        "{} API 请求失败: {} {}",
                        api_type,
                        status,
                        body
                    ));
                    continue;
                }
                // 历史推理签名无效（签名损坏或来自其他服务）：删掉被点名的签名立即重试。
                // 属请求内容问题，不计入账号失败、不退避；模型只是看不到那一轮的思考。
                if Self::is_thinking_signature_invalid(&body)
                    && signature_repair.advance(request_body, &body, history_offset)
                {
                    tracing::warn!(
                        stripped_all = signature_repair.outcome.stripped_all,
                        "历史推理签名被上游拒绝，删除后重试: {}",
                        body
                    );
                    attempt_limit += 1;
                    last_error = Some(anyhow::anyhow!(
                        "{} API 请求失败: {} {}",
                        api_type,
                        status,
                        body
                    ));
                    continue;
                }
                anyhow::bail!("{} API 请求失败: {} {}", api_type, status, body);
            }

            // 401/403 - 更可能是账号/权限问题：计入失败并允许故障转移
            if matches!(status.as_u16(), 401 | 403) {
                tracing::warn!(
                    "API 请求失败（可能为账号错误，尝试 {}/{}）: {} {}",
                    attempt + 1,
                    max_retries,
                    status,
                    body
                );

                let has_available = self.token_manager.report_failure(ctx.id);
                if let Some(ref store) = self.failure_log_store {
                    store.record(ctx.id, "api", status.as_u16(), &body);
                }
                if !has_available {
                    anyhow::bail!(
                        "{} API 请求失败（所有账号已用尽）: {} {}",
                        api_type,
                        status,
                        body
                    );
                }

                last_error = Some(anyhow::anyhow!(
                    "{} API 请求失败: {} {}",
                    api_type,
                    status,
                    body
                ));
                continue;
            }

            // 429 Too Many Requests - 限流：驱逐 sticky cache + rotation bias 轮转 + 封禁当前端点桶
            if status.as_u16() == 429 {
                tracing::warn!(
                    "API 请求失败（上游限流，{}重试，尝试 {}/{}）: {} {}",
                    if small_pool {
                        "单账号延长间隔"
                    } else {
                        "切换账号"
                    },
                    attempt + 1,
                    max_retries,
                    status,
                    body
                );
                self.token_manager.report_throttled(ctx.id);
                self.token_manager.report_throttled_for_rotation(ctx.id);
                // 端点级：封禁当前命中桶 30s（不动其它桶，不动账号）
                self.endpoint_registry
                    .throttle(ctx.id, endpoint.name, BUCKET_THROTTLE_DURATION);
                if let Some(cid) = continuation_id.as_deref() {
                    self.token_manager.report_sticky_throttled(cid, ctx.id);
                }
                if !throttled_in_request.contains(&ctx.id) {
                    throttled_in_request.push(ctx.id);
                }
                if let Some(ref store) = self.throttle_log_store {
                    store.record(
                        ctx.id,
                        "api",
                        status.as_u16(),
                        &body,
                        Some(endpoint.name.as_str()),
                    );
                }
                last_error = Some(anyhow::anyhow!(
                    "{} API 请求失败: {} {}",
                    api_type,
                    status,
                    body
                ));
                if attempt + 1 < max_retries {
                    let delay = Self::throttle_delay(attempt);
                    drop(_cred_permit);
                    sleep(delay).await;
                }
                continue;
            }

            // 408/5xx - 瞬态上游错误：重试但不禁用或切换账号
            // （避免 502 high load 等瞬态错误把所有账号锁死）
            if status.as_u16() == 408 || status.is_server_error() {
                tracing::warn!(
                    "API 请求失败（上游瞬态错误，尝试 {}/{}）: {} {}",
                    attempt + 1,
                    max_retries,
                    status,
                    body
                );
                last_error = Some(anyhow::anyhow!(
                    "{} API 请求失败: {} {}",
                    api_type,
                    status,
                    body
                ));
                if attempt + 1 < max_retries {
                    drop(_cred_permit);
                    sleep(Self::retry_delay(attempt)).await;
                }
                continue;
            }

            // 其他 4xx - 通常为请求/配置问题：直接返回，不计入账号失败
            if status.is_client_error() {
                anyhow::bail!("{} API 请求失败: {} {}", api_type, status, body);
            }

            // 兜底：当作可重试的瞬态错误处理（不切换账号）
            tracing::warn!(
                "API 请求失败（未知错误，尝试 {}/{}）: {} {}",
                attempt + 1,
                max_retries,
                status,
                body
            );
            last_error = Some(anyhow::anyhow!(
                "{} API 请求失败: {} {}",
                api_type,
                status,
                body
            ));
            if attempt + 1 < max_retries {
                drop(_cred_permit);
                sleep(Self::retry_delay(attempt)).await;
            }
        }

        // 所有重试都失败
        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "{} API 请求失败：已达到最大重试次数（{}次）",
                api_type,
                max_retries
            )
        }))
    }
}
