// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! AES-256-GCM 日志加解密与加密文件头解析。
//!
//! 从 `examples/src/bin/encryption.rs` 提取核心逻辑：
//!
//! - [`generate_temp_key`]：生成 Base64 编码的 32 字节随机密钥。
//! - [`encrypt_log_file`]：读取明文 → AES-256-GCM 加密 → 写入加密文件。
//! - [`decrypt_and_verify`]：读取加密文件 → 解密 → 返回明文。
//! - [`parse_encrypted_format`]：解析文件头，返回 [`EncryptionHeader`]。
//!
//! ## 加密文件格式
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ Magic Header (8 bytes)  - "ENCLOG1\0"   │
//! │ Version (2 bytes)       - 0x0001        │
//! │ Algorithm ID (2 bytes)  - 0x0001 (AES)  │
//! │ Nonce (12 bytes)        - 随机/文件唯一 │
//! │ Encrypted Data (可变)    - AES-GCM 密文 │
//! │ Auth Tag (16 bytes)     - GCM 认证标签  │
//! └─────────────────────────────────────────┘
//! ```

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use anyhow::{anyhow, Result};
use rand::Rng;
use std::io::Write;
use std::path::Path;

/// 加密文件魔数（8 字节，含结尾 NUL）。
pub const MAGIC_HEADER: &[u8] = b"ENCLOG1\0";
/// 当前加密格式版本。
pub const VERSION: u16 = 1;
/// AES-256-GCM 算法标识。
pub const ALGO_AES_GCM: u16 = 1;
/// 文件头总长度（magic 8 + version 2 + algo 2 + nonce 12）。
pub const HEADER_LEN: usize = 24;
/// GCM 认证标签长度（字节）。
pub const AUTH_TAG_LEN: usize = 16;

/// 解析后的加密文件头。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionHeader {
    /// 8 字节魔数，应为 `b"ENCLOG1\0"`。
    pub magic: [u8; 8],
    /// 文件格式版本（小端）。
    pub version: u16,
    /// 算法 ID（小端），1 = AES-256-GCM。
    pub algo: u16,
    /// 12 字节 GCM Nonce。
    pub nonce: [u8; 12],
}

impl EncryptionHeader {
    /// 判断魔数是否匹配 inklog 加密格式。
    pub fn is_valid_magic(&self) -> bool {
        self.magic == MAGIC_HEADER
    }

    /// 判断算法是否为 AES-256-GCM。
    pub fn is_aes_gcm(&self) -> bool {
        self.algo == ALGO_AES_GCM
    }
}

/// 生成临时加密密钥（Base64 编码的 32 字节随机数据）。
///
/// 返回长度固定为 44 字符的 Base64 字符串。
pub fn generate_temp_key() -> String {
    let mut key_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut key_bytes);
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD.encode(key_bytes)
}

/// 加密明文文件，写入加密文件。
///
/// # 参数
///
/// - `plaintext_path`：明文文件路径。
/// - `encrypted_path`：加密文件输出路径。
/// - `key_env`：包含 Base64 编码 32 字节密钥的环境变量名。
///
/// # 错误
///
/// - 密钥环境变量未设置或格式错误。
/// - 文件读写失败。
/// - AES 加密失败（极少见）。
pub fn encrypt_log_file(plaintext_path: &str, encrypted_path: &str, key_env: &str) -> Result<()> {
    let key = inklog::support::io::sink::encryption::get_encryption_key(key_env)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = std::fs::read(plaintext_path)?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_slice())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    let mut file = std::fs::File::create(encrypted_path)?;
    file.write_all(MAGIC_HEADER)?;
    file.write_all(&VERSION.to_le_bytes())?;
    file.write_all(&ALGO_AES_GCM.to_le_bytes())?;
    file.write_all(&nonce_bytes)?;
    file.write_all(&ciphertext)?;
    file.flush()?;
    Ok(())
}

/// 解密加密文件并返回明文。
///
/// 仅做解密，不做内容关键字断言（与 bin 的 `decrypt_and_verify` 区分）。
/// 调用方可以自行对返回的明文做内容验证。
///
/// # 错误
///
/// - 文件头不是 inklog 加密格式。
/// - 密钥环境变量未设置或格式错误。
/// - GCM 认证失败（密钥错误或数据被篡改）。
pub fn decrypt_file(encrypted_path: &str, key_env: &str) -> Result<String> {
    let header = parse_encrypted_format(encrypted_path)?;
    if !header.is_valid_magic() {
        return Err(anyhow!("Invalid file header: not an encrypted inklog file"));
    }

    let key = inklog::support::io::sink::encryption::get_encryption_key(key_env)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Nonce::from_slice(&header.nonce);

    let mut file = std::fs::File::open(encrypted_path)?;
    // 跳过文件头
    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::Start(HEADER_LEN as u64))?;
    let mut ciphertext = Vec::new();
    file.read_to_end(&mut ciphertext)?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| anyhow!("Invalid UTF-8 in decrypted data: {}", e))
}

/// 解密并验证内容包含指定关键字（与原 bin 行为一致）。
///
/// `required_keywords` 中每个关键字都必须出现在解密后的明文中，否则返回错误。
pub fn decrypt_and_verify(
    encrypted_path: &str,
    key_env: &str,
    required_keywords: &[&str],
) -> Result<String> {
    let plaintext = decrypt_file(encrypted_path, key_env)?;
    for kw in required_keywords {
        if !plaintext.contains(kw) {
            return Err(anyhow!(
                "Content validation failed: missing keyword '{}'",
                kw
            ));
        }
    }
    Ok(plaintext)
}

/// 解析加密文件头（24 字节）。
///
/// # 错误
///
/// - 文件不存在或小于 24 字节。
pub fn parse_encrypted_format(file_path: &str) -> Result<EncryptionHeader> {
    use std::io::Read;
    let path = Path::new(file_path);
    let mut file = std::fs::File::open(path)?;
    let mut header_buf = [0u8; HEADER_LEN];
    file.read_exact(&mut header_buf)?;

    let mut magic = [0u8; 8];
    magic.copy_from_slice(&header_buf[..8]);
    let version = u16::from_le_bytes([header_buf[8], header_buf[9]]);
    let algo = u16::from_le_bytes([header_buf[10], header_buf[11]]);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&header_buf[12..24]);

    Ok(EncryptionHeader {
        magic,
        version,
        algo,
        nonce,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // 环境变量是进程级全局状态，测试串行执行避免互相覆盖。
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn write_plaintext(path: &str, content: &str) {
        std::fs::write(path, content).expect("写入明文文件失败");
    }

    #[test]
    fn test_generate_temp_key_length() {
        // 验证：Base64 编码的 32 字节 → 44 字符（无 padding 也是 44，因 32 字节正好整除 3）。
        let key = generate_temp_key();
        assert_eq!(key.len(), 44, "Base64(32 bytes) 应为 44 字符");
        // 解码后必须是 32 字节
        use base64::{engine::general_purpose, Engine as _};
        let decoded = general_purpose::STANDARD
            .decode(&key)
            .expect("Base64 解码失败");
        assert_eq!(decoded.len(), 32, "解码后必须是 32 字节密钥");
    }

    #[test]
    fn test_generate_temp_key_uniqueness() {
        // 验证：两次生成应不同（极小概率相同，视为不同）。
        let a = generate_temp_key();
        let b = generate_temp_key();
        assert_ne!(a, b, "两次生成的密钥应不同");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // 验证：加密后能解密还原为原文。
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let plain = dir.path().join("plain.log");
        let enc = dir.path().join("plain.log.enc");
        let plain_path = plain.to_str().unwrap();
        let enc_path = enc.to_str().unwrap();

        let content =
            "2026-03-19 [INFO] encryption - 这是一条加密的日志消息\n敏感数据会被自动加密保护\n";
        write_plaintext(plain_path, content);

        let key = generate_temp_key();
        std::env::set_var("LOG_ENCRYPTION_KEY", &key);

        encrypt_log_file(plain_path, enc_path, "LOG_ENCRYPTION_KEY").expect("加密失败");
        let decrypted = decrypt_file(enc_path, "LOG_ENCRYPTION_KEY").expect("解密失败");

        assert_eq!(decrypted, content, "解密结果应与原文一致");

        std::env::remove_var("LOG_ENCRYPTION_KEY");
    }

    #[test]
    fn test_parse_encrypted_format() {
        // 验证：解析文件头返回正确的 magic/version/algo/nonce。
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let plain = dir.path().join("plain.log");
        let enc = dir.path().join("plain.log.enc");

        write_plaintext(plain.to_str().unwrap(), "hello world");
        let key = generate_temp_key();
        std::env::set_var("LOG_ENCRYPTION_KEY", &key);
        encrypt_log_file(
            plain.to_str().unwrap(),
            enc.to_str().unwrap(),
            "LOG_ENCRYPTION_KEY",
        )
        .expect("加密失败");

        let header = parse_encrypted_format(enc.to_str().unwrap()).expect("解析失败");
        assert!(header.is_valid_magic(), "magic 应匹配 ENCLOG1\\0");
        assert_eq!(header.version, VERSION);
        assert!(header.is_aes_gcm(), "算法应为 AES-256-GCM");
        // nonce 不应全零（极小概率）
        assert!(header.nonce.iter().any(|&b| b != 0), "nonce 不应全零");

        std::env::remove_var("LOG_ENCRYPTION_KEY");
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        // 验证：用错误密钥解密应失败（GCM 认证标签不匹配）。
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let plain = dir.path().join("plain.log");
        let enc = dir.path().join("plain.log.enc");

        write_plaintext(plain.to_str().unwrap(), "secret content");
        let key1 = generate_temp_key();
        std::env::set_var("LOG_ENCRYPTION_KEY", &key1);
        encrypt_log_file(
            plain.to_str().unwrap(),
            enc.to_str().unwrap(),
            "LOG_ENCRYPTION_KEY",
        )
        .expect("加密失败");

        // 用另一个密钥解密
        let key2 = generate_temp_key();
        std::env::set_var("LOG_ENCRYPTION_KEY", &key2);
        let result = decrypt_file(enc.to_str().unwrap(), "LOG_ENCRYPTION_KEY");
        assert!(result.is_err(), "用错误密钥解密应失败");
        // 错误信息应包含 "Decryption failed"
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Decryption failed"),
            "错误信息应说明解密失败，实际: {}",
            err_msg
        );

        std::env::remove_var("LOG_ENCRYPTION_KEY");
    }

    #[test]
    fn test_decrypt_invalid_header_fails() {
        // 验证：无效文件头（魔数错误）应失败。
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let bad = dir.path().join("bad.log.enc");
        // 写入 24 字节但魔数错误
        let mut content = vec![0u8; 24];
        content[..8].copy_from_slice(b"BADHEAD\0");
        std::fs::write(&bad, &content).unwrap();

        let result = decrypt_file(bad.to_str().unwrap(), "LOG_ENCRYPTION_KEY");
        assert!(result.is_err(), "无效文件头应导致失败");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid file header"),
            "错误信息应说明文件头无效"
        );
    }

    #[test]
    fn test_decrypt_short_file_fails() {
        // 验证：文件小于 24 字节（文件头长度）应失败。
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let short = dir.path().join("short.log.enc");
        std::fs::write(&short, b"too short").unwrap();

        let result = parse_encrypted_format(short.to_str().unwrap());
        assert!(result.is_err(), "过短文件应解析失败");
    }

    #[test]
    fn test_decrypt_and_verify_keywords() {
        // 验证：decrypt_and_verify 在关键字缺失时失败，全部命中时成功。
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let plain = dir.path().join("plain.log");
        let enc = dir.path().join("plain.log.enc");

        let content = "包含 keyword1 和 keyword2 的明文";
        write_plaintext(plain.to_str().unwrap(), content);
        let key = generate_temp_key();
        std::env::set_var("LOG_ENCRYPTION_KEY", &key);
        encrypt_log_file(
            plain.to_str().unwrap(),
            enc.to_str().unwrap(),
            "LOG_ENCRYPTION_KEY",
        )
        .expect("加密失败");

        // 全部命中 → 成功
        let ok = decrypt_and_verify(
            enc.to_str().unwrap(),
            "LOG_ENCRYPTION_KEY",
            &["keyword1", "keyword2"],
        );
        assert!(ok.is_ok(), "关键字全部命中应成功");

        // 缺失 → 失败
        let missing = decrypt_and_verify(
            enc.to_str().unwrap(),
            "LOG_ENCRYPTION_KEY",
            &["nonexistent"],
        );
        assert!(missing.is_err(), "关键字缺失应失败");

        std::env::remove_var("LOG_ENCRYPTION_KEY");
    }
}
