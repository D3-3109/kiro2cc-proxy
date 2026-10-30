// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! RPM（Requests Per Minute）与并发实时监控
//!
//! 使用滑动窗口统计最近 60 秒内的请求数量，另计正在处理的请求数（并发），
//! 支持全局、按账号、按 API Key 三个维度。
//!
//! 并发的区间：
//! - 全局 / API Key：进入 handler 起，到响应体发完或客户端断开；
//! - 账号：上游选中该账号并返回成功起，到响应体发完或客户端断开（故障转移试过的账号不计）。

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// 滑动窗口大小（秒）
const WINDOW_SECS: usize = 60;

#[derive(Clone, Copy, Default)]
struct Bucket {
    timestamp: u64,
    count: u64,
}

/// 单个维度的请求时间戳队列（基于 Ring Buffer 的 O(1) 实现）
struct TimestampQueue {
    buckets: [Bucket; WINDOW_SECS],
}

impl TimestampQueue {
    fn new() -> Self {
        Self {
            buckets: [Bucket::default(); WINDOW_SECS],
        }
    }

    /// 记录一次请求
    fn record(&mut self, now_secs: u64) {
        let index = (now_secs as usize) % WINDOW_SECS;
        let bucket = &mut self.buckets[index];
        if bucket.timestamp == now_secs {
            bucket.count += 1;
        } else {
            bucket.timestamp = now_secs;
            bucket.count = 1;
        }
    }

    /// 清理过期条目并返回当前窗口内的请求数
    fn count(&self, now_secs: u64) -> u64 {
        self.buckets
            .iter()
            .filter(|b| now_secs.saturating_sub(b.timestamp) < WINDOW_SECS as u64)
            .map(|b| b.count)
            .sum()
    }

    /// 计算当 RPM 达到上限时，最早的一个 bucket 将在多少秒后滑出窗口
    ///
    /// 返回 None 表示当前未满或无有效 bucket
    fn time_until_earliest_expires(&self, now_secs: u64) -> Option<u64> {
        self.buckets
            .iter()
            .filter(|b| b.count > 0 && now_secs.saturating_sub(b.timestamp) < WINDOW_SECS as u64)
            .map(|b| {
                // bucket 在 (b.timestamp + WINDOW_SECS) 时滑出窗口
                (b.timestamp + WINDOW_SECS as u64).saturating_sub(now_secs)
            })
            .min()
            .map(|s| s.max(1))
    }
}

/// RPM 追踪器
///
/// 线程安全，使用单个 Mutex 保护所有状态。
/// 内存开销恒定：每个队列固定为 `[Bucket; 60]` 大小。
pub struct RpmTracker {
    inner: Mutex<RpmTrackerInner>,
}

struct RpmTrackerInner {
    /// 全局请求队列
    global: TimestampQueue,
    /// 按账号 ID 分组
    by_credential: HashMap<u64, TimestampQueue>,
    /// 按 API Key ID 分组
    by_api_key: HashMap<u32, TimestampQueue>,
    /// 正在处理的请求数；计数归零的键即删除
    in_flight_global: u64,
    in_flight_by_credential: HashMap<u64, u64>,
    in_flight_by_api_key: HashMap<u32, u64>,
}

/// RPM 快照（用于 API 响应）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpmSnapshot {
    /// 全局 RPM
    pub global: u64,
    /// 按账号 ID 分组的 RPM
    pub by_credential: HashMap<u64, u64>,
    /// 按 API Key ID 分组的 RPM
    pub by_api_key: HashMap<u32, u64>,
    /// 全局并发
    pub concurrency_global: u64,
    /// 按账号 ID 分组的并发（无在途请求的账号不出现）
    pub concurrency_by_credential: HashMap<u64, u64>,
    /// 按 API Key ID 分组的并发（无在途请求的 Key 不出现）
    pub concurrency_by_api_key: HashMap<u32, u64>,
}

/// 一个在途请求计入的维度
enum InFlightScope {
    /// 全局，有 Key 时同时计入该 Key
    ApiKey(Option<u32>),
    Credential(u64),
}

/// 在途请求：创建时计入并发，drop 时扣除
///
/// 流式请求须随 SSE 流一起保存，流结束或客户端断开时才 drop。
#[must_use = "守卫被 drop 即视为请求结束"]
pub struct InFlightGuard {
    tracker: Arc<RpmTracker>,
    scope: InFlightScope,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.tracker.end_in_flight(&self.scope);
    }
}

impl RpmTracker {
    /// 创建新的 RPM 追踪器
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RpmTrackerInner {
                global: TimestampQueue::new(),
                by_credential: HashMap::new(),
                by_api_key: HashMap::new(),
                in_flight_global: 0,
                in_flight_by_credential: HashMap::new(),
                in_flight_by_api_key: HashMap::new(),
            }),
        }
    }

    /// 一次请求进入 handler：计入全局与该 Key 的 RPM 和并发
    pub fn begin_api_key_request(self: &Arc<Self>, api_key_id: Option<u32>) -> InFlightGuard {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut inner = self.lock();
        inner.global.record(now_secs);
        inner.in_flight_global += 1;
        if let Some(key_id) = api_key_id {
            inner
                .by_api_key
                .entry(key_id)
                .or_insert_with(TimestampQueue::new)
                .record(now_secs);
            *inner.in_flight_by_api_key.entry(key_id).or_default() += 1;
        }
        InFlightGuard {
            tracker: Arc::clone(self),
            scope: InFlightScope::ApiKey(api_key_id),
        }
    }

    /// 上游选中账号并返回成功：计入该账号的 RPM 和并发
    pub fn begin_credential_request(self: &Arc<Self>, credential_id: u64) -> InFlightGuard {
        self.record_credential(credential_id);
        *self
            .lock()
            .in_flight_by_credential
            .entry(credential_id)
            .or_default() += 1;
        InFlightGuard {
            tracker: Arc::clone(self),
            scope: InFlightScope::Credential(credential_id),
        }
    }

    fn end_in_flight(&self, scope: &InFlightScope) {
        let mut inner = self.lock();
        match *scope {
            InFlightScope::ApiKey(api_key_id) => {
                inner.in_flight_global -= 1;
                if let Some(key_id) = api_key_id {
                    decrement(&mut inner.in_flight_by_api_key, key_id);
                }
            }
            InFlightScope::Credential(credential_id) => {
                decrement(&mut inner.in_flight_by_credential, credential_id);
            }
        }
    }

    /// 守卫的 drop 也走这里：锁中毒时恢复而不是 panic
    fn lock(&self) -> std::sync::MutexGuard<'_, RpmTrackerInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 记录账号维度的请求（在 provider 成功调用后调用）
    pub fn record_credential(&self, credential_id: u64) {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut inner = self.inner.lock().unwrap();
        inner
            .by_credential
            .entry(credential_id)
            .or_insert_with(TimestampQueue::new)
            .record(now_secs);
    }

    /// 计算指定账号在 RPM 满时，最快多久后会有一个 slot 释放
    ///
    /// 返回 None 表示当前 RPM 未满或无数据
    pub fn time_until_slot(&self, credential_id: u64, max_rpm: u32) -> Option<std::time::Duration> {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let inner = self.inner.lock().unwrap();
        let queue = inner.by_credential.get(&credential_id)?;
        if queue.count(now_secs) < max_rpm as u64 {
            return None;
        }
        queue
            .time_until_earliest_expires(now_secs)
            .map(std::time::Duration::from_secs)
    }

    /// 查询指定账号的当前 RPM
    pub fn credential_rpm(&self, credential_id: u64) -> u64 {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let inner = self.inner.lock().unwrap();
        inner
            .by_credential
            .get(&credential_id)
            .map(|q| q.count(now_secs))
            .unwrap_or(0)
    }

    /// 获取当前 RPM 快照
    pub fn snapshot(&self) -> RpmSnapshot {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut inner = self.inner.lock().unwrap();

        let global = inner.global.count(now_secs);

        // 获取并顺便清理废弃为空的键，防止长时间未用的 map 残留
        inner
            .by_credential
            .retain(|_, queue| queue.count(now_secs) > 0);
        let by_credential: HashMap<u64, u64> = inner
            .by_credential
            .iter()
            .map(|(&id, queue)| (id, queue.count(now_secs)))
            .collect();

        inner
            .by_api_key
            .retain(|_, queue| queue.count(now_secs) > 0);
        let by_api_key: HashMap<u32, u64> = inner
            .by_api_key
            .iter()
            .map(|(&id, queue)| (id, queue.count(now_secs)))
            .collect();

        RpmSnapshot {
            global,
            by_credential,
            by_api_key,
            concurrency_global: inner.in_flight_global,
            concurrency_by_credential: inner.in_flight_by_credential.clone(),
            concurrency_by_api_key: inner.in_flight_by_api_key.clone(),
        }
    }
}

/// 在途计数减一，归零即删键；键必然存在（由对应的 begin_* 计入）
fn decrement<K: Eq + Hash>(counts: &mut HashMap<K, u64>, key: K) {
    let std::collections::hash_map::Entry::Occupied(mut entry) = counts.entry(key) else {
        unreachable!("在途计数缺少对应的 begin_* 记录");
    };
    *entry.get_mut() -= 1;
    if *entry.get() == 0 {
        entry.remove();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpm_ring_buffer() {
        let mut queue = TimestampQueue::new();
        // 模拟当前时间
        let now_secs = 1000;

        queue.record(now_secs);
        queue.record(now_secs);

        assert_eq!(queue.count(now_secs), 2);
        assert_eq!(queue.count(now_secs + 10), 2); // 10秒后依然在 60 秒窗口内

        // 测试过期的边界
        assert_eq!(queue.count(now_secs + 60), 0); // 正好 60 秒（已过期）
        assert_eq!(queue.count(now_secs + 61), 0);

        // 模拟一分钟后同一槽位被复用
        queue.record(now_secs + 60);
        assert_eq!(queue.count(now_secs + 60), 1);

        // 模拟多个不同秒数的请求
        queue.record(now_secs + 60);
        queue.record(now_secs + 61);
        queue.record(now_secs + 62);

        assert_eq!(queue.count(now_secs + 62), 4);

        // 当 now_secs 推进到 now_secs + 123 时，所有数据均应过期（最后一次记录在 1062）
        assert_eq!(queue.count(now_secs + 123), 0);
    }

    #[test]
    fn in_flight_guards_count_until_dropped() {
        let tracker = Arc::new(RpmTracker::new());
        let key_a = tracker.begin_api_key_request(Some(7));
        let key_b = tracker.begin_api_key_request(Some(7));
        let anonymous = tracker.begin_api_key_request(None);
        let credential = tracker.begin_credential_request(14);

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.concurrency_global, 3);
        assert_eq!(snapshot.concurrency_by_api_key, HashMap::from([(7, 2)]));
        assert_eq!(snapshot.concurrency_by_credential, HashMap::from([(14, 1)]));
        // 并发与 RPM 同时计入
        assert_eq!(snapshot.global, 3);
        assert_eq!(snapshot.by_api_key, HashMap::from([(7, 2)]));
        assert_eq!(snapshot.by_credential, HashMap::from([(14, 1)]));

        drop((key_a, anonymous, credential));
        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.concurrency_global, 1);
        assert_eq!(snapshot.concurrency_by_api_key, HashMap::from([(7, 1)]));
        // 归零的键删除
        assert!(snapshot.concurrency_by_credential.is_empty());
        // RPM 不随请求结束而减少
        assert_eq!(snapshot.global, 3);

        drop(key_b);
        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.concurrency_global, 0);
        assert!(snapshot.concurrency_by_api_key.is_empty());
    }
}
