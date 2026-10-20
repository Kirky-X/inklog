// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Builder 模式配置示例
//!
//! 演示 LoggerManager::builder 链式 API 的各种用法。
//!
//! # 运行
//! ```bash
//! cargo run --package inklog-examples --bin builder
//! ```
//!
//! # 示例内容
//! - 基础 Builder 用法：最简单的链式配置
//! - 多 Sink 配置：同时启用 Console 和 File Sink
//! - 高级选项：展示日志级别、JSON 输出、性能参数等配置
//! - 配置构建器展示：展示 Builder 的灵活性
//!
//! 各配置片段以字符串展示（不在本示例中实际初始化）；`LoggerManager::builder`
//! 本身没有限制，真实项目在 async 函数中调用 `build().await` 即生效。

use inklog_examples::common::print_section;
use inklog_examples::common::{tr, tr_args};

fn main() {
    println!("{}\n", tr("example-builder-inklog-builder-pattern"));

    show_basic_builder();
    show_multi_sink_builder();
    show_advanced_options_builder();
    show_performance_options();
    show_console_options();
    show_file_options();

    // 展示 Builder 配置构建（不实际初始化）
    show_builder_pattern_flexibility();

    println!(
        "\n{}",
        tr("example-builder-all-configuration-examples-have")
    );
    println!(
        "\nNote: the code snippets above are shown as strings (for section-by-section reading); they are not actually initialized in this example;"
    );
    println!(
        "Builder has no such limit: in a real project, call build().await inside an async function."
    );
}

/// 展示基础 Builder 用法
///
/// 演示最简单的链式配置，仅启用 Console Sink。
fn show_basic_builder() {
    print_section(&tr("example-builder-example-1-basic-builder-usage"));

    let code = r#"
// Simplest configuration: console sink only
let _logger = LoggerManager::builder()
    .level("debug")      // Set the log level to DEBUG
    .console(true)       // Enable console output
    .build()
    .await?;
"#;

    println!(
        "{}",
        tr_args("example-builder-code-example", &[("code", &code)])
    );
    println!("{}", tr("example-builder-configuration-notes"));
    println!("  {}", tr("example-builder-log-level-debug"));
    println!("  {}", tr("example-builder-console-sink-enabled"));
    println!("  {}\n", tr("example-builder-file-sink-not-enabled"));
}

/// 展示多 Sink 配置
///
/// 演示同时配置 Console 和 File Sink，日志同时输出到控制台和文件。
fn show_multi_sink_builder() {
    print_section(&tr("example-builder-example-2-multiple-sinks"));

    let code = r#"
// Enable both the console and file sinks
let _logger = LoggerManager::builder()
    .level("info")                    // Set the log level to INFO
    .console(true)                    // Enable console output
    .file("logs/app.log")             // Enable file output
    .build()
    .await?;
"#;

    println!(
        "{}",
        tr_args("example-builder-code-example", &[("code", &code)])
    );
    println!("{}", tr("example-builder-configuration-notes"));
    println!("  {}", tr("example-builder-log-level-info"));
    println!("  {}", tr("example-builder-console-sink-enabled"));
    println!("  {}\n", tr("example-builder-file-sink-logs-app-log"));
}

/// 展示高级配置选项
///
/// 演示 Builder 的高级配置选项，包括格式、输出格式等。
fn show_advanced_options_builder() {
    print_section(&tr("example-builder-example-3-advanced-options"));

    let code = r#"
// Console emits JSON lines + the most detailed level
let _logger = LoggerManager::builder()
    .level("trace")                   // The most detailed log level
    .console(true)                    // Enable the console
    .console_json(true)               // Console emits whole JSON records (not text templates)
    .file("logs/app.log")             // Enable file output (file format is configured separately)
    .build()
    .await?;
"#;

    println!(
        "{}",
        tr_args("example-builder-code-example", &[("code", &code)])
    );
    println!("{}", tr("example-builder-configuration-notes"));
    println!("  {}", tr("example-builder-log-level-trace-most-detailed"));
    println!("  {}", tr("example-builder-console-output-json-lines"));
    println!("  {}\n", tr("example-builder-file-sink-logs-app-log"));
    println!(
        "Note: format() sets the text render template (e.g. \"{{timestamp}} [{{level}}] ...\");"
    );
    println!(
        "it is orthogonal to the output format — format(\"json\") only prints the template text as-is,"
    );
    println!("{}\n", tr("example-builder-json-output-requires-console"));
}

/// 展示性能配置选项
///
/// 演示 Channel 容量、Worker 线程数等性能相关配置。
fn show_performance_options() {
    print_section(&tr("example-builder-example-4-performance-options"));

    let code = r#"
// High-performance configuration (for production)
let _logger = LoggerManager::builder()
    .level("info")
    .console(true)
    .file("logs/app.log")
    .channel_capacity(50000)          // Channel capacity 50,000
    .worker_threads(4)                // 4 worker threads
    .build()
    .await?;
"#;

    println!(
        "{}",
        tr_args("example-builder-code-example", &[("code", &code)])
    );
    println!("{}", tr("example-builder-configuration-notes"));
    println!(
        "  {}",
        tr("example-builder-channel-capacity-50-000-default")
    );
    println!("  {}", tr("example-builder-worker-threads-4-default-3"));
    println!("  {}\n", tr("example-builder-use-case-high-throughput"));
}

/// 展示 Console Sink 配置选项
///
/// 演示 Console 的彩色输出、错误级别输出等配置。
fn show_console_options() {
    print_section(&tr("example-builder-example-5-console-sink-options"));

    let code = r#"
// Custom console output behavior
let _logger = LoggerManager::builder()
    .level("debug")
    .console(true)                    // Enable the console
    .console_colored(true)            // Enable colored output
    .console_stderr_levels(&["error", "warn"])  // ERROR/WARN go to stderr
    .build()
    .await?;
"#;

    println!(
        "{}",
        tr_args("example-builder-code-example", &[("code", &code)])
    );
    println!("{}", tr("example-builder-configuration-notes"));
    println!("  {}", tr("example-builder-colored-output-enabled"));
    println!("  {}", tr("example-builder-stderr-output-error-and-warn"));
    println!(
        "  {}\n",
        tr("example-builder-stdout-output-all-other-levels")
    );
}

/// 展示 File Sink 配置选项
///
/// 演示文件轮转、压缩、保留文件数等配置。
fn show_file_options() {
    print_section(&tr("example-builder-example-6-file-sink-options"));

    let code = r#"
// File rotation, compression, and retention policy
let _logger = LoggerManager::builder()
    .level("info")
    .file("logs/app.log")             // File path
    .file_max_size("100MB")           // Single file max size 100MB
    .file_compress(true)              // Enable Zstd compression (requires the compression/gzip feature)
    .file_rotation_time("daily")      // Rotate daily
    .file_keep_files(7)               // Keep 7 historical files
    .build()
    .await?;
"#;

    println!(
        "{}",
        tr_args("example-builder-code-example", &[("code", &code)])
    );
    println!("{}", tr("example-builder-configuration-notes"));
    println!("  {}", tr("example-builder-file-path-logs-app-log"));
    println!("  {}", tr("example-builder-single-file-size-100mb"));
    println!("  {}", tr("example-builder-compression-zstd-requires-the"));
    println!("  {}", tr("example-builder-rotation-policy-daily-rotation"));
    println!("  {}\n", tr("example-builder-keep-files-7"));
    println!(
        "Note: the compression backend is an optional feature (off by default); when it is off, file_compress(true)"
    );
    println!("{}\n", tr("example-builder-only-warns-and-leaves-rotated"));
}

/// 展示 Builder 模式的灵活性
///
/// 演示如何根据条件构建不同的配置。
fn show_builder_pattern_flexibility() {
    print_section(&tr("example-builder-example-7-builder-pattern"));

    let code = r#"
// Configure dynamically from environment variables
let mut builder = LoggerManager::builder()
    .level("info")
    .console(true);

// Enable file output in production
if std::env::var("ENV").unwrap_or_default() == "production" {
    builder = builder
        .file("logs/production.log")
        .file_compress(true)
        .file_keep_files(30);
}

let _logger = builder.build().await?;

// Or use conditional configuration
let _logger = LoggerManager::builder()
    .level("debug")
    .console(cfg!(debug_assertions))  // Enable in debug builds only
    .file("logs/app.log")
    .build()
    .await?;
"#;

    println!(
        "{}",
        tr_args("example-builder-code-example", &[("code", &code)])
    );
    println!("{}", tr("example-builder-configuration-notes"));
    println!("  {}", tr("example-builder-supports-conditional"));
    println!("  {}", tr("example-builder-supports-environment-variable"));
    println!(
        "  {}",
        tr("example-builder-builder-instances-can-be-reused")
    );
    println!("  {}\n", tr("example-builder-fluent-chained-calls"));
}
