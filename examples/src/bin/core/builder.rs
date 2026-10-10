// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Builder 模式配置示例
//!
//! 演示 LoggerManager::builder() 链式 API 的各种用法。
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
//! 各配置片段以字符串展示（不在本示例中实际初始化）；`LoggerManager::builder()`
//! 本身没有限制，真实项目在 async 函数中调用 `build().await` 即生效。

use inklog_examples::common::print_section;

fn main() {
    println!("=== inklog Builder 模式配置示例 ===\n");

    show_basic_builder();
    show_multi_sink_builder();
    show_advanced_options_builder();
    show_performance_options();
    show_console_options();
    show_file_options();

    // 展示 Builder 配置构建（不实际初始化）
    show_builder_pattern_flexibility();

    println!("\n所有配置示例已展示完毕。");
    println!("\n注意：以上代码片段以字符串形式展示（便于逐段阅读），未在本示例中实际初始化；");
    println!("Builder 无此限制，真实项目中在 async 函数里调用 build().await 即生效。");
}

/// 展示基础 Builder 用法
///
/// 演示最简单的链式配置，仅启用 Console Sink。
fn show_basic_builder() {
    print_section("示例 1：基础 Builder 用法");

    let code = r#"
// 最简单的配置：仅启用 Console Sink
let _logger = LoggerManager::builder()
    .level("debug")      // 设置日志级别为 DEBUG
    .console(true)       // 启用控制台输出
    .build()
    .await?;
"#;

    println!("代码示例：\n{}", code);
    println!("配置说明：");
    println!("  ✓ 日志级别: DEBUG");
    println!("  ✓ Console Sink: 已启用");
    println!("  ✗ File Sink: 未启用\n");
}

/// 展示多 Sink 配置
///
/// 演示同时配置 Console 和 File Sink，日志同时输出到控制台和文件。
fn show_multi_sink_builder() {
    print_section("示例 2：多 Sink 配置");

    let code = r#"
// 同时启用 Console 和 File Sink
let _logger = LoggerManager::builder()
    .level("info")                    // 设置日志级别为 INFO
    .console(true)                    // 启用控制台输出
    .file("logs/app.log")             // 启用文件输出
    .build()
    .await?;
"#;

    println!("代码示例：\n{}", code);
    println!("配置说明：");
    println!("  ✓ 日志级别: INFO");
    println!("  ✓ Console Sink: 已启用");
    println!("  ✓ File Sink: logs/app.log\n");
}

/// 展示高级配置选项
///
/// 演示 Builder 的高级配置选项，包括格式、输出格式等。
fn show_advanced_options_builder() {
    print_section("示例 3：高级配置选项");

    let code = r#"
// Console 输出 JSON 行 + 最详细日志级别
let _logger = LoggerManager::builder()
    .level("trace")                   // 最详细的日志级别
    .console(true)                    // 启用控制台
    .console_json(true)               // Console 输出整条 JSON（而非文本模板）
    .file("logs/app.log")             // 启用文件输出（文件格式独立配置）
    .build()
    .await?;
"#;

    println!("代码示例：\n{}", code);
    println!("配置说明：");
    println!("  ✓ 日志级别: TRACE（最详细）");
    println!("  ✓ Console 输出: JSON 行（console_json(true)）");
    println!("  ✓ File Sink: logs/app.log\n");
    println!("注意：format() 设置的是文本渲染模板（如 \"{{timestamp}} [{{level}}] ...\"），");
    println!("与输出格式正交——format(\"json\") 只会把模板文字原样输出，");
    println!("JSON 输出必须用 console_json(true)（JSON 模式下不着色）\n");
}

/// 展示性能配置选项
///
/// 演示 Channel 容量、Worker 线程数等性能相关配置。
fn show_performance_options() {
    print_section("示例 4：性能配置选项");

    let code = r#"
// 高性能配置（适合生产环境）
let _logger = LoggerManager::builder()
    .level("info")
    .console(true)
    .file("logs/app.log")
    .channel_capacity(50000)          // Channel 容量 50,000
    .worker_threads(4)                // 4 个 Worker 线程
    .build()
    .await?;
"#;

    println!("代码示例：\n{}", code);
    println!("配置说明：");
    println!("  ✓ Channel 容量: 50,000（默认 10,000）");
    println!("  ✓ Worker 线程数: 4（默认 3）");
    println!("  ✓ 适用场景: 高吞吐量生产环境\n");
}

/// 展示 Console Sink 配置选项
///
/// 演示 Console 的彩色输出、错误级别输出等配置。
fn show_console_options() {
    print_section("示例 5：Console Sink 配置选项");

    let code = r#"
// 自定义 Console 输出行为
let _logger = LoggerManager::builder()
    .level("debug")
    .console(true)                    // 启用 Console
    .console_colored(true)            // 启用彩色输出
    .console_stderr_levels(&["error", "warn"])  // ERROR/WARN 输出到 stderr
    .build()
    .await?;
"#;

    println!("代码示例：\n{}", code);
    println!("配置说明：");
    println!("  ✓ 彩色输出: 已启用");
    println!("  ✓ stderr 输出: ERROR、WARN 级别");
    println!("  ✓ stdout 输出: 其他所有级别\n");
}

/// 展示 File Sink 配置选项
///
/// 演示文件轮转、压缩、保留文件数等配置。
fn show_file_options() {
    print_section("示例 6：File Sink 配置选项");

    let code = r#"
// 文件轮转、压缩、保留策略
let _logger = LoggerManager::builder()
    .level("info")
    .file("logs/app.log")             // 文件路径
    .file_max_size("100MB")           // 单文件最大 100MB
    .file_compress(true)              // 启用 Zstd 压缩（需 compression/gzip feature）
    .file_rotation_time("daily")      // 每日轮转
    .file_keep_files(7)               // 保留 7 个历史文件
    .build()
    .await?;
"#;

    println!("代码示例：\n{}", code);
    println!("配置说明：");
    println!("  ✓ 文件路径: logs/app.log");
    println!("  ✓ 单文件大小: 100MB");
    println!("  ✓ 压缩: Zstd（需 inklog 启用 `compression`/`zstd` 或 `gzip` feature）");
    println!("  ✓ 轮转策略: 每日轮转");
    println!("  ✓ 保留文件: 7 个\n");
    println!("注意：压缩后端是可选 feature（默认关闭）；未启用时 file_compress(true)");
    println!("仅告警并保持轮转文件未压缩，不会报错\n");
}

/// 展示 Builder 模式的灵活性
///
/// 演示如何根据条件构建不同的配置。
fn show_builder_pattern_flexibility() {
    print_section("示例 7：Builder 模式的灵活性");

    let code = r#"
// 根据环境变量动态配置
let mut builder = LoggerManager::builder()
    .level("info")
    .console(true);

// 生产环境启用文件输出
if std::env::var("ENV").unwrap_or_default() == "production" {
    builder = builder
        .file("logs/production.log")
        .file_compress(true)
        .file_keep_files(30);
}

let _logger = builder.build().await?;

// 或者使用条件配置
let _logger = LoggerManager::builder()
    .level("debug")
    .console(cfg!(debug_assertions))  // 仅在 debug 模式启用
    .file("logs/app.log")
    .build()
    .await?;
"#;

    println!("代码示例：\n{}", code);
    println!("配置说明：");
    println!("  ✓ 支持条件构建");
    println!("  ✓ 支持环境变量判断");
    println!("  ✓ 可复用 Builder 实例");
    println!("  ✓ 灵活的链式调用\n");
}
