// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 内存日志捕获层 — 将 tracing 事件存入 ring buffer 并通过 broadcast 广播

use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

const BROADCAST_CAPACITY: usize = 256;

/// ring buffer 初始预分配条数。容量可达 10 万，按容量预分配会白占约 10MB；
/// VecDeque 按需倍增，写满后 pop_front / push_back 不再扩容。
const INITIAL_PREALLOC: usize = 1024;

/// 捕获层单条日志消息的字节上限。容量放大到 10 万条后，个别超大消息（如 DEBUG 下的
/// websearch MCP 响应体）会让内存占用没有上界。仅截断内存捕获的副本（日志页 / 下载），
/// 控制台与日志文件仍是完整内容。
const MAX_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

pub struct LogCapture {
    ring_buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    sender: broadcast::Sender<LogEntry>,
    capacity: usize,
}

impl LogCapture {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            ring_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(
                capacity.min(INITIAL_PREALLOC),
            ))),
            sender,
            capacity,
        }
    }

    pub fn as_layer(&self) -> LogCaptureLayer {
        LogCaptureLayer {
            ring_buffer: self.ring_buffer.clone(),
            sender: self.sender.clone(),
            capacity: self.capacity,
        }
    }

    /// 返回当前 ring buffer 快照（全量复制）
    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.ring_buffer.lock().iter().cloned().collect()
    }

    /// 返回 ring buffer 中最新的至多 `limit` 条（按时间从旧到新）。
    ///
    /// 日志页首屏只需要最新一小段；只复制尾部，避免容量很大时每次连接都全量克隆。
    pub fn snapshot_tail(&self, limit: usize) -> Vec<LogEntry> {
        let buf = self.ring_buffer.lock();
        let skip = buf.len().saturating_sub(limit);
        buf.iter().skip(skip).cloned().collect()
    }

    /// 订阅后续新事件（返回 broadcast::Receiver）
    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.sender.subscribe()
    }
}

pub struct LogCaptureLayer {
    ring_buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    sender: broadcast::Sender<LogEntry>,
    capacity: usize,
}

impl<S: Subscriber> Layer<S> for LogCaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = match *metadata.level() {
            Level::TRACE => "TRACE",
            Level::DEBUG => "DEBUG",
            Level::INFO => "INFO",
            Level::WARN => "WARN",
            Level::ERROR => "ERROR",
        };

        let mut message = String::new();
        event.record(&mut MessageVisitor(&mut message));
        truncate_message(&mut message);

        let entry = LogEntry {
            timestamp: chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string(),
            level: level.to_string(),
            target: metadata.target().to_string(),
            message,
        };

        {
            let mut buf = self.ring_buffer.lock();
            if buf.len() >= self.capacity {
                buf.pop_front();
            }
            buf.push_back(entry.clone());
        }

        // 无接收者时 send 返回 Err，属正常情况，忽略
        let _ = self.sender.send(entry);
    }
}

/// 超过 `MAX_MESSAGE_BYTES` 的消息截断到最近的字符边界，并附上被截掉的字节数
fn truncate_message(message: &mut String) {
    if message.len() <= MAX_MESSAGE_BYTES {
        return;
    }
    let total = message.len();
    let mut end = MAX_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message.truncate(end);
    message.push_str(&format!("…[已截断 {} 字节]", total - end));
}

struct MessageVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0.push_str(&format!("{}={}", field.name(), value));
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let s = format!("{:?}", value);
        if field.name() == "message" {
            // 去掉 Debug 输出的外层引号（字符串类型会被加引号）
            if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                self.0.push_str(&s[1..s.len() - 1]);
            } else {
                self.0.push_str(&s);
            }
        } else {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0.push_str(&format!("{}={}", field.name(), s));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: usize) -> LogEntry {
        LogEntry {
            timestamp: format!("t{n}"),
            level: "INFO".to_string(),
            target: "t".to_string(),
            message: format!("m{n}"),
        }
    }

    fn filled(capacity: usize, count: usize) -> LogCapture {
        let cap = LogCapture::new(capacity);
        {
            let mut buf = cap.ring_buffer.lock();
            for n in 0..count {
                buf.push_back(entry(n));
            }
        }
        cap
    }

    #[test]
    fn snapshot_tail_returns_latest_in_order() {
        let cap = filled(100, 10);
        let tail = cap.snapshot_tail(3);
        let msgs: Vec<_> = tail.iter().map(|e| e.message.as_str()).collect();
        assert_eq!(msgs, ["m7", "m8", "m9"]);
    }

    #[test]
    fn truncate_message_keeps_short_and_cuts_long_on_char_boundary() {
        let mut short = "abc".to_string();
        truncate_message(&mut short);
        assert_eq!(short, "abc");

        // 多字节字符横跨上限：不能切在字符中间，也不能 panic
        let mut long = "日".repeat(MAX_MESSAGE_BYTES);
        let total = long.len();
        truncate_message(&mut long);
        let kept = long.split('…').next().unwrap();
        assert!(kept.len() <= MAX_MESSAGE_BYTES);
        assert!(kept.chars().all(|c| c == '日'));
        assert!(long.ends_with(&format!("…[已截断 {} 字节]", total - kept.len())));
    }

    #[test]
    fn new_does_not_preallocate_full_capacity() {
        let cap = LogCapture::new(100_000);
        assert!(cap.ring_buffer.lock().capacity() < 100_000);
    }

    #[test]
    fn snapshot_tail_limit_larger_than_len_returns_all() {
        let cap = filled(100, 4);
        assert_eq!(cap.snapshot_tail(1000).len(), 4);
        assert_eq!(cap.snapshot_tail(0).len(), 0);
        assert_eq!(cap.snapshot().len(), 4);
    }
}
