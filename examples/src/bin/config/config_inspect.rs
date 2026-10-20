// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 配置 inspect 示例：sinks_enabled + LoggerManager::load
//!
//! 演示 `InklogConfig::sinks_enabled` 和 `LoggerManager::load` 的使用：
//!
//! 1. `sinks_enabled` 返回已启用的 Sink 列表
//! 2. `LoggerManager::load` 从搜索路径自动加载配置并初始化
//! 3. 配置检查工作流
//!
//! # 运行
//!
//! ```bash
//! cargo run --package inklog-examples --bin config_inspect
//! ```

use inklog::config::{ConsoleSinkConfig, DatabaseSinkConfig, FileSinkConfig, InklogConfig};
use inklog_examples::common::{print_section, print_separator, tr, tr_args};

fn main() {
    print_separator(&tr(
        "example-config-inspect-inklog-config-inspection-example",
    ));

    show_sinks_enabled_all_disabled();
    show_sinks_enabled_console_only();
    show_sinks_enabled_multi_sink();
    show_logger_manager_load_usage();
    show_config_inspection_workflow();

    println!(
        "\n{}",
        tr("example-config-inspect-all-config-inspection-examples")
    );
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

/// 演示所有 Sink 禁用时 sinks_enabled 返回空
fn show_sinks_enabled_all_disabled() {
    print_section(&tr("example-config-inspect-example-1-all-sinks-disabled"));

    // 注意：InklogConfig::default 的 console_sink 默认启用，直接取
    // sinks_enabled 会得到 ["console"]；要得到空列表必须显式禁用全部 Sink
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

    println!(
        "{}",
        tr("example-config-inspect-all-sinks-explicitly-disabled")
    );
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-console-sink-enabled",
            &[("value", &(console_enabled(&config.console_sink)))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-file-sink-enabled",
            &[("value", &(file_enabled(&config.file_sink)))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-database-sink-enabled",
            &[("value", &(db_enabled(&config.database_sink)))]
        )
    );
    println!(
        "\n{}",
        tr_args(
            "example-config-inspect-sinks-enabled",
            &[("sinks", &format!("{:?}", sinks))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-inspect-returned-vec-length",
            &[("value", &(sinks.len()))]
        )
    );
    assert!(
        sinks.is_empty(),
        "sinks_enabled() should be empty when everything is disabled"
    );

    println!(
        "\n{}",
        tr("example-config-inspect-comparison-console-sink-in")
    );
    let default_sinks = InklogConfig::default().sinks_enabled();
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-default-sinks-enabled",
            &[("default_sinks", &format!("{:?}", default_sinks))]
        )
    );
    assert_eq!(default_sinks, vec!["console"]);
}

/// 演示仅 Console Sink 启用
fn show_sinks_enabled_console_only() {
    print_section(&tr(
        "example-config-inspect-example-2-only-the-console-sink",
    ));

    let config = InklogConfig {
        console_sink: Some(ConsoleSinkConfig {
            enabled: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let sinks = config.sinks_enabled();

    println!("{}", tr("example-config-inspect-config"));
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-console-sink-enabled-2",
            &[("value", &(console_enabled(&config.console_sink)))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-file-sink-enabled",
            &[("value", &(file_enabled(&config.file_sink)))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-database-sink-enabled",
            &[("value", &(db_enabled(&config.database_sink)))]
        )
    );
    println!(
        "\n{}",
        tr_args(
            "example-config-inspect-sinks-enabled",
            &[("sinks", &format!("{:?}", sinks))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-inspect-contains-console",
            &[("value", &(sinks.contains(&"console")))]
        )
    );
}

/// 演示多 Sink 启用
fn show_sinks_enabled_multi_sink() {
    print_section(&tr("example-config-inspect-example-3-multiple-sinks"));

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

    println!("{}", tr("example-config-inspect-config"));
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-console-sink-enabled-2",
            &[("value", &(console_enabled(&config.console_sink)))]
        )
    );
    let file_path = config
        .file_sink
        .as_ref()
        .map(|c| c.path.display().to_string())
        .unwrap_or_default();
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-file-sink-enabled-path",
            &[
                ("value", &(file_enabled(&config.file_sink))),
                ("file_path", &file_path)
            ]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-database-sink-enabled-2",
            &[("value", &(db_enabled(&config.database_sink)))]
        )
    );
    println!(
        "\n{}",
        tr_args(
            "example-config-inspect-sinks-enabled",
            &[("sinks", &format!("{:?}", sinks))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-config-inspect-sink-count-expected-3",
            &[("value", &(sinks.len()))]
        )
    );

    let has_console = sinks.contains(&"console");
    let has_file = sinks.contains(&"file");
    let has_database = sinks.contains(&"database");
    println!("\n{}", tr("example-config-inspect-verification"));
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-contains-console-2",
            &[
                ("has_console", &has_console),
                ("value", &(if has_console { "✓" } else { "✗" }))
            ]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-contains-file",
            &[
                ("has_file", &has_file),
                ("value", &(if has_file { "✓" } else { "✗" }))
            ]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-inspect-contains-database",
            &[
                ("has_database", &has_database),
                ("value", &(if has_database { "✓" } else { "✗" }))
            ]
        )
    );
}

/// 演示 LoggerManager::load 的使用方式
fn show_logger_manager_load_usage() {
    print_section(&tr("example-config-inspect-example-4-loggermanager-load"));

    println!("{}", tr("example-config-inspect-loggermanager-load-flow"));
    println!(
        "  {}",
        tr("example-config-inspect-1-call-inklogconfig-load-sync")
    );
    println!(
        "     {}",
        tr("example-config-inspect-search-paths-first-existing")
    );
    println!(
        "         {}",
        tr("example-config-inspect-1-inklog-config-path-explicit")
    );
    println!(
        "         {}",
        tr("example-config-inspect-2-inklog-config-toml-current")
    );
    println!(
        "         3. ~/.config/inklog/config.toml (user config dir; XDG_CONFIG_HOME on Linux)"
    );
    println!(
        "         4. /etc/inklog/config.toml (system config; %ProgramData%\\inklog\\config.toml on Windows)"
    );
    println!(
        "     - Returns Default::default() when no config file exists (console_sink enabled by default)"
    );
    println!(
        "  {}",
        tr("example-config-inspect-2-call-loggermanager-with")
    );
    println!(
        "  {}",
        tr("example-config-inspect-3-initialize-all-enabled-sinks")
    );

    println!("\n{}", tr("example-config-inspect-code-example"));
    println!("  use inklog::LoggerManager;");
    println!();
    println!("  #![tokio::main]");
    println!("  async fn main() -> Result<(), Box<dyn std::error::Error>> {{");
    println!("      // Auto-search the config file and initialize");
    println!("      let _logger = LoggerManager::load().await?;");
    println!("      Ok(())");
    println!("  }}");

    println!("\n{}", tr("example-config-inspect-custom-search-paths"));
    println!("  use inklog::InklogConfig;");
    println!();
    println!("  // Option 1: load from the default search paths");
    println!("  let config = InklogConfig::from_search_paths()?;");
    println!();
    println!("  // Option 2: combine with env var overrides");
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
    println!();
    println!("  let logger = LoggerManager::with_config(config).await?;");

    println!(
        "\n{}",
        tr("example-config-inspect-note-load-is-an-async-method")
    );
    println!(
        "This example only shows usage and does not actually call load() (it would start the logger and hold resources)."
    );
}

/// 演示配置检查工作流
fn show_config_inspection_workflow() {
    print_section(&tr("example-config-inspect-example-5-config-inspection"));

    println!(
        "{}\n",
        tr("example-config-inspect-recommended-config-inspection")
    );

    println!("{}", tr("example-config-inspect-step-1-load-the-config"));
    let config = InklogConfig::load_with_env_overrides().unwrap_or_else(|e| {
        println!(
            "  {}",
            tr_args(
                "example-config-inspect-failed-to-load-config-using-the",
                &[("e", &e)]
            )
        );
        InklogConfig::default()
    });
    println!("  {}", tr("example-config-inspect-config-loaded"));

    println!(
        "\n{}",
        tr("example-config-inspect-step-2-check-enabled-sinks")
    );
    let sinks = config.sinks_enabled();
    if sinks.is_empty() {
        println!(
            "  {}",
            tr("example-config-inspect-no-sink-enabled-logs-will-not")
        );
    } else {
        println!(
            "  {}",
            tr_args(
                "example-config-inspect-enabled-sinks",
                &[("sinks", &format!("{:?}", sinks))]
            )
        );
    }

    println!(
        "\n{}",
        tr("example-config-inspect-step-3-verify-config-consistency")
    );
    let db_enabled_flag = db_enabled(&config.database_sink);
    if db_enabled_flag && !sinks.contains(&"database") {
        println!(
            "  ✗ database_sink.enabled=true but sinks_enabled() does not contain \"database\""
        );
    } else {
        println!(
            "  {}",
            tr("example-config-inspect-sink-state-is-consistent")
        );
    }

    println!("\n{}", tr("example-config-inspect-step-4-choose-the"));
    if sinks.is_empty() {
        println!(
            "  {}",
            tr("example-config-inspect-use-loggermanager-new-default")
        );
    } else if InklogConfig::load_with_env_overrides().is_ok() {
        println!(
            "  → Use LoggerManager::load() (auto-search config; falls back to defaults when no file is found)"
        );
    } else {
        println!(
            "  {}",
            tr("example-config-inspect-use-loggermanager-with-config")
        );
    }

    println!("\n{}", tr("example-config-inspect-full-code-example"));
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
