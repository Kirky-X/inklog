// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 加密相关工具模块
//!
//! 提供文件加密所需的密钥派生和加密功能

use crate::error::InklogError;
use base64::{engine::general_purpose, Engine as _};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use zeroize::Zeroizing;

/// 从环境变量获取加密密钥
///
/// 支持以下格式：
/// - Base64 编码的 32 字节密钥
/// - 原始 32 字节密钥
/// - 密码字符串（1-127 字符），使用 PBKDF2 派生
///
/// # 参数
///
/// * `env_var` - 环境变量名称
///
/// # 返回值
///
/// 返回 32 字节的加密密钥
///
/// # 错误
///
/// 如果环境变量未设置、密钥格式无效或长度不正确，返回错误
pub fn get_encryption_key(env_var: &str) -> Result<[u8; 32], InklogError> {
    // 使用 Zeroizing 安全读取环境变量，防止密钥驻留内存
    let env_value = Zeroizing::new(std::env::var(env_var).map_err(|_| {
        InklogError::ConfigError(
            "Encryption key environment variable not set. Please configure INKLOG_ENCRYPTION_KEY."
                .to_string(),
        )
    })?);

    let raw_bytes = env_value.as_bytes();

    // 如果长度是32字节，尝试直接使用原始字节
    if raw_bytes.len() == 32 {
        let mut result = [0u8; 32];
        result.copy_from_slice(raw_bytes);
        return Ok(result);
    }

    // 尝试解码 Base64 编码的密钥
    if let Ok(decoded) = general_purpose::STANDARD.decode(env_value.as_str()) {
        if decoded.len() == 32 {
            let mut result = [0u8; 32];
            result.copy_from_slice(&decoded);
            return Ok(result);
        }
        // Base64 解码成功但长度不对，拒绝使用
        return Err(InklogError::ConfigError(format!(
            "Encryption key from Base64 must be exactly 32 bytes (256 bits), got {} bytes. \
             Please provide a valid 32-byte key encoded in Base64.",
            decoded.len()
        )));
    }

    // 如果长度不是32字节，尝试使用 PBKDF2 从密码派生密钥
    if !raw_bytes.is_empty() && raw_bytes.len() < 128 {
        return derive_key_from_password(env_value.as_str(), None);
    }

    // 密钥长度无效
    Err(InklogError::ConfigError(format!(
        "Encryption key must be exactly 32 bytes (256 bits) for raw keys, or a password string (1-127 chars) for key derivation. Got {} bytes. \
         Please provide a valid 32-byte key in raw or Base64 format, or use a password string.",
        raw_bytes.len()
    )))
}

/// 使用 PBKDF2 从密码派生加密密钥
///
/// # 参数
///
/// * `password` - 密码字符串
/// * `salt` - 可选的盐值，如果为 None 则使用默认盐
///
/// # 返回值
///
/// 返回 32 字节的派生密钥
pub fn derive_key_from_password(
    password: &str,
    salt: Option<&[u8]>,
) -> Result<[u8; 32], InklogError> {
    let mut key = [0u8; 32];

    let salt = salt.unwrap_or(b"inklog-encryption-salt-v1");

    // 使用 PBKDF2-HMAC-SHA256 派生密钥
    pbkdf2_hmac::<Sha256>(
        password.as_bytes(),
        salt,
        100_000, // 迭代次数，增加计算成本
        &mut key,
    );

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_encryption_key_from_base64() {
        let key_b64 = "MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE=";
        std::env::set_var("INKLOG_TEST_KEY", key_b64);
        let result = get_encryption_key("INKLOG_TEST_KEY");
        std::env::remove_var("INKLOG_TEST_KEY");
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_get_encryption_key_from_raw_bytes() {
        let key_raw = "abcdefghijklmnopqrstuvwxyz123456";
        std::env::set_var("INKLOG_TEST_KEY", key_raw);
        let result = get_encryption_key("INKLOG_TEST_KEY");
        std::env::remove_var("INKLOG_TEST_KEY");
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_get_encryption_key_missing() {
        std::env::remove_var("INKLOG_NONEXISTENT_KEY");
        let result = get_encryption_key("INKLOG_NONEXISTENT_KEY");
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_key_from_password() {
        let result = derive_key_from_password("test_password", Some(b"test_salt"));
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_derive_key_deterministic() {
        let result1 = derive_key_from_password("password", Some(b"salt"));
        let result2 = derive_key_from_password("password", Some(b"salt"));
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_derive_key_different_salts() {
        let result1 = derive_key_from_password("password", Some(b"salt1"));
        let result2 = derive_key_from_password("password", Some(b"salt2"));
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_ne!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_derive_key_different_passwords() {
        let result1 = derive_key_from_password("password1", Some(b"salt"));
        let result2 = derive_key_from_password("password2", Some(b"salt"));
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_ne!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_derive_key_with_default_salt() {
        let result = derive_key_from_password("test_password", None);
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 32);
    }
}
