// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 加密+压缩组合示例
// 演示加密与压缩不能同时使用的原因和解决方案

use inklog::{FileSinkConfig, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Inklog 加密与压缩组合示例 ===\n");

    // 说明加密与压缩不兼容的原因
    println!("重要说明：加密与压缩不能同时启用");
    println!("原因：");
    println!("  1. 压缩依赖于数据冗余，而加密后的数据看起来是随机的");
    println!("  2. 加密后的数据压缩率极低，甚至可能增加文件大小");
    println!("  3. 同时启用会造成 CPU 浪费而无存储收益\n");

    // 生成加密密钥
    let encryption_key = "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI="; // Base64 32字节
    std::env::set_var("INKLOG_ENCRYPTION_KEY", encryption_key);

    // 方案1：仅加密（推荐用于敏感数据）
    println!("方案1：仅加密（推荐用于敏感数据）");
    println!("-------------------------------------------");

    let encrypted_config = InklogConfig {
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: "logs/encrypted_only.log.enc".into(),
            max_size: "10MB".into(),
            rotation_time: "daily".into(),
            keep_files: 7,
            compress: false, // 重要：加密时不压缩
            encrypt: true,
            encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(encrypted_config).await?;

    log::info!("This message will be encrypted (not compressed)");
    log::warn!("Sensitive data protected at rest");
    log::error!("Error details also encrypted");

    // 方案2：仅压缩（推荐用于非敏感数据）
    println!("\n方案2：仅压缩（推荐用于非敏感数据）");
    println!("-------------------------------------------");

    let compressed_config = InklogConfig {
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: "logs/compressed_only.log.gz".into(),
            max_size: "10MB".into(),
            rotation_time: "daily".into(),
            keep_files: 14,
            compress: true, // 压缩
            compression_level: 3,
            encrypt: false, // 不加密
            ..Default::default()
        }),
        ..Default::default()
    };

    let _logger2 = LoggerManager::with_config(compressed_config).await?;

    log::info!("This message will be compressed (not encrypted)");
    log::warn!("Logs optimized for storage efficiency");
    log::error!("Standard error logging with compression");

    // 方案3：分阶段处理（高级用例）
    println!("\n方案3：分阶段处理（高级用例）");
    println!("-------------------------------------------");
    println!("如果需要同时压缩和加密，建议分阶段处理：");
    println!("  1. 压缩日志文件（未加密）");
    println!("  2. 使用外部工具加密压缩文件（如 openssl）");
    println!("  3. 加密后可安全上传到云存储\n");

    println!("示例代码（伪代码）：");
    println!("```bash");
    println!("# 压缩日志");
    println!("gzip -k logs/app.log");
    println!();
    println!("# 加密压缩文件");
    println!("openssl enc -aes-256-cbc -salt -in logs/app.log.gz -out logs/app.log.gz.enc -k $ENCRYPTION_KEY");
    println!("```");

    // 清理
    std::env::remove_var("INKLOG_ENCRYPTION_KEY");

    println!("\n=== 示例完成 ===");
    Ok(())
}
