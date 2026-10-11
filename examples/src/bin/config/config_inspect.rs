// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
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
//! cargo run --package inklog-examples --bin config_inspect
//! ```

use inklog::config::{ConsoleSinkConfig, DatabaseSinkConfig, FileSinkConfig, InklogConfig};
use inklog_examples::common::{print_section, print_separator};

fn main() {
    print_separator("inklog config inspection example");

    show_sinks_enabled_all_disabled();
    show_sinks_enabled_console_only();
    show_sinks_enabled_multi_sink();
    show_logger_manager_load_usage();
    show_config_inspection_workflow();

    println!("\nAll config inspection examples completed.");
}

/// 辅助函数：安全访问 `Option<ConsoleSinkConfig>`.enabled
fn console_enabled(c: &Option<ConsoleSinkConfig>) -> bool {
    c.as_ref().is_some_and(|c| c.enabled)
}

/// 辅助函数：安全访问 `Option<FileSinkConfig>`.enabled
fn file_enabled(c: &Option<FileSinkConfig>) -> bool {
    c.as_ref().is_some_and(|c| c.enabled)
}

/// 辅助函数：安全访问 `Option<DatabaseSinkConfig>`.enabled
fn db_enabled(c: &Option<DatabaseSinkConfig>) -> bool {
    c.as_ref().is_some_and(|c| c.enabled)
}

/// 演示所有 Sink 禁用时 sinks_enabled() 返回空
fn show_sinks_enabled_all_disabled() {
    print_section("Example 1: all sinks disabled → sinks_enabled() returns empty");

    // 注意：InklogConfig::default() 的 console_sink 默认启用，直接取
    // sinks_enabled() 会得到 ["console"]；要得到空列表必须显式禁用全部 Sink
    let config = InklogConfig {
        console_sink: Some(ConsoleSinkConfig {
            enabled: false,
            ..Default::default()
        }),
        file_sink: Some(FileSinkConfig {
            enabled: false,
            ..Default::default()
        }),
        database_sink: Some(DatabaseSinkConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    let sinks = config.sinks_enabled();

    println!("All sinks explicitly disabled:");
    println!(
        "  console_sink.enabled   = {}",
        console_enabled(&config.console_sink)
    );
    println!(
        "  file_sink.enabled      = {}",
        file_enabled(&config.file_sink)
    );
    println!(
        "  database_sink.enabled  = {}",
        db_enabled(&config.database_sink)
    );
    println!("\nsinks_enabled() = {:?}", sinks);
    println!("Returned Vec length = {}", sinks.len());
    assert!(
        sinks.is_empty(),
        "sinks_enabled() should be empty when everything is disabled"
    );

    println!("\nComparison: console_sink in InklogConfig::default() is enabled by default");
    let default_sinks = InklogConfig::default().sinks_enabled();
    println!("  default().sinks_enabled() = {:?}", default_sinks);
    assert_eq!(default_sinks, vec!["console"]);
}

/// 演示仅 Console Sink 启用
fn show_sinks_enabled_console_only() {
    print_section("Example 2: only the console sink enabled");

    let config = InklogConfig {
        console_sink: Some(ConsoleSinkConfig {
            enabled: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let sinks = config.sinks_enabled();

    println!("Config:");
    println!(
        "  console_sink.enabled   = {} ✓",
        console_enabled(&config.console_sink)
    );
    println!(
        "  file_sink.enabled      = {}",
        file_enabled(&config.file_sink)
    );
    println!(
        "  database_sink.enabled  = {}",
        db_enabled(&config.database_sink)
    );
    println!("\nsinks_enabled() = {:?}", sinks);
    println!("contains \"console\" = {}", sinks.contains(&"console"));
}

/// 演示多 Sink 启用
fn show_sinks_enabled_multi_sink() {
    print_section("Example 3: multiple sinks enabled (Console + File + Database)");

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

    println!("Config:");
    println!(
        "  console_sink.enabled   = {} ✓",
        console_enabled(&config.console_sink)
    );
    let file_path = config
        .file_sink
        .as_ref()
        .map(|c| c.path.display().to_string())
        .unwrap_or_default();
    println!(
        "  file_sink.enabled      = {} ✓ (path={})",
        file_enabled(&config.file_sink),
        file_path
    );
    println!(
        "  database_sink.enabled  = {} ✓",
        db_enabled(&config.database_sink)
    );
    println!("\nsinks_enabled() = {:?}", sinks);
    println!("Sink count = {} (expected 3)", sinks.len());

    let has_console = sinks.contains(&"console");
    let has_file = sinks.contains(&"file");
    let has_database = sinks.contains(&"database");
    println!("\nVerification:");
    println!(
        "  contains \"console\"  = {} → {}",
        has_console,
        if has_console { "✓" } else { "✗" }
    );
    println!(
        "  contains \"file\"     = {} → {}",
        has_file,
        if has_file { "✓" } else { "✗" }
    );
    println!(
        "  contains \"database\" = {} → {}",
        has_database,
        if has_database { "✓" } else { "✗" }
    );
}

/// 演示 LoggerManager::load() 的使用方式
fn show_logger_manager_load_usage() {
    print_section("Example 4: LoggerManager::load() usage");

    println!("LoggerManager::load() flow:");
    println!("  1. Call InklogConfig::load_sync() to search for the config file");
    println!("     - Search paths (first existing file wins):");
    println!("         1. $INKLOG_CONFIG_PATH (explicit env var, highest priority)");
    println!("         2. ./inklog_config.toml (current directory)");
    println!(
        "         3. ~/.config/inklog/config.toml (user config dir; XDG_CONFIG_HOME on Linux)"
    );
    println!(
        "         4. /etc/inklog/config.toml (system config; %ProgramData%\\inklog\\config.toml on Windows)"
    );
    println!(
        "     - Returns Default::default() when no config file exists (console_sink enabled by default)"
    );
    println!("  2. Call LoggerManager::with_config() with the loaded config");
    println!("  3. Initialize all enabled sinks and the HTTP server");

    println!("\nCode example:");
    println!("  use inklog::LoggerManager;");
    println!();
    println!("  #![tokio::main]");
    println!("  async fn main() -> Result<(), Box<dyn std::error::Error>> {{");
    println!("      // Auto-search the config file and initialize");
    println!("      let _logger = LoggerManager::load().await?;");
    println!("      Ok(())");
    println!("  }}");

    println!("\nCustom search paths:");
    println!("  use inklog::InklogConfig;");
    println!();
    println!("  // Option 1: load from the default search paths");
    println!("  let config = InklogConfig::from_search_paths()?;");
    println!();
    println!("  // Option 2: combine with env var overrides");
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
    println!();
    println!("  let logger = LoggerManager::with_config(config).await?;");

    println!("\nNote: load() is an async method and must be called inside a tokio runtime.");
    println!(
        "This example only shows usage and does not actually call load() (it would start the logger and hold resources)."
    );
}

/// 演示配置检查工作流
fn show_config_inspection_workflow() {
    print_section("Example 5: config inspection workflow");

    println!("Recommended config inspection workflow:\n");

    println!("Step 1: load the config");
    let config = InklogConfig::load_with_env_overrides().unwrap_or_else(|e| {
        println!("  ⚠ Failed to load config: {}, using the default config", e);
        InklogConfig::default()
    });
    println!("  ✓ Config loaded");

    println!("\nStep 2: check enabled sinks");
    let sinks = config.sinks_enabled();
    if sinks.is_empty() {
        println!("  ⚠ No sink enabled; logs will not be output");
    } else {
        println!("  ✓ Enabled sinks: {:?}", sinks);
    }

    println!("\nStep 3: verify config consistency");
    let db_enabled_flag = db_enabled(&config.database_sink);
    if db_enabled_flag && !sinks.contains(&"database") {
        println!(
            "  ✗ database_sink.enabled=true but sinks_enabled() does not contain \"database\""
        );
    } else {
        println!("  ✓ Sink state is consistent");
    }

    println!("\nStep 4: choose the initialization method based on the config");
    if sinks.is_empty() {
        println!("  → Use LoggerManager::new() (default console only)");
    } else if InklogConfig::load_with_env_overrides().is_ok() {
        println!(
            "  → Use LoggerManager::load() (auto-search config; falls back to defaults when no file is found)"
        );
    } else {
        println!("  → Use LoggerManager::with_config() (explicit config)");
    }

    println!("\nFull code example:");
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
