//! 流式辅助工具：生成 Kiro 事件所需的伪造 signature。
/// 生成伪造的 thinking 签名（长度 >= 100 的 base64 形状字符串）
///
/// 上游不返回真实签名，流式与非流式路径共用这一份伪造实现，保证两端
/// thinking 块结构一致。
pub(crate) fn generate_fake_signature() -> String {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let len = 160;
    let mut sig = String::with_capacity(len + 2);
    for _ in 0..len {
        let idx = fastrand::usize(..BASE64_CHARS.len());
        sig.push(BASE64_CHARS[idx] as char);
    }
    sig.push('=');
    sig.push('=');
    sig
}
