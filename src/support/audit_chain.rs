// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 归档文件防篡改 HMAC 链（复用工作区 HMAC-SHA256(prev||event) 模式）。
//!
//! 与 dbnexus 权限审计链同构，差异在链首：本链条以**每次
//! [`ArchiveChain::new`] 生成的随机盐**为链首——`hmac_0 = HMAC(key, salt)`，
//! `hmac_n = HMAC(key, salt || prev_hash_n || canonical_event_n)`。盐随
//! manifest 存储（secret 由使用方持有），校验方重算整条链：任一事件被
//! 篡改、删除、重排或伪造即校验失败。
//!
//! ```rust
//! use inklog::support::audit_chain::ArchiveChain;
//!
//! let mut chain = ArchiveChain::new(b"archive-chain-key");
//! chain.append(r#"{"kind":"rotate","file":"app.001.log"}"#);
//! chain.append(r#"{"kind":"rotate","file":"app.002.log"}"#);
//! assert!(chain.verify());
//!
//! // 篡改任一事件 → 校验失败
//! let mut entries = chain.entries().to_vec();
//! entries[0].event = r#"{"kind":"rotate","file":"evil.log"}"#.to_string();
//! assert!(!ArchiveChain::verify_entries(&entries, b"archive-chain-key"));
//! ```

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// HMAC-SHA256 输出长度（字节）
pub const CHAIN_HASH_LEN: usize = 32;

/// 链首随机盐长度（字节）
pub const CHAIN_SALT_LEN: usize = 16;

/// HMAC 块长（SHA-256 为 64 字节）
const HMAC_BLOCK_SIZE: usize = 64;

/// HMAC-SHA256（RFC 2104，经 sha2 手工构造——不引入 hmac 依赖，与 dbnexus 同模式）
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; CHAIN_HASH_LEN] {
    let mut key_block = [0u8; HMAC_BLOCK_SIZE];
    let normalized: Vec<u8> = if key.len() > HMAC_BLOCK_SIZE {
        Sha256::digest(key).to_vec()
    } else {
        key.to_vec()
    };
    key_block[..normalized.len()].copy_from_slice(&normalized);

    let mut inner = Sha256::new();
    let mut outer = Sha256::new();
    for byte in &key_block {
        inner.update([byte ^ 0x36]);
        outer.update([byte ^ 0x5c]);
    }
    inner.update(message);
    outer.update(inner.finalize());
    let mut bytes = [0u8; CHAIN_HASH_LEN];
    bytes.copy_from_slice(&outer.finalize());
    bytes
}

/// 十六进制编码（小写）
fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// 十六进制解码（奇数长度/非 hex 字符返回 None）
fn hex_decode(raw: &str) -> Option<Vec<u8>> {
    if !raw.len().is_multiple_of(2) {
        return None;
    }
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).ok())
        .collect()
}

/// 归档链条目（manifest 友好：哈希以 hex 存储，可直接 JSONL 落盘）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArchiveChainEntry {
    /// 链内序号（0 起，严格递增）
    pub index: u64,
    /// 链首随机盐（hex；每个条目冗余携带，支持独立校验）
    pub salt: String,
    /// 上一条目的 HMAC（hex；首条为全零）
    pub prev_hash: String,
    /// 事件规范化 JSON
    pub event: String,
    /// HMAC-SHA256(key, salt || prev_hash || event)（hex）
    pub hmac: String,
}

/// 归档链封条（链外锚点）：条目总数 + 尾哈希 + 封条 HMAC。
///
/// 链内自洽校验（序号连续 + 盐一致 + prev_hash 衔接 + HMAC 匹配）无法证明
/// 「没有条目被删掉」——被截尾到**非空前缀**的 manifest 仍然自洽。封条是
/// 链外的唯一锚点：条目总数与尾哈希必须与 manifest 一致，且封条 HMAC 以
/// 链密钥签发，无法伪造。
///
/// 封条必须与 manifest **分开落盘**：同文件内的封条会随截尾一并丢失，
/// 截断检测随之失效（`seal_path_for_manifest`）。
///
/// 威胁模型边界：封条与 manifest 同目录同可写域——把某一代的 manifest 与
/// 当时合法的封条一并回滚对校验不可见。封条只挡单侧截尾，不挡成对回滚。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArchiveChainSeal {
    /// 封条覆盖的条目总数
    pub count: u64,
    /// 末条目的 HMAC（hex）
    pub tail_hash: String,
    /// 链首随机盐（hex；与 manifest 首条目同盐）
    pub salt: String,
    /// HMAC-SHA256(key, salt || count || tail_hash)（hex）
    pub hmac: String,
}

/// 封条路径：由 manifest 路径推出（`<stem>.chain.jsonl` → `<stem>.chain.seal.json`）。
///
/// sink 写入与 CLI 校验共用同一命名规则——封条与 manifest 必须成对推导，
/// 否则校验方找不到锚点。
pub fn seal_path_for_manifest(manifest: &std::path::Path) -> std::path::PathBuf {
    let stem = manifest
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let name = format!("{stem}.seal.json");
    match manifest.parent() {
        Some(parent) => parent.join(name),
        None => std::path::PathBuf::from(name),
    }
}

/// 归档防篡改 HMAC 链。
pub struct ArchiveChain {
    key: Vec<u8>,
    salt: [u8; CHAIN_SALT_LEN],
    prev_hash: [u8; CHAIN_HASH_LEN],
    entries: Vec<ArchiveChainEntry>,
}

impl ArchiveChain {
    /// 以密钥创建链条；链首盐取加密级随机数。
    pub fn new(key: &[u8]) -> Self {
        Self {
            key: key.to_vec(),
            salt: rand::random(),
            prev_hash: [0u8; CHAIN_HASH_LEN],
            entries: Vec::new(),
        }
    }

    /// 追加事件（规范化 JSON 字符串）；返回条目序号。
    pub fn append(&mut self, canonical_event: &str) -> u64 {
        let index = self.entries.len() as u64;
        let mut message =
            Vec::with_capacity(CHAIN_SALT_LEN + CHAIN_HASH_LEN * 2 + canonical_event.len());
        message.extend_from_slice(&self.salt);
        message.extend_from_slice(&self.prev_hash);
        message.extend_from_slice(canonical_event.as_bytes());
        let hmac = hmac_sha256(&self.key, &message);

        let entry = ArchiveChainEntry {
            index,
            salt: hex_encode(&self.salt),
            prev_hash: hex_encode(&self.prev_hash),
            event: canonical_event.to_string(),
            hmac: hex_encode(&hmac),
        };
        self.prev_hash = hmac;
        self.entries.push(entry);
        index
    }

    /// 已追加的条目（manifest 落盘形态）。
    pub fn entries(&self) -> &[ArchiveChainEntry] {
        &self.entries
    }

    /// 生成封条（当前链状态的链外锚点）；空链返回 `None`。
    ///
    /// 封条须与 manifest 分文件落盘（见 [`seal_path_for_manifest`])——
    /// 同文件内的封条会随截尾一并丢失，截断检测随之失效。
    pub fn seal(&self) -> Option<ArchiveChainSeal> {
        let last = self.entries.last()?;
        let count = self.entries.len() as u64;
        let mut message = Vec::with_capacity(CHAIN_SALT_LEN + 8 + last.hmac.len());
        message.extend_from_slice(&self.salt);
        message.extend_from_slice(&count.to_be_bytes());
        message.extend_from_slice(last.hmac.as_bytes());
        let hmac = hmac_sha256(&self.key, &message);
        Some(ArchiveChainSeal {
            count,
            tail_hash: last.hmac.clone(),
            salt: hex_encode(&self.salt),
            hmac: hex_encode(&hmac),
        })
    }

    /// 校验当前链条（实例方法便捷封装）。
    pub fn verify(&self) -> bool {
        Self::verify_entries(self.entries(), &self.key)
    }

    /// 校验完整链条：重算每条 HMAC 并核对 prev_hash 链与序号连续性。
    ///
    /// 任一事件被篡改、删除、重排或伪造（prev_hash 链断裂 / hmac 不匹配 /
    /// 序号跳跃 / 盐不一致）即返回 `false`。
    ///
    /// **空链返回 `false`**：链内自洽性校验无法证明「没有条目被删掉」——
    /// 空 manifest（被删空或被截断）在没有外部锚点（条目总数 / 尾哈希）
    /// 时与「本就没有归档」不可区分，故按校验失败处理（fail-closed）。
    ///
    /// **本函数检不出「截尾到非空前缀」**：截断后的链仍然自洽。截尾检测
    /// 需要链外锚点——见 [`Self::verify_seal`]。
    pub fn verify_entries(entries: &[ArchiveChainEntry], key: &[u8]) -> bool {
        if entries.is_empty() {
            return false;
        }
        let mut prev_hash = [0u8; CHAIN_HASH_LEN];
        let mut salt: Option<[u8; CHAIN_SALT_LEN]> = None;
        for (expected_index, entry) in entries.iter().enumerate() {
            if entry.index != expected_index as u64 {
                return false;
            }
            let entry_salt = hex_decode(&entry.salt).and_then(|v| {
                let bytes: [u8; CHAIN_SALT_LEN] = v.try_into().ok()?;
                Some(bytes)
            });
            let Some(entry_salt) = entry_salt else {
                return false;
            };
            // 链首盐在整条链中必须一致（防换盐重放）
            match salt {
                None => salt = Some(entry_salt),
                Some(s) if s != entry_salt => return false,
                Some(_) => {}
            }
            let Some(prev) = hex_decode(&entry.prev_hash) else {
                return false;
            };
            if prev != prev_hash {
                return false;
            }
            let mut message =
                Vec::with_capacity(CHAIN_SALT_LEN + CHAIN_HASH_LEN * 2 + entry.event.len());
            message.extend_from_slice(&entry_salt);
            message.extend_from_slice(&prev_hash);
            message.extend_from_slice(entry.event.as_bytes());
            let expected = hmac_sha256(key, &message);
            let Some(actual) = hex_decode(&entry.hmac) else {
                return false;
            };
            // 常量时间比较（subtle::ConstantTimeEq，volatile 读取防编译器
            // 重排引入时序方差）；长度不等立即 false——长度本身非秘密
            if !bool::from(expected.ct_eq(&actual)) {
                return false;
            }
            prev_hash = expected;
        }
        true
    }

    /// 校验 manifest 与封条的一致性（截尾检测）。
    ///
    /// 链内自洽（[`Self::verify_entries`]）无法证明「没有条目被删掉」：
    /// 被截尾到**非空前缀**的 manifest 仍然自洽（序号连续、盐一致、
    /// prev_hash 衔接、HMAC 匹配）。本函数用链外锚点补上这一缺口：
    ///
    /// - 条目总数必须与封条一致（截尾 → 数量不符）；
    /// - 尾哈希必须与封条一致（截尾 → 尾哈希不符）；
    /// - 链首盐必须与 manifest 首条目一致（换链 → 盐不符）；
    /// - 封条自身的 HMAC 必须以链密钥验证通过（封条不可伪造）。
    ///
    /// 封条缺失或不可解析时**无法判定完整性**——调用方须按失败处理
    /// （fail-closed）：删掉封条是绕过截尾检测的最短路径。
    pub fn verify_seal(seal: &ArchiveChainSeal, entries: &[ArchiveChainEntry], key: &[u8]) -> bool {
        if entries.is_empty() {
            return false;
        }
        if seal.count != entries.len() as u64 {
            return false;
        }
        let Some(last) = entries.last() else {
            return false;
        };
        if seal.tail_hash != last.hmac {
            return false;
        }
        if seal.salt != entries[0].salt {
            return false;
        }
        let Some(salt) = hex_decode(&seal.salt) else {
            return false;
        };
        if salt.len() != CHAIN_SALT_LEN {
            return false;
        }
        let Some(tail) = hex_decode(&seal.tail_hash) else {
            return false;
        };
        if tail.len() != CHAIN_HASH_LEN {
            return false;
        }
        let mut message = Vec::with_capacity(CHAIN_SALT_LEN + 8 + seal.tail_hash.len());
        message.extend_from_slice(&salt);
        message.extend_from_slice(&seal.count.to_be_bytes());
        message.extend_from_slice(seal.tail_hash.as_bytes());
        let expected = hmac_sha256(key, &message);
        let Some(actual) = hex_decode(&seal.hmac) else {
            return false;
        };
        // 常量时间比较（与链内校验同口径）
        bool::from(expected.ct_eq(&actual))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: &str, file: &str) -> String {
        serde_json::json!({ "kind": kind, "file": file }).to_string()
    }

    #[test]
    fn test_verify_rejects_empty_chain() {
        // 空链无法证明「没有条目被删掉」：空 manifest（被删空或被截断）
        // 与「本就没有归档」不可区分，必须判为校验失败（fail-closed）。
        assert!(!ArchiveChain::verify_entries(&[], b"key-1"));
    }

    #[test]
    fn test_chain_append_and_verify_roundtrip() {
        let mut chain = ArchiveChain::new(b"key-1");
        chain.append(&event("rotate", "app.001.log"));
        chain.append(&event("rotate", "app.002.log"));
        chain.append(&event("cleanup", "app.000.log"));
        assert_eq!(chain.entries().len(), 3);
        assert!(chain.verify());
        // 序号严格递增
        assert_eq!(chain.entries()[2].index, 2);
        // 链首盐为 32-hex（16 字节）
        assert_eq!(chain.entries()[0].salt.len(), CHAIN_SALT_LEN * 2);
    }

    #[test]
    fn test_verify_detects_tampered_event() {
        let mut chain = ArchiveChain::new(b"key-1");
        chain.append(&event("rotate", "app.001.log"));
        chain.append(&event("rotate", "app.002.log"));
        let mut entries = chain.entries().to_vec();
        entries[0].event = event("rotate", "evil.log");
        assert!(
            !ArchiveChain::verify_entries(&entries, b"key-1"),
            "tampered event must be detected"
        );
    }

    #[test]
    fn test_verify_detects_deletion_reorder_and_forgery() {
        let mut chain = ArchiveChain::new(b"key-1");
        chain.append(&event("a", "1"));
        chain.append(&event("b", "2"));
        chain.append(&event("c", "3"));

        let mut entries = chain.entries().to_vec();
        entries.remove(1);
        assert!(
            !ArchiveChain::verify_entries(&entries, b"key-1"),
            "deletion must break the chain"
        );

        // 重排
        let mut entries = chain.entries().to_vec();
        entries.swap(0, 1);
        assert!(
            !ArchiveChain::verify_entries(&entries, b"key-1"),
            "reorder must break the chain"
        );

        // 伪造（追加一条非链上条目）
        let mut entries = chain.entries().to_vec();
        entries.push(ArchiveChainEntry {
            index: 3,
            salt: entries[0].salt.clone(),
            prev_hash: entries[2].hmac.clone(),
            event: event("forged", "4"),
            hmac: hex_encode(&[0u8; 32]),
        });
        assert!(
            !ArchiveChain::verify_entries(&entries, b"key-1"),
            "forgery must be detected"
        );
    }

    #[test]
    fn test_verify_rejects_wrong_key_and_salt_swap() {
        let mut chain = ArchiveChain::new(b"key-1");
        chain.append(&event("a", "1"));
        assert!(!ArchiveChain::verify_entries(chain.entries(), b"wrong-key"));

        // 换盐重放：修改盐 → 链断裂
        let mut entries = chain.entries().to_vec();
        entries[0].salt = hex_encode(&[9u8; CHAIN_SALT_LEN]);
        assert!(
            !ArchiveChain::verify_entries(&entries, b"key-1"),
            "salt swap must be detected"
        );
    }

    #[test]
    fn test_chains_are_unpredictable_across_instances() {
        // 不同实例的链首盐必须不同（随机盐语义）
        let mut a = ArchiveChain::new(b"k");
        let mut b = ArchiveChain::new(b"k");
        a.append(&event("a", "1"));
        b.append(&event("a", "1"));
        assert_ne!(
            a.entries()[0].salt,
            b.entries()[0].salt,
            "chain-start salt must be random per instance"
        );
    }

    /// 截尾到非空前缀：链内校验仍自洽（检不出），封条比对必须判失败。
    #[test]
    fn test_seal_detects_truncation_to_nonempty_prefix() {
        let mut chain = ArchiveChain::new(b"key-1");
        chain.append(&event("a", "1"));
        chain.append(&event("b", "2"));
        chain.append(&event("c", "3"));
        let seal = chain.seal().expect("seal");

        // 截尾：去掉末尾条目——链内自洽（序号连续 + prev_hash 衔接 +
        // HMAC 匹配）仍成立，链内校验检不出
        let truncated: Vec<ArchiveChainEntry> = chain.entries()[..2].to_vec();
        assert!(
            ArchiveChain::verify_entries(&truncated, b"key-1"),
            "truncated prefix stays self-consistent — the seal is the only anchor"
        );
        assert!(
            !ArchiveChain::verify_seal(&seal, &truncated, b"key-1"),
            "truncation must be detected by the seal"
        );

        // 完整链：封条一致
        assert!(ArchiveChain::verify_seal(&seal, chain.entries(), b"key-1"));
    }

    /// 封条自身不可伪造：改 count / 尾哈希 / 换盐 / 换密钥均判失败。
    #[test]
    fn test_seal_rejects_tampering() {
        let mut chain = ArchiveChain::new(b"key-1");
        chain.append(&event("a", "1"));
        chain.append(&event("b", "2"));
        let seal = chain.seal().expect("seal");
        let entries = chain.entries().to_vec();

        let mut wrong_count = seal.clone();
        wrong_count.count = 1;
        assert!(!ArchiveChain::verify_seal(&wrong_count, &entries, b"key-1"));

        let mut wrong_tail = seal.clone();
        wrong_tail.tail_hash = hex_encode(&[0u8; CHAIN_HASH_LEN]);
        assert!(!ArchiveChain::verify_seal(&wrong_tail, &entries, b"key-1"));

        let mut wrong_salt = seal.clone();
        wrong_salt.salt = hex_encode(&[9u8; CHAIN_SALT_LEN]);
        assert!(!ArchiveChain::verify_seal(&wrong_salt, &entries, b"key-1"));

        assert!(!ArchiveChain::verify_seal(&seal, &entries, b"other-key"));
    }

    /// 封条路径与 manifest 路径成对推导。
    #[test]
    fn test_seal_path_is_derived_from_manifest_path() {
        let path = seal_path_for_manifest(std::path::Path::new("/var/log/app.chain.jsonl"));
        assert_eq!(
            path,
            std::path::PathBuf::from("/var/log/app.chain.seal.json")
        );
    }

    #[test]
    fn test_manifest_jsonl_roundtrip() {
        // manifest 以 JSONL 落盘 → 读回校验
        let mut chain = ArchiveChain::new(b"key-1");
        chain.append(&event("rotate", "app.001.log"));
        chain.append(&event("rotate", "app.002.log"));
        let jsonl: String = chain
            .entries()
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n");

        let parsed: Vec<ArchiveChainEntry> = jsonl
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(ArchiveChain::verify_entries(&parsed, b"key-1"));
    }
}
