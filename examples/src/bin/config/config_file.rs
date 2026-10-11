// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 配置文件加载示例（Layer 1 本地资源）
//!
//! 演示从 TOML 配置文件加载 InklogConfig，使用临时目录自动清理：
//! - 创建临时 TOML 配置文件并展示其格式
//! - `InklogConfig::from_str` 同步解析（不启动 logger）
//! - `InklogConfig::from_search_paths` 演示 `LoggerManager::load()` 的搜索路径
//! - `LoggerManager::from_file` 加载配置并初始化 logger
//!
//! # 运行
//! ```bash
//! cargo run --bin config_file
//! ```

// LoggerManager 初始化链的 async 嵌套 future 在 mysql 后端下类型层次较深，
// 默认递归上限（128）会让 rustc 在布局计算时报 "queries overflow the depth
// limit"，此处按编译器建议上调。
#![recursion_limit = "256"]

use inklog::LoggerManager;
use inklog::config::InklogConfig;
use inklog_examples::common::{print_section, print_separator};
use std::fs;
use tempfile::TempDir;

/// 示例 TOML 配置内容（展示完整格式）
const TOML_CONFIG: &str = r#"# inklog configuration file example

[global]
level = "info"
format = "{timestamp} [{level}] {target} - {message}"
masking_enabled = true
auto_fallback = true

[console_sink]
enabled = true
colored = false
stderr_levels = ["error", "warn"]

[file_sink]
enabled = false
path = "logs/app.log"
max_size = "100MB"
rotation_time = "daily"
keep_files = 30
compress = true
compression_level = 3

[database_sink]
enabled = false
driver = "sqlite"
url = "sqlite::memory:"
batch_size = 100

[performance]
channel_capacity = 10000
worker_threads = 3

[http_server]
enabled = false
host = "127.0.0.1"
port = 9090
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog config file loading example ===\n");

    show_toml_format();
    show_sync_parse();
    show_search_paths().await?;
    show_from_file_initialization().await?;

    println!("\n✓ All config file loading examples completed");
    Ok(())
}

/// 展示 TOML 配置文件格式
fn show_toml_format() {
    print_separator("1. TOML config file format");
    println!("{}", TOML_CONFIG);
    println!("--- The config file contains the following sections ---");
    println!("- [global]            Global log level, format, masking, degradation");
    println!("- [console_sink]      Console output (color, stderr routing)");
    println!("- [file_sink]         File output (path, rotation, compression, encryption)");
    println!("- [database_sink]     Database output (driver, connection, batching)");
    println!("- [performance]       Performance tuning (channel capacity, worker threads)");
    println!("- [http_server]       HTTP health check and Prometheus metrics endpoint");
}

/// 展示 InklogConfig 同步解析（不初始化 logger）
fn show_sync_parse() {
    print_separator("2. InklogConfig sync parsing (from_str)");

    print_section("2.1 Parsing config with toml::from_str");
    let config: InklogConfig = TOML_CONFIG.parse().expect("failed to parse TOML");
    println!("global.level           = {}", config.global.level);
    println!("global.format          = {}", config.global.format);
    println!("global.masking_enabled = {}", config.global.masking_enabled);
    println!(
        "console_sink.enabled   = {}",
        config.console_sink.as_ref().unwrap().enabled
    );
    println!(
        "console_sink.colored   = {}",
        config.console_sink.as_ref().unwrap().colored
    );
    assert_eq!(config.global.level, "info");
    assert!(!config.console_sink.as_ref().unwrap().colored);

    print_section("2.2 file_sink fields");
    let file = config.file_sink.as_ref().unwrap();
    println!("file_sink.enabled          = {}", file.enabled);
    println!("file_sink.path             = {}", file.path.display());
    println!("file_sink.max_size         = {}", file.max_size);
    println!("file_sink.rotation_time    = {}", file.rotation_time);
    println!("file_sink.compress         = {}", file.compress);
    println!("file_sink.compression_level = {}", file.compression_level);
    assert!(!file.enabled);
    assert_eq!(file.max_size, "100MB");

    print_section("2.3 database_sink fields");
    let db = config.database_sink.as_ref().unwrap();
    println!("database_sink.enabled    = {}", db.enabled);
    println!("database_sink.driver     = {:?}", db.driver);
    println!("database_sink.url        = {}", db.url);
    assert!(!db.enabled);

    print_section("2.4 performance fields");
    println!("channel_capacity = {}", config.performance.channel_capacity);
    println!("worker_threads   = {}", config.performance.worker_threads);
    assert_eq!(config.performance.channel_capacity, 10000);

    print_section("2.5 http_server fields");
    let http = config.http_server.as_ref().unwrap();
    println!("http_server.enabled = {}", http.enabled);
    println!("http_server.host    = {}", http.host);
    println!("http_server.port    = {}", http.port);
    assert!(!http.enabled);

    print_section("2.6 validate() check");
    config.validate().expect("config should pass validation");
    println!("✓ Config validation passed");
}

/// 展示 InklogConfig::from_search_paths（LoggerManager::load 的搜索逻辑）
async fn show_search_paths() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("3. from_search_paths auto search (config source of load)");

    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("auto_search.toml");
    fs::write(&config_path, TOML_CONFIG)?;

    print_section("3.1 Setting the INKLOG_CONFIG_PATH env var");
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap()) };
    println!("INKLOG_CONFIG_PATH = {}", config_path.display());

    print_section("3.2 Reading and parsing with from_search_paths");
    let config = InklogConfig::from_search_paths().expect("search should succeed");
    assert_eq!(config.global.level, "info");
    println!(
        "✓ Loaded from INKLOG_CONFIG_PATH: level = {}",
        config.global.level
    );

    // 清理环境变量
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("INKLOG_CONFIG_PATH") };
    println!("INKLOG_CONFIG_PATH env var removed");

    println!("\nNote: LoggerManager::load() calls InklogConfig::load_sync(),");
    println!("      searching for the config file in this order (first existing file wins):");
    println!("      1. $INKLOG_CONFIG_PATH");
    println!("      2. ./inklog_config.toml (current directory)");
    println!("      3. ~/.config/inklog/config.toml");
    println!("      4. /etc/inklog/config.toml");
    println!("      If none exist, Default::default() is used.");

    Ok(())
}

/// 展示 LoggerManager::from_file 初始化 logger
async fn show_from_file_initialization() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("4. Initializing the logger with LoggerManager::from_file");

    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("inklog_config.toml");

    // 为演示 from_file，构造一个仅启用 console 的精简配置
    let minimal_config = r#"
[global]
level = "info"

[console_sink]
enabled = true
colored = false

[performance]
channel_capacity = 1000
worker_threads = 2
"#;
    fs::write(&config_path, minimal_config)?;
    println!("Temporary config file: {}", config_path.display());

    print_section("4.1 LoggerManager::from_file(path)");
    let _logger = LoggerManager::from_file(&config_path).await?;
    println!("✓ LoggerManager initialized");

    print_section("4.2 Logging via tracing macros (verifying the logger is active)");
    tracing::info!("INFO log from the config_file example");
    tracing::warn!("WARN log from the config_file example");
    tracing::error!("ERROR log from the config_file example");

    // 给后台 worker 一点时间处理日志
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    print_section("4.3 from_file vs load comparison");
    println!("from_file(path) : load the config file from the given path");
    println!(
        "load()          : auto-search INKLOG_CONFIG_PATH / current dir / user config / system config"
    );
    println!("\nNote: LoggerManager internally creates logs/error.log for system error logs,");
    println!("      and installs the global tracing subscriber and the log crate adapter.");

    Ok(())
}
