// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 被上游拒绝过的推理签名（拒绝缓存）
//!
//! 签名损坏或来自其他服务时，上游返回 `THINKING_SIGNATURE_INVALID`。provider 逐个删除
//! 并确认坏签名后，由 handler 记入这里；之后的请求在发送前直接丢弃它们，不再触发报错。
//! 只存签名的 SHA-256，进程内有效：重启后每个坏签名最多再触发一次报错。

use std::collections::{HashSet, VecDeque};
use std::sync::LazyLock;

use parking_lot::Mutex;
use sha2::{Digest, Sha256};

/// 缓存条数上限（每条 32 字节），满了先进先出淘汰
const CAPACITY: usize = 10_000;

type SignatureDigest = [u8; 32];

static REJECTED: LazyLock<Mutex<RejectedSignatures>> =
    LazyLock::new(|| Mutex::new(RejectedSignatures::new(CAPACITY)));

/// 记下被上游确认无效的签名
pub(crate) fn remember_rejected_signatures(signatures: &[String]) {
    let mut rejected = REJECTED.lock();
    for signature in signatures {
        rejected.insert(digest(signature));
    }
}

/// 签名是否被上游拒绝过
pub(crate) fn is_rejected_signature(signature: &str) -> bool {
    REJECTED.lock().contains(&digest(signature))
}

fn digest(signature: &str) -> SignatureDigest {
    Sha256::digest(signature.as_bytes()).into()
}

/// 有容量上限的摘要集合，按插入顺序淘汰
struct RejectedSignatures {
    capacity: usize,
    members: HashSet<SignatureDigest>,
    insertion_order: VecDeque<SignatureDigest>,
}

impl RejectedSignatures {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            members: HashSet::new(),
            insertion_order: VecDeque::new(),
        }
    }

    fn insert(&mut self, digest: SignatureDigest) {
        if !self.members.insert(digest) {
            return;
        }
        self.insertion_order.push_back(digest);
        if self.insertion_order.len() > self.capacity
            && let Some(oldest) = self.insertion_order.pop_front()
        {
            self.members.remove(&oldest);
        }
    }

    fn contains(&self, digest: &SignatureDigest) -> bool {
        self.members.contains(digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembered_signature_is_rejected() {
        let signature = format!("bad-signature-{}", uuid::Uuid::new_v4());
        assert!(!is_rejected_signature(&signature));
        remember_rejected_signatures(std::slice::from_ref(&signature));
        assert!(is_rejected_signature(&signature));
    }

    #[test]
    fn oldest_entry_is_evicted_beyond_capacity() {
        let mut rejected = RejectedSignatures::new(2);
        let [a, b, c] = ["a", "b", "c"].map(digest);
        rejected.insert(a);
        rejected.insert(b);
        // 重复插入不占容量、不刷新顺序
        rejected.insert(a);
        rejected.insert(c);
        assert!(!rejected.contains(&a));
        assert!(rejected.contains(&b));
        assert!(rejected.contains(&c));
    }
}
