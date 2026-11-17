// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 源头敏感字段脱敏示例
//!
//! 演示如何用 `mark_sensitive_fields` 声明源头层敏感字段：命中字段名的
//! 值在订阅器扇出前被整值替换为 `<redacted:{field}>` 占位串，原始值不落盘。
//!
//! # 内容
//!
//! 1. 构建前注册两个敏感字段（email / phone）
//! 2. 产生携带敏感字段与普通字段的 NDJSON 文件日志
//! 3. 读回落盘内容，核对占位串已落盘、原始值不存在
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin sensitive_fields
//! ```

use inklog::config::FileSinkConfig;
use inklog::{InklogConfig, LoggerManager, OutputFormat, mark_sensitive_fields, sensitive_fields};
use inklog_examples::common::{print_separator, tr, tr_args};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-sensitive-inklog-source-level-redaction-examples")
    );

    print_separator(&tr("example-sensitive-1-register-before-build"));

    // 1. 进程级注册：必须在 logger 构建之前声明——每个订阅者在构建期对
    //    注册表做快照，事后 mark 不影响已构建的 subscriber。
    mark_sensitive_fields(["email", "phone"]);
    let registered = sensitive_fields();
    println!(
        "{}",
        tr_args(
            "example-sensitive-registered-count",
            &[("count", &registered.len())]
        )
    );
    assert!(registered.contains("email"));
    assert!(registered.contains("phone"));

    print_separator(&tr("example-sensitive-2-emit-records-with-fields"));

    // 2. NDJSON 文件 sink：字段以 JSON 对象序列化，占位串可逐行核对。
    let temp_dir = TempDir::new()?;
    let log_file = temp_dir.path().join("sensitive_fields.log");
    let config = InklogConfig {
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_file.clone(),
            output_format: OutputFormat::Json,
            ..Default::default()
        }),
        ..Default::default()
    };
    let logger = LoggerManager::with_config(config).await?;

    tracing::info!(
        target: "app::auth",
        message = "user login",
        email = "alice@example.com",
        username = "alice"
    );
    tracing::info!(
        target: "app::api",
        message = "token refreshed",
        phone = "13812345678",
        username = "bob"
    );
    println!("{}", tr("example-sensitive-emitted-two-records"));

    // shutdown 触发 flush_batch 保证落盘后再读（仓库口径：先关停后读文件）
    logger.shutdown()?;

    print_separator(&tr("example-sensitive-3-read-back-and-verify"));

    // 3. 读回落盘 NDJSON：命中字段的值为占位串，普通字段与消息原样保留。
    let content = std::fs::read_to_string(&log_file)?;
    for line in content.lines() {
        println!(
            "{}",
            tr_args("example-sensitive-disk-line", &[("line", &line)])
        );
    }
    assert!(
        content.contains("<redacted:email>"),
        "source-layer placeholder must reach disk"
    );
    assert!(
        content.contains("<redacted:phone>"),
        "source-layer placeholder must reach disk"
    );
    assert!(
        !content.contains("alice@example.com"),
        "raw email must never hit disk"
    );
    assert!(
        !content.contains("13812345678"),
        "raw phone must never hit disk"
    );
    assert!(
        content.contains("alice") && content.contains("bob"),
        "non-registered fields must stay verbatim"
    );
    println!(
        "{}",
        tr("example-sensitive-placeholders-on-disk-raw-values-absent")
    );

    println!("\n{}", tr("example-sensitive-all-examples-completed"));

    Ok(())
}
