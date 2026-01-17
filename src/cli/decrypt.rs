// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
#[cfg(test)]
use sha2::Digest as Sha256Digest;
#[cfg(test)]
use sha2::Sha256;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// 验证文件路径是否在允许的目录内，防止路径遍历攻击
fn validate_file_path(file_path: &Path, base_dir: &Path) -> Result<()> {
    // 检查路径中是否包含可疑字符（包括 Unicode 变体）
    let path_str = file_path.to_string_lossy();
    let suspicious_chars = ['.', '~', '\0', '\u{2024}', '\u{2025}', '\u{FE52}']; // 包括各种点字符
    for c in path_str.chars() {
        if suspicious_chars.contains(&c) {
            return Err(anyhow!(
                "Invalid path character detected in: {}",
                file_path.display()
            ));
        }
    }

    // 规范化路径
    let canonical_path = file_path
        .canonicalize()
        .map_err(|e| anyhow!("Cannot canonicalize file path: {}", e))?;

    let canonical_base = base_dir
        .canonicalize()
        .map_err(|e| anyhow!("Cannot canonicalize base directory: {}", e))?;

    // 检查规范化后的路径是否以基础目录开头
    if !canonical_path.starts_with(&canonical_base) {
        return Err(anyhow!(
            "Path traversal attempt detected: {} is outside base directory {}",
            file_path.display(),
            base_dir.display()
        ));
    }

    // 检查符号链接
    if let Ok(metadata) = file_path.metadata() {
        if metadata.file_type().is_symlink() {
            return Err(anyhow!(
                "Symbolic links are not allowed: {}",
                file_path.display()
            ));
        }
    }

    Ok(())
}

/// 验证 glob 模式是否安全
fn validate_glob_pattern(pattern: &str) -> Result<()> {
    // 检查绝对路径
    if pattern.starts_with('/') || pattern.starts_with('\\') {
        return Err(anyhow!("Absolute paths are not allowed in glob patterns"));
    }

    // 检查路径遍历
    if pattern.contains("..") || pattern.contains("~") {
        return Err(anyhow!("Path traversal is not allowed in glob patterns"));
    }

    // 检查可疑字符（包括 Unicode 变体）
    let suspicious_chars = ['\0', '\u{2024}', '\u{2025}', '\u{FE52}'];
    for c in pattern.chars() {
        if suspicious_chars.contains(&c) {
            return Err(anyhow!("Invalid character in glob pattern"));
        }
    }

    // 尝试解析为路径，确保不包含危险元素
    let path = Path::new(pattern);
    if path.is_absolute() {
        return Err(anyhow!("Absolute paths are not allowed"));
    }

    // 检查组件
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(anyhow!("Parent directory references are not allowed"));
            }
            std::path::Component::Prefix(_) => {
                return Err(anyhow!("Path prefixes are not allowed"));
            }
            std::path::Component::RootDir => {
                return Err(anyhow!("Root directory references are not allowed"));
            }
            _ => {}
        }
    }

    Ok(())
}

const MAGIC_HEADER: &[u8] = b"ENCLOG1\0";

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum EncryptionVersion {
    V1WithAlgo,
    V1Legacy,
    Unknown,
}

#[allow(dead_code)]
pub fn detect_version(header: &[u8]) -> EncryptionVersion {
    if header.len() < 10 {
        return EncryptionVersion::Unknown;
    }

    if &header[..8] == MAGIC_HEADER {
        let version = u16::from_le_bytes([header[8], header[9]]);
        if version == 1 {
            return EncryptionVersion::V1Legacy;
        }
    }
    EncryptionVersion::Unknown
}

#[allow(dead_code)]
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

    let key = get_encryption_key(key_env)
        .with_context(|| format!("Failed to get encryption key from env var: {}", key_env))?;

    let nonce = aes_gcm::Nonce::from_slice(&header[12..24]);

    let mut ciphertext = Vec::new();
    file.read_to_end(&mut ciphertext)
        .with_context(|| "Failed to read ciphertext")?;

    let cipher = Aes256Gcm::new((&key).into());

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    let mut output_file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    output_file
        .write_all(&plaintext)
        .with_context(|| "Failed to write decrypted data")?;

    Ok(())
}

#[allow(dead_code)]
pub fn decrypt_file_legacy(
    input_path: &PathBuf,
    output_path: &PathBuf,
    key_env: &str,
) -> Result<()> {
    let key = get_encryption_key(key_env)
        .with_context(|| format!("Failed to get encryption key from env var: {}", key_env))?;

    let mut file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {}", input_path.display()))?;

    let mut header = [0u8; 10];
    file.read_exact(&mut header)
        .with_context(|| "Failed to read file header")?;

    if &header[..8] != MAGIC_HEADER {
        return Err(anyhow!("Invalid file header: not an encrypted inklog file"));
    }

    let version = u16::from_le_bytes([header[8], header[9]]);
    if version != 1 {
        return Err(anyhow!("Unsupported file version: {}", version));
    }

    let mut nonce_bytes = [0u8; 12];
    file.read_exact(&mut nonce_bytes)
        .with_context(|| "Failed to read nonce")?;

    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

    let mut ciphertext = Vec::new();
    file.read_to_end(&mut ciphertext)
        .with_context(|| "Failed to read ciphertext")?;

    let cipher = Aes256Gcm::new((&key).into());

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
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
    let mut file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {}", input_path.display()))?;

    let mut header = [0u8; 24];
    let read_count = file
        .read(&mut header)
        .with_context(|| "Failed to read file header")?;

    if read_count < 10 {
        return Err(anyhow!("File too small to be a valid encrypted file"));
    }

    if &header[..8] != MAGIC_HEADER {
        return Err(anyhow!("Invalid file header: not an encrypted inklog file"));
    }

    let version = u16::from_le_bytes([header[8], header[9]]);
    if version != 1 {
        return Err(anyhow!("Unsupported file version: {}", version));
    }

    let key = get_encryption_key(key_env)
        .with_context(|| format!("Failed to get encryption key from env var: {}", key_env))?;

    let algo = u16::from_le_bytes([header[10], header[11]]);
    let plaintext: Vec<u8>;

    if algo == 1 {
        if read_count < 24 {
            return Err(anyhow!("File too small for V1 format"));
        }
        let nonce_slice: [u8; 12] = header[12..24].try_into().unwrap();
        let nonce = aes_gcm::Nonce::from_slice(&nonce_slice);

        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext)
            .with_context(|| "Failed to read ciphertext")?;

        let cipher = Aes256Gcm::new((&key).into());
        plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
    } else {
        // Assume Legacy format (MAGIC + VER + NONCE + CIPHERTEXT)
        // Legacy header is 22 bytes (8 MAGIC + 2 VER + 12 NONCE)
        if read_count < 22 {
            return Err(anyhow!("File too small to be a valid encrypted file"));
        }

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes.copy_from_slice(&header[10..22]);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

        let mut ciphertext = Vec::new();
        // If we read more than 22 bytes, the extras are part of the ciphertext
        if read_count > 22 {
            ciphertext.extend_from_slice(&header[22..read_count]);
        }
        file.read_to_end(&mut ciphertext)
            .with_context(|| "Failed to read ciphertext")?;

        let cipher = Aes256Gcm::new((&key).into());
        plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
    }

    let mut output_file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    output_file
        .write_all(&plaintext)
        .with_context(|| "Failed to write decrypted data")?;

    Ok(())
}

#[allow(dead_code)]
pub fn decrypt_file_to_string(input_path: &PathBuf, key_env: &str) -> Result<String> {
    let temp_output = input_path.with_extension("decrypted");
    decrypt_file(input_path, &temp_output, key_env)?;

    let mut content = String::new();
    File::open(&temp_output)?.read_to_string(&mut content)?;

    std::fs::remove_file(&temp_output)?;

    Ok(content)
}

fn get_encryption_key(env_var: &str) -> Result<[u8; 32]> {
    let key_str = std::env::var(env_var)
        .map_err(|_| anyhow!("Encryption key environment variable not set. Please ensure INKLOG_DECRYPT_KEY or INKLOG_ENCRYPTION_KEY is defined."))?;

    // 尝试解码 Base64 编码的密钥
    if let Ok(decoded) = general_purpose::STANDARD.decode(key_str.trim()) {
        if decoded.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&decoded);
            return Ok(key);
        } else {
            return Err(anyhow!(
                "Encryption key must be exactly 32 bytes (256 bits) after Base64 decoding, got {} bytes. \
                 Please provide a valid 32-byte key encoded in Base64.",
                decoded.len()
            ));
        }
    }

    // 如果不是 Base64，尝试使用原始字节
    let bytes = key_str.as_bytes();
    if bytes.len() == 32 {
        let mut key = [0u8; 32];
        key.copy_from_slice(bytes);
        Ok(key)
    } else {
        Err(anyhow!(
            "Encryption key must be exactly 32 bytes (256 bits), got {} bytes. \
             Please provide a valid 32-byte key.",
            bytes.len()
        ))
    }
}

#[allow(dead_code)]
pub fn decrypt_directory(
    input_dir: &PathBuf,
    output_dir: &PathBuf,
    key_env: &str,
    recursive: bool,
) -> Result<()> {
    if !input_dir.exists() {
        return Err(anyhow!(
            "Input directory does not exist: {}",
            input_dir.display()
        ));
    }

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    let entries = std::fs::read_dir(input_dir)
        .with_context(|| format!("Failed to read input directory: {}", input_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "enc" {
                    let file_name = path.file_name().unwrap();
                    let output_path = output_dir.join(file_name).with_extension("log");

                    // 验证输出路径是否在允许的目录内
                    if let Err(e) = validate_file_path(&output_path, output_dir) {
                        eprintln!(
                            "Path validation failed for {}: {}",
                            output_path.display(),
                            e
                        );
                        continue;
                    }

                    println!(
                        "Decrypting: {} -> {}",
                        path.display(),
                        output_path.display()
                    );

                    if let Err(e) = decrypt_file(&path, &output_path, key_env) {
                        eprintln!("Failed to decrypt {}: {}", path.display(), e);
                    }
                }
            }
        } else if recursive && path.is_dir() {
            let file_name = path.file_name().unwrap();
            let sub_output_dir = output_dir.join(file_name);

            // 验证子目录路径是否在允许的目录内
            if let Err(e) = validate_file_path(&sub_output_dir, output_dir) {
                eprintln!(
                    "Path validation failed for {}: {}",
                    sub_output_dir.display(),
                    e
                );
                continue;
            }

            decrypt_directory(&path, &sub_output_dir, key_env, recursive)?;
        }
    }

    Ok(())
}

pub fn decrypt_directory_compatible(
    input_dir: &PathBuf,
    output_dir: &PathBuf,
    key_env: &str,
    recursive: bool,
) -> Result<()> {
    if !input_dir.exists() {
        return Err(anyhow!(
            "Input directory does not exist: {}",
            input_dir.display()
        ));
    }

    // 验证输出目录路径安全
    if let Err(e) = validate_file_path(output_dir, output_dir) {
        return Err(anyhow!("Invalid output directory: {}", e));
    }

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    let entries = std::fs::read_dir(input_dir)
        .with_context(|| format!("Failed to read input directory: {}", input_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "enc" {
                    let file_name = path.file_name().unwrap();
                    let output_path = output_dir.join(file_name).with_extension("log");

                    // 验证输出路径是否在允许的目录内
                    if let Err(e) = validate_file_path(&output_path, output_dir) {
                        eprintln!(
                            "Path validation failed for {}: {}",
                            output_path.display(),
                            e
                        );
                        continue;
                    }

                    println!(
                        "Decrypting: {} -> {}",
                        path.display(),
                        output_path.display()
                    );

                    if let Err(e) = decrypt_file_compatible(&path, &output_path, key_env) {
                        eprintln!("Failed to decrypt {}: {}", path.display(), e);
                    }
                }
            }
        } else if recursive && path.is_dir() {
            let file_name = path.file_name().unwrap();
            let sub_output_dir = output_dir.join(file_name);

            // 验证子目录路径是否在允许的目录内
            if let Err(e) = validate_file_path(&sub_output_dir, output_dir) {
                eprintln!(
                    "Path validation failed for {}: {}",
                    sub_output_dir.display(),
                    e
                );
                continue;
            }

            decrypt_directory_compatible(&path, &sub_output_dir, key_env, recursive)?;
        }
    }

    Ok(())
}

pub fn batch_decrypt(input_pattern: &str, output_dir: &PathBuf, key_env: &str) -> Result<()> {
    // 验证 glob 模式安全性 - 防止路径遍历
    validate_glob_pattern(input_pattern)?;

    let paths = glob::glob(input_pattern)
        .map_err(|e| anyhow!("Invalid glob pattern: {}", e))?
        .filter_map(|p| p.ok())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "enc"));

    // 验证输出目录路径安全
    if let Err(e) = validate_file_path(output_dir, output_dir) {
        return Err(anyhow!("Invalid output directory: {}", e));
    }

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    for path in paths {
        let file_name = path.file_name().unwrap();
        let output_path = output_dir.join(file_name).with_extension("log");

        // 验证输出路径是否在允许的目录内
        if let Err(e) = validate_file_path(&output_path, output_dir) {
            eprintln!(
                "Path validation failed for {}: {}",
                output_path.display(),
                e
            );
            continue;
        }

        println!(
            "Decrypting: {} -> {}",
            path.display(),
            output_path.display()
        );

        if let Err(e) = decrypt_file_compatible(&path, &output_path, key_env) {
            eprintln!("Failed to decrypt {}: {}", path.display(), e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::Aes256Gcm;
    use rand::Rng;
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
        rand::thread_rng().fill(&mut nonce_bytes);
        file.write_all(&nonce_bytes)?;

        let cipher = Aes256Gcm::new(key.into());
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
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
        rand::thread_rng().fill(&mut nonce_bytes);
        file.write_all(&nonce_bytes)?;

        let cipher = Aes256Gcm::new(key.into());
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
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
        std::env::set_var("TEST_ENCRYPTION_KEY", &key_base64);

        let key = get_encryption_key("TEST_ENCRYPTION_KEY").unwrap();
        assert_eq!(key, test_key);

        std::env::remove_var("TEST_ENCRYPTION_KEY");
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
        std::env::set_var("TEST_KEY_V1", key_base64);

        decrypt_file(&input_file, &output_file, "TEST_KEY_V1").unwrap();

        let decrypted_content = std::fs::read(&output_file).unwrap();
        assert_eq!(decrypted_content, plaintext);

        std::env::remove_var("TEST_KEY_V1");
    }

    #[test]
    fn test_decrypt_file_legacy_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_file = temp_dir.path().join("test_legacy.enc");
        let output_file = temp_dir.path().join("test_legacy.log");
        let plaintext = b"Hello, World! Legacy format test.";
        let test_key = generate_test_key();

        create_encrypted_file_legacy(&input_file, plaintext, &test_key).unwrap();

        let key_base64 = general_purpose::STANDARD.encode(test_key);
        std::env::set_var("TEST_KEY_LEGACY", key_base64);

        decrypt_file_legacy(&input_file, &output_file, "TEST_KEY_LEGACY").unwrap();

        let decrypted_content = std::fs::read(&output_file).unwrap();
        assert_eq!(decrypted_content, plaintext);

        std::env::remove_var("TEST_KEY_LEGACY");
    }

    #[test]
    fn test_decrypt_file_compatible() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_key = generate_test_key();
        let key_base64 = general_purpose::STANDARD.encode(test_key);
        std::env::set_var("TEST_KEY_COMPAT", &key_base64);

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

        std::env::remove_var("TEST_KEY_COMPAT");
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
}
