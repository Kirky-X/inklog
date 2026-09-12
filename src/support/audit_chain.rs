// Copyright (c) 2026 Kirky.X
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
    if raw.len() % 2 != 0 {
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
        let mut message = Vec::with_capacity(CHAIN_SALT_LEN + CHAIN_HASH_LEN * 2 + canonical_event.len());
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

    /// 校验当前链条（实例方法便捷封装）。
    pub fn verify(&self) -> bool {
        Self::verify_entries(self.entries(), &self.key)
    }

    /// 校验完整链条：重算每条 HMAC 并核对 prev_hash 链与序号连续性。
    ///
    /// 任一事件被篡改、删除、重排或伪造（prev_hash 链断裂 / hmac 不匹配 /
    /// 序号跳跃 / 盐不一致）即返回 `false`。
    pub fn verify_entries(entries: &[ArchiveChainEntry], key: &[u8]) -> bool {
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
            let mut message = Vec::with_capacity(CHAIN_SALT_LEN + CHAIN_HASH_LEN * 2 + entry.event.len());
            message.extend_from_slice(&entry_salt);
            message.extend_from_slice(&prev_hash);
            message.extend_from_slice(entry.event.as_bytes());
            let expected = hmac_sha256(key, &message);
            let Some(actual) = hex_decode(&entry.hmac) else {
                return false;
            };
            // 常量时间比较（subtle），防时序侧信道
            let expected_key = subtle_ct_eq(&expected, &actual);
            if !expected_key {
                return false;
            }
            prev_hash = expected;
        }
        true
    }
}

/// 常量时间字节比较（长度不等立即 false——长度本身非秘密）。
fn subtle_ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: &str, file: &str) -> String {
        serde_json::json!({ "kind": kind, "file": file }).to_string()
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

        // 删除中间条目
        let mut entries = chain.entries().to_vec();
        entries.remove(1);
        assert!(!ArchiveChain::verify_entries(&entries, b"key-1"), "deletion must break the chain");

        // 重排
        let mut entries = chain.entries().to_vec();
        entries.swap(0, 1);
        assert!(!ArchiveChain::verify_entries(&entries, b"key-1"), "reorder must break the chain");

        // 伪造（追加一条非链上条目）
        let mut entries = chain.entries().to_vec();
        entries.push(ArchiveChainEntry {
            index: 3,
            salt: entries[0].salt.clone(),
            prev_hash: entries[2].hmac.clone(),
            event: event("forged", "4"),
            hmac: hex_encode(&[0u8; 32]),
        });
        assert!(!ArchiveChain::verify_entries(&entries, b"key-1"), "forgery must be detected");
    }

    #[test]
    fn test_verify_rejects_wrong_key_and_salt_swap() {
        let mut chain = ArchiveChain::new(b"key-1");
        chain.append(&event("a", "1"));
        assert!(!ArchiveChain::verify_entries(chain.entries(), b"wrong-key"));

        // 换盐重放：修改盐 → 链断裂
        let mut entries = chain.entries().to_vec();
        entries[0].salt = hex_encode(&[9u8; CHAIN_SALT_LEN]);
        assert!(!ArchiveChain::verify_entries(&entries, b"key-1"), "salt swap must be detected");
    }

    #[test]
    fn test_chains_are_unpredictable_across_instances() {
        // 不同实例的链首盐必须不同（随机盐语义）
        let mut a = ArchiveChain::new(b"k");
        let mut b = ArchiveChain::new(b"k");
        a.append(&event("a", "1"));
        b.append(&event("a", "1"));
        assert_ne!(
            a.entries()[0].salt, b.entries()[0].salt,
            "chain-start salt must be random per instance"
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
