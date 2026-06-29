// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 配置 inspect 示例：sinks_enabled() + LoggerManager::load()
//!
//! 演示 `InklogConfig::sinks_enabled()` 和 `LoggerManager::load()` 的使用：
//!
//! 1. `sinks_enabled()` 返回已启用的 Sink 列表
//! 2. `LoggerManager::load()` 从搜索路径自动加载配置并初始化
//! 3. 配置检查工作流
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin config_inspect
//! ```

use inklog::config::{ConsoleSinkConfig, DatabaseSinkConfig, FileSinkConfig, InklogConfig};
use inklog_examples::common::{print_section, print_separator};

fn main() {
	print_separator("inklog 配置 inspect 示例");

	show_sinks_enabled_all_disabled();
	show_sinks_enabled_console_only();
	show_sinks_enabled_multi_sink();
	show_logger_manager_load_usage();
	show_config_inspection_workflow();

	println!("\n所有配置 inspect 示例展示完毕。");
}

/// 辅助函数：安全访问 Option<ConsoleSinkConfig>.enabled
fn console_enabled(c: &Option<ConsoleSinkConfig>) -> bool {
	c.as_ref().is_some_and(|c| c.enabled)
}

/// 辅助函数：安全访问 Option<FileSinkConfig>.enabled
fn file_enabled(c: &Option<FileSinkConfig>) -> bool {
	c.as_ref().is_some_and(|c| c.enabled)
}

/// 辅助函数：安全访问 Option<DatabaseSinkConfig>.enabled
fn db_enabled(c: &Option<DatabaseSinkConfig>) -> bool {
	c.as_ref().is_some_and(|c| c.enabled)
}

/// 演示所有 Sink 禁用时 sinks_enabled() 返回空
fn show_sinks_enabled_all_disabled() {
	print_section("示例 1：所有 Sink 禁用 → sinks_enabled() 返回空");

	let config = InklogConfig::default();
	let sinks = config.sinks_enabled();

	println!("默认配置（仅 console_sink 默认启用）：");
	println!("  console_sink.enabled   = {}", console_enabled(&config.console_sink));
	println!("  file_sink.enabled      = {}", file_enabled(&config.file_sink));
	println!("  database_sink.enabled  = {}", db_enabled(&config.database_sink));
	println!("\nsinks_enabled() = {:?}", sinks);
	println!("返回 Vec 长度 = {}", sinks.len());
}

/// 演示仅 Console Sink 启用
fn show_sinks_enabled_console_only() {
	print_section("示例 2：仅 Console Sink 启用");

	let config = InklogConfig {
		console_sink: Some(ConsoleSinkConfig {
			enabled: true,
			..Default::default()
		}),
		..Default::default()
	};
	let sinks = config.sinks_enabled();

	println!("配置：");
	println!("  console_sink.enabled   = {} ✓", console_enabled(&config.console_sink));
	println!("  file_sink.enabled      = {}", file_enabled(&config.file_sink));
	println!("  database_sink.enabled  = {}", db_enabled(&config.database_sink));
	println!("\nsinks_enabled() = {:?}", sinks);
	println!("包含 \"console\" = {}", sinks.contains(&"console"));
}

/// 演示多 Sink 启用
fn show_sinks_enabled_multi_sink() {
	print_section("示例 3：多 Sink 启用（Console + File + Database）");

	let config = InklogConfig {
		console_sink: Some(ConsoleSinkConfig {
			enabled: true,
			..Default::default()
		}),
		file_sink: Some(FileSinkConfig {
			enabled: true,
			path: std::path::PathBuf::from("logs/app.log"),
			..Default::default()
		}),
		database_sink: Some(DatabaseSinkConfig {
			enabled: true,
			..Default::default()
		}),
		..Default::default()
	};
	let sinks = config.sinks_enabled();

	println!("配置：");
	println!("  console_sink.enabled   = {} ✓", console_enabled(&config.console_sink));
	let file_path = config
		.file_sink
		.as_ref()
		.map(|c| c.path.display().to_string())
		.unwrap_or_default();
	println!("  file_sink.enabled      = {} ✓ (path={})", file_enabled(&config.file_sink), file_path);
	println!("  database_sink.enabled  = {} ✓", db_enabled(&config.database_sink));
	println!("\nsinks_enabled() = {:?}", sinks);
	println!("Sink 数量 = {} (期望 3)", sinks.len());

	// 验证所有 Sink 都在列表中
	let has_console = sinks.contains(&"console");
	let has_file = sinks.contains(&"file");
	let has_database = sinks.contains(&"database");
	println!("\n验证：");
	println!("  contains \"console\"  = {} → {}", has_console, if has_console { "✓" } else { "✗" });
	println!("  contains \"file\"     = {} → {}", has_file, if has_file { "✓" } else { "✗" });
	println!("  contains \"database\" = {} → {}", has_database, if has_database { "✓" } else { "✗" });
}

/// 演示 LoggerManager::load() 的使用方式
fn show_logger_manager_load_usage() {
	print_section("示例 4：LoggerManager::load() 使用方式");

	println!("LoggerManager::load() 流程：");
	println!("  1. 调用 InklogConfig::load_sync() 搜索配置文件");
	println!("     - 搜索路径：当前目录 → ~/.config/inklog/ → /etc/inklog/");
	println!("     - 文件名：inklog_config.toml");
	println!("     - 找不到则返回 Default::default()");
	println!("  2. 用加载的配置调用 LoggerManager::with_config()");
	println!("  3. 初始化所有启用的 Sink 和 HTTP 服务器");

	println!("\n代码示例：");
	println!("  use inklog::LoggerManager;");
	println!();
	println!("  #![tokio::main]");
	println!("  async fn main() -> Result<(), Box<dyn std::error::Error>> {{");
	println!("      // 自动搜索配置文件并初始化");
	println!("      let _logger = LoggerManager::load().await?;");
	println!("      Ok(())");
	println!("  }}");

	println!("\n自定义搜索路径：");
	println!("  use inklog::InklogConfig;");
	println!();
	println!("  // 方式 1：从默认搜索路径加载");
	println!("  let config = InklogConfig::from_search_paths()?;");
	println!();
	println!("  // 方式 2：结合环境变量覆盖");
	println!("  let config = InklogConfig::load_with_env_overrides()?;");
	println!();
	println!("  let logger = LoggerManager::with_config(config).await?;");

	println!("\n注意：load() 是 async 方法，需要在 tokio runtime 中调用。");
	println!("本示例仅展示用法，不实际调用 load()（会启动 logger 占用资源）。");
}

/// 演示配置检查工作流
fn show_config_inspection_workflow() {
	print_section("示例 5：配置检查工作流");

	println!("推荐的配置检查工作流：\n");

	println!("步骤 1：加载配置");
	let config = InklogConfig::load_with_env_overrides()
		.unwrap_or_else(|e| {
			println!("  ⚠ 配置加载失败: {}，使用默认配置", e);
			InklogConfig::default()
		});
	println!("  ✓ 配置加载完成");

	println!("\n步骤 2：检查启用的 Sink");
	let sinks = config.sinks_enabled();
	if sinks.is_empty() {
		println!("  ⚠ 没有启用任何 Sink，日志将不会输出");
	} else {
		println!("  ✓ 已启用 Sink: {:?}", sinks);
	}

	println!("\n步骤 3：验证配置一致性");
	let db_enabled_flag = db_enabled(&config.database_sink);
	if db_enabled_flag && !sinks.contains(&"database") {
		println!("  ✗ database_sink.enabled=true 但 sinks_enabled() 不含 \"database\"");
	} else {
		println!("  ✓ Sink 状态一致");
	}

	println!("\n步骤 4：根据配置选择初始化方式");
	if sinks.is_empty() {
		println!("  → 使用 LoggerManager::new() (仅默认 Console)");
	} else if InklogConfig::load_with_env_overrides().is_ok() {
		println!("  → 使用 LoggerManager::load() (从配置文件加载)");
	} else {
		println!("  → 使用 LoggerManager::with_config() (显式配置)");
	}

	println!("\n完整代码示例：");
	println!("  fn inspect_config() -> Result<(), inklog::InklogError> {{");
	println!("      let config = InklogConfig::load_with_env_overrides()?;");
	println!("      let sinks = config.sinks_enabled();");
	println!("      println!(\"Enabled sinks: {{:?}}\", sinks);");
	println!("      if sinks.is_empty() {{");
	println!("          eprintln!(\"Warning: no sinks enabled\");");
	println!("      }}");
	println!("      Ok(())");
	println!("  }}");
}
