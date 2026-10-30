// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 被上游拒绝过的推理签名（拒绝缓存）
//!
//! 签名损坏或来自其他服务时，上游返回 `THINKING_SIGNATURE_INVALID`。provider 逐个删除
//! 并确认坏签名后，由 handler 记入这里；之后的请求在发送前直接丢弃它们，不再触发报错。
//! 只存签名的 SHA-256，进程内有效：重启后每个坏签名最多再触发一次报错。
//!
//! 按模型区分：真签名也会被别的模型拒绝（2026-10-30 实测 opus-4.5 / sonnet-4.5 拒绝
//! opus-5.5 的签名），某个模型拒绝过的签名回到其他模型仍要回传。模型取客户端请求的
//! 模型名，同一模型的不同写法各记一份，坏签名在每种写法下最多多报错一次。

use std::collections::{HashSet, VecDeque};
use std::sync::LazyLock;

use parking_lot::Mutex;
use sha2::{Digest, Sha256};

/// 缓存条数上限（每条 32 字节），满了先进先出淘汰
const CAPACITY: usize = 10_000;

type SignatureDigest = [u8; 32];

static REJECTED: LazyLock<Mutex<RejectedSignatures>> =
    LazyLock::new(|| Mutex::new(RejectedSignatures::new(CAPACITY)));

/// 记下被 `model` 确认无效的签名
pub(crate) fn remember_rejected_signatures(model: &str, signatures: &[String]) {
    let mut rejected = REJECTED.lock();
    for signature in signatures {
        rejected.insert(digest(model, signature));
    }
}

/// 签名是否被 `model` 拒绝过
pub(crate) fn is_rejected_signature(model: &str, signature: &str) -> bool {
    REJECTED.lock().contains(&digest(model, signature))
}

/// 模型名前缀长度，避免 ("ab", "c") 与 ("a", "bc") 拼接后撞键
fn digest(model: &str, signature: &str) -> SignatureDigest {
    let mut hasher = Sha256::new();
    hasher.update((model.len() as u64).to_le_bytes());
    hasher.update(model.as_bytes());
    hasher.update(signature.as_bytes());
    hasher.finalize().into()
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
        assert!(!is_rejected_signature("claude-opus-5.5", &signature));
        remember_rejected_signatures("claude-opus-5.5", std::slice::from_ref(&signature));
        assert!(is_rejected_signature("claude-opus-5.5", &signature));
    }

    #[test]
    fn rejection_by_one_model_does_not_affect_others() {
        let signature = format!("genuine-signature-{}", uuid::Uuid::new_v4());
        remember_rejected_signatures("claude-sonnet-4.5", std::slice::from_ref(&signature));
        assert!(is_rejected_signature("claude-sonnet-4.5", &signature));
        assert!(!is_rejected_signature("claude-opus-5.5", &signature));
    }

    #[test]
    fn oldest_entry_is_evicted_beyond_capacity() {
        let mut rejected = RejectedSignatures::new(2);
        let [a, b, c] = ["a", "b", "c"].map(|s| digest("m", s));
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
