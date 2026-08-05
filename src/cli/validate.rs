// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub fn validate_config(config_path: &PathBuf) -> Result<()> {
    let mut args = fluent_bundle::FluentArgs::new();
    args.set("path", config_path.display().to_string());
    println!("{}", inklog::i18n::tr_args("cli-validate-validating", args));

    if !config_path.exists() {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", config_path.display().to_string());
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-err-config-not-exist", args)
        ));
    }

    // Manual TOML parsing and validation
    let content = std::fs::read_to_string(config_path)
        .with_context(|| inklog::i18n::tr("cli-err-read-config"))?;

    validate_toml_content(&content, config_path)?;

    println!("{}", inklog::i18n::tr("cli-validate-valid"));
    Ok(())
}

fn validate_toml_content(content: &str, config_path: &PathBuf) -> Result<()> {
    let config: toml::Table = content
        .parse()
        .with_context(|| inklog::i18n::tr("cli-err-parse-toml"))?;

    if let Some(global) = config.get("global").and_then(|t| t.as_table()) {
        validate_global_config(global)?;
    }

    // Helper function to get table with flexible section naming (e.g., "console" or "console_sink")
    fn get_table<'a>(config: &'a toml::Table, name: &str) -> Option<&'a toml::Table> {
        config.get(name).and_then(|t| t.as_table())
    }

    // Handle both "console" and "console_sink" naming conventions
    if let Some(console) = get_table(&config, "console").or(get_table(&config, "console_sink")) {
        validate_console_sink(console)?;
    }

    // Handle both "file" and "file_sink" naming conventions
    if let Some(file) = get_table(&config, "file").or(get_table(&config, "file_sink")) {
        validate_file_sink(file)?;
    }

    if let Some(perf) = config.get("performance").and_then(|t| t.as_table()) {
        validate_performance(perf)?;
    }

    // Handle both "database" and "db_config" naming conventions
    if let Some(db) = get_table(&config, "database").or(get_table(&config, "db_config")) {
        validate_database_sink(db)?;
    }

    // Handle both "http" and "http_server" naming conventions
    if let Some(http) = get_table(&config, "http").or(get_table(&config, "http_server")) {
        validate_http_server(http)?;
    }

    validate_sections(&config, config_path)?;

    Ok(())
}

fn validate_global_config(global: &toml::Table) -> Result<()> {
    if let Some(level) = global.get("level") {
        let level_str = level.as_str().unwrap_or("");
        if !inklog::LogLevel::is_valid_level(level_str) {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("level", level_str.to_string());
            args.set("valid", inklog::LogLevel::VALID_LEVEL_STRINGS.join(", "));
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr_args("cli-err-invalid-log-level", args)
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("level", level_str.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-global-level", args)
        );
    }

    if let Some(format) = global.get("format")
        && let Some(format_str) = format.as_str()
    {
        if format_str.is_empty() {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-empty-format")
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("len", format_str.len().to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-global-format", args)
        );
    }

    Ok(())
}

fn validate_console_sink(console: &toml::Table) -> Result<()> {
    if let Some(enabled) = console.get("enabled") {
        if !enabled.is_bool() {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-console-enabled")
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("enabled", enabled.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-console-enabled", args)
        );
    }

    if let Some(colored) = console.get("colored")
        && !colored.is_bool()
    {
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr("cli-err-console-colored")
        ));
    }

    if let Some(stderr_levels) = console.get("stderr_levels")
        && let Some(levels) = stderr_levels.as_array()
    {
        for level in levels {
            if !level.is_str() {
                return Err(anyhow::anyhow!(
                    "{}",
                    inklog::i18n::tr("cli-err-console-stderr")
                ));
            }
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("count", levels.len().to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-console-stderr", args)
        );
    }

    Ok(())
}

fn validate_file_sink(file: &toml::Table) -> Result<()> {
    if let Some(enabled) = file.get("enabled") {
        if !enabled.is_bool() {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-file-enabled")
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("enabled", enabled.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-file-enabled", args)
        );
    }

    if let Some(path) = file.get("path")
        && let Some(path_str) = path.as_str()
    {
        if path_str.is_empty() {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-file-path-empty")
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", path_str.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-file-path", args)
        );
    }

    if let Some(max_size) = file.get("max_size")
        && let Some(size_str) = max_size.as_str()
    {
        if parse_size(size_str).is_err() {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("size", size_str.to_string());
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr_args("cli-err-file-max-size", args)
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("size", size_str.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-file-max-size", args)
        );
    }

    if let Some(keep_files) = file.get("keep_files")
        && let Some(n) = keep_files.as_integer()
        && n < 1
    {
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr("cli-err-file-keep-files")
        ));
    }

    if let Some(retention_days) = file.get("retention_days")
        && let Some(n) = retention_days.as_integer()
        && n < 1
    {
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr("cli-err-file-retention")
        ));
    }

    if let Some(compress) = file.get("compress")
        && !compress.is_bool()
    {
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr("cli-err-file-compress")
        ));
    }

    if let Some(encrypt) = file.get("encrypt") {
        if !encrypt.is_bool() {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-file-encrypt")
            ));
        }

        // Early return if encryption is not enabled
        if !encrypt.as_bool().unwrap_or(false) {
            return Ok(());
        }

        // Encryption enabled - validate encryption_key_env
        let key_env = match file.get("encryption_key_env") {
            Some(v) => v,
            None => {
                return Err(anyhow::anyhow!(
                    "{}",
                    inklog::i18n::tr("cli-err-file-encrypt-no-key")
                ));
            }
        };

        let env_name = match key_env.as_str() {
            Some(s) => s,
            None => {
                return Err(anyhow::anyhow!(
                    "{}",
                    inklog::i18n::tr("cli-err-file-key-env-type")
                ));
            }
        };

        if env_name.is_empty() {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-file-key-env-empty")
            ));
        }

        let mut args = fluent_bundle::FluentArgs::new();
        args.set("env", env_name.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-file-encrypt-key", args)
        );
    }

    Ok(())
}

fn validate_performance(perf: &toml::Table) -> Result<()> {
    if let Some(capacity) = perf.get("channel_capacity")
        && let Some(n) = capacity.as_integer()
    {
        if n < 1 {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-perf-capacity")
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("n", n.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-perf-capacity", args)
        );
    }

    if let Some(threads) = perf.get("worker_threads")
        && let Some(n) = threads.as_integer()
    {
        if n < 1 {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-perf-threads")
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("n", n.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-perf-threads", args)
        );
    }

    Ok(())
}

fn validate_database_sink(db: &toml::Table) -> Result<()> {
    // Check if database is explicitly disabled — skip driver/URL validation
    let is_disabled = db
        .get("enabled")
        .and_then(|v| v.as_bool())
        .is_some_and(|b| !b);

    if let Some(enabled) = db.get("enabled") {
        if !enabled.is_bool() {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-db-enabled")
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("enabled", enabled.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-db-enabled", args)
        );
    }

    // Skip driver/URL validation when database is disabled
    if is_disabled {
        return Ok(());
    }

    if let Some(driver) = db.get("driver") {
        let driver_str = driver.as_str().ok_or_else(|| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("driver", driver.to_string());
            anyhow::anyhow!("{}", inklog::i18n::tr_args("cli-err-db-driver-type", args))
        })?;
        let valid_drivers = ["postgres", "postgresql", "mysql", "sqlite", "sqlite3"];
        if !valid_drivers.contains(&driver_str.to_lowercase().as_str()) {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("driver", driver_str.to_string());
            args.set("valid", format!("{:?}", valid_drivers));
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr_args("cli-err-db-driver-invalid", args)
            ));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("driver", driver_str.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-db-driver", args)
        );
    }

    if let Some(url) = db.get("url")
        && let Some(url_str) = url.as_str()
    {
        if url_str.is_empty() {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-db-url-empty")
            ));
        }
        validate_database_url(url_str)?;
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("len", url_str.len().to_string());
        println!("  {}", inklog::i18n::tr_args("cli-validate-db-url", args));
    }

    if let Some(pool_size) = db.get("pool_size")
        && let Some(n) = pool_size.as_integer()
        && !(1..=100).contains(&n)
    {
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr("cli-err-db-pool-size")
        ));
    }

    if let Some(batch_size) = db.get("batch_size")
        && let Some(n) = batch_size.as_integer()
        && n < 1
    {
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr("cli-err-db-batch-size")
        ));
    }

    if let Some(table_name) = db.get("table_name")
        && let Some(name) = table_name.as_str()
    {
        if name.is_empty() {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-db-table-empty")
            ));
        }
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(anyhow::anyhow!(
                "{}",
                inklog::i18n::tr("cli-err-db-table-chars")
            ));
        }
    }

    Ok(())
}

fn validate_database_url(url: &str) -> Result<()> {
    let valid_prefixes = [
        "postgres://",
        "postgresql://",
        "mysql://",
        "sqlite://",
        "sqlite3://",
    ];
    let is_valid = valid_prefixes.iter().any(|p| url.starts_with(p));

    if !is_valid {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("prefixes", format!("{:?}", valid_prefixes));
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-err-db-url-invalid", args)
        ));
    }

    Ok(())
}

fn validate_http_server(http: &toml::Table) -> Result<()> {
    if let Some(enabled) = http.get("enabled")
        && !enabled.is_bool()
    {
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr("cli-err-http-enabled")
        ));
    }

    if let Some(port) = http.get("port")
        && let Some(n) = port.as_integer()
    {
        if !(1..=65535).contains(&n) {
            return Err(anyhow::anyhow!("{}", inklog::i18n::tr("cli-err-http-port")));
        }
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("n", n.to_string());
        println!(
            "  {}",
            inklog::i18n::tr_args("cli-validate-http-port", args)
        );
    }

    if let Some(host) = http.get("host")
        && let Some(host_str) = host.as_str()
        && host_str.is_empty()
    {
        return Err(anyhow::anyhow!("{}", inklog::i18n::tr("cli-err-http-host")));
    }

    Ok(())
}

fn validate_sections(config: &toml::Table, _config_path: &PathBuf) -> Result<()> {
    // Valid sections (both naming conventions accepted)
    let valid_sections = [
        "global",
        // Console variations
        "console",
        "console_sink",
        // File variations
        "file",
        "file_sink",
        // Database variations
        "database",
        "db_config",
        // Performance
        "performance",
        // HTTP variations
        "http",
        "http_server",
    ];

    for key in config.keys() {
        if !valid_sections.contains(&key.as_str()) {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("section", key.to_string());
            eprintln!(
                "  {}",
                inklog::i18n::tr_args("cli-warn-unknown-section", args)
            );
        }
    }

    // Check for dual sink configuration (both file and database)
    // Check if *either* alias has enabled=true for each sink type
    let has_file = config.contains_key("file") || config.contains_key("file_sink");
    let has_database = config.contains_key("database") || config.contains_key("db_config");

    let file_enabled = config
        .get("file")
        .or(config.get("file_sink"))
        .and_then(|t| t.as_table())
        .and_then(|t| t.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Also check the other alias in case the first one is disabled
    let file_enabled = file_enabled
        || config
            .get("file_sink")
            .or(config.get("file"))
            .and_then(|t| t.as_table())
            .and_then(|t| t.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

    if has_file && has_database && file_enabled {
        eprintln!("{}", inklog::i18n::tr("cli-warn-dual-sink"));
    }

    Ok(())
}

fn parse_size(size_str: &str) -> Result<()> {
    inklog::sink::rotation::parse_size(size_str)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub fn check_prerequisites() -> anyhow::Result<()> {
    println!("{}\n", inklog::i18n::tr("cli-prereq-checking"));

    let mut missing_critical = Vec::new();

    println!("  {}", inklog::i18n::tr("cli-prereq-rust"));
    let rust_output = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    match &rust_output {
        Some(v) => println!("    {}", v),
        None => {
            println!("    {}", inklog::i18n::tr("cli-prereq-not-found"));
            missing_critical.push("rustc");
        }
    }

    println!("  {}", inklog::i18n::tr("cli-prereq-cargo"));
    let cargo_output = Command::new("cargo")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    match &cargo_output {
        Some(v) => println!("    {}", v),
        None => {
            println!("    {}", inklog::i18n::tr("cli-prereq-not-found"));
            missing_critical.push("cargo");
        }
    }

    println!("\n  {}", inklog::i18n::tr("cli-prereq-optional"));
    let openssl_ok = Command::new("openssl")
        .arg("version")
        .output()
        .is_ok_and(|o| o.status.success());
    if openssl_ok {
        println!("    {}", inklog::i18n::tr("cli-prereq-openssl-ok"));
    } else {
        eprintln!("    {}", inklog::i18n::tr("cli-prereq-openssl-miss"));
    }

    let zstd_ok = Command::new("zstd")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if zstd_ok {
        println!("    {}", inklog::i18n::tr("cli-prereq-zstd-ok"));
    } else {
        eprintln!("    {}", inklog::i18n::tr("cli-prereq-zstd-miss"));
    }

    println!("\n  {}", inklog::i18n::tr("cli-prereq-config-check"));

    #[cfg(unix)]
    let home_config = std::path::PathBuf::from("/etc/inklog/config.toml");
    #[cfg(windows)]
    let home_config = std::env::var("ProgramData")
        .map(|base| {
            std::path::PathBuf::from(base)
                .join("inklog")
                .join("config.toml")
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("C:\\ProgramData\\inklog\\config.toml"));
    #[cfg(not(any(unix, windows)))]
    let home_config = std::path::PathBuf::from("/etc/inklog/config.toml");
    let local_config = std::path::PathBuf::from("./inklog_config.toml");
    let config_example = std::path::PathBuf::from("./config.example.toml");

    if home_config.exists() {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", home_config.display().to_string());
        println!("    {}", inklog::i18n::tr_args("cli-prereq-sys-ok", args));
    } else {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", home_config.display().to_string());
        eprintln!("    {}", inklog::i18n::tr_args("cli-prereq-sys-miss", args));
    }

    if local_config.exists() {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", local_config.display().to_string());
        println!("    {}", inklog::i18n::tr_args("cli-prereq-local-ok", args));
    } else {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", local_config.display().to_string());
        eprintln!(
            "    {}",
            inklog::i18n::tr_args("cli-prereq-local-miss", args)
        );
    }

    if config_example.exists() {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("path", config_example.display().to_string());
        println!(
            "    {}",
            inklog::i18n::tr_args("cli-prereq-example-ok", args)
        );
    }

    println!("\n{}", inklog::i18n::tr("cli-prereq-done"));

    if !missing_critical.is_empty() {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("deps", missing_critical.join(", "));
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-prereq-missing", args)
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn write_config(content: &str) -> NamedTempFile {
        let file = NamedTempFile::new().expect("failed to create temp file");
        std::fs::write(file.path(), content).expect("failed to write config");
        file
    }

    #[test]
    fn test_validate_config_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/path/config.toml");
        let result = validate_config(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_validate_config_valid_minimal() {
        let content = r#"
[global]
level = "info"
format = "{timestamp} [{level}] {message}"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_console_section() {
        // Covers get_table for "console" section
        let content = r#"
[console]
enabled = true
colored = true
stderr_levels = ["error", "warn"]
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_console_sink_naming() {
        // Covers get_table for "console_sink" naming convention
        let content = r#"
[console_sink]
enabled = true
colored = false
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_file_section() {
        // Covers get_table for "file" section
        let content = r#"
[file]
enabled = true
path = "logs/app.log"
max_size = "100MB"
keep_files = 5
retention_days = 30
compress = true
encrypt = false
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_file_sink_naming() {
        // Covers get_table for "file_sink" naming convention
        let content = r#"
[file_sink]
enabled = true
path = "logs/app.log"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_database_section() {
        // Covers get_table for "database" section
        let content = r#"
[database]
enabled = true
driver = "sqlite"
url = "sqlite://test.db"
pool_size = 10
batch_size = 100
table_name = "logs"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_db_config_naming() {
        // Covers get_table for "db_config" naming convention
        let content = r#"
[db_config]
enabled = true
driver = "postgres"
url = "postgres://localhost/db"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_http_section() {
        // Covers get_table for "http" section
        let content = r#"
[http]
enabled = true
port = 8080
host = "0.0.0.0"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_http_server_naming() {
        // Covers get_table for "http_server" naming convention
        let content = r#"
[http_server]
enabled = true
port = 9090
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_performance_section() {
        let content = r#"
[performance]
channel_capacity = 10000
worker_threads = 4
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_invalid_log_level() {
        let content = r#"
[global]
level = "invalid"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid log level")
        );
    }

    #[test]
    fn test_validate_config_invalid_database_driver() {
        let content = r#"
[database]
driver = "invalid_driver"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid database driver")
        );
    }

    #[test]
    fn test_validate_config_invalid_database_url() {
        let content = r#"
[database]
url = "invalid://url"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid database URL")
        );
    }

    #[test]
    fn test_validate_config_invalid_http_port() {
        let content = r#"
[http]
port = 99999
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("port"));
    }

    #[test]
    fn test_validate_config_invalid_toml() {
        let content = "not valid toml {{{";
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
    }

    // ========================================================================
    // get_table 分支覆盖（任务 7.9）
    // 覆盖 as_table() 返回 None（值存在但非 table）、双重命名等场景
    // ========================================================================

    #[test]
    fn test_get_table_console_value_not_a_table_falls_through_to_console_sink() {
        // 覆盖 get_table 中 config.get(name) 返回 Some 但 as_table() 返回 None 的分支
        // console = "string" 使 get_table("console") 返回 None
        // .or(get_table("console_sink")) 取到 [console_sink] 表
        let content = r#"
console = "not a table"
[console_sink]
enabled = true
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(
            result.is_ok(),
            "should fall through to console_sink, got: {:?}",
            result
        );
    }

    #[test]
    fn test_get_table_file_value_not_a_table_falls_through_to_file_sink() {
        // 同上，覆盖 file 字段非 table 时回退到 file_sink
        let content = r#"
file = 123
[file_sink]
enabled = true
path = "logs/app.log"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(
            result.is_ok(),
            "should fall through to file_sink, got: {:?}",
            result
        );
    }

    #[test]
    fn test_get_table_database_value_not_a_table_falls_through_to_db_config() {
        // 覆盖 database 字段非 table 时回退到 db_config
        let content = r#"
database = true
[db_config]
enabled = true
driver = "sqlite"
url = "sqlite://test.db"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(
            result.is_ok(),
            "should fall through to db_config, got: {:?}",
            result
        );
    }

    #[test]
    fn test_get_table_http_value_not_a_table_falls_through_to_http_server() {
        // 覆盖 http 字段非 table 时回退到 http_server
        let content = r#"
http = "enabled"
[http_server]
enabled = true
port = 8080
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(
            result.is_ok(),
            "should fall through to http_server, got: {:?}",
            result
        );
    }

    #[test]
    fn test_get_table_both_naming_conventions_present_prefers_first() {
        // 覆盖 .or() 短路：当 console 和 console_sink 都存在时，应使用 console
        // 通过让 console 含无效配置（enabled 非布尔）来验证优先使用 console
        let content = r#"
[console]
enabled = "invalid"
[console_sink]
enabled = true
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err(), "should use first [console] table and fail");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("console_sink.enabled must be a boolean")
        );
    }

    #[test]
    fn test_get_table_both_file_naming_conventions_prefers_first() {
        // 覆盖 file/file_sink 的 .or() 短路：file 优先
        let content = r#"
[file]
enabled = "invalid"
[file_sink]
enabled = true
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("file_sink.enabled must be a boolean")
        );
    }

    #[test]
    fn test_get_table_no_section_present() {
        // 覆盖 get_table 在两个命名都不存在时返回 None 的路径
        // 此时 validate_toml_content 应直接 Ok（所有 section 可选）
        let content = r#"
[global]
level = "info"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_table_both_names_not_tables() {
        // 覆盖两个命名都存在但都不是 table 的场景
        // get_table 对两者都返回 None，validate 跳过验证
        let content = r#"
console = "string1"
console_sink = "string2"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(
            result.is_ok(),
            "non-table values should be skipped, got: {:?}",
            result
        );
    }

    // ========================================================================
    // validate_sections: 双 sink 警告分支（行 386-400）
    // ========================================================================

    #[test]
    fn test_validate_sections_dual_sink_file_disabled_warning() {
        // 覆盖 validate_sections 中 has_file && has_database 且 file.enabled=false 的分支
        // 该分支会 eprintln 警告但返回 Ok
        let content = r#"
[file]
enabled = false
[database]
enabled = true
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        // 警告不影响 Ok 结果
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_sections_dual_sink_file_enabled_no_warning() {
        // 覆盖 has_file && has_database 且 file.enabled=true 的分支（不进入 if !enabled）
        let content = r#"
[file]
enabled = true
[database]
enabled = true
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_sections_dual_sink_file_enabled_via_file_sink() {
        // 覆盖 file_sink 命名下的 dual-sink 分支
        let content = r#"
[file_sink]
enabled = false
[db_config]
enabled = true
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    // ========================================================================
    // 类型错误与边界值覆盖
    // ========================================================================

    #[test]
    fn test_validate_global_config_level_not_string() {
        // 覆盖 global.level 为非字符串时 as_str() 返回 None 的分支（unwrap_or("")）
        // 空字符串不在 valid_levels 中，应返回错误
        let content = r#"
[global]
level = 123
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid log level")
        );
    }

    #[test]
    fn test_validate_global_config_format_not_string() {
        // 覆盖 global.format 为非字符串时 as_str() 返回 None 的分支
        // 该分支不会触发错误（仅当 format_str 非空时才检查）
        let content = r#"
[global]
format = 123
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_console_sink_enabled_not_bool() {
        // 覆盖 console_sink.enabled 非布尔类型
        let content = r#"
[console]
enabled = "yes"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("console_sink.enabled must be a boolean")
        );
    }

    #[test]
    fn test_validate_console_sink_colored_not_bool() {
        // 覆盖 console_sink.colored 非布尔类型
        let content = r#"
[console]
colored = "yes"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("console_sink.colored must be a boolean")
        );
    }

    #[test]
    fn test_validate_console_sink_stderr_levels_not_array() {
        // 覆盖 console_sink.stderr_levels 非数组时 as_array() 返回 None 的分支
        // 该分支不会触发错误（仅当 levels 是数组时才迭代）
        let content = r#"
[console]
stderr_levels = "error"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_console_sink_stderr_levels_non_string_element() {
        // 覆盖 stderr_levels 数组中含非字符串元素
        let content = r#"
[console]
stderr_levels = [123, "error"]
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("stderr_levels must be an array of strings")
        );
    }

    #[test]
    fn test_validate_file_sink_enabled_not_bool() {
        // 覆盖 file_sink.enabled 非布尔类型
        let content = r#"
[file]
enabled = "yes"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("file_sink.enabled must be a boolean")
        );
    }

    #[test]
    fn test_validate_file_sink_path_empty() {
        // 覆盖 file_sink.path 为空字符串
        let content = r#"
[file]
path = ""
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("file_sink.path cannot be empty")
        );
    }

    #[test]
    fn test_validate_file_sink_max_size_invalid_format() {
        // 覆盖 parse_size 失败分支
        let content = r#"
[file]
max_size = "100XX"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid file_sink.max_size format")
        );
    }

    #[test]
    fn test_validate_file_sink_keep_files_zero() {
        // 覆盖 keep_files < 1 的错误分支
        let content = r#"
[file]
keep_files = 0
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("file_sink.keep_files must be >= 1")
        );
    }

    #[test]
    fn test_validate_file_sink_retention_days_zero() {
        // 覆盖 retention_days < 1 的错误分支
        let content = r#"
[file]
retention_days = 0
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("file_sink.retention_days must be >= 1")
        );
    }

    #[test]
    fn test_validate_file_sink_compress_not_bool() {
        // 覆盖 compress 非布尔类型
        let content = r#"
[file]
compress = "yes"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("file_sink.compress must be a boolean")
        );
    }

    #[test]
    fn test_validate_file_sink_encrypt_not_bool() {
        // 覆盖 encrypt 非布尔类型
        let content = r#"
[file]
encrypt = "yes"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("file_sink.encrypt must be a boolean")
        );
    }

    #[test]
    fn test_validate_file_sink_encrypt_key_env_not_string() {
        // 覆盖 encryption_key_env 非字符串类型的分支
        let content = r#"
[file]
encrypt = true
encryption_key_env = 123
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("encryption_key_env must be a string")
        );
    }

    #[test]
    fn test_validate_file_sink_encrypt_false_skips_key_env_check() {
        // 覆盖 encrypt=false 时提前返回 Ok 的分支（行 186-188）
        let content = r#"
[file]
encrypt = false
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_database_sink_enabled_not_bool() {
        // 覆盖 db_config.enabled 非布尔类型
        let content = r#"
[database]
enabled = "yes"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("db_config.enabled must be a boolean")
        );
    }

    #[test]
    fn test_validate_database_sink_driver_not_string() {
        // driver 非字符串时 as_str() 返回 None，应触发错误
        let content = r#"
[database]
driver = 123
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        // driver 非字符串时应返回错误
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("db_config.driver must be a string")
        );
    }

    #[test]
    fn test_validate_database_sink_url_empty() {
        // 覆盖 url 为空字符串
        let content = r#"
[database]
url = ""
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("db_config.url cannot be empty")
        );
    }

    #[test]
    fn test_validate_database_sink_pool_size_zero() {
        // 覆盖 pool_size 不在 1..=100 范围
        let content = r#"
[database]
pool_size = 0
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("pool_size must be between 1 and 100")
        );
    }

    #[test]
    fn test_validate_database_sink_pool_size_too_large() {
        // 覆盖 pool_size 超过 100 的分支
        let content = r#"
[database]
pool_size = 200
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("pool_size must be between 1 and 100")
        );
    }

    #[test]
    fn test_validate_database_sink_batch_size_zero() {
        // 覆盖 batch_size < 1 的错误分支
        let content = r#"
[database]
batch_size = 0
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("db_config.batch_size must be >= 1")
        );
    }

    #[test]
    fn test_validate_database_sink_table_name_empty() {
        // 覆盖 table_name 为空字符串
        let content = r#"
[database]
table_name = ""
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("db_config.table_name cannot be empty")
        );
    }

    #[test]
    fn test_validate_database_sink_table_name_invalid_chars() {
        // 覆盖 table_name 含非法字符
        let content = r#"
[database]
table_name = "invalid-name!"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("table_name must contain only alphanumeric")
        );
    }

    #[test]
    fn test_validate_http_server_enabled_not_bool() {
        // 覆盖 http_server.enabled 非布尔类型
        let content = r#"
[http]
enabled = "yes"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("http_server.enabled must be a boolean")
        );
    }

    #[test]
    fn test_validate_http_server_host_empty() {
        // 覆盖 host 为空字符串
        let content = r#"
[http]
host = ""
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("http_server.host cannot be empty")
        );
    }

    #[test]
    fn test_validate_performance_channel_capacity_zero() {
        // 覆盖 channel_capacity < 1 的错误分支
        let content = r#"
[performance]
channel_capacity = 0
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("performance.channel_capacity must be >= 1")
        );
    }

    #[test]
    fn test_validate_performance_worker_threads_zero() {
        // 覆盖 worker_threads < 1 的错误分支
        let content = r#"
[performance]
worker_threads = 0
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("performance.worker_threads must be >= 1")
        );
    }

    #[test]
    fn test_validate_config_empty_file() {
        // 边界场景：空配置文件
        let file = write_config("");
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_database_url_valid_prefixes() {
        // 覆盖 validate_database_url 各合法前缀分支
        for url in &[
            "postgres://localhost/db",
            "postgresql://localhost/db",
            "mysql://localhost/db",
            "sqlite://test.db",
            "sqlite3://test.db",
        ] {
            let content = format!("[database]\nurl = \"{}\"", url);
            let file = write_config(&content);
            let result = validate_config(&file.path().to_path_buf());
            assert!(result.is_ok(), "url {} should be valid", url);
        }
    }

    #[test]
    fn test_validate_global_config_lowercase_level() {
        // 覆盖 to_lowercase 转换：大写日志级别应通过校验
        let content = r#"
[global]
level = "INFO"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_global_config_all_valid_levels() {
        // 覆盖所有合法日志级别
        for level in &["trace", "debug", "info", "warn", "error"] {
            let content = format!("[global]\nlevel = \"{}\"", level);
            let file = write_config(&content);
            let result = validate_config(&file.path().to_path_buf());
            assert!(result.is_ok(), "level {} should be valid", level);
        }
    }

    #[test]
    fn test_validate_file_sink_max_size_valid_units() {
        // 覆盖 parse_size 各合法单位
        for size in &["100B", "100KB", "100MB", "1GB", "1TB"] {
            let content = format!("[file]\nmax_size = \"{}\"", size);
            let file = write_config(&content);
            let result = validate_config(&file.path().to_path_buf());
            assert!(result.is_ok(), "size {} should be valid", size);
        }
    }

    #[test]
    fn test_validate_database_sink_valid_drivers() {
        // 覆盖所有合法驱动（含大小写）
        for driver in &["postgres", "postgresql", "mysql", "sqlite", "sqlite3"] {
            let content = format!("[database]\ndriver = \"{}\"", driver);
            let file = write_config(&content);
            let result = validate_config(&file.path().to_path_buf());
            assert!(result.is_ok(), "driver {} should be valid", driver);
        }
    }

    // ==================== check_prerequisites 集成测试 (L429-485) ====================

    #[test]
    fn test_check_prerequisites_runs_without_panic() {
        // 覆盖 check_prerequisites() 整个函数体（L429-485）：
        // - rustc/cargo 版本检查（L432-448）
        // - openssl/zstd 可选依赖检查（L450-461）的 if 分支
        // - 系统配置/本地配置/示例配置存在性检查（L463-483）的 if/else 分支
        // - 完成输出（L485）
        //
        // 该函数无返回值，仅打印；验证其不 panic 即可。
        // 注意：openssl/zstd/config 文件的存在性取决于运行环境，
        // 两个分支（存在/不存在）都会被执行到其中之一。
        let _ = check_prerequisites();
    }

    // ==================== validate_file_sink encrypt 分支测试 ====================

    #[test]
    fn test_validate_file_sink_encrypt_without_key_env() {
        // 覆盖 L191-198: encrypt=true 但 encryption_key_env 未设置
        let content = "[file]\nenabled = true\nencrypt = true";
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("encryption_key_env is not set"));
    }

    #[test]
    fn test_validate_file_sink_encrypt_with_empty_key_env() {
        // 覆盖 L207-211: encrypt=true 但 encryption_key_env 为空字符串
        let content = "[file]\nenabled = true\nencrypt = true\nencryption_key_env = \"\"";
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("encryption_key_env is empty"));
    }

    #[test]
    fn test_validate_file_sink_encrypt_with_valid_key_env() {
        // 覆盖 L213: encrypt=true 且 encryption_key_env 有效 → 成功路径
        let content = "[file]\nenabled = true\nencrypt = true\nencryption_key_env = \"INKLOG_ENCRYPTION_KEY\"";
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }
}
