// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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

use inklog::config::InklogConfig;
use inklog::LoggerManager;
use inklog_examples::common::{print_section, print_separator};
use std::fs;
use tempfile::TempDir;

/// 示例 TOML 配置内容（展示完整格式）
const TOML_CONFIG: &str = r#"# inklog 配置文件示例

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
	println!("=== inklog 配置文件加载示例 ===\n");

	show_toml_format();
	show_sync_parse();
	show_search_paths().await?;
	show_from_file_initialization().await?;

	println!("\n✓ 所有配置文件加载示例演示完成");
	Ok(())
}

/// 展示 TOML 配置文件格式
fn show_toml_format() {
	print_separator("1. TOML 配置文件格式");
	println!("{}", TOML_CONFIG);
	println!("--- 配置文件包含以下 section ---");
	println!("- [global]            全局日志级别、格式、脱敏、降级");
	println!("- [console_sink]      控制台输出（颜色、stderr 路由）");
	println!("- [file_sink]         文件输出（路径、轮转、压缩、加密）");
	println!("- [database_sink]     数据库输出（驱动、连接、批处理）");
	println!("- [performance]       性能调优（channel 容量、worker 线程数）");
	println!("- [http_server]       HTTP 健康检查与 Prometheus 指标端点");
}

/// 展示 InklogConfig 同步解析（不初始化 logger）
fn show_sync_parse() {
	print_separator("2. InklogConfig 同步解析（from_str）");

	print_section("2.1 toml::from_str 解析配置");
	let config: InklogConfig = TOML_CONFIG
		.parse()
		.expect("TOML 解析失败");
	println!("global.level           = {}", config.global.level);
	println!("global.format          = {}", config.global.format);
	println!("global.masking_enabled = {}", config.global.masking_enabled);
	println!("console_sink.enabled   = {}", config.console_sink.as_ref().unwrap().enabled);
	println!("console_sink.colored   = {}", config.console_sink.as_ref().unwrap().colored);
	assert_eq!(config.global.level, "info");
	assert!(!config.console_sink.as_ref().unwrap().colored);

	print_section("2.2 file_sink 字段");
	let file = config.file_sink.as_ref().unwrap();
	println!("file_sink.enabled          = {}", file.enabled);
	println!("file_sink.path             = {}", file.path.display());
	println!("file_sink.max_size         = {}", file.max_size);
	println!("file_sink.rotation_time    = {}", file.rotation_time);
	println!("file_sink.compress         = {}", file.compress);
	println!("file_sink.compression_level = {}", file.compression_level);
	assert!(!file.enabled);
	assert_eq!(file.max_size, "100MB");

	print_section("2.3 database_sink 字段");
	let db = config.database_sink.as_ref().unwrap();
	println!("database_sink.enabled    = {}", db.enabled);
	println!("database_sink.driver     = {:?}", db.driver);
	println!("database_sink.url        = {}", db.url);
	assert!(!db.enabled);

	print_section("2.4 performance 字段");
	println!("channel_capacity = {}", config.performance.channel_capacity);
	println!("worker_threads   = {}", config.performance.worker_threads);
	assert_eq!(config.performance.channel_capacity, 10000);

	print_section("2.5 http_server 字段");
	let http = config.http_server.as_ref().unwrap();
	println!("http_server.enabled = {}", http.enabled);
	println!("http_server.host    = {}", http.host);
	println!("http_server.port    = {}", http.port);
	assert!(!http.enabled);

	print_section("2.6 validate() 校验");
	config.validate().expect("配置应通过校验");
	println!("✓ 配置校验通过");
}

/// 展示 InklogConfig::from_search_paths（LoggerManager::load 的搜索逻辑）
async fn show_search_paths() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("3. from_search_paths 自动搜索（load 的配置来源）");

	let temp_dir = TempDir::new()?;
	let config_path = temp_dir.path().join("auto_search.toml");
	fs::write(&config_path, TOML_CONFIG)?;

	print_section("3.1 设置 INKLOG_CONFIG_PATH 环境变量");
	std::env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap());
	println!("INKLOG_CONFIG_PATH = {}", config_path.display());

	print_section("3.2 from_search_paths 读取并解析");
	let config = InklogConfig::from_search_paths().expect("搜索应成功");
	assert_eq!(config.global.level, "info");
	println!("✓ 从 INKLOG_CONFIG_PATH 加载成功: level = {}", config.global.level);

	// 清理环境变量
	std::env::remove_var("INKLOG_CONFIG_PATH");
	println!("已清理 INKLOG_CONFIG_PATH 环境变量");

	println!("\n说明: LoggerManager::load() 内部调用 InklogConfig::load_sync(),");
	println!("      按以下顺序搜索配置文件（首个存在的文件胜出）:");
	println!("      1. $INKLOG_CONFIG_PATH");
	println!("      2. ./inklog_config.toml (当前目录)");
	println!("      3. ~/.config/inklog/config.toml");
	println!("      4. /etc/inklog/config.toml");
	println!("      若都不存在则使用 Default::default()。");

	Ok(())
}

/// 展示 LoggerManager::from_file 初始化 logger
async fn show_from_file_initialization() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("4. LoggerManager::from_file 初始化 logger");

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
	println!("临时配置文件: {}", config_path.display());

	print_section("4.1 LoggerManager::from_file(path)");
	let _logger = LoggerManager::from_file(&config_path).await?;
	println!("✓ LoggerManager 初始化成功");

	print_section("4.2 通过 tracing 宏输出日志（验证 logger 已生效）");
	tracing::info!("来自 config_file 示例的 INFO 日志");
	tracing::warn!("来自 config_file 示例的 WARN 日志");
	tracing::error!("来自 config_file 示例的 ERROR 日志");

	// 给后台 worker 一点时间处理日志
	tokio::time::sleep(std::time::Duration::from_millis(200)).await;

	print_section("4.3 from_file vs load 对比");
	println!("from_file(path) : 从指定路径加载配置文件");
	println!("load()          : 自动搜索 INKLOG_CONFIG_PATH / 当前目录 / 用户配置 / 系统配置");
	println!("\n说明: LoggerManager 内部会创建 logs/error.log 用于系统错误日志,");
	println!("      并安装全局 tracing subscriber 与 log crate 适配器。");

	Ok(())
}
