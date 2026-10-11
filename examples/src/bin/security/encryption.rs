// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 日志加密示例
//!
//! 演示如何使用 inklog 的加密功能保护日志数据。
//!
//! # 功能演示
//!
//! - 生成临时加密密钥（32字节，Base64编码）
//! - 使用 AES-256-GCM 加密日志写入
//! - 加密文件格式解析
//! - 解密验证和明文对比
//!
//! # 加密原理
//!
//! inklog 使用 AES-256-GCM（Galois/Counter Mode）算法：
//!
//! - **密钥长度**：256位（32字节）
//! - **Nonce长度**：96位（12字节）
//! - **认证标签**：128位（16字节），自动附加
//! - **加密模式**：AEAD（Authenticated Encryption with Associated Data）
//!
//! # 加密文件格式
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ Magic Header (8 bytes)  - "ENCLOG1\0"   │
//! ├─────────────────────────────────────────┤
//! │ Version (2 bytes)       - 0x0001        │
//! ├─────────────────────────────────────────┤
//! │ Algorithm ID (2 bytes)  - 0x0001 (AES)  │
//! ├─────────────────────────────────────────┤
//! │ Nonce (12 bytes)        - 随机/文件唯一  │
//! ├─────────────────────────────────────────┤
//! │ Encrypted Data (可变)    - AES-GCM 密文  │
//! ├─────────────────────────────────────────┤
//! │ Auth Tag (16 bytes)     - GCM 认证标签   │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # 安全特性
//!
//! - **完整性保护**：GCM 模式自动验证数据完整性
//! - **随机 Nonce**：每个文件使用唯一的 12 字节 Nonce
//! - **密钥派生**：支持从密码派生密钥（PBKDF2-HMAC-SHA256）
//! - **零化内存**：密钥在内存中使用后自动清零
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin encryption
//! ```

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit};
use inklog::sink::LogSink;
use inklog::sink::encryption::get_encryption_key;
use inklog::tracing::Level;
use inklog::{FileSinkConfig, LogRecord};
use inklog_examples::common::{print_section, print_separator, temp_file_path, tr, tr_args};
use rand::Rng;
use std::io::Write;
use std::path::PathBuf;

/// 加密文件魔数和版本常量
const MAGIC_HEADER: &[u8] = b"ENCLOG1\0";
const VERSION: u16 = 1;
const ALGO_AES_GCM: u16 = 1;

/// 生成临时加密密钥（32字节）
///
/// 使用 UUID v4 生成一个随机密钥，并转换为 Base64 编码格式。
/// 该密钥用于 AES-256-GCM 加密。
///
/// # 返回值
///
/// 返回 Base64 编码的 32 字节随机密钥
fn generate_temp_key() -> String {
    // 使用两个 UUID v4 拼接生成足够的随机数据
    let uuid1 = uuid::Uuid::new_v4();
    let uuid2 = uuid::Uuid::new_v4();

    let mut key_bytes = [0u8; 32];
    key_bytes[..16].copy_from_slice(uuid1.as_bytes());
    key_bytes[16..].copy_from_slice(uuid2.as_bytes());

    // 使用 inklog 的 base64 编码
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.encode(key_bytes)
}

/// 写入明文日志到临时文件
///
/// 使用 FileSink 写入测试日志，返回明文内容供后续加密使用。
///
/// # 参数
///
/// * `file_path` - 日志文件路径
///
/// # 返回值
///
/// 返回 Ok(String) 表示成功，否则返回错误
async fn write_plaintext_log(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    print_separator("Step 1: write the plaintext log");

    let config = FileSinkConfig {
        enabled: true,
        path: PathBuf::from(file_path),
        encrypt: false, // 先写入明文
        batch_size: 10,
        flush_interval_ms: 100,
        fsync: false,
        audit_chain_enabled: false,
        ..Default::default()
    };

    let sink = inklog::sink::file::FileSink::new(config)?;

    // 创建测试日志内容
    let plaintext = r#"2026-03-19T10:30:00.000Z [INFO] encryption - this is an encrypted log message
2026-03-19T10:30:00.001Z [INFO] encryption - sensitive data is protected automatically
2026-03-19T10:30:00.002Z [WARN] encryption - this is a warning log and it is encrypted too
2026-03-19T10:30:00.003Z [INFO] encryption - test log entry 1
2026-03-19T10:30:00.004Z [INFO] encryption - test log entry 2
2026-03-19T10:30:00.005Z [INFO] encryption - test log entry 3"#;

    for line in plaintext.lines() {
        let record = LogRecord::new(Level::INFO, "encryption".to_string(), line.to_string());
        sink.write(&record).await?;
    }

    sink.flush().await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    println!("✓ plaintext log written: {}", file_path);
    println!("  File size: {} bytes", plaintext.len());

    Ok(plaintext.to_string())
}

/// 使用 AES-256-GCM 加密日志内容
///
/// 读取明文日志，使用加密密钥加密后写入新文件。
///
/// # 参数
///
/// * `plaintext_path` - 明文日志文件路径
/// * `encrypted_path` - 加密日志文件路径
/// * `key_env` - 环境变量名，包含加密密钥
///
/// # 返回值
///
/// 返回 Ok(()) 表示成功，否则返回错误
fn encrypt_log_file(
    plaintext_path: &str,
    encrypted_path: &str,
    key_env: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    print_separator("Step 2: encrypt the log file");

    let key = get_encryption_key(key_env)?;
    println!("✓ key retrieved: {} bytes", key.len());

    // 创建 AES-256-GCM 密码器
    let cipher =
        Aes256Gcm::new_from_slice(&*key).map_err(|e| format!("Failed to create cipher: {}", e))?;

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = aes_gcm::Nonce::from(nonce_bytes);

    let plaintext = std::fs::read(plaintext_path)?;
    println!("✓ plaintext read: {} bytes", plaintext.len());

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_slice())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    println!(
        "✓ encryption done: {} bytes -> {} bytes (includes the auth tag)",
        plaintext.len(),
        ciphertext.len()
    );

    let mut file = std::fs::File::create(encrypted_path)?;
    file.write_all(MAGIC_HEADER)?; // Magic Header
    file.write_all(&VERSION.to_le_bytes())?; // Version
    file.write_all(&ALGO_AES_GCM.to_le_bytes())?; // Algorithm ID
    file.write_all(&nonce_bytes)?; // Nonce
    file.write_all(&ciphertext)?; // Ciphertext + Auth Tag

    println!("✓ encrypted file written: {}", encrypted_path);

    Ok(())
}

/// 解析并展示加密文件格式
///
/// 读取加密文件头部，解析并展示各字段的含义。
///
/// # 参数
///
/// * `file_path` - 加密日志文件路径
///
/// # 返回值
///
/// 返回 Ok(()) 表示成功，否则返回错误
fn parse_encrypted_format(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    print_separator("Step 3: parse the encrypted file format");

    let path = PathBuf::from(file_path);
    let mut file = std::fs::File::open(&path)?;

    // 读取文件头（24 字节）
    let mut header = [0u8; 24];
    std::io::Read::read_exact(&mut file, &mut header)?;

    let file_size = std::fs::metadata(&path)?.len();

    print_section("Header parsing (24 bytes)");

    // Magic Header (8 bytes)
    let magic = &header[..8];
    println!(
        "  Magic Header (8 bytes): {:?}",
        String::from_utf8_lossy(magic)
    );
    println!("    └─ marks the file as an inklog encrypted format");

    // Version (2 bytes)
    let version = u16::from_le_bytes([header[8], header[9]]);
    println!("  Version (2 bytes):       {}", version);
    println!("    └─ the format version, for forward compatibility");

    // Algorithm ID (2 bytes)
    let algo = u16::from_le_bytes([header[10], header[11]]);
    println!(
        "  Algorithm ID (2 bytes):  {} ({})",
        algo,
        match algo {
            1 => "AES-256-GCM",
            _ => "Unknown",
        }
    );
    println!("    └─ the encryption algorithm identifier");

    // Nonce (12 bytes)
    let nonce_bytes = &header[12..24];
    println!("  Nonce (12 bytes):        {:?}", nonce_bytes);
    println!("    └─ the nonce, unique per file, which prevents replay");

    // 展示文件结构
    print_section("File structure");

    let data_size = file_size - 24;
    let ciphertext_size = data_size - 16;

    println!("  ┌──────────────────────────────────────┐");
    println!("  │ Magic Header    │    8 bytes        │");
    println!("  ├──────────────────────────────────────┤");
    println!("  │ Version         │    2 bytes        │");
    println!("  ├──────────────────────────────────────┤");
    println!("  │ Algorithm ID    │    2 bytes        │");
    println!("  ├──────────────────────────────────────┤");
    println!("  │ Nonce           │   12 bytes        │");
    println!("  ├──────────────────────────────────────┤");
    println!("  │ Ciphertext      │ {:3} bytes        │", ciphertext_size);
    println!("  ├──────────────────────────────────────┤");
    println!("  │ Auth Tag        │   16 bytes        │");
    println!("  └──────────────────────────────────────┘");

    println!("\n  Total file size: {} bytes", file_size);
    println!("  Plaintext size (estimated): ~{} bytes", ciphertext_size);

    Ok(())
}

/// 解密并验证加密文件
///
/// 读取加密文件，使用密钥解密后验证内容。
///
/// # 参数
///
/// * `file_path` - 加密日志文件路径
/// * `key_env` - 环境变量名，包含加密密钥
///
/// # 返回值
///
/// 返回 Ok(String) 表示解密后的明文，否则返回错误
fn decrypt_and_verify(
    file_path: &str,
    key_env: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    print_separator("Step 4: decrypt and verify");

    let path = PathBuf::from(file_path);
    let mut file = std::fs::File::open(&path)?;

    let mut header = [0u8; 24];
    std::io::Read::read_exact(&mut file, &mut header)?;

    if &header[..8] != MAGIC_HEADER {
        return Err("Invalid file header: not an encrypted inklog file".into());
    }

    let key = get_encryption_key(key_env)?;
    println!("✓ key retrieved");

    let nonce_bytes: [u8; 12] = header[12..24].try_into().unwrap();
    let nonce = aes_gcm::Nonce::from(nonce_bytes);

    let mut ciphertext = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut ciphertext)?;

    let cipher =
        Aes256Gcm::new_from_slice(&*key).map_err(|e| format!("Failed to create cipher: {}", e))?;

    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|e| format!("Decryption failed: {}", e))?;

    let plaintext_str = String::from_utf8(plaintext.clone())
        .map_err(|e| format!("Invalid UTF-8 in decrypted data: {}", e))?;

    println!("✓ decryption succeeded");
    println!("  Ciphertext size: {} bytes", ciphertext.len());
    println!("  Plaintext size: {} bytes", plaintext.len());

    print_section("Content verification");

    let has_expected_content = plaintext_str.contains("encrypted log message")
        && plaintext_str.contains("sensitive data")
        && plaintext_str.contains("warning log");

    if has_expected_content {
        println!("✓ content verified");
        println!("  - contains the test message");
        println!("  - contains the sensitive data keyword");
        println!("  - contains the warning log");
    } else {
        return Err("Content validation failed".into());
    }

    Ok(plaintext_str)
}

/// 展示明文和密文对比
///
/// # 参数
///
/// * `plaintext` - 解密后的明文
fn show_comparison(plaintext: &str) {
    print_separator("Step 5: plaintext vs ciphertext");

    println!("Decrypted plaintext content:");
    println!("{}", "─".repeat(50));
    println!("{}", plaintext);
    println!("{}", "─".repeat(50));

    println!("\nKey differences:");
    println!("  - Plaintext: human readable, log content visible directly");
    println!("  - Ciphertext: binary, needs the key before it can be read");
    println!("  - Auth tag: verifies integrity and detects tampering");
}

/// 清理临时文件
///
/// # 参数
///
/// * `file_path` - 要删除的文件路径
fn cleanup(file_path: &str) {
    print_separator("Clean up temporary files");

    if let Err(e) = std::fs::remove_file(file_path) {
        eprintln!("warning: failed to delete the file: {}", e);
    } else {
        println!("✓ temporary file deleted: {}", file_path);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog log encryption examples ===\n");

    print_separator("Generate a temporary encryption key");

    // 安全约束：密钥属于敏感凭据，不得以任何形式输出到 stdout
    //（可能被终端回显、CI 日志或日志系统采集），仅通过环境变量传递
    let key = generate_temp_key();

    let key_env = "LOG_ENCRYPTION_KEY";
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var(key_env, &key) };
    println!(
        "✓ temporary key generated and written to the environment variable: {}",
        key_env
    );
    println!(
        "  (the key itself is not printed to stdout; see \"never print keys in logs\" below)\n"
    );

    let plain_path = temp_file_path("plaintext");
    let encrypted_path = format!("{}.enc", plain_path);
    println!("\n{}", tr("example-encryption-temporary-file-paths"));
    println!(
        "  {}",
        tr_args(
            "example-encryption-plaintext",
            &[("plain_path", &plain_path)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-encryption-encrypted",
            &[("encrypted_path", &encrypted_path)]
        )
    );

    write_plaintext_log(&plain_path).await?;

    encrypt_log_file(&plain_path, &encrypted_path, key_env)?;

    parse_encrypted_format(&encrypted_path)?;

    let plaintext = decrypt_and_verify(&encrypted_path, key_env)?;

    show_comparison(&plaintext);

    cleanup(&plain_path);
    cleanup(&encrypted_path);

    // 移除环境变量
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var(key_env) };
    println!(
        "{}",
        tr_args(
            "example-encryption-environment-variable-cleared",
            &[("key_env", &key_env)]
        )
    );

    print_separator(&tr("example-encryption-key-management-best-practices"));

    println!(
        "{}\n",
        tr("example-encryption-in-production-follow-these-best")
    );

    println!("{}", tr("example-encryption-1-key-storage"));
    println!(
        "   {}",
        tr("example-encryption-use-a-managed-key-service-such")
    );
    println!(
        "   {}",
        tr("example-encryption-never-hard-code-keys-in-source")
    );
    println!(
        "   {}\n",
        tr("example-encryption-pass-keys-through-environment")
    );

    println!("{}", tr("example-encryption-2-key-rotation"));
    println!(
        "   {}",
        tr("example-encryption-rotate-encryption-keys-regularly")
    );
    println!("   {}", tr("example-encryption-keep-old-keys-to-decrypt"));
    println!("   {}\n", tr("example-encryption-use-key-versioning"));

    println!("{}", tr("example-encryption-3-access-control"));
    println!(
        "   {}",
        tr("example-encryption-restrict-who-can-access-keys")
    );
    println!("   {}", tr("example-encryption-log-key-usage"));
    println!("   {}\n", tr("example-encryption-apply-least-privilege"));

    println!("{}", tr("example-encryption-4-secure-transport"));
    println!("   {}", tr("example-encryption-transport-keys-over-tls"));
    println!("   {}", tr("example-encryption-never-print-keys-in-logs"));
    println!(
        "   {}\n",
        tr("example-encryption-use-a-secure-ci-cd-pipeline")
    );

    print_separator(&tr("example-encryption-encryption-performance"));

    println!("{}\n", tr("example-encryption-aes-256-gcm-performance"));

    println!("{}", tr("example-encryption-1-hardware-acceleration"));
    println!(
        "   {}",
        tr("example-encryption-modern-cpus-support-the-aes-ni")
    );
    println!(
        "   {}",
        tr("example-encryption-hardware-acceleration-reaches")
    );
    println!(
        "   {}\n",
        tr("example-encryption-log-write-performance-is-barely")
    );

    println!("{}", tr("example-encryption-2-aead-authentication"));
    println!("   {}", tr("example-encryption-provides-encryption-and"));
    println!(
        "   {}",
        tr("example-encryption-detects-tampering-automatically")
    );
    println!(
        "   {}\n",
        tr("example-encryption-protects-against-ciphertext")
    );

    println!("{}", tr("example-encryption-3-parallelism"));
    println!("   {}", tr("example-encryption-gcm-mode-supports-parallel"));
    println!("   {}", tr("example-encryption-uses-multiple-cores-for"));
    println!(
        "   {}\n",
        tr("example-encryption-suits-bulk-log-processing")
    );

    print_separator(&tr("example-encryption-supported-key-formats"));

    println!("{}\n", tr("example-encryption-inklog-supports-three-key"));

    println!("{}", tr("example-encryption-1-base64-encoded-recommended"));
    println!(
        "   {}",
        tr("example-encryption-the-base64-encoding-of-a-32")
    );
    println!("   {}", tr("example-encryption-example"));
    println!(
        "   {}\n",
        tr("example-encryption-safe-standard-and-easy-to")
    );

    println!("{}", tr("example-encryption-2-raw-bytes"));
    println!("   {}", tr("example-encryption-uses-the-32-raw-key-bytes"));
    println!("   {}", tr("example-encryption-example-2"));
    println!(
        "   {}\n",
        tr("example-encryption-fits-simple-test-scenarios")
    );

    println!("{}", tr("example-encryption-3-password-derivation"));
    println!(
        "   {}",
        tr("example-encryption-derives-a-key-from-a-password")
    );
    println!("   {}", tr("example-encryption-example-my-secret-password"));
    println!(
        "   {}\n",
        tr("example-encryption-fits-user-supplied-passwords")
    );

    println!("\n{}", tr("example-encryption-all-examples-completed"));

    Ok(())
}
