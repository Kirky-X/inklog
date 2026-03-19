// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use inklog::sink::encryption::get_encryption_key;
use inklog::{FileSinkConfig, LogRecord};
use inklog::sink::LogSink;
use inklog_examples::common::{print_section, print_separator, temp_file_path};
use rand::Rng;
use std::io::Write;
use std::path::PathBuf;
use tracing::Level;

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
    use base64::{engine::general_purpose, Engine as _};
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
    print_separator("步骤 1: 写入明文日志");

    // 手动创建配置
    let config = FileSinkConfig {
        enabled: true,
        path: PathBuf::from(file_path),
        encrypt: false, // 先写入明文
        batch_size: 10,
        flush_interval_ms: 100,
        ..Default::default()
    };

    // 创建 FileSink 实例
    let mut sink = inklog::sink::file::FileSink::new(config)?;

    // 创建测试日志内容
    let plaintext = r#"2026-03-19T10:30:00.000Z [INFO] encryption - 这是一条加密的日志消息
2026-03-19T10:30:00.001Z [INFO] encryption - 敏感数据会被自动加密保护
2026-03-19T10:30:00.002Z [WARN] encryption - 这是警告日志，同样会被加密
2026-03-19T10:30:00.003Z [INFO] encryption - 测试日志条目 1
2026-03-19T10:30:00.004Z [INFO] encryption - 测试日志条目 2
2026-03-19T10:30:00.005Z [INFO] encryption - 测试日志条目 3
"#;

    // 逐行写入
    for line in plaintext.lines() {
        let record = LogRecord::new(
            Level::INFO,
            "encryption".to_string(),
            line.to_string(),
        );
        sink.write(&record)?;
    }

    sink.flush()?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    println!("✓ 明文日志写入完成: {}", file_path);
    println!("  文件大小: {} bytes", plaintext.len());

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
    print_separator("步骤 2: 加密日志文件");

    // 获取加密密钥
    let key = get_encryption_key(key_env)?;
    println!("✓ 密钥获取成功: {} 字节", key.len());

    // 创建 AES-256-GCM 密码器
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;

    // 生成随机 nonce
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

    // 读取明文
    let plaintext = std::fs::read(plaintext_path)?;
    println!("✓ 读取明文: {} bytes", plaintext.len());

    // 加密
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_slice())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    println!("✓ 加密完成: {} bytes -> {} bytes (含认证标签)",
        plaintext.len(), ciphertext.len());

    // 写入加密文件
    let mut file = std::fs::File::create(encrypted_path)?;
    file.write_all(MAGIC_HEADER)?;                          // Magic Header
    file.write_all(&VERSION.to_le_bytes())?;                 // Version
    file.write_all(&ALGO_AES_GCM.to_le_bytes())?;           // Algorithm ID
    file.write_all(&nonce_bytes)?;                           // Nonce
    file.write_all(&ciphertext)?;                            // Ciphertext + Auth Tag

    println!("✓ 加密文件写入完成: {}", encrypted_path);

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
    print_separator("步骤 3: 解析加密文件格式");

    let path = PathBuf::from(file_path);
    let mut file = std::fs::File::open(&path)?;

    // 读取文件头（24 字节）
    let mut header = [0u8; 24];
    std::io::Read::read_exact(&mut file, &mut header)?;

    // 获取文件大小
    let file_size = std::fs::metadata(&path)?.len();

    // 解析并展示各字段
    print_section("文件头解析 (24 bytes)");

    // Magic Header (8 bytes)
    let magic = &header[..8];
    println!("  Magic Header (8 bytes): {:?}", String::from_utf8_lossy(magic));
    println!("    └─ 标识文件为 inklog 加密格式");

    // Version (2 bytes)
    let version = u16::from_le_bytes([header[8], header[9]]);
    println!("  Version (2 bytes):       {}", version);
    println!("    └─ 格式版本号，用于向前兼容");

    // Algorithm ID (2 bytes)
    let algo = u16::from_le_bytes([header[10], header[11]]);
    println!("  Algorithm ID (2 bytes):  {} ({})", algo, match algo {
        1 => "AES-256-GCM",
        _ => "Unknown",
    });
    println!("    └─ 加密算法标识");

    // Nonce (12 bytes)
    let nonce_bytes = &header[12..24];
    println!("  Nonce (12 bytes):        {:?}", nonce_bytes);
    println!("    └─ 随机数，每个文件唯一，防止重放攻击");

    // 展示文件结构
    print_section("文件结构");

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

    println!("\n  文件总大小: {} bytes", file_size);
    println!("  明文大小（估算）: ~{} bytes", ciphertext_size);

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
    print_separator("步骤 4: 解密验证");

    let path = PathBuf::from(file_path);
    let mut file = std::fs::File::open(&path)?;

    // 读取文件头
    let mut header = [0u8; 24];
    std::io::Read::read_exact(&mut file, &mut header)?;

    // 验证 Magic Header
    if &header[..8] != MAGIC_HEADER {
        return Err("Invalid file header: not an encrypted inklog file".into());
    }

    // 获取密钥
    let key = get_encryption_key(key_env)?;
    println!("✓ 密钥获取成功");

    // 提取 nonce
    let nonce_bytes = &header[12..24];
    let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);

    // 读取密文
    let mut ciphertext = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut ciphertext)?;

    // 创建密码器并解密
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("Decryption failed: {}", e))?;

    let plaintext_str = String::from_utf8(plaintext.clone())
        .map_err(|e| format!("Invalid UTF-8 in decrypted data: {}", e))?;

    println!("✓ 解密成功");
    println!("  密文大小: {} bytes", ciphertext.len());
    println!("  明文大小: {} bytes", plaintext.len());

    // 验证内容
    print_section("内容验证");

    let has_expected_content = plaintext_str.contains("加密的日志消息")
        && plaintext_str.contains("敏感数据")
        && plaintext_str.contains("警告日志");

    if has_expected_content {
        println!("✓ 内容验证通过");
        println!("  - 包含测试消息");
        println!("  - 包含敏感数据关键字");
        println!("  - 包含警告日志");
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
    print_separator("步骤 5: 明文与密文对比");

    println!("解密后的明文内容:");
    println!("{}", "─".repeat(50));
    println!("{}", plaintext);
    println!("{}", "─".repeat(50));

    println!("\n关键差异:");
    println!("  - 明文: 人类可读，可直接查看日志内容");
    println!("  - 密文: 二进制格式，需要密钥解密才能查看");
    println!("  - 认证标签: 验证数据完整性，防止篡改");
}

/// 清理临时文件
///
/// # 参数
///
/// * `file_path` - 要删除的文件路径
fn cleanup(file_path: &str) {
    print_separator("清理临时文件");

    if let Err(e) = std::fs::remove_file(file_path) {
        eprintln!("警告: 删除文件失败: {}", e);
    } else {
        println!("✓ 已删除临时文件: {}", file_path);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog 日志加密示例 ===\n");

    // 1. 生成临时密钥
    print_separator("生成临时加密密钥");

    let key = generate_temp_key();
    println!("生成的临时密钥（Base64 编码）:");
    println!("{}\n", key);

    // 设置环境变量
    let key_env = "LOG_ENCRYPTION_KEY";
    std::env::set_var(key_env, &key);
    println!("✓ 已设置环境变量: {}", key_env);

    // 2. 生成临时文件路径
    let plain_path = temp_file_path("plaintext");
    let encrypted_path = format!("{}.enc", plain_path);
    println!("\n临时文件路径:");
    println!("  明文:   {}", plain_path);
    println!("  加密:   {}", encrypted_path);

    // 3. 写入明文日志
    write_plaintext_log(&plain_path).await?;

    // 4. 加密日志文件
    encrypt_log_file(&plain_path, &encrypted_path, key_env)?;

    // 5. 解析加密文件格式
    parse_encrypted_format(&encrypted_path)?;

    // 6. 解密验证
    let plaintext = decrypt_and_verify(&encrypted_path, key_env)?;

    // 7. 展示对比
    show_comparison(&plaintext);

    // 8. 清理
    cleanup(&plain_path);
    cleanup(&encrypted_path);

    // 移除环境变量
    std::env::remove_var(key_env);
    println!("✓ 已清除环境变量: {}", key_env);

    // 展示密钥管理最佳实践
    print_separator("密钥管理最佳实践");

    println!("在生产环境中，应遵循以下最佳实践：\n");

    println!("1. 密钥存储：");
    println!("   - 使用安全的密钥管理服务（如 AWS KMS、HashiCorp Vault）");
    println!("   - 不要将密钥硬编码在代码中");
    println!("   - 通过环境变量或密钥文件传递密钥\n");

    println!("2. 密钥轮换：");
    println!("   - 定期更换加密密钥");
    println!("   - 保留旧密钥用于解密历史日志");
    println!("   - 使用密钥版本管理\n");

    println!("3. 访问控制：");
    println!("   - 限制密钥访问权限");
    println!("   - 记录密钥使用日志");
    println!("   - 实施最小权限原则\n");

    println!("4. 安全传输：");
    println!("   - 使用 TLS 加密传输密钥");
    println!("   - 避免在日志中输出密钥");
    println!("   - 使用安全的 CI/CD 管道\n");

    // 展示加密性能特性
    print_separator("加密性能特性");

    println!("AES-256-GCM 加密性能优势：\n");

    println!("1. 硬件加速：");
    println!("   - 现代 CPU 支持 AES-NI 指令集");
    println!("   - 硬件加速可达 10+ GB/s");
    println!("   - 几乎不影响日志写入性能\n");

    println!("2. AEAD 认证：");
    println!("   - 同时提供加密和完整性验证");
    println!("   - 自动检测数据篡改");
    println!("   - 防止密文攻击\n");

    println!("3. 并行处理：");
    println!("   - GCM 模式支持并行加密");
    println!("   - 利用多核 CPU 提高性能");
    println!("   - 适合批量日志处理\n");

    // 展示密钥格式支持
    print_separator("支持的密钥格式");

    println!("inklog 支持三种密钥格式：\n");

    println!("1. Base64 编码（推荐）：");
    println!("   - 32 字节密钥的 Base64 编码");
    println!("   - 示例: MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE=");
    println!("   - 安全、标准化、易于传输\n");

    println!("2. 原始字节：");
    println!("   - 直接使用 32 字节原始密钥");
    println!("   - 示例: abcdefghijklmnopqrstuvwxyz123456");
    println!("   - 适用于简单测试场景\n");

    println!("3. 密码派生：");
    println!("   - 从密码使用 PBKDF2 派生密钥");
    println!("   - 示例: my-secret-password");
    println!("   - 适用于用户提供的密码场景\n");

    // 完成
    println!("\n✓ 所有示例演示完成");

    Ok(())
}
