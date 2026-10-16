//! 用量追踪模块测试（原 usage.rs 内联测试搬移）

#[cfg(test)]
mod tests {

    use crate::model::usage::{RecordOptions, UsageTracker, calculate_cost, get_k_ref};
    use std::collections::HashMap;

    /// 每个测试使用独立目录隔离明细文件与累计基数文件（api_key_lifetime.json
    /// 固定落在明细同目录，仅隔离文件名会让所有测试共享/互踩同一累计文件）
    fn temp_usage_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kiro2cc_usage_test_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("api_key_usage.json")
    }

    /// 清理测试独立目录（明细与累计基数同目录）
    fn cleanup_usage_path(path: &std::path::Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[tokio::test]
    async fn test_get_total_credits_uses_real_credits_when_present() {
        let path = temp_usage_path("credits_present");
        let tracker = UsageTracker::load(&path).unwrap();
        // credits_used 显式提供时，应直接累加该值而非回退估算
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            1000,
            100,
            RecordOptions {
                credits_used: Some(3.43),
                ..Default::default()
            },
        );
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            1000,
            100,
            RecordOptions {
                credits_used: Some(1.0),
                ..Default::default()
            },
        );
        assert!((tracker.get_total_credits(1) - 4.43).abs() < 1e-9);
        cleanup_usage_path(&path);
    }

    #[tokio::test]
    async fn test_get_total_credits_falls_back_to_estimated_cost_times_k_ref() {
        let path = temp_usage_path("credits_fallback");
        let tracker = UsageTracker::load(&path).unwrap();
        // 旧记录无 credits_used 时按 estimated_cost * k_ref 回退估算
        tracker.record(
            1,
            "claude-sonnet-4.5".to_string(),
            1_000_000,
            0,
            RecordOptions::default(),
        );
        let expected_cost = calculate_cost("claude-sonnet-4.5", 1_000_000, 0);
        let expected_credits = expected_cost * get_k_ref("claude-sonnet-4.5");
        assert!((tracker.get_total_credits(1) - expected_credits).abs() < 1e-9);
        cleanup_usage_path(&path);
    }

    #[tokio::test]
    async fn test_get_total_credits_only_counts_matching_api_key() {
        let path = temp_usage_path("credits_scoped");
        let tracker = UsageTracker::load(&path).unwrap();
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            100,
            10,
            RecordOptions {
                credits_used: Some(5.0),
                ..Default::default()
            },
        );
        tracker.record(
            2,
            "claude-opus-4.6".to_string(),
            100,
            10,
            RecordOptions {
                credits_used: Some(99.0),
                ..Default::default()
            },
        );
        assert!((tracker.get_total_credits(1) - 5.0).abs() < 1e-9);
        assert!((tracker.get_total_credits(2) - 99.0).abs() < 1e-9);
        assert_eq!(tracker.get_total_credits(3), 0.0);
        cleanup_usage_path(&path);
    }

    #[tokio::test]
    async fn test_get_summary_total_credits_matches_get_total_credits() {
        let path = temp_usage_path("summary_credits");
        let tracker = UsageTracker::load(&path).unwrap();
        tracker.record(
            1,
            "claude-opus-4.8".to_string(),
            500,
            50,
            RecordOptions {
                credits_used: Some(2.5),
                ..Default::default()
            },
        );
        let summary = tracker.get_summary(1);
        assert!((summary.total_credits - tracker.get_total_credits(1)).abs() < 1e-9);
        cleanup_usage_path(&path);
    }

    #[tokio::test]
    async fn test_record_effort_persisted_and_none_fallback() {
        let path = temp_usage_path("effort_field");
        let tracker = UsageTracker::load(&path).unwrap();
        tracker.record(
            1,
            "claude-opus-4.8".to_string(),
            100,
            10,
            RecordOptions {
                effort: Some("xhigh".to_string()),
                ..Default::default()
            },
        );
        tracker.record(
            1,
            "claude-sonnet-4.5".to_string(),
            100,
            10,
            RecordOptions::default(),
        );
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            100,
            10,
            RecordOptions {
                credential_id: Some(7),
                first_token_ms: Some(1234),
                duration_ms: Some(5678),
                ..Default::default()
            },
        );
        let page = tracker.get_records_paged(1, 1, 10, &HashMap::new());
        assert_eq!(page.records.len(), 3);
        // 有 effort 的记录映射到 UsageRecordItem
        assert!(
            page.records
                .iter()
                .any(|r| r.effort.as_deref() == Some("xhigh"))
        );
        // None 兜底：未传 output_config 的旧请求 effort 保持 None
        assert!(page.records.iter().any(|r| r.effort.is_none()));
        // 带耗时的记录：record() 入参 → 分页 DTO 的 Some 值透传（按值 find 定位，
        // 不依赖 newest-first 排序在纳秒同刻并列时的稳定性）
        let timed = page
            .records
            .iter()
            .find(|r| r.first_token_ms == Some(1234))
            .expect("带耗时的记录应在分页结果中");
        assert_eq!(timed.duration_ms, Some(5678));
        // 未传耗时的旧记录兜底为 None（前两条 effort 测试记录均无耗时）
        assert!(
            page.records[1..3]
                .iter()
                .all(|r| r.first_token_ms.is_none() && r.duration_ms.is_none())
        );
        cleanup_usage_path(&path);
    }

    #[test]
    fn test_usage_record_duration_serde_compat() {
        // 新字段序列化：camelCase + None 时省略
        let record = crate::model::usage::UsageRecord {
            api_key_id: 1,
            credential_id: None,
            model: "claude-opus-4.6".to_string(),
            input_tokens: 100,
            output_tokens: 10,
            estimated_cost: 0.0,
            credits_used: None,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            cache_creation_5m_input_tokens: 0,
            cache_creation_1h_input_tokens: 0,
            created_at: chrono::Utc::now(),
            client_ip: None,
            effort: None,
            first_token_ms: Some(1234),
            duration_ms: Some(5678),
            thinking_token_count: None,
            is_stream: None,
            user_agent: None,
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["firstTokenMs"], 1234);
        assert_eq!(json["durationMs"], 5678);
        // None 时省略（与既有 Option 字段同构）
        assert!(json.get("thinkingTokenCount").is_none());
        assert!(json.get("isStream").is_none());

        // 旧数据（无新字段）反序列化兼容
        let old_json = serde_json::json!({
            "apiKeyId": 1,
            "model": "claude-opus-4.6",
            "inputTokens": 100,
            "outputTokens": 10,
            "estimatedCost": 0.0,
            "cacheCreation5mInputTokens": 0,
            "cacheCreation1hInputTokens": 0,
            "createdAt": "2026-01-01T00:00:00Z"
        });
        let parsed: crate::model::usage::UsageRecord = serde_json::from_value(old_json).unwrap();
        assert_eq!(parsed.first_token_ms, None);
        assert_eq!(parsed.duration_ms, None);
        assert_eq!(parsed.thinking_token_count, None);
        assert_eq!(parsed.is_stream, None);
    }

    #[test]
    fn test_usage_record_thinking_token_serde_compat() {
        // Some 值序列化为 camelCase thinkingTokenCount
        let record = crate::model::usage::UsageRecord {
            api_key_id: 1,
            credential_id: None,
            model: "claude-opus-4.6".to_string(),
            input_tokens: 100,
            output_tokens: 10,
            estimated_cost: 0.0,
            credits_used: None,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            cache_creation_5m_input_tokens: 0,
            cache_creation_1h_input_tokens: 0,
            created_at: chrono::Utc::now(),
            client_ip: None,
            effort: None,
            first_token_ms: None,
            duration_ms: None,
            thinking_token_count: Some(1034),
            is_stream: Some(true),
            user_agent: Some("claude-cli/2.1.5 (external, cli)".to_string()),
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["thinkingTokenCount"], 1034);
        assert_eq!(json["userAgent"], "claude-cli/2.1.5 (external, cli)");
        assert_eq!(json["isStream"], true);

        // 旧数据（无该字段）反序列化为 None
        let old_json = serde_json::json!({
            "apiKeyId": 1,
            "model": "claude-opus-4.6",
            "inputTokens": 100,
            "outputTokens": 10,
            "estimatedCost": 0.0,
            "cacheCreation5mInputTokens": 0,
            "cacheCreation1hInputTokens": 0,
            "createdAt": "2026-01-01T00:00:00Z"
        });
        let parsed: crate::model::usage::UsageRecord = serde_json::from_value(old_json).unwrap();
        assert_eq!(parsed.thinking_token_count, None);
        assert_eq!(parsed.is_stream, None);
        assert_eq!(parsed.user_agent, None);
    }

    /// user_agent 落库与分页透传：UA 值应完整传递到分页 DTO
    #[tokio::test]
    async fn test_record_user_agent_persisted_and_paged() {
        let path = temp_usage_path("user_agent_field");
        let tracker = UsageTracker::load(&path).unwrap();
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            100,
            10,
            RecordOptions {
                user_agent: Some("claude-cli/2.1.5 (external, cli)".to_string()),
                ..Default::default()
            },
        );
        let page = tracker.get_records_paged(1, 1, 10, &HashMap::new());
        assert_eq!(page.records.len(), 1);
        assert_eq!(
            page.records[0].user_agent.as_deref(),
            Some("claude-cli/2.1.5 (external, cli)")
        );
        // daily 路径透传：同一条记录经 get_records_paged_by_date 查询 UA 同样在
        let cst_today = {
            let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            chrono::Utc::now()
                .with_timezone(&cst)
                .format("%Y-%m-%d")
                .to_string()
        };
        let daily = tracker.get_records_paged_by_date(&cst_today, 1, 10, &HashMap::new());
        assert_eq!(daily.records.len(), 1);
        assert_eq!(
            daily.records[0].user_agent.as_deref(),
            Some("claude-cli/2.1.5 (external, cli)")
        );
        cleanup_usage_path(&path);
    }

    /// 构造指定 UTC 时间戳的记录并直接注入（绕过 record() 的 Utc::now()，
    /// 供 date_range 系列测试控制记录的 CST 归属日）
    fn inject_record_at(tracker: &UsageTracker, created_at: chrono::DateTime<chrono::Utc>) {
        let record = crate::model::usage::UsageRecord {
            api_key_id: 1,
            credential_id: None,
            model: "claude-opus-4.6".to_string(),
            input_tokens: 100,
            output_tokens: 10,
            estimated_cost: 0.0,
            credits_used: None,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            cache_creation_5m_input_tokens: 0,
            cache_creation_1h_input_tokens: 0,
            created_at,
            client_ip: None,
            effort: None,
            first_token_ms: None,
            duration_ms: None,
            thinking_token_count: None,
            is_stream: None,
            user_agent: None,
        };
        tracker.records.write().push(record);
    }

    /// 解析 RFC3339 UTC 时间戳（比手拼 NaiveDateTime+TimeZone 可读）
    fn utc_at(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    /// date_range：from == to 时区间版与单日版结果等价（单日版已改为区间版薄包装，
    /// 此测试锁住等价性防回归）。
    /// 等价性前提：结果未触及截断上限——单日版截断 2000 / 区间版截断 10000，
    /// 总条数落在 2001~10000 时两者不等价，不在本测试覆盖范围
    #[tokio::test]
    async fn test_date_range_from_eq_to_equivalent_to_daily() {
        let path = temp_usage_path("date_range_equiv");
        let tracker = UsageTracker::load(&path).unwrap();
        // 三天（CST）各两条
        inject_record_at(&tracker, utc_at("2026-10-01T02:00:00Z")); // CST 10-01 10:00
        inject_record_at(&tracker, utc_at("2026-10-01T10:00:00Z")); // CST 10-01 18:00
        inject_record_at(&tracker, utc_at("2026-10-02T02:00:00Z"));
        inject_record_at(&tracker, utc_at("2026-10-02T10:00:00Z"));
        inject_record_at(&tracker, utc_at("2026-10-03T02:00:00Z"));
        inject_record_at(&tracker, utc_at("2026-10-03T10:00:00Z"));

        for date in ["2026-10-01", "2026-10-02", "2026-10-03"] {
            let daily = tracker.get_records_paged_by_date(date, 1, 10, &HashMap::new());
            let ranged =
                tracker.get_records_paged_by_date_range(date, date, 1, 10, &HashMap::new());
            assert_eq!(ranged.total, daily.total);
            assert_eq!(ranged.total_pages, daily.total_pages);
            assert_eq!(ranged.page, daily.page);
            assert_eq!(ranged.page_size, daily.page_size);
            let ranged_times: Vec<_> = ranged.records.iter().map(|r| r.created_at).collect();
            let daily_times: Vec<_> = daily.records.iter().map(|r| r.created_at).collect();
            assert_eq!(ranged_times, daily_times);
        }
        cleanup_usage_path(&path);
    }

    /// date_range：闭区间含两端日，边界日各含记录时全部返回且按时间降序
    #[tokio::test]
    async fn test_date_range_inclusive_boundary_days() {
        let path = temp_usage_path("date_range_boundary");
        let tracker = UsageTracker::load(&path).unwrap();
        inject_record_at(&tracker, utc_at("2026-10-01T16:00:00Z")); // CST 10-02 00:00（区间首日）
        inject_record_at(&tracker, utc_at("2026-10-01T18:00:00Z")); // CST 10-02 02:00
        inject_record_at(&tracker, utc_at("2026-10-02T18:00:00Z")); // CST 10-03 02:00（中间日）
        inject_record_at(&tracker, utc_at("2026-10-03T16:00:00Z")); // CST 10-04 00:00（区间末日）
        inject_record_at(&tracker, utc_at("2026-10-05T02:00:00Z")); // CST 10-05（区间外）

        let page = tracker.get_records_paged_by_date_range(
            "2026-10-02",
            "2026-10-04",
            1,
            10,
            &HashMap::new(),
        );
        assert_eq!(page.total, 4);
        assert_eq!(page.total_pages, 1);
        let times: Vec<_> = page.records.iter().map(|r| r.created_at).collect();
        assert!(times.windows(2).all(|w| w[0] >= w[1])); // 降序
        assert!(times.contains(&utc_at("2026-10-01T16:00:00Z"))); // 首日边界含
        assert!(times.contains(&utc_at("2026-10-03T16:00:00Z"))); // 末日边界含
        cleanup_usage_path(&path);
    }

    /// date_range：跨 CST 午夜记录按 CST 归属日过滤（UTC 15:30/16:30 分归 CST 前后日）
    #[tokio::test]
    async fn test_date_range_cross_cst_midnight_attribution() {
        let path = temp_usage_path("date_range_midnight");
        let tracker = UsageTracker::load(&path).unwrap();
        inject_record_at(&tracker, utc_at("2026-10-01T15:30:00Z")); // CST 10-01 23:30
        inject_record_at(&tracker, utc_at("2026-10-01T16:30:00Z")); // CST 10-02 00:30

        let day1 = tracker.get_records_paged_by_date_range(
            "2026-10-01",
            "2026-10-01",
            1,
            10,
            &HashMap::new(),
        );
        assert_eq!(day1.total, 1);
        assert_eq!(day1.records[0].created_at, utc_at("2026-10-01T15:30:00Z"));

        let day2 = tracker.get_records_paged_by_date_range(
            "2026-10-02",
            "2026-10-02",
            1,
            10,
            &HashMap::new(),
        );
        assert_eq!(day2.total, 1);
        assert_eq!(day2.records[0].created_at, utc_at("2026-10-01T16:30:00Z"));

        let both = tracker.get_records_paged_by_date_range(
            "2026-10-01",
            "2026-10-02",
            1,
            10,
            &HashMap::new(),
        );
        assert_eq!(both.total, 2);
        assert_eq!(both.records[0].created_at, utc_at("2026-10-01T16:30:00Z")); // 降序：更晚在前
        assert_eq!(both.records[1].created_at, utc_at("2026-10-01T15:30:00Z"));
        cleanup_usage_path(&path);
    }

    /// date_range：无匹配记录返回空页（total=0、total_pages=0）
    #[tokio::test]
    async fn test_date_range_empty_result() {
        let path = temp_usage_path("date_range_empty");
        let tracker = UsageTracker::load(&path).unwrap();
        inject_record_at(&tracker, utc_at("2026-10-05T02:00:00Z"));
        let page = tracker.get_records_paged_by_date_range(
            "2026-10-01",
            "2026-10-03",
            1,
            10,
            &HashMap::new(),
        );
        assert!(page.records.is_empty());
        assert_eq!(page.total, 0);
        assert_eq!(page.total_pages, 0);
        assert_eq!(page.page, 1);
        cleanup_usage_path(&path);
    }

    /// date_range：page_size 越界钳制到 [1, 500]（与单日端点一致）
    #[tokio::test]
    async fn test_date_range_page_size_clamp() {
        let path = temp_usage_path("date_range_clamp");
        let tracker = UsageTracker::load(&path).unwrap();
        inject_record_at(&tracker, utc_at("2026-10-02T02:00:00Z"));
        inject_record_at(&tracker, utc_at("2026-10-02T06:00:00Z"));
        inject_record_at(&tracker, utc_at("2026-10-02T10:00:00Z"));

        let zero = tracker.get_records_paged_by_date_range(
            "2026-10-02",
            "2026-10-02",
            1,
            0,
            &HashMap::new(),
        );
        assert_eq!(zero.page_size, 1);
        assert_eq!(zero.records.len(), 1);
        assert_eq!(zero.total_pages, 3);

        let huge = tracker.get_records_paged_by_date_range(
            "2026-10-02",
            "2026-10-02",
            1,
            9999,
            &HashMap::new(),
        );
        assert_eq!(huge.page_size, 500);
        assert_eq!(huge.records.len(), 3);
        assert_eq!(huge.total_pages, 1);
        cleanup_usage_path(&path);
    }

    /// date_range：区间匹配超 10000 条时降序保留最新 10000 条再分页
    #[tokio::test]
    async fn test_date_range_truncate_over_limit() {
        let path = temp_usage_path("date_range_truncate");
        let tracker = UsageTracker::load(&path).unwrap();
        // 注入 10005 条（CST 10-02 全天，毫秒递增）：最早的 5 条应被截掉
        let base = utc_at("2026-10-02T00:00:00Z");
        for i in 0..10005 {
            inject_record_at(&tracker, base + chrono::Duration::milliseconds(i));
        }
        let first = tracker.get_records_paged_by_date_range(
            "2026-10-02",
            "2026-10-02",
            1,
            100,
            &HashMap::new(),
        );
        assert_eq!(first.total, 10_000);
        assert_eq!(first.total_pages, 100);
        assert_eq!(
            first.records[0].created_at,
            base + chrono::Duration::milliseconds(10004)
        ); // 首页首条 = 全局最新
        let last = tracker.get_records_paged_by_date_range(
            "2026-10-02",
            "2026-10-02",
            100,
            100,
            &HashMap::new(),
        );
        assert_eq!(last.records.len(), 100);
        assert_eq!(
            last.records[99].created_at,
            base + chrono::Duration::milliseconds(5)
        ); // 末页末条 = 截断后最旧（i=5，恰被截掉最早 5 条）
        cleanup_usage_path(&path);
    }

    #[test]
    fn test_get_k_ref_opus_5() {
        assert_eq!(get_k_ref("claude-opus-5"), 2.36);
        assert_eq!(get_k_ref("claude-opus-5-thinking"), 2.36);
        assert_eq!(get_k_ref("Claude-Opus-5"), 2.36);

        // 回归：其他档位不变
        assert_eq!(get_k_ref("claude-opus-4-7"), 2.36);
        assert_eq!(get_k_ref("claude-opus-4-8"), 2.36);
        assert_eq!(get_k_ref("claude-opus-4-6"), 1.90);
        assert_eq!(get_k_ref("claude-opus-4-5"), 1.90);
        assert_eq!(get_k_ref("claude-sonnet-5"), 1.43);
        assert_eq!(get_k_ref("claude-sonnet-4.6"), 1.43);
        assert_eq!(get_k_ref("claude-haiku-4.5"), 1.43);
    }

    #[test]
    fn test_get_k_ref_haiku_5_5() {
        // haiku 5.5 沿用 haiku 默认档（暂无 credits 实测系数）
        assert_eq!(get_k_ref("claude-haiku-5.5"), 1.43);
        assert_eq!(get_k_ref("claude-haiku-5-5"), 1.43);
        assert_eq!(get_k_ref("claude-haiku-5-5-thinking"), 1.43);
    }

    #[test]
    fn test_pricing_haiku_5_5() {
        // 低档（≤100K）：1M output 单独不触发跳档（阈值只判 input）
        assert!((calculate_cost("claude-haiku-5.5", 0, 1_000_000) - 0.50).abs() < 1e-9);

        // 回归：haiku 4.5 定价不变（1 + 5 = 6.0）
        assert!((calculate_cost("claude-haiku-4.5", 1_000_000, 1_000_000) - 6.0).abs() < 1e-9);

        // 回归：连字符别名与映射层口径一致（曾按 4.5 定价高估 10 倍）
        // 取 50K 输入以留在低档，避免与跳档测试混淆
        assert!((calculate_cost("claude-haiku-5-5", 50_000, 0) - 0.005).abs() < 1e-9);
    }

    #[test]
    fn test_pricing_haiku_5_5_long_context_tier() {
        // 整档跳价：超 100K 时**整个请求**按高档，非超出部分累进
        // 200_000 input + 1_000 output = 0.5*0.2 + 2.5*0.001 = 0.1025
        assert!(
            (calculate_cost("claude-haiku-5.5", 200_000, 1_000) - 0.1025).abs() < 1e-9,
            "超阈值应整档跳价至 $0.50/$2.50"
        );

        // 边界：100_000 走低档，100_001 跳高档
        assert!(
            (calculate_cost("claude-haiku-5.5", 100_000, 0) - 0.01).abs() < 1e-9,
            "恰好 100K 仍属低档"
        );
        assert!(
            (calculate_cost("claude-haiku-5.5", 100_001, 0) - 0.0500005).abs() < 1e-9,
            "超过 100K 即整档跳价"
        );

        // 别名口径一致
        assert!(
            (calculate_cost("Claude Haiku 5.5", 200_000, 0) - 0.10).abs() < 1e-9,
            "空格别名同样跳档"
        );
        // 点号变体（回归：谓词改写曾丢失点号写法，会回退 4.5 定价高估 10 倍）
        assert!(
            (calculate_cost("claude-haiku.5.5", 200_000, 0) - 0.10).abs() < 1e-9,
            "点号别名同样跳档"
        );

        // 回归：haiku 4.5 无分档，1M 输入仍按低档单档计价
        assert!((calculate_cost("claude-haiku-4.5", 1_000_000, 0) - 1.0).abs() < 1e-9);
    }

    /// is_stream 落库与分页透传：流式/非流式两种取值均应完整传递到分页 DTO
    #[tokio::test]
    async fn test_record_is_stream_persisted_and_paged() {
        let path = temp_usage_path("is_stream_field");
        let tracker = UsageTracker::load(&path).unwrap();
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            100,
            10,
            RecordOptions {
                is_stream: Some(true),
                ..Default::default()
            },
        );
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            100,
            10,
            RecordOptions {
                is_stream: Some(false),
                ..Default::default()
            },
        );
        let page = tracker.get_records_paged(1, 1, 10, &HashMap::new());
        assert_eq!(page.records.len(), 2);
        // 两种取值均透传到 UsageRecordItem（按值断言，不依赖排序）
        assert!(page.records.iter().any(|r| r.is_stream == Some(true)));
        assert!(page.records.iter().any(|r| r.is_stream == Some(false)));
        cleanup_usage_path(&path);
    }

    /// 回归：明细超过 MAX_RECORDS_PER_KEY 被裁剪后，total_requests 仍持续增长
    /// （历史缺陷：直接数现存记录条数导致请求数封顶 10,000）
    #[tokio::test]
    async fn test_total_requests_not_capped_by_record_pruning() {
        let path = temp_usage_path("pruning_not_capped");
        let tracker = UsageTracker::load(&path).unwrap();
        // 记录上限 + 5 条：应裁掉最老的 5 条并累计进基数
        let total = crate::model::usage::MAX_RECORDS_PER_KEY_FOR_TEST + 5;
        for _ in 0..total {
            tracker.record(
                9,
                "claude-opus-4.6".to_string(),
                10,
                10,
                RecordOptions::default(),
            );
        }
        let summary = tracker.get_summary(9);
        assert_eq!(summary.total_requests, total as u64);
        // 现存明细恰好为上限条数
        cleanup_usage_path(&path);
    }

    /// 回归：reset 后请求数从 0 重新计数（基数同步清零）
    #[tokio::test]
    async fn test_reset_clears_lifetime_base() {
        let path = temp_usage_path("reset_clears_base");
        let tracker = UsageTracker::load(&path).unwrap();
        for _ in 0..(crate::model::usage::MAX_RECORDS_PER_KEY_FOR_TEST + 3) {
            tracker.record(
                8,
                "claude-opus-4.6".to_string(),
                10,
                10,
                RecordOptions::default(),
            );
        }
        assert!(tracker.get_summary(8).total_requests > 0);
        tracker.reset(8).unwrap();
        assert_eq!(tracker.get_summary(8).total_requests, 0);
        cleanup_usage_path(&path);
    }

    /// 回归：生命周期基数持久化后重启不丢（裁剪掉的部分仍计入请求数）
    #[tokio::test]
    async fn test_lifetime_base_persisted_across_reload() {
        let path = temp_usage_path("lifetime_persisted");
        {
            let tracker = UsageTracker::load(&path).unwrap();
            for _ in 0..(crate::model::usage::MAX_RECORDS_PER_KEY_FOR_TEST + 7) {
                tracker.record(
                    7,
                    "claude-opus-4.6".to_string(),
                    10,
                    10,
                    RecordOptions::default(),
                );
            }
            // 等待后台任务把脏数据落盘（周期 5s 太长，直接触发 shutdown 刷盘：
            // drop tracker 即关闭通道，graceful shutdown 分支会执行落盘）
        }
        // 轮询等待刷盘完成。不能仅等基数文件存在：后台先处理 interval tick 时
        // （局部 dirty 尚为 false）可能只写基数、不写明细，此时明细文件可能
        // 还未就绪；改为等待明细文件落盘且记录数达到上限，配合基数文件存在的
        // 前置条件，两者都就绪才认为 shutdown 刷盘完成
        let lifetime_path = path.parent().unwrap().join("api_key_lifetime.json");
        let mut persisted = false;
        for _ in 0..100 {
            let records_ready = std::fs::read_to_string(&path)
                .map(|c| {
                    serde_json::from_str::<Vec<crate::model::usage::UsageRecord>>(&c)
                        .map(|r| r.len() == crate::model::usage::MAX_RECORDS_PER_KEY_FOR_TEST)
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            if records_ready && lifetime_path.exists() {
                persisted = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(
            persisted,
            "lifetime base file should be written on shutdown"
        );
        let tracker2 = UsageTracker::load(&path).unwrap();
        // 精确断言：明细（上限条）+ 基数（裁剪掉的 7 条）完整持久化，
        // 基数文件缺失或仅靠现存明细求和都会在此失败
        assert_eq!(
            tracker2.get_summary(7).total_requests,
            (crate::model::usage::MAX_RECORDS_PER_KEY_FOR_TEST + 7) as u64
        );
        cleanup_usage_path(&path);
    }

    #[tokio::test]
    async fn test_daily_summaries_cache_fields() {
        let path = temp_usage_path("daily_cache_fields");
        let tracker = UsageTracker::load(&path).unwrap();
        // 两条记录：(input=1000, cacheRead=800) 与 (input=500, cacheRead=None)
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            1000,
            100,
            RecordOptions {
                cache_read_input_tokens: Some(800),
                cache_creation_input_tokens: Some(100),
                ..Default::default()
            },
        );
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            500,
            50,
            RecordOptions::default(),
        );

        let summaries = tracker.get_daily_summaries();
        assert_eq!(summaries.len(), 1);
        let s = &summaries[0];
        assert_eq!(s.total_requests, 2);
        assert_eq!(s.total_input_tokens, 1500);
        assert_eq!(s.total_cache_read_tokens, 800);

        // 凭证维度：cacheRead None 按 0 计入
        tracker.record(
            1,
            "claude-opus-4.6".to_string(),
            200,
            20,
            RecordOptions {
                credential_id: Some(42),
                ..Default::default()
            },
        );
        let cred = tracker.get_today_summary_for_credential(42);
        assert_eq!(cred.total_requests, 1);
        assert_eq!(cred.total_input_tokens, 200);
        assert_eq!(cred.total_cache_read_tokens, 0);

        cleanup_usage_path(&path);
    }
}
