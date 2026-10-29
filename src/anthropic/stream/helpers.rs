//! 流式辅助工具：生成 Kiro 事件所需的伪造 signature。

const FAKE_SIGNATURE_BODY_LEN: usize = 160;
const FAKE_SIGNATURE_PADDING: &str = "==";

/// 生成伪造的 thinking 签名（长度 >= 100 的 base64 形状字符串）
///
/// 上游不返回真实签名，流式与非流式路径共用这一份伪造实现，保证两端
/// thinking 块结构一致。
pub(crate) fn generate_fake_signature() -> String {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let len = FAKE_SIGNATURE_BODY_LEN;
    let mut sig = String::with_capacity(len + FAKE_SIGNATURE_PADDING.len());
    for _ in 0..len {
        let idx = fastrand::usize(..BASE64_CHARS.len());
        sig.push(BASE64_CHARS[idx] as char);
    }
    sig.push_str(FAKE_SIGNATURE_PADDING);
    sig
}

/// 是否为 [`generate_fake_signature`] 的产物（上游真实签名实测 900+ 字符，不会撞形）
pub(crate) fn is_fake_signature(signature: &str) -> bool {
    signature.len() == FAKE_SIGNATURE_BODY_LEN + FAKE_SIGNATURE_PADDING.len()
        && signature.ends_with(FAKE_SIGNATURE_PADDING)
}
