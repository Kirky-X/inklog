// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 配置文件加载示例（Layer 1 本地资源）
//!
//! 演示从 TOML 配置文件加载 InklogConfig，使用临时目录自动清理：
//! - 创建临时 TOML 配置文件并展示其格式
//! - `InklogConfig::from_str` 同步解析（不启动 logger）
//! - `InklogConfig::from_search_paths` 演示 `LoggerManager::load` 的搜索路径
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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
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
    println!("{}\n", tr("example-config-file-inklog-config-file-loading"));

    show_toml_format();
    show_sync_parse();
    show_search_paths().await?;
    show_from_file_initialization().await?;

    println!("\n{}", tr("example-config-file-all-config-file-loading"));
    Ok(())
}

/// 展示 TOML 配置文件格式
fn show_toml_format() {
    print_separator(&tr("example-config-file-1-toml-config-file-format"));
    println!("{}", TOML_CONFIG);
    println!("{}", tr("example-config-file-the-config-file-contains-the"));
    println!(
        "{}",
        tr("example-config-file-global-global-log-level-format")
    );
    println!("{}", tr("example-config-file-console-sink-console-output"));
    println!("{}", tr("example-config-file-file-sink-file-output-path"));
    println!(
        "{}",
        tr("example-config-file-database-sink-database-output")
    );
    println!(
        "{}",
        tr("example-config-file-performance-performance-tuning")
    );
    println!(
        "{}",
        tr("example-config-file-http-server-http-health-check")
    );
}

/// 展示 InklogConfig 同步解析（不初始化 logger）
fn show_sync_parse() {
    print_separator(&tr("example-config-file-2-inklogconfig-sync-parsing"));

    print_section(&tr("example-config-file-2-1-parsing-config-with-toml"));
    let config: InklogConfig = TOML_CONFIG.parse().expect("failed to parse TOML");
    println!(
        "{}",
        tr_args(
            "example-config-file-global-level",
            &[("level", &config.global.level)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-global-format",
            &[("format", &config.global.format)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-global-masking-enabled",
            &[("masking_enabled", &config.global.masking_enabled)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-console-sink-enabled",
            &[("value", &(config.console_sink.as_ref().unwrap().enabled))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-console-sink-colored",
            &[("value", &(config.console_sink.as_ref().unwrap().colored))]
        )
    );
    assert_eq!(config.global.level, "info");
    assert!(!config.console_sink.as_ref().unwrap().colored);

    print_section(&tr("example-config-file-2-2-file-sink-fields"));
    let file = config.file_sink.as_ref().unwrap();
    println!(
        "{}",
        tr_args(
            "example-config-file-file-sink-enabled",
            &[("enabled", &file.enabled)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-file-sink-path",
            &[("value", &(file.path.display()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-file-sink-max-size",
            &[("max_size", &file.max_size)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-file-sink-rotation-time",
            &[("rotation_time", &file.rotation_time)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-file-sink-compress",
            &[("compress", &file.compress)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-file-sink-compression-level",
            &[("compression_level", &file.compression_level)]
        )
    );
    assert!(!file.enabled);
    assert_eq!(file.max_size, "100MB");

    print_section(&tr("example-config-file-2-3-database-sink-fields"));
    let db = config.database_sink.as_ref().unwrap();
    println!(
        "{}",
        tr_args(
            "example-config-file-database-sink-enabled",
            &[("enabled", &db.enabled)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-database-sink-driver",
            &[("driver", &format!("{:?}", db.driver))]
        )
    );
    println!(
        "{}",
        tr_args("example-config-file-database-sink-url", &[("url", &db.url)])
    );
    assert!(!db.enabled);

    print_section(&tr("example-config-file-2-4-performance-fields"));
    println!(
        "{}",
        tr_args(
            "example-config-file-channel-capacity",
            &[("channel_capacity", &config.performance.channel_capacity)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-worker-threads",
            &[("worker_threads", &config.performance.worker_threads)]
        )
    );
    assert_eq!(config.performance.channel_capacity, 10000);

    print_section(&tr("example-config-file-2-5-http-server-fields"));
    let http = config.http_server.as_ref().unwrap();
    println!(
        "{}",
        tr_args(
            "example-config-file-http-server-enabled",
            &[("enabled", &http.enabled)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-http-server-host",
            &[("host", &http.host)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-file-http-server-port",
            &[("port", &http.port)]
        )
    );
    assert!(!http.enabled);

    print_section(&tr("example-config-file-2-6-validate-check"));
    config.validate().expect("config should pass validation");
    println!("{}", tr("example-config-file-config-validation-passed"));
}

/// 展示 InklogConfig::from_search_paths（LoggerManager::load 的搜索逻辑）
async fn show_search_paths() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-config-file-3-from-search-paths-auto-search"));

    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("auto_search.toml");
    fs::write(&config_path, TOML_CONFIG)?;

    print_section(&tr("example-config-file-3-1-setting-the-inklog-config"));
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap()) };
    println!(
        "{}",
        tr_args(
            "example-config-file-inklog-config-path",
            &[("value", &(config_path.display()))]
        )
    );

    print_section(&tr("example-config-file-3-2-reading-and-parsing-with"));
    let config = InklogConfig::from_search_paths().expect("search should succeed");
    assert_eq!(config.global.level, "info");
    println!(
        "{}",
        tr_args(
            "example-config-file-loaded-from-inklog-config-path",
            &[("level", &config.global.level)]
        )
    );

    // 清理环境变量
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("INKLOG_CONFIG_PATH") };
    println!("{}", tr("example-config-file-inklog-config-path-env-var"));

    println!(
        "\n{}",
        tr("example-config-file-note-loggermanager-load-calls")
    );
    println!(
        "      {}",
        tr("example-config-file-searching-for-the-config-file")
    );
    println!("      {}", tr("example-config-file-1-inklog-config-path"));
    println!(
        "      {}",
        tr("example-config-file-2-inklog-config-toml-current")
    );
    println!(
        "      {}",
        tr("example-config-file-3-config-inklog-config-toml")
    );
    println!(
        "      {}",
        tr("example-config-file-4-etc-inklog-config-toml")
    );
    println!(
        "      {}",
        tr("example-config-file-if-none-exist-default-default")
    );

    Ok(())
}

/// 展示 LoggerManager::from_file 初始化 logger
async fn show_from_file_initialization() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-config-file-4-initializing-the-logger-with"));

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
    println!(
        "{}",
        tr_args(
            "example-config-file-temporary-config-file",
            &[("value", &(config_path.display()))]
        )
    );

    print_section(&tr("example-config-file-4-1-loggermanager-from-file-path"));
    let _logger = LoggerManager::from_file(&config_path).await?;
    println!("{}", tr("example-config-file-loggermanager-initialized"));

    print_section(&tr("example-config-file-4-2-logging-via-tracing-macros"));
    tracing::info!(
        "{}",
        tr("example-config-file-info-log-from-the-config-file")
    );
    tracing::warn!(
        "{}",
        tr("example-config-file-warn-log-from-the-config-file")
    );
    tracing::error!(
        "{}",
        tr("example-config-file-error-log-from-the-config-file")
    );

    // 给后台 worker 一点时间处理日志
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    print_section(&tr("example-config-file-4-3-from-file-vs-load-comparison"));
    println!(
        "{}",
        tr("example-config-file-from-file-path-load-the-config")
    );
    println!(
        "load()          : auto-search INKLOG_CONFIG_PATH / current dir / user config / system config"
    );
    println!(
        "\n{}",
        tr("example-config-file-note-loggermanager-internally")
    );
    println!(
        "      {}",
        tr("example-config-file-and-installs-the-global-tracing")
    );

    Ok(())
}
