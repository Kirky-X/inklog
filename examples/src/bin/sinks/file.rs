// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! File Sink 示例
//!
//! 演示 inklog 文件输出的三种核心功能：
//!
//! 1. **basic_file**: 基础文件写入
//! 2. **file_rotation**: 文件轮转（按大小）
//! 3. **file_compression**: Zstd 压缩
//!
//! ## 运行
//!
//! ```bash
//! cargo run --package inklog-examples --features compression --bin file
//! ```
//!
//! 示例3（Zstd 压缩）需要 `compression` feature 提供 zstd 后端，故本 binary 声明了
//! `required-features`；省略 `--features compression` 时 cargo 会直接报
//! “target requires the features” 错误，而不是运行后才失败。
//! 注意：在自身项目中 `compress = true` 但未启用任何压缩后端（`compression`/`zstd`
//! 或 `gzip`）时，FileSink 只告警并保持文件未压缩——该功能必须显式启用。
//!
//! ## 核心特性
//!
//! - **文件轮转**: 按 size/time 自动轮转，支持保留文件数量控制
//! - **压缩**: 支持 ZSTD、GZIP、Brotli 压缩算法
//! - **加密**: 支持 AES-256-GCM 加密（见 encryption 示例）
//! - **清理**: 自动清理过期日志文件（retention_days）
//!
//! ## 文件命名规则
//!
//! - 当前日志: `app.log`
//! - 轮转日志: `app.log.1`, `app.log.2`, ...
//! - 压缩日志: `app.log.1.zst`, `app.log.2.zst`, ...

use inklog::LogRecord;
use inklog::chrono::Utc;
use inklog::config::FileSinkConfig;
use inklog::sink::LogSink;
use inklog::sink::file::FileSink;
use inklog_examples::common::{print_section, print_separator, temp_file_path, tr, tr_args};
use std::path::PathBuf;

/// 示例1: 基础文件写入
///
/// 演示最基本的文件日志写入功能：
/// - 创建 FileSink 配置
/// - 写入多级别日志
/// - 查看文件内容
async fn basic_file() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-file-example-1-basic-file-writing"));

    let log_path = temp_file_path("basic");
    println!(
        "{}",
        tr_args("example-file-log-file-path", &[("log_path", &log_path)])
    );

    let config = FileSinkConfig {
        enabled: true,
        path: PathBuf::from(&log_path),
        max_size: "10MB".to_string(),       // 单个文件最大 10MB
        rotation_time: "daily".to_string(), // 每天轮转
        keep_files: 10,                     // 保留 10 个历史文件
        compress: false,                    // 不启用压缩
        compression_level: 3,
        encrypt: false,
        encryption_key_env: None,
        encryption_key_file: None,
        retention_days: 7,
        max_total_size: "1GB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 100,
        flush_interval_ms: 1000,
        fsync: false,
        audit_chain_enabled: false,
        masking_enabled: false,
        output_format: Default::default(),
    };

    let sink = FileSink::new(config)?;

    print_section(&tr("example-file-write-logs"));
    let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
    for level in levels {
        let record = LogRecord {
            timestamp: Utc::now(),
            level: level.to_string(),
            message: format!("this is a {} level log", level),
            target: "file_example::basic".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
            trace_id: None,
            span_id: None,
        };
        sink.write(&record).await?;
    }

    sink.flush().await?;

    // 读取并展示文件内容（tokio::fs 异步读取，避免阻塞 worker 线程）
    print_section(&tr("example-file-file-content"));
    let content = tokio::fs::read_to_string(&log_path).await?;
    println!("{}", content);

    cleanup_files(&log_path, "inklog_example_basic").await?;
    println!("\n{}\n", tr("example-file-basic-file-writing-complete"));

    Ok(())
}

/// 示例2: 文件轮转
///
/// 演示文件轮转功能：
/// - 配置小文件大小（100字节）触发轮转
/// - 写入大量日志触发多次轮转
/// - 展示轮转文件命名规则
async fn file_rotation() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-file-example-2-file-rotation"));

    let log_path = temp_file_path("rotation");
    println!(
        "{}",
        tr_args("example-file-log-file-path", &[("log_path", &log_path)])
    );

    // 创建 FileSink 配置（小文件触发轮转）
    let config = FileSinkConfig {
        enabled: true,
        path: PathBuf::from(&log_path),
        max_size: "100".to_string(), // 100 字节触发轮转（非常小的值）
        rotation_time: "daily".to_string(),
        keep_files: 10,  // 保留 10 个历史文件
        compress: false, // 暂不压缩（下个示例演示）
        compression_level: 3,
        encrypt: false,
        encryption_key_env: None,
        encryption_key_file: None,
        retention_days: 7,
        max_total_size: "1GB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 1,         // 每条日志立即 flush
        flush_interval_ms: 10, // 10ms flush 间隔
        fsync: false,
        audit_chain_enabled: false,
        masking_enabled: false,
        output_format: Default::default(),
    };

    let sink = FileSink::new(config)?;

    print_section(&tr("example-file-write-logs-trigger-rotation"));
    for i in 1..=50 {
        let record = LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: format!(
                "log message #{:03} - a longer log message that triggers rotation",
                i
            ),
            target: "file_example::rotation".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
            trace_id: None,
            span_id: None,
        };
        sink.write(&record).await?;
        // 立即 flush 以触发轮转检查
        sink.flush().await?;
    }

    // 展示轮转后的文件列表
    print_section(&tr("example-file-rotated-file-list"));
    let log_dir = PathBuf::from(&log_path)
        .parent()
        .ok_or_else(|| format!("the path has no parent directory: {}", log_path))?
        .to_path_buf();

    println!(
        "{}",
        tr_args("example-file-directory", &[("value", &(log_dir.display()))])
    );
    println!("\n{}", tr("example-file-file-list"));

    // 列出目录中所有文件（tokio::fs 异步遍历，避免阻塞 worker 线程）
    let mut dir_entries = tokio::fs::read_dir(&log_dir).await?;
    let mut all_files: Vec<_> = Vec::new();
    while let Some(entry) = dir_entries.next_entry().await? {
        all_files.push(entry);
    }

    all_files.sort_by_key(|entry| entry.file_name());

    // 轮转文件名带创建年份后缀；按运行时当前年份识别，避免硬编码过期
    let year_tag = format!("_{}", chrono::Utc::now().format("%Y"));

    let mut rotation_count = 0;
    for file in &all_files {
        let file_name = file.file_name().to_string_lossy().to_string();
        if file_name.contains("inklog_example_rotation") {
            let metadata = file.metadata().await?;
            let size = metadata.len();
            let marker = if file_name.contains(&year_tag) {
                rotation_count += 1;
                " [rotated]"
            } else {
                " [current]"
            };
            println!(
                "  {}",
                tr_args(
                    "example-file-bytes",
                    &[
                        ("file_name", &file_name),
                        ("size", &size),
                        ("marker", &marker)
                    ]
                )
            );
        }
    }

    println!(
        "\n{}",
        tr_args(
            "example-file-rotation-count",
            &[("rotation_count", &rotation_count)]
        )
    );

    cleanup_files(&log_path, "inklog_example_rotation").await?;
    println!("\n{}\n", tr("example-file-file-rotation-demo-complete"));

    Ok(())
}

/// 示例3: Zstd 压缩
///
/// 演示日志文件压缩功能：
/// - 启用 Zstd 压缩
/// - 配置小文件触发轮转
/// - 展示压缩后的 .zst 文件
async fn file_compression() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-file-example-3-zstd-compression"));

    let log_path = temp_file_path("compression");
    println!(
        "{}",
        tr_args("example-file-log-file-path", &[("log_path", &log_path)])
    );

    // 创建 FileSink 配置（启用压缩）
    let config = FileSinkConfig {
        enabled: true,
        path: PathBuf::from(&log_path),
        max_size: "100".to_string(), // 100 字节触发轮转
        rotation_time: "daily".to_string(),
        keep_files: 10,
        compress: true,       // 启用压缩
        compression_level: 3, // Zstd 压缩级别（1-21，默认 3）
        encrypt: false,
        encryption_key_env: None,
        encryption_key_file: None,
        retention_days: 7,
        max_total_size: "1GB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 1,         // 每条日志立即 flush
        flush_interval_ms: 10, // 10ms flush 间隔
        fsync: false,
        audit_chain_enabled: false,
        masking_enabled: false,
        output_format: Default::default(),
    };

    let sink = FileSink::new(config)?;

    print_section(&tr("example-file-write-logs-trigger-rotation-and"));
    for i in 1..=50 {
        let record = LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: format!(
                "log message #{:03} - repeated content shows compression well (AAABBBCCCDDDEEEFFF)",
                i
            ),
            target: "file_example::compression".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
            trace_id: None,
            span_id: None,
        };
        sink.write(&record).await?;
        // 立即 flush 以触发轮转检查
        sink.flush().await?;
    }

    print_section(&tr("example-file-wait-for-compression-to-finish"));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // 展示压缩文件列表
    print_section(&tr("example-file-compressed-file-list"));
    let log_dir = PathBuf::from(&log_path)
        .parent()
        .ok_or_else(|| format!("the path has no parent directory: {}", log_path))?
        .to_path_buf();

    println!(
        "{}",
        tr_args("example-file-directory", &[("value", &(log_dir.display()))])
    );
    println!("\n{}", tr("example-file-file-list"));

    // 列出目录中所有文件（tokio::fs 异步遍历，避免阻塞 worker 线程）
    let mut dir_entries = tokio::fs::read_dir(&log_dir).await?;
    let mut all_files: Vec<_> = Vec::new();
    while let Some(entry) = dir_entries.next_entry().await? {
        all_files.push(entry);
    }

    all_files.sort_by_key(|entry| entry.file_name());

    let mut compression_count = 0;
    for file in &all_files {
        let file_name = file.file_name().to_string_lossy().to_string();
        if file_name.contains("inklog_example_compression") {
            let metadata = file.metadata().await?;
            let size = metadata.len();

            let marker = if file_name.ends_with(".zst") {
                compression_count += 1;
                " [compressed]"
            } else {
                " [current]"
            };

            println!(
                "  {}",
                tr_args(
                    "example-file-bytes",
                    &[
                        ("file_name", &file_name),
                        ("size", &size),
                        ("marker", &marker)
                    ]
                )
            );
        }
    }

    println!(
        "\n{}",
        tr_args(
            "example-file-compressed-file-count",
            &[("compression_count", &compression_count)]
        )
    );

    cleanup_files(&log_path, "inklog_example_compression").await?;
    println!("\n{}\n", tr("example-file-zstd-compression-demo-complete"));

    Ok(())
}

/// 清理临时文件（包括轮转文件）
///
/// 删除指定路径相关的所有文件（包括轮转文件和压缩文件）。
/// 使用 `tokio::fs` 异步 API，避免在 async 上下文中
/// 调用同步文件 IO 阻塞 tokio worker 线程。
async fn cleanup_files(log_path: &str, prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    print_section(&tr("example-file-clean-up-temporary-files"));

    let log_dir = PathBuf::from(log_path)
        .parent()
        .ok_or_else(|| format!("the path has no parent directory: {}", log_path))?
        .to_path_buf();

    let mut deleted_count = 0;

    let mut entries = tokio::fs::read_dir(&log_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_name = entry.file_name().to_string_lossy().to_string();

        // 只删除当前示例相关的文件
        if file_name.contains(prefix) {
            tokio::fs::remove_file(entry.path()).await?;
            println!(
                "  {}",
                tr_args("example-file-deleting", &[("file_name", &file_name)])
            );
            deleted_count += 1;
        }
    }

    if deleted_count == 0 {
        println!("  {}", tr("example-file-no-files-needed-cleaning"));
    } else {
        println!(
            "\n  {}",
            tr_args(
                "example-file-deleted-files-in-total",
                &[("deleted_count", &deleted_count)]
            )
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}\n", tr("example-file-inklog-file-sink-examples"));

    basic_file().await?;

    file_rotation().await?;

    file_compression().await?;

    println!("{}", tr("example-file-all-examples-complete"));
    Ok(())
}
