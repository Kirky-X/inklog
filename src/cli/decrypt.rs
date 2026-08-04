// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit};
use anyhow::{Context, Result, anyhow};
#[cfg(test)]
use base64::{Engine as _, engine::general_purpose};
#[cfg(test)]
use inklog::sink::encryption::derive_key_from_password;
#[cfg(test)]
use sha2::Digest as Sha256Digest;
#[cfg(test)]
use sha2::Sha256;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// 检查路径中是否包含可疑字符或遍历模式（共享逻辑）
fn check_path_syntax(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();
    let suspicious_chars = ['~', '\0', '\u{2024}', '\u{2025}', '\u{FE52}'];
    for c in path_str.chars() {
        if suspicious_chars.contains(&c) {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("path", path.display().to_string());
            return Err(anyhow!(
                "{}",
                inklog::i18n::tr_args("cli-decrypt-err-path-char", args)
            ));
        }
    }
    let path_str_lower = path_str.to_lowercase();
    if path_str_lower.contains("..") || path_str_lower.contains("~/") {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", path.display().to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-traversal", args)
        ));
    }
    Ok(())
}

/// 验证文件路径是否在允许的目录内，防止路径遍历攻击。
/// 要求路径已存在（用于输入路径验证）。
fn validate_file_path(file_path: &Path, base_dir: &Path) -> Result<()> {
    check_path_syntax(file_path)?;

    // 检查符号链接 — 必须在 canonicalize 之前执行，防止 TOCTOU 竞态
    if let Ok(metadata) = file_path.symlink_metadata()
        && metadata.file_type().is_symlink()
    {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", file_path.display().to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-symlink", args)
        ));
    }

    // 规范化路径
    let canonical_path = file_path.canonicalize().map_err(|e| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("err", e.to_string());
        anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-canonical", args)
        )
    })?;

    let canonical_base = base_dir.canonicalize().map_err(|e| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("err", e.to_string());
        anyhow!("{}", inklog::i18n::tr_args("cli-decrypt-err-base", args))
    })?;

    if !canonical_path.starts_with(&canonical_base) {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", file_path.display().to_string());
        args.set("base", base_dir.display().to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-traversal-detail", args)
        ));
    }

    Ok(())
}

/// 验证输出路径安全性（不要求路径已存在）。
/// 通过验证父目录的 canonicalize 结果来确保输出在 base_dir 内。
fn validate_output_path(output_path: &Path, base_dir: &Path) -> Result<()> {
    check_path_syntax(output_path)?;

    // 验证文件名部分不含路径遍历
    if let Some(file_name) = output_path.file_name() {
        let name_str = file_name.to_string_lossy();
        if name_str.contains('\0') || name_str.contains('/') || name_str.contains('\\') {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("path", output_path.display().to_string());
            return Err(anyhow!(
                "{}",
                inklog::i18n::tr_args("cli-decrypt-err-output-name", args)
            ));
        }
    }

    // 验证父目录：canonicalize 父目录（应已存在）并检查前缀
    let parent = output_path
        .parent()
        .ok_or_else(|| anyhow!("{}", inklog::i18n::tr("cli-decrypt-err-no-parent")))?;

    let canonical_parent = parent.canonicalize().map_err(|e| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", parent.display().to_string());
        args.set("err", e.to_string());
        anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-canonical-parent", args)
        )
    })?;

    let canonical_base = base_dir.canonicalize().map_err(|e| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", base_dir.display().to_string());
        args.set("err", e.to_string());
        anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-canonical-base", args)
        )
    })?;

    if !canonical_parent.starts_with(&canonical_base) {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", output_path.display().to_string());
        args.set("base", base_dir.display().to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-traversal-output", args)
        ));
    }

    Ok(())
}

/// 验证 glob 模式是否安全
fn validate_glob_pattern(pattern: &str) -> Result<()> {
    // 检查绝对路径
    if pattern.starts_with('/') || pattern.starts_with('\\') {
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr("cli-decrypt-err-glob-absolute")
        ));
    }

    // 检查路径遍历
    if pattern.contains("..") || pattern.contains("~") {
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr("cli-decrypt-err-glob-traversal")
        ));
    }

    // 检查可疑字符（包括 Unicode 变体）
    let suspicious_chars = ['\0', '\u{2024}', '\u{2025}', '\u{FE52}'];
    for c in pattern.chars() {
        if suspicious_chars.contains(&c) {
            return Err(anyhow!("{}", inklog::i18n::tr("cli-decrypt-err-glob-char")));
        }
    }

    // 尝试解析为路径，确保不包含危险元素
    let path = Path::new(pattern);
    if path.is_absolute() {
        return Err(anyhow!("{}", inklog::i18n::tr("cli-decrypt-err-glob-abs")));
    }

    // 检查组件
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(anyhow!(
                    "{}",
                    inklog::i18n::tr("cli-decrypt-err-glob-parent")
                ));
            }
            std::path::Component::Prefix(_) => {
                return Err(anyhow!(
                    "{}",
                    inklog::i18n::tr("cli-decrypt-err-glob-prefix")
                ));
            }
            std::path::Component::RootDir => {
                return Err(anyhow!("{}", inklog::i18n::tr("cli-decrypt-err-glob-root")));
            }
            _ => {}
        }
    }

    Ok(())
}

const MAGIC_HEADER: &[u8] = b"ENCLOG1\0";

/// Decrypt a single encrypted file (legacy format).
///
/// Supports the original header-less encryption format.
#[cfg(test)]
pub fn decrypt_file(input_path: &PathBuf, output_path: &PathBuf, key_env: &str) -> Result<()> {
    let mut file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {}", input_path.display()))?;

    let mut header = [0u8; 24];
    file.read_exact(&mut header)
        .with_context(|| "Failed to read file header")?;

    if &header[..8] != MAGIC_HEADER {
        return Err(anyhow!("Invalid file header: not an encrypted inklog file"));
    }

    let version = u16::from_le_bytes([header[8], header[9]]);
    if version != 1 {
        return Err(anyhow!("Unsupported file version: {}", version));
    }

    let algo = u16::from_le_bytes([header[10], header[11]]);
    if algo != 1 {
        return Err(anyhow!("Unsupported encryption algorithm: {}", algo));
    }

    let key = get_encryption_key_cli(key_env)
        .with_context(|| format!("Failed to get encryption key from env var: {}", key_env))?;

    let nonce_arr: [u8; 12] = header[12..24]
        .try_into()
        .expect("nonce slice must be 12 bytes");
    let nonce = aes_gcm::Nonce::from(nonce_arr);

    let mut ciphertext = Vec::new();
    file.read_to_end(&mut ciphertext)
        .with_context(|| "Failed to read ciphertext")?;

    let cipher = Aes256Gcm::new((&key).into());

    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    let mut output_file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    output_file
        .write_all(&plaintext)
        .with_context(|| "Failed to write decrypted data")?;

    Ok(())
}

pub fn decrypt_file_compatible(
    input_path: &PathBuf,
    output_path: &PathBuf,
    key_env: &str,
) -> Result<()> {
    let mut file = File::open(input_path).with_context(|| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", input_path.display().to_string());
        inklog::i18n::tr_args("cli-decrypt-err-open", args)
    })?;

    let mut header = [0u8; 24];
    let read_count = file
        .read(&mut header)
        .with_context(|| inklog::i18n::tr("cli-decrypt-err-read-header"))?;

    if read_count < 10 {
        return Err(anyhow!("{}", inklog::i18n::tr("cli-decrypt-err-small")));
    }

    if &header[..8] != MAGIC_HEADER {
        return Err(anyhow!("{}", inklog::i18n::tr("cli-decrypt-err-header")));
    }

    let version = u16::from_le_bytes([header[8], header[9]]);
    if version != 1 {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("version", version.to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-version", args)
        ));
    }

    let key = get_encryption_key_cli(key_env).with_context(|| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("env", key_env.to_string());
        inklog::i18n::tr_args("cli-decrypt-err-key", args)
    })?;

    let algo = u16::from_le_bytes([header[10], header[11]]);
    let plaintext = if algo == 1 {
        if read_count < 24 {
            return Err(anyhow!("{}", inklog::i18n::tr("cli-decrypt-err-small-v1")));
        }
        let nonce_slice: [u8; 12] = header[12..24].try_into().unwrap();
        let nonce = aes_gcm::Nonce::from(nonce_slice);

        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext)
            .with_context(|| inklog::i18n::tr("cli-decrypt-err-read-cipher"))?;

        let cipher = Aes256Gcm::new((&key).into());
        cipher.decrypt(&nonce, ciphertext.as_ref()).map_err(|e| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("err", e.to_string());
            anyhow!("{}", inklog::i18n::tr_args("cli-decrypt-err-decrypt", args))
        })?
    } else {
        // Assume Legacy format (MAGIC + VER + NONCE + CIPHERTEXT)
        // Legacy header is 22 bytes (8 MAGIC + 2 VER + 12 NONCE)
        if read_count < 22 {
            return Err(anyhow!("{}", inklog::i18n::tr("cli-decrypt-err-small")));
        }

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes.copy_from_slice(&header[10..22]);
        let nonce = aes_gcm::Nonce::from(nonce_bytes);

        let mut ciphertext = Vec::new();
        // If we read more than 22 bytes, the extras are part of the ciphertext
        if read_count > 22 {
            ciphertext.extend_from_slice(&header[22..read_count]);
        }
        file.read_to_end(&mut ciphertext)
            .with_context(|| inklog::i18n::tr("cli-decrypt-err-read-cipher"))?;

        let cipher = Aes256Gcm::new((&key).into());
        cipher.decrypt(&nonce, ciphertext.as_ref()).map_err(|e| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("err", e.to_string());
            anyhow!("{}", inklog::i18n::tr_args("cli-decrypt-err-decrypt", args))
        })?
    };

    let mut output_file = File::create(output_path).with_context(|| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", output_path.display().to_string());
        inklog::i18n::tr_args("cli-decrypt-err-create", args)
    })?;

    output_file
        .write_all(&plaintext)
        .with_context(|| inklog::i18n::tr("cli-decrypt-err-write"))?;

    Ok(())
}

fn get_encryption_key_cli(env_var: &str) -> Result<[u8; 32]> {
    inklog::sink::encryption::get_encryption_key(env_var).map_err(|e| anyhow!("{}", e))
}

pub fn decrypt_directory_compatible(
    input_dir: &PathBuf,
    output_dir: &PathBuf,
    key_env: &str,
    recursive: bool,
) -> Result<()> {
    if !input_dir.exists() {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", input_dir.display().to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-input-dir", args)
        ));
    }

    // 先创建输出目录，再验证（canonicalize 要求目录存在）
    std::fs::create_dir_all(output_dir).with_context(|| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", output_dir.display().to_string());
        inklog::i18n::tr_args("cli-decrypt-err-create-dir", args)
    })?;

    // 验证已存在的输出目录路径安全
    if let Err(e) = validate_file_path(output_dir, output_dir) {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("err", e.to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-output-dir", args)
        ));
    }

    let entries = std::fs::read_dir(input_dir).with_context(|| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", input_dir.display().to_string());
        inklog::i18n::tr_args("cli-decrypt-err-read-dir", args)
    })?;

    let mut failure_count = 0u32;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension()
                && ext == "enc"
            {
                let file_name = path.file_name().ok_or_else(|| {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("path", path.display().to_string());
                    anyhow!(
                        "{}",
                        inklog::i18n::tr_args("cli-decrypt-err-no-filename", args)
                    )
                })?;
                let output_path = output_dir.join(file_name).with_extension("log");

                // 验证输出路径（不要求文件已存在）
                if let Err(e) = validate_output_path(&output_path, output_dir) {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("path", output_path.display().to_string());
                    args.set("err", e.to_string());
                    eprintln!("{}", inklog::i18n::tr_args("cli-decrypt-path-fail", args));
                    failure_count += 1;
                    continue;
                }

                let mut args = fluent_bundle::FluentArgs::new();
                args.set("input", path.display().to_string());
                args.set("output", output_path.display().to_string());
                println!("{}", inklog::i18n::tr_args("cli-decrypt-progress", args));

                if let Err(e) = decrypt_file_compatible(&path, &output_path, key_env) {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("path", path.display().to_string());
                    args.set("err", e.to_string());
                    eprintln!("{}", inklog::i18n::tr_args("cli-decrypt-fail", args));
                    failure_count += 1;
                }
            }
        } else if recursive && path.is_dir() {
            let file_name = path.file_name().ok_or_else(|| {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("path", path.display().to_string());
                anyhow!(
                    "{}",
                    inklog::i18n::tr_args("cli-decrypt-err-no-filename", args)
                )
            })?;
            let sub_output_dir = output_dir.join(file_name);

            // 验证子目录输出路径（不要求目录已存在）
            if let Err(e) = validate_output_path(&sub_output_dir, output_dir) {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("path", sub_output_dir.display().to_string());
                args.set("err", e.to_string());
                eprintln!("{}", inklog::i18n::tr_args("cli-decrypt-path-fail", args));
                failure_count += 1;
                continue;
            }

            decrypt_directory_compatible(&path, &sub_output_dir, key_env, recursive)?;
        }
    }

    if failure_count > 0 {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("count", failure_count.to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-partial", args)
        ));
    }
    Ok(())
}

pub fn batch_decrypt(input_pattern: &str, output_dir: &PathBuf, key_env: &str) -> Result<()> {
    // 验证 glob 模式安全性 - 防止路径遍历
    validate_glob_pattern(input_pattern)?;

    // 先创建输出目录，再验证（canonicalize 要求目录存在）
    std::fs::create_dir_all(output_dir).with_context(|| {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", output_dir.display().to_string());
        inklog::i18n::tr_args("cli-decrypt-err-create-dir", args)
    })?;

    // 验证已存在的输出目录路径安全
    if let Err(e) = validate_file_path(output_dir, output_dir) {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("err", e.to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-err-output-dir", args)
        ));
    }

    let canonical_output = output_dir.canonicalize()?;

    let paths = glob::glob(input_pattern)
        .map_err(|e| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("err", e.to_string());
            anyhow!("{}", inklog::i18n::tr_args("cli-decrypt-err-glob", args))
        })?
        .filter_map(|p| p.ok())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "enc"));

    let mut failure_count = 0u32;
    let mut success_count = 0u32;

    for path in paths {
        // 验证 glob 展开的输入路径是否在安全范围内
        if let Ok(canonical_input) = path.canonicalize()
            && !canonical_input.starts_with(&canonical_output)
        {
            // 输入路径不在输出目录内是正常场景（输入输出通常不同目录），
            // 但仍需确保输入路径不含符号链接等危险元素
            if let Ok(metadata) = path.symlink_metadata()
                && metadata.file_type().is_symlink()
            {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("path", path.display().to_string());
                eprintln!(
                    "{}",
                    inklog::i18n::tr_args("cli-decrypt-skip-symlink", args)
                );
                failure_count += 1;
                continue;
            }
        }

        let file_name = path.file_name().ok_or_else(|| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("path", path.display().to_string());
            anyhow!(
                "{}",
                inklog::i18n::tr_args("cli-decrypt-err-no-filename", args)
            )
        })?;
        let output_path = output_dir.join(file_name).with_extension("log");

        // 验证输出路径（不要求文件已存在）
        if let Err(e) = validate_output_path(&output_path, output_dir) {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("path", output_path.display().to_string());
            args.set("err", e.to_string());
            eprintln!("{}", inklog::i18n::tr_args("cli-decrypt-path-fail", args));
            failure_count += 1;
            continue;
        }

        let mut args = fluent_bundle::FluentArgs::new();
        args.set("input", path.display().to_string());
        args.set("output", output_path.display().to_string());
        println!("{}", inklog::i18n::tr_args("cli-decrypt-progress", args));

        if let Err(e) = decrypt_file_compatible(&path, &output_path, key_env) {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("path", path.display().to_string());
            args.set("err", e.to_string());
            eprintln!("{}", inklog::i18n::tr_args("cli-decrypt-fail", args));
            failure_count += 1;
        } else {
            success_count += 1;
        }
    }

    if failure_count > 0 {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("ok", success_count.to_string());
        args.set("fail", failure_count.to_string());
        return Err(anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-decrypt-batch-result", args)
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use aes_gcm::Aes256Gcm;
    use aes_gcm::aead::{Aead, KeyInit};
    use rand::RngExt;
    use std::io::Write;

    /// Generate a test key from a seed (allows deterministic or environment-based keys)
    fn get_test_key(seed: &str) -> [u8; 32] {
        let seed = std::env::var("INKLOG_TEST_KEY_SEED").unwrap_or_else(|_| seed.to_string());
        let hash = Sha256::digest(seed);
        let mut key = [0u8; 32];
        key.copy_from_slice(hash.as_slice());
        key
    }

    /// Generate a test encryption key (with optional seed for determinism)
    fn generate_test_key() -> [u8; 32] {
        get_test_key("inklog-test-seed-2024")
    }

    fn create_encrypted_file_v1(path: &PathBuf, plaintext: &[u8], key: &[u8; 32]) -> Result<()> {
        let mut file = File::create(path)?;

        file.write_all(MAGIC_HEADER)?;
        file.write_all(&1u16.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?;

        let mut nonce_bytes = [0u8; 12];
        let mut rng = rand::rng();
        rng.fill(&mut nonce_bytes);
        file.write_all(&nonce_bytes)?;

        let cipher = Aes256Gcm::new(key.into());
        let nonce = aes_gcm::Nonce::from(nonce_bytes);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| anyhow!("Encryption error: {}", e))?;

        file.write_all(&ciphertext)?;

        Ok(())
    }

    fn create_encrypted_file_legacy(
        path: &PathBuf,
        plaintext: &[u8],
        key: &[u8; 32],
    ) -> Result<()> {
        let mut file = File::create(path)?;

        file.write_all(MAGIC_HEADER)?;
        file.write_all(&1u16.to_le_bytes())?;

        let mut nonce_bytes = [0u8; 12];
        let mut rng = rand::rng();
        rng.fill(&mut nonce_bytes);
        file.write_all(&nonce_bytes)?;

        let cipher = Aes256Gcm::new(key.into());
        let nonce = aes_gcm::Nonce::from(nonce_bytes);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| anyhow!("Encryption error: {}", e))?;

        file.write_all(&ciphertext)?;

        Ok(())
    }

    #[test]
    fn test_magic_header_validation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let invalid_file = temp_dir.path().join("invalid.enc");
        let mut file = File::create(&invalid_file).unwrap();
        let mut invalid_header = [0u8; 24];
        invalid_header[..14].copy_from_slice(b"INVALID_HEADER");
        file.write_all(&invalid_header).unwrap();

        let result = decrypt_file(&invalid_file, &PathBuf::from("output.log"), "TEST_KEY");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Invalid file header"),
            "Expected error about invalid header, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_version_validation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let invalid_file = temp_dir.path().join("invalid_version.enc");
        let mut file = File::create(&invalid_file).unwrap();
        let mut header = [0u8; 24];
        header[..8].copy_from_slice(MAGIC_HEADER);
        header[8..10].copy_from_slice(&999u16.to_le_bytes());
        header[10..12].copy_from_slice(&1u16.to_le_bytes());
        file.write_all(&header).unwrap();

        let result = decrypt_file(&invalid_file, &PathBuf::from("output.log"), "TEST_KEY");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Unsupported file version"),
            "Expected error about unsupported version, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_get_encryption_key_base64() {
        let test_key = generate_test_key();
        let key_base64 = general_purpose::STANDARD.encode(test_key);
        unsafe {
            std::env::set_var("TEST_ENCRYPTION_KEY", &key_base64);
        };

        let key = get_encryption_key_cli("TEST_ENCRYPTION_KEY").unwrap();
        assert_eq!(key, test_key);

        unsafe {
            std::env::remove_var("TEST_ENCRYPTION_KEY");
        };
    }

    #[test]
    fn test_get_encryption_key_password_derivation() {
        // 使用明确的盐值进行测试，以确保可重现性
        let salt = b"test-salt-16b";
        let (key1, returned_salt) =
            derive_key_from_password("my-secret-password", Some(salt)).unwrap();
        assert_eq!(key1.len(), 32);
        assert_eq!(returned_salt, salt);

        // 使用相同密码和盐值再次派生，应该得到相同的密钥
        let (key2, _) = derive_key_from_password("my-secret-password", Some(salt)).unwrap();
        assert_eq!(key1, key2);

        // 使用不同盐值派生，应该得到不同的密钥
        let (key3, _) =
            derive_key_from_password("my-secret-password", Some(b"different-salt!")).unwrap();
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_get_encryption_key_raw_32_bytes() {
        let raw_key = [0x42u8; 32];
        unsafe {
            std::env::set_var("TEST_RAW_KEY", std::str::from_utf8(&raw_key).unwrap());
        };

        let key = get_encryption_key_cli("TEST_RAW_KEY").unwrap();
        assert_eq!(key, raw_key);

        unsafe {
            std::env::remove_var("TEST_RAW_KEY");
        };
    }

    #[test]
    fn test_decrypt_file_v1_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_file = temp_dir.path().join("test_v1.enc");
        let output_file = temp_dir.path().join("test_v1.log");
        let plaintext = b"Hello, World! V1 format test.";
        let test_key = generate_test_key();

        create_encrypted_file_v1(&input_file, plaintext, &test_key).unwrap();

        let key_base64 = general_purpose::STANDARD.encode(test_key);
        unsafe {
            std::env::set_var("TEST_KEY_V1", key_base64);
        };

        decrypt_file(&input_file, &output_file, "TEST_KEY_V1").unwrap();

        let decrypted_content = std::fs::read(&output_file).unwrap();
        assert_eq!(decrypted_content, plaintext);

        unsafe {
            std::env::remove_var("TEST_KEY_V1");
        };
    }

    #[test]
    fn test_decrypt_file_compatible() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_key = generate_test_key();
        let key_base64 = general_purpose::STANDARD.encode(test_key);
        unsafe {
            std::env::set_var("TEST_KEY_COMPAT", &key_base64);
        };

        // Test V1 format
        let v1_file = temp_dir.path().join("v1.enc");
        let v1_out = temp_dir.path().join("v1.log");
        let v1_text = b"V1 Content";
        create_encrypted_file_v1(&v1_file, v1_text, &test_key).unwrap();

        decrypt_file_compatible(&v1_file, &v1_out, "TEST_KEY_COMPAT").unwrap();
        assert_eq!(std::fs::read(&v1_out).unwrap(), v1_text);

        // Test Legacy format
        let legacy_file = temp_dir.path().join("legacy.enc");
        let legacy_out = temp_dir.path().join("legacy.log");
        let legacy_text = b"Legacy Content";
        create_encrypted_file_legacy(&legacy_file, legacy_text, &test_key).unwrap();

        decrypt_file_compatible(&legacy_file, &legacy_out, "TEST_KEY_COMPAT").unwrap();
        assert_eq!(std::fs::read(&legacy_out).unwrap(), legacy_text);

        unsafe {
            std::env::remove_var("TEST_KEY_COMPAT");
        };
    }

    #[test]
    fn test_path_traversal_protection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_dir = temp_dir.path();

        // Test parent directory traversal
        let malicious_path = base_dir.join("../passwd");
        assert!(validate_file_path(&malicious_path, base_dir).is_err());

        // Test valid path
        let valid_path = base_dir.join("valid.log");
        // Create file to make canonicalize work
        File::create(&valid_path).unwrap();
        assert!(validate_file_path(&valid_path, base_dir).is_ok());
    }

    #[test]
    fn test_validate_glob_pattern_valid() {
        assert!(validate_glob_pattern("*.enc").is_ok());
        assert!(validate_glob_pattern("logs/*.enc").is_ok());
        assert!(validate_glob_pattern("data/2024/*.log.enc").is_ok());
    }

    #[test]
    fn test_validate_glob_pattern_rejects_absolute_paths() {
        assert!(validate_glob_pattern("/var/log/*.enc").is_err());
        assert!(validate_glob_pattern("\\server\\share").is_err());
    }

    #[test]
    fn test_validate_glob_pattern_rejects_path_traversal() {
        assert!(validate_glob_pattern("../secret.enc").is_err());
        assert!(validate_glob_pattern("~/secret.enc").is_err());
        assert!(validate_glob_pattern("logs/../../etc/passwd").is_err());
    }

    #[test]
    fn test_validate_glob_pattern_rejects_suspicious_chars() {
        assert!(validate_glob_pattern("file\0.enc").is_err());
        assert!(validate_glob_pattern("file\u{2024}.enc").is_err());
        assert!(validate_glob_pattern("file\u{2025}.enc").is_err());
        assert!(validate_glob_pattern("file\u{FE52}.enc").is_err());
    }

    #[test]
    fn test_validate_file_path_rejects_suspicious_chars() {
        // 覆盖 L27-29：validate_file_path 的可疑字符错误路径（在 canonicalize 之前拦截）
        let temp_dir = tempfile::tempdir().unwrap();
        let base_dir = temp_dir.path();
        let malicious_path = base_dir.join("file\0.log");
        assert!(validate_file_path(&malicious_path, base_dir).is_err());

        // Unicode 变体点字符
        let unicode_path = base_dir.join("file\u{2024}.log");
        assert!(validate_file_path(&unicode_path, base_dir).is_err());
    }

    #[test]
    fn test_get_encryption_key_base64_wrong_length() {
        // 覆盖 L271-277: Base64 解码成功但长度不是 32
        let wrong_key = general_purpose::STANDARD.encode([0u8; 16]);
        unsafe {
            std::env::set_var("TEST_WRONG_LEN_KEY", &wrong_key);
        };
        let result = get_encryption_key_cli("TEST_WRONG_LEN_KEY");
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("32 bytes"));
        unsafe {
            std::env::remove_var("TEST_WRONG_LEN_KEY");
        };
    }

    #[test]
    fn test_get_encryption_key_password_via_cli() {
        // 覆盖 L281-288: 通过 get_encryption_key_cli 调用 PBKDF2 派生
        // "my-short-password" 不是 32 字节，也不是有效 Base64（含 '-'），长度 < 128
        unsafe {
            std::env::set_var("TEST_PASSWORD_KEY_CLI", "my-short-password");
        };
        let result = get_encryption_key_cli("TEST_PASSWORD_KEY_CLI");
        assert!(result.is_ok());
        unsafe {
            std::env::remove_var("TEST_PASSWORD_KEY_CLI");
        };
    }

    #[test]
    fn test_get_encryption_key_too_long() {
        // 覆盖 L291-295: 密钥长度 >= 128 且非有效 Base64
        let long_key = "!".repeat(128);
        unsafe {
            std::env::set_var("TEST_LONG_KEY", &long_key);
        };
        let result = get_encryption_key_cli("TEST_LONG_KEY");
        assert!(result.is_err());
        unsafe {
            std::env::remove_var("TEST_LONG_KEY");
        };
    }

    #[test]
    fn test_symlink_detected_before_canonicalize() {
        // T005: symlink check must use symlink_metadata() to detect symlinks
        // on the original path before canonicalize resolves them.
        let temp_dir = tempfile::tempdir().unwrap();
        let base_dir = temp_dir.path();

        // Create a real file and a symlink to it
        let real_file = base_dir.join("real.log");
        File::create(&real_file).unwrap();
        let symlink_path = base_dir.join("link.log");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_file, &symlink_path).unwrap();

        #[cfg(unix)]
        {
            // The symlink should be rejected
            let result = validate_file_path(&symlink_path, base_dir);
            assert!(result.is_err(), "symlinks should be rejected");
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("Symbolic links"),
                "error should mention symbolic links, got: {}",
                err_msg
            );
        }
    }
}
