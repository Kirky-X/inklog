// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! CLI 集成测试
//!
//! 使用 assert_cmd 测试 inklog-cli 二进制的端到端行为。
//! 覆盖 validate/generate/decrypt 三个子命令的成功与错误路径。

#![cfg(feature = "cli")]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// validate 子命令
// ============================================================================

#[test]
fn test_cli_validate_nonexistent_config() {
    let mut cmd = Command::cargo_bin("inklog-cli").expect("inklog-cli binary not found");
    cmd.args(["validate", "-c", "/nonexistent/path/config.toml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_cli_validate_valid_config() {
    let dir = TempDir::new().expect("tempdir");
    let config_path = dir.path().join("valid.toml");
    fs::write(
        &config_path,
        r#"
[global]
level = "info"
format = "{timestamp} [{level}] {message}"
"#,
    )
    .expect("write config");

    let mut cmd = Command::cargo_bin("inklog-cli").expect("inklog-cli binary not found");
    cmd.args(["validate", "-c", config_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn test_cli_validate_invalid_config_level() {
    let dir = TempDir::new().expect("tempdir");
    let config_path = dir.path().join("invalid.toml");
    fs::write(
        &config_path,
        r#"
[global]
level = "invalid_level"
"#,
    )
    .expect("write config");

    let mut cmd = Command::cargo_bin("inklog-cli").expect("inklog-cli binary not found");
    cmd.args(["validate", "-c", config_path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid log level"));
}

// ============================================================================
// generate 子命令
// ============================================================================

#[test]
fn test_cli_generate_minimal_config() {
    let dir = TempDir::new().expect("tempdir");
    let mut cmd = Command::cargo_bin("inklog-cli").expect("inklog-cli binary not found");
    cmd.args([
        "generate",
        "-o",
        dir.path().to_str().unwrap(),
        "-c",
        "minimal",
    ])
    .assert()
    .success();

    let generated = dir.path().join("inklog_config.toml");
    assert!(generated.exists(), "minimal config file should be created");
    let content = fs::read_to_string(&generated).expect("read generated config");
    assert!(content.contains("[global]"));
    assert!(content.contains("[console]"));
}

#[test]
fn test_cli_generate_full_config() {
    let dir = TempDir::new().expect("tempdir");
    let mut cmd = Command::cargo_bin("inklog-cli").expect("inklog-cli binary not found");
    cmd.args(["generate", "-o", dir.path().to_str().unwrap(), "-c", "full"])
        .assert()
        .success();

    let generated = dir.path().join("inklog_config.toml");
    let content = fs::read_to_string(&generated).expect("read generated config");
    assert!(content.contains("[file]"));
    assert!(content.contains("[performance]"));
}

#[test]
fn test_cli_generate_env_example() {
    let dir = TempDir::new().expect("tempdir");
    let mut cmd = Command::cargo_bin("inklog-cli").expect("inklog-cli binary not found");
    cmd.args([
        "generate",
        "-o",
        dir.path().to_str().unwrap(),
        "--env-example",
    ])
    .assert()
    .success();

    let env_example = dir.path().join(".env.example");
    assert!(env_example.exists(), ".env.example should be created");
    let content = fs::read_to_string(&env_example).expect("read env example");
    assert!(content.contains("INKLOG_LEVEL"));
}

// ============================================================================
// decrypt 子命令
// ============================================================================

#[test]
fn test_cli_decrypt_nonexistent_input() {
    let dir = TempDir::new().expect("tempdir");
    let output = dir.path().join("output.log");
    let mut cmd = Command::cargo_bin("inklog-cli").expect("inklog-cli binary not found");
    cmd.args([
        "decrypt",
        "-i",
        "/nonexistent/file.enc",
        "-o",
        output.to_str().unwrap(),
        "-k",
        "INKLOG_TEST_DECRYPT_KEY",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_cli_decrypt_success() {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};
    use base64::{engine::general_purpose, Engine as _};
    use std::io::Write;

    let dir = TempDir::new().expect("tempdir");

    // Generate a deterministic 32-byte key
    let key = [0x42u8; 32];
    let key_base64 = general_purpose::STANDARD.encode(key);

    // Create an encrypted file in V1 format
    let enc_path = dir.path().join("test.enc");
    let output_path = dir.path().join("test.log");
    let plaintext = b"CLI decrypt integration test plaintext.";

    {
        let mut file = fs::File::create(&enc_path).expect("create enc file");
        file.write_all(b"ENCLOG1\0").expect("write magic");
        file.write_all(&1u16.to_le_bytes()).expect("write version");
        file.write_all(&1u16.to_le_bytes()).expect("write algo");
        let nonce_bytes = [0xAAu8; 12];
        file.write_all(&nonce_bytes).expect("write nonce");
        let cipher = Aes256Gcm::new((&key).into());
        let nonce = Nonce::from(nonce_bytes);
        let ciphertext = cipher.encrypt(&nonce, plaintext.as_ref()).expect("encrypt");
        file.write_all(&ciphertext).expect("write ciphertext");
    }

    let mut cmd = Command::cargo_bin("inklog-cli").expect("inklog-cli binary not found");
    cmd.args([
        "decrypt",
        "-i",
        enc_path.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
        "-k",
        "INKLOG_CLI_TEST_KEY",
    ])
    .env("INKLOG_CLI_TEST_KEY", &key_base64)
    .assert()
    .success();

    let decrypted = fs::read_to_string(&output_path).expect("read decrypted output");
    assert_eq!(
        decrypted.as_bytes(),
        plaintext,
        "Decrypted content must match original plaintext"
    );
}

// ============================================================================
// 无参数 / 错误处理
// ============================================================================

#[test]
fn test_cli_no_args_fails() {
    let mut cmd = Command::cargo_bin("inklog-cli").expect("inklog-cli binary not found");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage").or(predicate::str::contains("usage")));
}
