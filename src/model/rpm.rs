// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! RPM（Requests Per Minute）与并发实时监控，兼本地限额
//!
//! 使用滑动窗口统计最近 60 秒内的请求数量，另计正在处理的请求数（并发），
//! 支持全局、按账号、按 API Key 三个维度。
//!
//! 计数口径：
//! - 全局 / API Key：进入 handler 计 1 次请求；并发从进入 handler 起，到响应体发完或客户端断开；
//! - 账号：每向上游发出一次请求计 1 次（含之后被限流、失败的尝试）；并发从发出起，
//!   到响应体发完、客户端断开或本次尝试失败。
//!
//! 本地限额（[`LocalLimits`]）在计入前原子检查：满了不计入，返回 [`LimitFull`]。

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tokio::sync::Notify;
use tokio::time::Instant;

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
    /// 有在途请求结束时唤醒排队者
    released: Notify,
}

/// 单个维度的本地限额，0 = 不限
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LocalLimits {
    /// 最近 60 秒请求数上限
    pub rpm: u32,
    /// 同时在途请求数上限
    pub concurrency: u32,
}

/// 本地限额已满
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitFull {
    /// 最近 60 秒请求数已达上限；`retry_after` 为最早一个名额释放的等待时长
    Rpm { retry_after: Duration },
    /// 在途请求数已达上限（何时释放取决于在途请求何时结束）
    Concurrency,
}

impl std::fmt::Display for LimitFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rpm { retry_after } => {
                write!(f, "RPM 已满，{}s 后释放名额", retry_after.as_secs())
            }
            Self::Concurrency => write!(f, "并发已满"),
        }
    }
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
            released: Notify::new(),
        }
    }

    /// 一次请求进入 handler：该 Key 未超 `limits` 时计入全局与该 Key 的 RPM 和并发
    ///
    /// 无 Key（`None`）时只计全局，不受 `limits` 约束。
    pub fn try_begin_api_key_request(
        self: &Arc<Self>,
        api_key_id: Option<u32>,
        limits: LocalLimits,
    ) -> Result<InFlightGuard, LimitFull> {
        let now_secs = now_secs();
        let mut inner = self.lock();
        if let Some(key_id) = api_key_id {
            let in_flight = inner.in_flight_by_api_key.get(&key_id).copied();
            check_limits(inner.by_api_key.get(&key_id), in_flight, limits, now_secs)?;
        }
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
        Ok(InFlightGuard {
            tracker: Arc::clone(self),
            scope: InFlightScope::ApiKey(api_key_id),
        })
    }

    /// [`Self::try_begin_api_key_request`] 的排队版：满了等到名额释放，`deadline` 前仍满则放弃
    pub async fn begin_api_key_request_by(
        self: &Arc<Self>,
        api_key_id: Option<u32>,
        limits: LocalLimits,
        deadline: Instant,
    ) -> Result<InFlightGuard, LimitFull> {
        self.queue_until(deadline, || {
            self.try_begin_api_key_request(api_key_id, limits)
        })
        .await
    }

    /// 向上游发出一次请求：该账号未超 `limits` 时计入其 RPM 和并发
    pub fn try_begin_credential_request(
        self: &Arc<Self>,
        credential_id: u64,
        limits: LocalLimits,
    ) -> Result<InFlightGuard, LimitFull> {
        let now_secs = now_secs();
        let mut inner = self.lock();
        let in_flight = inner.in_flight_by_credential.get(&credential_id).copied();
        check_limits(
            inner.by_credential.get(&credential_id),
            in_flight,
            limits,
            now_secs,
        )?;
        inner
            .by_credential
            .entry(credential_id)
            .or_insert_with(TimestampQueue::new)
            .record(now_secs);
        *inner
            .in_flight_by_credential
            .entry(credential_id)
            .or_default() += 1;
        Ok(InFlightGuard {
            tracker: Arc::clone(self),
            scope: InFlightScope::Credential(credential_id),
        })
    }

    /// [`Self::try_begin_credential_request`] 的排队版：满了等到名额释放，`deadline` 前仍满则放弃
    pub async fn begin_credential_request_by(
        self: &Arc<Self>,
        credential_id: u64,
        limits: LocalLimits,
        deadline: Instant,
    ) -> Result<InFlightGuard, LimitFull> {
        self.queue_until(deadline, || {
            self.try_begin_credential_request(credential_id, limits)
        })
        .await
    }

    /// 反复尝试 `try_begin`，两次之间等到「有在途请求结束」或「最早的 RPM 名额释放」
    async fn queue_until(
        &self,
        deadline: Instant,
        try_begin: impl Fn() -> Result<InFlightGuard, LimitFull>,
    ) -> Result<InFlightGuard, LimitFull> {
        loop {
            // 先登记唤醒再检查，检查与等待之间的释放不会漏掉
            let released = self.released.notified();
            tokio::pin!(released);
            released.as_mut().enable();

            let full = match try_begin() {
                Ok(guard) => return Ok(guard),
                Err(full) => full,
            };
            let now = Instant::now();
            if now >= deadline {
                return Err(full);
            }
            let wake_at = match full {
                LimitFull::Rpm { retry_after } => (now + retry_after).min(deadline),
                LimitFull::Concurrency => deadline,
            };
            tokio::select! {
                () = tokio::time::sleep_until(wake_at) => {}
                () = released => {}
            }
        }
    }

    fn end_in_flight(&self, scope: &InFlightScope) {
        {
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
        self.released.notify_waiters();
    }

    /// 守卫的 drop 也走这里：锁中毒时恢复而不是 panic
    fn lock(&self) -> std::sync::MutexGuard<'_, RpmTrackerInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
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

    /// 查询指定账号正在处理的请求数
    pub fn credential_in_flight(&self, credential_id: u64) -> u64 {
        self.lock()
            .in_flight_by_credential
            .get(&credential_id)
            .copied()
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

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 再计入一次请求是否会超过 `limits`；`requests` / `in_flight` 为 None 表示该维度尚无记录
fn check_limits(
    requests: Option<&TimestampQueue>,
    in_flight: Option<u64>,
    limits: LocalLimits,
    now_secs: u64,
) -> Result<(), LimitFull> {
    if limits.concurrency > 0 && in_flight.unwrap_or(0) >= u64::from(limits.concurrency) {
        return Err(LimitFull::Concurrency);
    }
    if limits.rpm == 0 {
        return Ok(());
    }
    let Some(queue) = requests else {
        return Ok(());
    };
    if queue.count(now_secs) < u64::from(limits.rpm) {
        return Ok(());
    }
    // 窗口内计数 ≥ rpm ≥ 1，必有未过期的 bucket
    let wait_secs = queue
        .time_until_earliest_expires(now_secs)
        .expect("窗口内计数非零时必有未过期 bucket");
    Err(LimitFull::Rpm {
        retry_after: Duration::from_secs(wait_secs),
    })
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

    const UNLIMITED: LocalLimits = LocalLimits {
        rpm: 0,
        concurrency: 0,
    };

    fn key(tracker: &Arc<RpmTracker>, id: Option<u32>) -> InFlightGuard {
        tracker.try_begin_api_key_request(id, UNLIMITED).unwrap()
    }

    #[test]
    fn concurrency_limit_rejects_until_a_request_ends() {
        let tracker = Arc::new(RpmTracker::new());
        let limits = LocalLimits {
            rpm: 0,
            concurrency: 2,
        };
        let first = tracker.try_begin_credential_request(14, limits).unwrap();
        let _second = tracker.try_begin_credential_request(14, limits).unwrap();

        let full = tracker.try_begin_credential_request(14, limits);
        assert_eq!(full.err(), Some(LimitFull::Concurrency));
        // 被拒的请求不计入 RPM
        assert_eq!(tracker.credential_rpm(14), 2);
        // 其它账号互不影响
        assert!(tracker.try_begin_credential_request(10, limits).is_ok());

        drop(first);
        assert!(tracker.try_begin_credential_request(14, limits).is_ok());
    }

    #[test]
    fn rpm_limit_counts_finished_requests_and_reports_wait() {
        let tracker = Arc::new(RpmTracker::new());
        let limits = LocalLimits {
            rpm: 2,
            concurrency: 0,
        };
        // RPM 按发出计：请求结束也仍占名额
        drop(tracker.try_begin_api_key_request(Some(7), limits).unwrap());
        drop(tracker.try_begin_api_key_request(Some(7), limits).unwrap());

        let Err(LimitFull::Rpm { retry_after }) =
            tracker.try_begin_api_key_request(Some(7), limits)
        else {
            panic!("第 3 次应被 RPM 限额拒绝");
        };
        assert!((1..=60).contains(&retry_after.as_secs()));
        // 被拒的请求不计入全局
        assert_eq!(tracker.snapshot().global, 2);
        // 无 Key 的请求不受 Key 限额约束
        assert!(tracker.try_begin_api_key_request(None, limits).is_ok());
    }

    #[tokio::test]
    async fn queued_request_starts_when_in_flight_request_ends() {
        let tracker = Arc::new(RpmTracker::new());
        let limits = LocalLimits {
            rpm: 0,
            concurrency: 1,
        };
        let holder = tracker.try_begin_credential_request(14, limits).unwrap();

        let waiter = {
            let tracker = Arc::clone(&tracker);
            let deadline = Instant::now() + Duration::from_secs(10);
            tokio::spawn(async move {
                tracker
                    .begin_credential_request_by(14, limits, deadline)
                    .await
                    .is_ok()
            })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        let released_at = Instant::now();
        drop(holder);

        assert!(waiter.await.unwrap());
        assert!(released_at.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn queued_request_gives_up_at_deadline() {
        let tracker = Arc::new(RpmTracker::new());
        let limits = LocalLimits {
            rpm: 0,
            concurrency: 1,
        };
        let _holder = tracker.try_begin_api_key_request(Some(7), limits).unwrap();

        let deadline = Instant::now() + Duration::from_millis(50);
        let result = tracker
            .begin_api_key_request_by(Some(7), limits, deadline)
            .await;
        assert_eq!(result.err(), Some(LimitFull::Concurrency));
        assert!(Instant::now() >= deadline);
    }

    #[test]
    fn in_flight_guards_count_until_dropped() {
        let tracker = Arc::new(RpmTracker::new());
        let key_a = key(&tracker, Some(7));
        let key_b = key(&tracker, Some(7));
        let anonymous = key(&tracker, None);
        let credential = tracker.try_begin_credential_request(14, UNLIMITED).unwrap();

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
