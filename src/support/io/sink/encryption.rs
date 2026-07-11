// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 加密相关工具模块
//!
//! 提供文件加密所需的密钥派生和加密功能

use crate::InklogError;
use base64::{Engine as _, engine::general_purpose};
use pbkdf2::pbkdf2_hmac;
use rand::Rng;
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
        let (key, _salt) = derive_key_from_password(env_value.as_str(), None)?;
        return Ok(key);
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
/// * `salt` - 可选的盐值，如果为 None 则生成随机盐值
///
/// # 返回值
///
/// 返回 `(32 字节的派生密钥, 使用的盐值)` 元组
pub fn derive_key_from_password(
    password: &str,
    salt: Option<&[u8]>,
) -> Result<([u8; 32], Vec<u8>), InklogError> {
    let mut key = [0u8; 32];

    let salt: Vec<u8> = match salt {
        Some(s) => s.to_vec(),
        None => {
            // 生成 16 字节的随机盐值
            let mut salt_bytes = vec![0u8; 16];
            rand::rng().fill_bytes(&mut salt_bytes);
            salt_bytes
        }
    };

    // 使用 PBKDF2-HMAC-SHA256 派生密钥
    pbkdf2_hmac::<Sha256>(
        password.as_bytes(),
        &salt,
        100_000, // 迭代次数
        &mut key,
    );

    Ok((key, salt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_encryption_key_from_base64() {
        let key_b64 = "MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE=";
        unsafe {
            std::env::set_var("INKLOG_TEST_KEY", key_b64);
        }
        let result = get_encryption_key("INKLOG_TEST_KEY");
        unsafe {
            std::env::remove_var("INKLOG_TEST_KEY");
        }
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_get_encryption_key_from_raw_bytes() {
        let key_raw = "abcdefghijklmnopqrstuvwxyz123456";
        unsafe {
            std::env::set_var("INKLOG_TEST_KEY", key_raw);
        }
        let result = get_encryption_key("INKLOG_TEST_KEY");
        unsafe {
            std::env::remove_var("INKLOG_TEST_KEY");
        }
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_get_encryption_key_missing() {
        unsafe {
            std::env::remove_var("INKLOG_NONEXISTENT_KEY");
        }
        let result = get_encryption_key("INKLOG_NONEXISTENT_KEY");
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_key_from_password() {
        let result = derive_key_from_password("test_password", Some(b"test_salt"));
        assert!(result.is_ok());
        let (key, salt) = result.unwrap();
        assert_eq!(key.len(), 32);
        assert_eq!(salt, b"test_salt");
    }

    #[test]
    fn test_derive_key_deterministic() {
        let result1 = derive_key_from_password("password", Some(b"salt"));
        let result2 = derive_key_from_password("password", Some(b"salt"));
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap().0, result2.unwrap().0);
    }

    #[test]
    fn test_derive_key_different_salts() {
        let result1 = derive_key_from_password("password", Some(b"salt1"));
        let result2 = derive_key_from_password("password", Some(b"salt2"));
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_ne!(result1.unwrap().0, result2.unwrap().0);
    }

    #[test]
    fn test_derive_key_different_passwords() {
        let result1 = derive_key_from_password("password1", Some(b"salt"));
        let result2 = derive_key_from_password("password2", Some(b"salt"));
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_ne!(result1.unwrap().0, result2.unwrap().0);
    }

    #[test]
    fn test_derive_key_with_random_salt() {
        let (key1, salt1) = derive_key_from_password("test_password", None).unwrap();
        assert_eq!(key1.len(), 32);
        assert_eq!(salt1.len(), 16); // 随机生成的盐值应该是 16 字节

        // 使用相同密码再次调用，应该得到不同的盐值和密钥
        let (key2, salt2) = derive_key_from_password("test_password", None).unwrap();
        assert_ne!(salt1, salt2); // 盐值应该不同
        assert_ne!(key1, key2); // 由于盐值不同，密钥也应该不同
    }

    #[test]
    fn test_get_encryption_key_from_password() {
        // Test PBKDF2 password derivation branch (1-127 chars)
        unsafe {
            std::env::set_var("INKLOG_TEST_PWD_DERIVE", "my_password");
        }
        let result = get_encryption_key("INKLOG_TEST_PWD_DERIVE");
        unsafe {
            std::env::remove_var("INKLOG_TEST_PWD_DERIVE");
        }
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_get_encryption_key_base64_wrong_length() {
        // Base64 decodes successfully but length is not 32 bytes
        // Use a valid Base64 string that decodes to 16 bytes (not 32)
        use base64::{Engine as _, engine::general_purpose};
        let key_16_bytes = [0u8; 16];
        let b64 = general_purpose::STANDARD.encode(key_16_bytes);
        unsafe {
            std::env::set_var("INKLOG_TEST_B64_WRONG_LEN", &b64);
        }
        let result = get_encryption_key("INKLOG_TEST_B64_WRONG_LEN");
        unsafe {
            std::env::remove_var("INKLOG_TEST_B64_WRONG_LEN");
        }
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("32 bytes") || err_msg.contains("256 bits"));
    }

    #[test]
    fn test_get_encryption_key_too_long_input() {
        // Input longer than 127 bytes should return error
        let long_password = "a".repeat(128);
        unsafe {
            std::env::set_var("INKLOG_TEST_TOO_LONG", &long_password);
        }
        let result = get_encryption_key("INKLOG_TEST_TOO_LONG");
        unsafe {
            std::env::remove_var("INKLOG_TEST_TOO_LONG");
        }
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("32 bytes") || err_msg.contains("password"));
    }

    #[test]
    fn test_get_encryption_key_empty_string() {
        // Empty string should return error (is_empty check)
        unsafe {
            std::env::set_var("INKLOG_TEST_EMPTY", "");
        }
        let result = get_encryption_key("INKLOG_TEST_EMPTY");
        unsafe {
            std::env::remove_var("INKLOG_TEST_EMPTY");
        }
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_key_with_empty_password() {
        // Empty password should still work (PBKDF2 allows it)
        let result = derive_key_from_password("", Some(b"salt"));
        assert!(result.is_ok());
        let (key, _) = result.unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_derive_key_with_long_salt() {
        let long_salt = vec![0u8; 64];
        let result = derive_key_from_password("password", Some(&long_salt));
        assert!(result.is_ok());
        let (key, salt) = result.unwrap();
        assert_eq!(key.len(), 32);
        assert_eq!(salt.len(), 64);
    }

    #[test]
    fn test_get_encryption_key_long_non_base64_input() {
        // 覆盖行 75-78: 长度 >= 128 且不是有效 Base64 时返回错误
        // 使用 128 个 '!' 字符（非 Base64 字符），确保到达最后的 Err 分支
        let long_non_base64 = "!".repeat(128);
        unsafe {
            std::env::set_var("INKLOG_TEST_LONG_NON_B64", &long_non_base64);
        }
        let result = get_encryption_key("INKLOG_TEST_LONG_NON_B64");
        unsafe {
            std::env::remove_var("INKLOG_TEST_LONG_NON_B64");
        }
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("32 bytes") || err_msg.contains("password"),
            "error should mention 32 bytes or password, got: {}",
            err_msg
        );
    }
}
