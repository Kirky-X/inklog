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

/// PBKDF2-HMAC-SHA256 迭代次数。
///
/// 单一事实来源：派生路径与守卫测试都引用本常量。
/// 600,000 为 OWASP 对 PBKDF2-HMAC-SHA256 的推荐值；调低前必须同步
/// 评估兼容性与安全影响，并更新 [`test_pbkdf2_iteration_count_is_at_least_600k`]。
pub(crate) const PBKDF2_ITERATIONS: u32 = 600_000;

/// 从环境变量获取加密密钥
///
/// 支持以下格式：
/// - Base64 编码的 32 字节密钥
/// - 原始 32 字节密钥
/// - 密码字符串（1-127 字符），使用 PBKDF2 派生
///
/// > 注意：密码分支使用**随机生成**的盐值且盐值不随返回值暴露，因此本函数
/// > 派生结果不可复现。加密文件（v2 头存盐）请使用
/// > [`get_encryption_key_with_salt`]；本函数保留作为既有调用方的兼容包装。
///
/// # 参数
///
/// * `env_var` - 环境变量名称
///
/// # 返回值
///
/// 返回 `Zeroizing` 包裹的 32 字节加密密钥，离开作用域时自动清零
///
/// # 错误
///
/// 如果环境变量未设置、密钥格式无效或长度不正确，返回错误
///
/// # 兼容性
///
/// 长度恰为 32 字节的输入会被**直接用作原始密钥**（不经过 PBKDF2 派生），
/// 即使内容是可读密码。这是为解密既有按此格式加密的文件而保留的兼容行为，
/// 存在语义歧义（32 字符密码与 32 字节随机密钥无法区分）。新部署应使用
/// Base64 编码的随机密钥（`openssl rand -base64 32` 生成），或长度不等于
/// 32 字节的密码。运行期遇到此路径会输出 `tracing::warn`。
pub fn get_encryption_key(env_var: &str) -> Result<Zeroizing<[u8; 32]>, InklogError> {
    let env_value = read_key_env_value(env_var)?;
    key_from_env_value(env_var, &env_value, None)
}

/// 从环境变量获取加密密钥，密码模式使用**调用方提供的盐值**做确定性派生。
///
/// 这是 [`get_encryption_key`] 的确定性变体，专用于加密文件头中存储了盐值的
/// 场景（v2 加密头）：加密方把随机盐写入文件头，解密方用同一盐重导出同一
/// 密钥。各分支行为：
/// - 原始 32 字节输入：直接用作密钥，`salt` 被忽略；
/// - Base64 编码的 32 字节密钥：解码后直接用作密钥，`salt` 被忽略；
/// - 密码字符串（1-127 字符）：以**传入的 `salt`** 调用 PBKDF2 确定性派生。
///
/// # 参数
///
/// * `env_var` - 环境变量名称
/// * `salt` - 密码派生使用的盐值（对前两个分支无影响）
///
/// # 错误
///
/// 与 [`get_encryption_key`] 一致：环境变量未设置、密钥格式无效或长度不正确
/// 时返回错误。
pub fn get_encryption_key_with_salt(
    env_var: &str,
    salt: &[u8],
) -> Result<Zeroizing<[u8; 32]>, InklogError> {
    let env_value = read_key_env_value(env_var)?;
    key_from_env_value(env_var, &env_value, Some(salt))
}

/// 判断环境变量中的密钥是否会按**密码模式**（PBKDF2 派生）处理。
///
/// 用于解密端诊断：v1 加密头未存储盐值，密码模式加密的 v1 文件的密钥无法
/// 确定性重放（解密必败）。解密方据此提前返回明确错误，而非误导性的
/// "decryption failed"。
///
/// 判定与 [`get_encryption_key`] 的分支逻辑一致：
/// - 非 32 字节、非空、长度 < 128，且**不是**合法 Base64 → 密码模式；
/// - Base64 能解码（无论长度）→ 不是密码模式（密钥路径或 base64 长度错误）；
/// - 环境变量未设置 / 恰为 32 字节 / 长度 >= 128 → 不是密码模式。
pub fn env_key_is_password(env_var: &str) -> bool {
    let Ok(value) = std::env::var(env_var) else {
        return false;
    };
    let raw = value.as_bytes();
    raw.len() != 32
        && !raw.is_empty()
        && raw.len() < 128
        && general_purpose::STANDARD.decode(value.as_str()).is_err()
}

/// 使用 Zeroizing 安全读取环境变量，防止密钥驻留内存
fn read_key_env_value(env_var: &str) -> Result<Zeroizing<String>, InklogError> {
    let value = std::env::var(env_var).map_err(|_| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("env", env_var);
        InklogError::ConfigError(crate::i18n::tr_args("config-encryption_key_not_set", args))
    })?;
    Ok(Zeroizing::new(value))
}

/// 按密钥格式分支解析出 32 字节密钥。
///
/// `salt` 仅在密码分支生效：`None` 生成随机盐（加密端兼容包装），`Some(s)`
/// 用调用方盐值确定性派生（v2 加密头场景）。
fn key_from_env_value(
    env_var: &str,
    env_value: &str,
    salt: Option<&[u8]>,
) -> Result<Zeroizing<[u8; 32]>, InklogError> {
    let raw_bytes = env_value.as_bytes();

    // 如果长度是32字节，尝试直接使用原始字节。
    // 注意语义歧义：32 字符的密码会走此分支被当作原始密钥而非 PBKDF2 派生。
    // 该行为保留用于解密既有按此格式加密的文件；新部署建议使用
    // Base64 编码的随机密钥，或长度不等于 32 字节的密码。
    if raw_bytes.len() == 32 {
        tracing::warn!(
            env = %env_var,
            "32-byte input used directly as a raw encryption key; \
             prefer a Base64-encoded random key or a password whose length is not 32"
        );
        let mut result = [0u8; 32];
        result.copy_from_slice(raw_bytes);
        return Ok(Zeroizing::new(result));
    }

    // 尝试解码 Base64 编码的密钥
    if let Ok(decoded) = general_purpose::STANDARD.decode(env_value) {
        if decoded.len() == 32 {
            let mut result = [0u8; 32];
            result.copy_from_slice(&decoded);
            return Ok(Zeroizing::new(result));
        }
        // Base64 解码成功但长度不对，拒绝使用
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("got", decoded.len());
        return Err(InklogError::ConfigError(crate::i18n::tr_args(
            "config-encryption_base64_wrong_length",
            args,
        )));
    }

    // 如果长度不是32字节，尝试使用 PBKDF2 从密码派生密钥
    // salt: None → 随机盐（历史兼容路径），Some(s) → 调用方提供的确定性盐（v2 头）
    if !raw_bytes.is_empty() && raw_bytes.len() < 128 {
        let (key, _salt) = derive_key_from_password(env_value, salt)?;
        return Ok(Zeroizing::new(key));
    }

    // 密钥长度无效
    let mut args = fluent_bundle::FluentArgs::new();
    args.set("got", raw_bytes.len());
    Err(InklogError::ConfigError(crate::i18n::tr_args(
        "config-encryption_key_wrong_length",
        args,
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
    // Security: enforce minimum password length
    if password.len() < 12 {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("got", password.len());
        return Err(InklogError::ConfigError(crate::i18n::tr_args(
            "config-encryption_password_too_short",
            args,
        )));
    }

    // Warn about weak passwords
    if password.len() < 16 {
        tracing::warn!("{}", crate::i18n::tr("warn-weak_password"));
    }

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
        PBKDF2_ITERATIONS,
        &mut key,
    );

    Ok((key, salt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
        let result1 = derive_key_from_password("password1234", Some(b"salt"));
        let result2 = derive_key_from_password("password1234", Some(b"salt"));
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap().0, result2.unwrap().0);
    }

    #[test]
    fn test_derive_key_different_salts() {
        let result1 = derive_key_from_password("password1234", Some(b"salt1"));
        let result2 = derive_key_from_password("password1234", Some(b"salt2"));
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_ne!(result1.unwrap().0, result2.unwrap().0);
    }

    #[test]
    fn test_derive_key_different_passwords() {
        let result1 = derive_key_from_password("password1234a", Some(b"salt"));
        let result2 = derive_key_from_password("password1234b", Some(b"salt"));
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
            std::env::set_var("INKLOG_TEST_PWD_DERIVE", "my_password12");
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
        // Empty password should fail minimum length check
        let result = derive_key_from_password("", Some(b"salt"));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("at least 12 characters"));
    }

    #[test]
    fn test_derive_key_with_short_password() {
        // Password shorter than 12 chars should fail
        let result = derive_key_from_password("short", Some(b"salt"));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("at least 12 characters"));
    }

    #[test]
    fn test_derive_key_minimum_length() {
        // Exactly 12 chars should pass
        let result = derive_key_from_password("123456789012", Some(b"salt"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_derive_key_with_long_salt() {
        let long_salt = vec![0u8; 64];
        let result = derive_key_from_password("password1234", Some(&long_salt));
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

    #[test]
    fn test_pbkdf2_iteration_count_is_at_least_600k() {
        // (a) 常量本身不得低于 OWASP 对 PBKDF2-HMAC-SHA256 的推荐值（600,000）。
        // 该断言对当前编译单元恒真是有意为之——它守卫的是未来对该常量的改动。
        #[allow(clippy::assertions_on_constants)]
        {
            assert!(
                PBKDF2_ITERATIONS >= 600_000,
                "PBKDF2 iterations must stay at or above the OWASP recommendation \
                 (600,000), got {PBKDF2_ITERATIONS}"
            );
        }

        // (b) 生产派生路径必须真实使用 PBKDF2_ITERATIONS：固定盐经
        // derive_key_from_password 的输出应与本地按常量重算逐字节一致。
        // 若派生路径绕开常量（内联字面量）或常量被改动，比对即失败。
        // 固定测试向量（非真实凭据），与确定性比对用途一致。
        let test_vector = "pbkdf2-guard-test-vector-01";
        let test_salt: &[u8] = b"pbkdf2-guard-salt";
        let (derived, used_salt) =
            derive_key_from_password(test_vector, Some(test_salt)).expect("derive should succeed");
        assert_eq!(
            used_salt,
            test_salt.to_vec(),
            "provided salt must be used as-is"
        );

        let mut expected = [0u8; 32];
        pbkdf2_hmac::<Sha256>(
            test_vector.as_bytes(),
            used_salt.as_slice(),
            PBKDF2_ITERATIONS,
            &mut expected,
        );
        assert_eq!(
            derived, expected,
            "derive_key_from_password must derive with PBKDF2_ITERATIONS iterations"
        );
    }

    #[test]
    fn test_minimum_password_length_is_12() {
        // 11 chars should fail
        let result = derive_key_from_password("12345678901", Some(b"salt"));
        assert!(result.is_err(), "11-char password should be rejected");

        // 12 chars should pass
        let result = derive_key_from_password("123456789012", Some(b"salt"));
        assert!(result.is_ok(), "12-char password should be accepted");
    }

    // ==================== get_encryption_key_with_salt 测试 ====================

    #[test]
    #[serial]
    fn test_get_encryption_key_with_salt_password_deterministic() {
        // Critical 修复的核心保证：密码模式 + 同一盐 → 确定性密钥，
        // 解密方用文件头中的盐重导出同一密钥。
        unsafe {
            std::env::set_var("INKLOG_TEST_KEY_WITH_SALT", "round-trip-password-01");
        }
        let salt = b"0123456789abcdef"; // 16 字节，与 v2 头中的盐长度一致
        let key1 = get_encryption_key_with_salt("INKLOG_TEST_KEY_WITH_SALT", salt).unwrap();
        let key2 = get_encryption_key_with_salt("INKLOG_TEST_KEY_WITH_SALT", salt).unwrap();
        assert_eq!(*key1, *key2, "same password + same salt must derive the same key");

        let key3 =
            get_encryption_key_with_salt("INKLOG_TEST_KEY_WITH_SALT", b"different-salt!!").unwrap();
        assert_ne!(*key1, *key3, "different salt must derive a different key");

        // 与显式 PBKDF2 结果逐字节一致（证明传入的盐被真正使用）
        let (expected, used_salt) =
            derive_key_from_password("round-trip-password-01", Some(salt)).unwrap();
        assert_eq!(used_salt, salt.to_vec());
        assert_eq!(*key1, expected);

        unsafe {
            std::env::remove_var("INKLOG_TEST_KEY_WITH_SALT");
        }
    }

    #[test]
    #[serial]
    fn test_get_encryption_key_with_salt_ignores_salt_for_base64() {
        // Base64/32 字节分支必须忽略 salt，直接返回原始密钥
        let key_bytes: [u8; 32] = core::array::from_fn(|i| (i as u8) * 7 + 3);
        let key_b64 = general_purpose::STANDARD.encode(key_bytes);
        unsafe {
            std::env::set_var("INKLOG_TEST_KEY_WITH_SALT_B64", &key_b64);
        }
        let key = get_encryption_key_with_salt("INKLOG_TEST_KEY_WITH_SALT_B64", b"ignored-salt!").unwrap();
        assert_eq!(*key, key_bytes, "Base64 branch must ignore salt");

        unsafe {
            std::env::remove_var("INKLOG_TEST_KEY_WITH_SALT_B64");
        }
    }

    #[test]
    #[serial]
    fn test_get_encryption_key_with_salt_ignores_salt_for_raw_32() {
        // 恰为 32 字节的原始输入：直接用作密钥，salt 被忽略
        let raw = "abcdefghijklmnopqrstuvwxyz123456"; // 32 bytes
        unsafe {
            std::env::set_var("INKLOG_TEST_KEY_WITH_SALT_RAW", raw);
        }
        let key = get_encryption_key_with_salt("INKLOG_TEST_KEY_WITH_SALT_RAW", b"ignored!").unwrap();
        assert_eq!(&*key, raw.as_bytes());

        unsafe {
            std::env::remove_var("INKLOG_TEST_KEY_WITH_SALT_RAW");
        }
    }

    #[test]
    fn test_get_encryption_key_with_salt_missing_env() {
        unsafe {
            std::env::remove_var("INKLOG_TEST_KEY_WITH_SALT_MISSING");
        }
        let result = get_encryption_key_with_salt("INKLOG_TEST_KEY_WITH_SALT_MISSING", b"salt");
        assert!(result.is_err());
    }

    // ==================== env_key_is_password 测试 ====================

    #[test]
    #[serial]
    fn test_env_key_is_password_classification() {
        // 密码（非 Base64、非 32 字节、1-127 字符）→ true
        unsafe {
            std::env::set_var("INKLOG_TEST_CLASSIFY_PWD", "plain-password-01");
            std::env::set_var(
                "INKLOG_TEST_CLASSIFY_B64",
                general_purpose::STANDARD.encode([0x5Au8; 32]).as_str(),
            );
            std::env::set_var("INKLOG_TEST_CLASSIFY_RAW32", "abcdefghijklmnopqrstuvwxyz123456");
            // Base64 可解码但长度不对 → 走 base64_wrong_length 错误，不是密码
            std::env::set_var(
                "INKLOG_TEST_CLASSIFY_B64_SHORT",
                general_purpose::STANDARD.encode([0u8; 16]).as_str(),
            );
        }

        assert!(env_key_is_password("INKLOG_TEST_CLASSIFY_PWD"));
        assert!(!env_key_is_password("INKLOG_TEST_CLASSIFY_B64"));
        assert!(!env_key_is_password("INKLOG_TEST_CLASSIFY_RAW32"));
        assert!(!env_key_is_password("INKLOG_TEST_CLASSIFY_B64_SHORT"));
        assert!(!env_key_is_password("INKLOG_TEST_CLASSIFY_MISSING"));

        unsafe {
            for var in [
                "INKLOG_TEST_CLASSIFY_PWD",
                "INKLOG_TEST_CLASSIFY_B64",
                "INKLOG_TEST_CLASSIFY_RAW32",
                "INKLOG_TEST_CLASSIFY_B64_SHORT",
            ] {
                std::env::remove_var(var);
            }
        }
    }
}
