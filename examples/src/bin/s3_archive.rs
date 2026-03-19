// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! S3 日志归档功能示例
//!
//! 演示 inklog 的 S3 云存储归档能力，包括：
//!
//! 1. **AWS 凭据检查**: 检测环境变量中的 AWS 访问凭据
//! 2. **LocalStack 连接**: 使用 LocalStack 进行本地开发和测试
//! 3. **日志归档演示**: 模拟日志数据归档到 S3 的完整流程
//!
//! ## 运行
//!
//! ```bash
//! # 运行示例（需要 AWS 凭据或 LocalStack）
//! cargo run --bin s3_archive
//!
//! # 或者使用环境变量指定凭据
//! export AWS_ACCESS_KEY_ID=your_key_id
//! export AWS_SECRET_ACCESS_KEY=your_secret_key
//! cargo run --bin s3_archive
//! ```
//!
//! ## AWS 凭据配置
//!
//! ### 方式 1: 环境变量
//!
//! ```bash
//! export AWS_ACCESS_KEY_ID=your_access_key_id
//! export AWS_SECRET_ACCESS_KEY=your_secret_access_key
//! export AWS_DEFAULT_REGION=us-east-1
//! ```
//!
//! ### 方式 2: AWS 凭证文件
//!
//! ```ini
//! # ~/.aws/credentials
//! [default]
//! aws_access_key_id = your_access_key_id
//! aws_secret_access_key = your_secret_access_key
//! ```
//!
//! ### 方式 3: IAM 角色 (EC2/ECS/Lambda)
//!
//! 在 EC2 实例上运行时，inklog 会自动使用实例元数据服务获取临时凭证。
//!
//! ## LocalStack 配置
//!
//! LocalStack 是一个本地 AWS 云模拟器，适合本地开发和测试。
//!
//! ### 启动 LocalStack
//!
//! ```bash
//! # 使用 Docker 启动 LocalStack
//! docker run -d --name localstack \
//!   -p 4566:4566 \
//!   -e SERVICES=s3 \
//!   -e DEBUG=1 \
//!   localstack/localstack:latest
//! ```
//!
//! ### 配置 LocalStack 端点
//!
//! ```bash
//! export AWS_ACCESS_KEY_ID=test
//! export AWS_SECRET_ACCESS_KEY=test
//! export AWS_ENDPOINT_URL=http://localhost:4566
//! ```
//!
//! ## S3 存储配置
//!
//! ### 创建 S3 存储桶
//!
//! ```bash
//! # 使用 AWS CLI
//! aws s3 mb s3://your-log-bucket --region us-east-1
//!
//! # 使用 LocalStack
//! aws --endpoint-url=http://localhost:4566 s3 mb s3://your-log-bucket
//! ```
//!
//! ### 配置生命周期策略
//!
//! 建议在 S3 控制台配置生命周期策略，自动将旧日志移动到低成本存储类别。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use inklog::archive::{CompressionType, S3ArchiveConfig, S3ArchiveManager, SecretString, StorageClass};
use inklog::archive::{ArchiveMetadata, ArchiveStatus};
use inklog_examples::common::{print_section, print_separator};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

/// AWS 凭据信息结构
struct AwsCredentials {
    access_key_id: String,
    secret_access_key: String,
    region: String,
}

/// 检查 AWS 凭据
///
/// 检查环境变量中是否设置了 AWS 访问凭据：
/// - `AWS_ACCESS_KEY_ID`: AWS 访问密钥 ID
/// - `AWS_SECRET_ACCESS_KEY`: AWS 秘密访问密钥
/// - `AWS_DEFAULT_REGION`: AWS 区域（可选，默认 us-east-1）
///
/// # 返回值
///
/// - `Ok(Some(credentials))`: 凭据完整，返回凭据信息
/// - `Ok(None)`: 缺少凭据，返回 None
/// - `Err`: 环境变量格式错误
fn check_aws_credentials() -> Result<Option<AwsCredentials>> {
    print_separator("AWS 凭据检查");

    // 检查必需的凭据环境变量
    let access_key = match env::var("AWS_ACCESS_KEY_ID") {
        Ok(val) => {
            if val.is_empty() {
                println!("  AWS_ACCESS_KEY_ID: [空字符串]");
                None
            } else {
                println!("  AWS_ACCESS_KEY_ID: [已设置]");
                Some(val)
            }
        }
        Err(_) => {
            println!("  AWS_ACCESS_KEY_ID: [未设置]");
            None
        }
    };

    let secret_key = match env::var("AWS_SECRET_ACCESS_KEY") {
        Ok(val) => {
            if val.is_empty() {
                println!("  AWS_SECRET_ACCESS_KEY: [空字符串]");
                None
            } else {
                println!("  AWS_SECRET_ACCESS_KEY: [已设置]");
                Some(val)
            }
        }
        Err(_) => {
            println!("  AWS_SECRET_ACCESS_KEY: [未设置]");
            None
        }
    };

    // 检查可选的区域配置
    let region = match env::var("AWS_DEFAULT_REGION") {
        Ok(val) => {
            println!("  AWS_DEFAULT_REGION: {}", val);
            val
        }
        Err(_) => {
            println!("  AWS_DEFAULT_REGION: [默认: us-east-1]");
            "us-east-1".to_string()
        }
    };

    // 检查可选的端点 URL（用于 LocalStack）
    let endpoint_url = match env::var("AWS_ENDPOINT_URL") {
        Ok(val) => {
            println!("  AWS_ENDPOINT_URL: {}", val);
            Some(val)
        }
        Err(_) => {
            println!("  AWS_ENDPOINT_URL: [未设置，将使用 AWS]");
            None
        }
    };

    println!();

    // 如果两个关键凭据都存在，返回凭据信息
    match (access_key, secret_key) {
        (Some(access), Some(secret)) => {
            println!("✓ 检测到 AWS 凭据");
            if endpoint_url.is_some() {
                println!("  (使用自定义端点: LocalStack 或兼容 S3 服务)");
            }
            Ok(Some(AwsCredentials {
                access_key_id: access,
                secret_access_key: secret,
                region,
            }))
        }
        (None, None) => {
            println!("✗ 未检测到 AWS 凭据");
            Ok(None)
        }
        (None, Some(_)) => {
            println!("✗ 缺少 AWS_ACCESS_KEY_ID");
            Ok(None)
        }
        (Some(_), None) => {
            println!("✗ 缺少 AWS_SECRET_ACCESS_KEY");
            Ok(None)
        }
    }
}

/// 显示优雅退出提示
///
/// 当 AWS 凭据不可用时，显示友好提示信息和配置指南。
fn show_graceful_exit() {
    print_separator("S3 归档示例 - 需要 AWS 凭据");

    println!("本示例需要 AWS 凭据才能运行。");
    println!();

    println!("请选择以下任一方式提供凭据：");
    println!();

    println!("【方式 1】设置环境变量");
    println!();
    println!("  # Linux/macOS");
    println!("  export AWS_ACCESS_KEY_ID=your_access_key_id");
    println!("  export AWS_SECRET_ACCESS_KEY=your_secret_access_key");
    println!("  export AWS_DEFAULT_REGION=us-east-1");
    println!();
    println!("  # Windows (PowerShell)");
    println!("  $env:AWS_ACCESS_KEY_ID='your_access_key_id'");
    println!("  $env:AWS_SECRET_ACCESS_KEY='your_secret_access_key'");
    println!("  $env:AWS_DEFAULT_REGION='us-east-1'");
    println!();

    println!("【方式 2】使用 AWS 配置文件");
    println!();
    println!("  # 创建 ~/.aws/credentials 文件");
    println!("  [default]");
    println!("  aws_access_key_id = your_access_key_id");
    println!("  aws_secret_access_key = your_secret_access_key");
    println!();
    println!("  # 创建 ~/.aws/config 文件");
    println!("  [default]");
    println!("  region = us-east-1");
    println!();

    println!("【方式 3】使用 LocalStack (本地开发)");
    println!();
    println!("  # 启动 LocalStack");
    println!("  docker run -d --name localstack \\");
    println!("    -p 4566:4566 \\");
    println!("    -e SERVICES=s3 \\");
    println!("    localstack/localstack:latest");
    println!();
    println!("  # 配置环境变量");
    println!("  export AWS_ACCESS_KEY_ID=test");
    println!("  export AWS_SECRET_ACCESS_KEY=test");
    println!("  export AWS_ENDPOINT_URL=http://localhost:4566");
    println!();

    println!("【方式 4】IAM 角色 (生产环境)");
    println!();
    println!("  如果在 AWS EC2/ECS/Lambda 上运行，");
    println!("  inklog 会自动使用实例元数据服务获取临时凭证。");
    println!();

    println!("{}", "=".repeat(60));
    println!("提示: 设置好凭据后，重新运行本示例即可");
    println!("{}", "=".repeat(60));
    println!();
}

/// 连接 LocalStack 或 AWS S3
///
/// 根据检测到的端点配置，连接到 LocalStack 或真实的 AWS S3。
///
/// # 参数
///
/// - `credentials`: AWS 凭据信息
///
/// # 返回值
///
/// 成功返回 S3ArchiveManager 实例，失败返回错误信息。
async fn connect_s3(credentials: &AwsCredentials) -> Result<S3ArchiveManager> {
    print_separator("连接 S3 服务");

    // 检查是否有自定义端点（LocalStack）
    let endpoint_url = env::var("AWS_ENDPOINT_URL").ok();
    let is_localstack = endpoint_url.is_some();

    // 确定目标服务
    let target_service = if is_localstack {
        "LocalStack"
    } else {
        "AWS S3"
    };

    print_section("S3 连接配置");
    println!("  目标服务: {}", target_service);
    println!("  区域: {}", credentials.region);
    if let Some(ref url) = endpoint_url {
        println!("  端点: {}", url);
    }
    println!();

    // 创建 S3 归档配置
    let config = S3ArchiveConfig {
        enabled: true,
        bucket: if is_localstack {
            "test-bucket".to_string()
        } else {
            // 注意：这里需要替换为真实存在的 S3 桶名称
            "your-log-archive-bucket".to_string()
        },
        region: credentials.region.clone(),
        archive_interval_days: 7,
        schedule_expression: None,
        local_retention_days: 30,
        local_retention_path: PathBuf::from("/tmp/inklog_archive_failures"),
        compression: CompressionType::Zstd,
        storage_class: StorageClass::Standard,
        prefix: "logs/".to_string(),
        access_key_id: SecretString::new(credentials.access_key_id.clone()),
        secret_access_key: SecretString::new(credentials.secret_access_key.clone()),
        session_token: SecretString::default(),
        endpoint_url,
        force_path_style: is_localstack,
        skip_bucket_validation: false,
        max_file_size_mb: 100,
        encryption: None,
        archive_format: "json".to_string(),
        parquet_config: Default::default(),
    };

    print_section("创建 S3 归档管理器");

    // 尝试创建 S3ArchiveManager
    match S3ArchiveManager::new(config.clone()).await {
        Ok(manager) => {
            println!("✓ S3 归档管理器创建成功");
            println!("  存储桶: {}", config.bucket);
            println!("  区域: {}", config.region);
            println!("  压缩: {:?}", config.compression);
            println!("  存储类别: {:?}", config.storage_class);
            Ok(manager)
        }
        Err(e) => {
            // 诊断连接失败的原因
            if is_localstack {
                println!("✗ LocalStack 连接失败");
                println!();
                println!("请确保 LocalStack 已启动:");
                println!("  docker run -d --name localstack \\");
                println!("    -p 4566:4566 \\");
                println!("    -e SERVICES=s3 \\");
                println!("    localstack/localstack:latest");
                println!();
                println!("并确保存储桶已创建:");
                println!("  aws --endpoint-url=http://localhost:4566 \\");
                println!("    s3 mb s3://test-bucket");
                println!();
                println!("错误详情: {:?}", e);
            } else {
                println!("✗ AWS S3 连接失败");
                println!();
                println!("请检查:");
                println!("  1. 凭据是否正确");
                println!("  2. 存储桶是否存在: {}", config.bucket);
                println!("  3. IAM 权限是否足够");
                println!();
                println!("创建存储桶:");
                println!("  aws s3 mb s3://{}", config.bucket);
                println!();
                println!("错误详情: {:?}", e);
            }
            Err(e.into())
        }
    }
}

/// 日志归档演示
///
/// 模拟完整的日志归档流程：
/// 1. 创建模拟日志数据
/// 2. 压缩日志数据
/// 3. 上传到 S3
/// 4. 验证归档结果
async fn archive_demo(manager: &S3ArchiveManager) -> Result<()> {
    print_separator("日志归档演示");

    // 生成模拟日志数据
    print_section("1. 生成模拟日志数据");

    let log_entries = vec![
        r#"{"timestamp":"2024-01-15T10:00:00Z","level":"INFO","message":"Application started","service":"auth-service","version":"1.0.0"}"#,
        r#"{"timestamp":"2024-01-15T10:00:01Z","level":"DEBUG","message":"Database connection pool initialized","service":"auth-service","pool_size":10}"#,
        r#"{"timestamp":"2024-01-15T10:00:05Z","level":"INFO","message":"User login attempt","service":"auth-service","user_id":"user123","ip":"192.168.1.100"}"#,
        r#"{"timestamp":"2024-01-15T10:00:06Z","level":"WARN","message":"Failed login attempt","service":"auth-service","user_id":"user123","reason":"invalid_password"}"#,
        r#"{"timestamp":"2024-01-15T10:00:10Z","level":"ERROR","message":"Database connection timeout","service":"auth-service","error":"Connection refused"}"#,
        r#"{"timestamp":"2024-01-15T10:00:15Z","level":"INFO","message":"Health check passed","service":"auth-service","latency_ms":5}"#,
        r#"{"timestamp":"2024-01-15T10:00:20Z","level":"INFO","message":"User logout","service":"auth-service","user_id":"user456"}"#,
        r#"{"timestamp":"2024-01-15T10:00:25Z","level":"DEBUG","message":"Cache hit","service":"auth-service","cache_key":"session:user123"}"#,
        r#"{"timestamp":"2024-01-15T10:00:30Z","level":"WARN","message":"Rate limit approaching","service":"auth-service","current_rate":90,"limit":100}"#,
        r#"{"timestamp":"2024-01-15T10:00:35Z","level":"INFO","message":"Token refresh successful","service":"auth-service","user_id":"user789"}"#,
    ];

    // 生成日志数据并拼接
    let log_data: Vec<u8> = log_entries.join("\n").into_bytes();

    let original_size = log_data.len();
    println!("✓ 生成 {} 条日志记录", log_entries.len());
    println!("  原始大小: {} bytes", original_size);

    // 定义时间范围
    let start_date = DateTime::parse_from_rfc3339("2024-01-15T10:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let end_date = DateTime::parse_from_rfc3339("2024-01-15T10:00:35Z")
        .unwrap()
        .with_timezone(&Utc);

    // 创建归档元数据
    print_section("2. 准备归档元数据");
    let metadata = ArchiveMetadata {
        original_size: original_size as i64,
        compressed_size: 0,
        record_count: log_entries.len() as i64,
        checksum: String::new(),
        start_date: None,
        end_date: None,
        archive_version: "1.0".to_string(),
        archive_type: "logs".to_string(),
        status: ArchiveStatus::InProgress,
        compression_type: None,
        storage_class: None,
        compression_ratio: 0.0,
        parquet_version: None,
        row_group_count: 0,
        tags: Vec::new(),
        s3_key: String::new(),
    };

    println!("  记录数量: {}", metadata.record_count);
    println!("  开始时间: {}", start_date);
    println!("  结束时间: {}", end_date);
    println!();

    // 执行归档（仅当存储桶存在时）
    print_section("3. 归档到 S3");
    println!("注意: 如果存储桶不存在，此步骤会失败");
    println!("      这是正常的，因为示例不会自动创建存储桶");
    println!();

    match manager
        .archive_logs(log_data, start_date, end_date, metadata)
        .await
    {
        Ok(s3_key) => {
            println!("✓ 归档成功!");
            println!("  S3 键: {}", s3_key);
            println!();
            println!("归档的数据可以通过以下方式查看:");
            println!("  aws s3 ls s3://test-bucket/logs/");
            println!("  aws s3 cp s3://test-bucket/{} ./", s3_key);
        }
        Err(e) => {
            println!("✗ 归档失败（预期行为）");
            println!("  错误: {:?}", e);
            println!();
            println!("这通常是因为:");
            println!("  1. 存储桶不存在");
            println!("  2. 凭据权限不足");
            println!("  3. 网络连接问题");
            println!();
            println!("请先创建存储桶:");
            println!("  aws --endpoint-url=http://localhost:4566 \\");
            println!("    s3 mb s3://test-bucket");
        }
    }

    Ok(())
}

/// 创建本地测试文件
///
/// 使用 `inklog_examples::common::temp_file_path()` 生成临时文件，
/// 演示如何使用本地文件进行归档前测试。
fn create_local_test_file() -> Result<String> {
    print_section("4. 创建本地测试文件");

    // 使用 common 模块的 temp_file_path 函数
    let file_path = inklog_examples::common::temp_file_path("s3_archive_demo");

    // 生成测试日志内容
    let test_content = r#"{"timestamp":"2024-01-15T10:00:00Z","level":"INFO","message":"Test log entry"}
{"timestamp":"2024-01-15T10:00:01Z","level":"DEBUG","message":"Another test entry"}
"#;

    fs::write(&file_path, test_content)
        .with_context(|| format!("Failed to write test file: {}", file_path))?;

    println!("✓ 创建测试文件: {}", file_path);
    println!("  大小: {} bytes", test_content.len());
    println!();

    // 打印清理指令
    print_section("文件清理");
    println!("测试完成后，请手动删除临时文件:");
    println!("  rm {}", file_path);
    println!();

    Ok(file_path)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== inklog S3 日志归档示例 ===\n");
    println!("本示例演示如何使用 inklog 将日志归档到 S3 存储。\n");

    // 步骤 1: 检查 AWS 凭据
    let credentials = match check_aws_credentials()? {
        Some(creds) => creds,
        None => {
            // 无凭据，显示优雅退出提示
            show_graceful_exit();
            return Ok(());
        }
    };

    // 等待一小段时间，让用户看到凭据检测结果
    sleep(Duration::from_millis(500)).await;

    // 步骤 2: 连接 S3 服务
    let manager = match connect_s3(&credentials).await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("连接失败: {:?}", e);
            return Ok(());
        }
    };

    // 等待连接建立
    sleep(Duration::from_millis(500)).await;

    // 步骤 3: 执行归档演示
    archive_demo(&manager).await?;

    // 步骤 4: 创建本地测试文件
    if let Err(e) = create_local_test_file() {
        eprintln!("创建测试文件失败: {:?}", e);
    }

    // 总结
    print_separator("示例完成");
    println!("本示例演示了:");
    println!();
    println!("  1. 如何检查 AWS 凭据");
    println!("  2. 如何连接到 LocalStack 或 AWS S3");
    println!("  3. 如何配置 S3 归档参数");
    println!("  4. 如何执行日志归档操作");
    println!("  5. 如何创建本地测试文件");
    println!();

    println!("下一步:");
    println!("  1. 创建 S3 存储桶");
    println!("  2. 重新运行本示例");
    println!("  3. 验证日志已归档到 S3");
    println!();

    Ok(())
}
