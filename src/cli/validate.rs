// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub fn validate_config(config_path: &PathBuf) -> Result<()> {
    println!("Validating configuration file: {}", config_path.display());

    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Config file does not exist: {}",
            config_path.display()
        ));
    }

    // Manual TOML parsing and validation
    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    validate_toml_content(&content, config_path)?;

    println!("✓ Configuration file is valid");
    Ok(())
}

fn validate_toml_content(content: &str, config_path: &PathBuf) -> Result<()> {
    let config: toml::Table = content
        .parse()
        .with_context(|| "Failed to parse TOML content")?;

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

    // Handle both "s3" and "s3_archive" naming conventions
    if let Some(s3) = get_table(&config, "s3").or(get_table(&config, "s3_archive")) {
        validate_s3_archive(s3)?;
    }

    validate_sections(&config, config_path)?;

    Ok(())
}

fn validate_global_config(global: &toml::Table) -> Result<()> {
    if let Some(level) = global.get("level") {
        let level_str = level.as_str().unwrap_or("");
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&level_str.to_lowercase().as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid log level '{}'. Valid levels: {:?}",
                level_str,
                valid_levels
            ));
        }
        println!("  ✓ Global level: {}", level_str);
    }

    if let Some(format) = global.get("format") {
        if let Some(format_str) = format.as_str() {
            if format_str.is_empty() {
                return Err(anyhow::anyhow!("Global format cannot be empty"));
            }
            println!("  ✓ Global format: {} chars", format_str.len());
        }
    }

    Ok(())
}

fn validate_console_sink(console: &toml::Table) -> Result<()> {
    if let Some(enabled) = console.get("enabled") {
        if !enabled.is_bool() {
            return Err(anyhow::anyhow!("console_sink.enabled must be a boolean"));
        }
        println!("  ✓ Console sink enabled: {}", enabled);
    }

    if let Some(colored) = console.get("colored") {
        if !colored.is_bool() {
            return Err(anyhow::anyhow!("console_sink.colored must be a boolean"));
        }
    }

    if let Some(stderr_levels) = console.get("stderr_levels") {
        if let Some(levels) = stderr_levels.as_array() {
            for level in levels {
                if !level.is_str() {
                    return Err(anyhow::anyhow!(
                        "console_sink.stderr_levels must be an array of strings"
                    ));
                }
            }
            println!("  ✓ Console stderr_levels: {} levels", levels.len());
        }
    }

    Ok(())
}

fn validate_file_sink(file: &toml::Table) -> Result<()> {
    if let Some(enabled) = file.get("enabled") {
        if !enabled.is_bool() {
            return Err(anyhow::anyhow!("file_sink.enabled must be a boolean"));
        }
        println!("  ✓ File sink enabled: {}", enabled);
    }

    if let Some(path) = file.get("path") {
        if let Some(path_str) = path.as_str() {
            if path_str.is_empty() {
                return Err(anyhow::anyhow!("file_sink.path cannot be empty"));
            }
            println!("  ✓ File path: {}", path_str);
        }
    }

    if let Some(max_size) = file.get("max_size") {
        if let Some(size_str) = max_size.as_str() {
            if parse_size(size_str).is_err() {
                return Err(anyhow::anyhow!(
                    "Invalid file_sink.max_size format: {}. Use format like '100MB', '1GB'",
                    size_str
                ));
            }
            println!("  ✓ Max size: {}", size_str);
        }
    }

    if let Some(keep_files) = file.get("keep_files") {
        if let Some(n) = keep_files.as_integer() {
            if n < 1 {
                return Err(anyhow::anyhow!("file_sink.keep_files must be >= 1"));
            }
        }
    }

    if let Some(retention_days) = file.get("retention_days") {
        if let Some(n) = retention_days.as_integer() {
            if n < 1 {
                return Err(anyhow::anyhow!("file_sink.retention_days must be >= 1"));
            }
        }
    }

    if let Some(compress) = file.get("compress") {
        if !compress.is_bool() {
            return Err(anyhow::anyhow!("file_sink.compress must be a boolean"));
        }
    }

    if let Some(encrypt) = file.get("encrypt") {
        if !encrypt.is_bool() {
            return Err(anyhow::anyhow!("file_sink.encrypt must be a boolean"));
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
                    "file_sink.encrypt is true but encryption_key_env is not set"
                ));
            }
        };

        let env_name = match key_env.as_str() {
            Some(s) => s,
            None => {
                return Err(anyhow::anyhow!("encryption_key_env must be a string"));
            }
        };

        if env_name.is_empty() {
            return Err(anyhow::anyhow!(
                "file_sink.encrypt is true but encryption_key_env is empty"
            ));
        }

        println!("  ✓ Encryption key env: {}", env_name);
    }

    Ok(())
}

fn validate_performance(perf: &toml::Table) -> Result<()> {
    if let Some(capacity) = perf.get("channel_capacity") {
        if let Some(n) = capacity.as_integer() {
            if n < 1 {
                return Err(anyhow::anyhow!("performance.channel_capacity must be >= 1"));
            }
            println!("  ✓ Channel capacity: {}", n);
        }
    }

    if let Some(threads) = perf.get("worker_threads") {
        if let Some(n) = threads.as_integer() {
            if n < 1 {
                return Err(anyhow::anyhow!("performance.worker_threads must be >= 1"));
            }
            println!("  ✓ Worker threads: {}", n);
        }
    }

    Ok(())
}

fn validate_database_sink(db: &toml::Table) -> Result<()> {
    if let Some(enabled) = db.get("enabled") {
        if !enabled.is_bool() {
            return Err(anyhow::anyhow!("db_config.enabled must be a boolean"));
        }
        println!("  ✓ Database sink enabled: {}", enabled);
    }

    if let Some(driver) = db.get("driver") {
        if let Some(driver_str) = driver.as_str() {
            let valid_drivers = ["postgres", "postgresql", "mysql", "sqlite", "sqlite3"];
            if !valid_drivers.contains(&driver_str.to_lowercase().as_str()) {
                return Err(anyhow::anyhow!(
                    "Invalid database driver '{}'. Valid drivers: {:?}",
                    driver_str,
                    valid_drivers
                ));
            }
            println!("  ✓ Database driver: {}", driver_str);
        }
    }

    if let Some(url) = db.get("url") {
        if let Some(url_str) = url.as_str() {
            if url_str.is_empty() {
                return Err(anyhow::anyhow!("db_config.url cannot be empty"));
            }
            validate_database_url(url_str)?;
            println!("  ✓ Database URL: {} bytes", url_str.len());
        }
    }

    if let Some(pool_size) = db.get("pool_size") {
        if let Some(n) = pool_size.as_integer() {
            if !(1..=100).contains(&n) {
                return Err(anyhow::anyhow!(
                    "db_config.pool_size must be between 1 and 100"
                ));
            }
        }
    }

    if let Some(batch_size) = db.get("batch_size") {
        if let Some(n) = batch_size.as_integer() {
            if n < 1 {
                return Err(anyhow::anyhow!("db_config.batch_size must be >= 1"));
            }
        }
    }

    if let Some(table_name) = db.get("table_name") {
        if let Some(name) = table_name.as_str() {
            if name.is_empty() {
                return Err(anyhow::anyhow!("db_config.table_name cannot be empty"));
            }
            if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(anyhow::anyhow!(
                    "db_config.table_name must contain only alphanumeric characters and underscores"
                ));
            }
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
        return Err(anyhow::anyhow!(
            "Invalid database URL. Must start with one of: {:?}",
            valid_prefixes
        ));
    }

    Ok(())
}

fn validate_http_server(http: &toml::Table) -> Result<()> {
    if let Some(enabled) = http.get("enabled") {
        if !enabled.is_bool() {
            return Err(anyhow::anyhow!("http_server.enabled must be a boolean"));
        }
    }

    if let Some(port) = http.get("port") {
        if let Some(n) = port.as_integer() {
            if !(1..=65535).contains(&n) {
                return Err(anyhow::anyhow!(
                    "http_server.port must be between 1 and 65535"
                ));
            }
            println!("  ✓ HTTP port: {}", n);
        }
    }

    if let Some(host) = http.get("host") {
        if let Some(host_str) = host.as_str() {
            if host_str.is_empty() {
                return Err(anyhow::anyhow!("http_server.host cannot be empty"));
            }
        }
    }

    Ok(())
}

fn validate_s3_archive(s3: &toml::Table) -> Result<()> {
    if let Some(enabled) = s3.get("enabled") {
        if !enabled.is_bool() {
            return Err(anyhow::anyhow!("s3_archive.enabled must be a boolean"));
        }
        println!("  ✓ S3 archive enabled: {}", enabled);
    }

    if let Some(bucket) = s3.get("bucket") {
        if let Some(bucket_str) = bucket.as_str() {
            if bucket_str.is_empty() {
                return Err(anyhow::anyhow!("s3_archive.bucket cannot be empty"));
            }
            println!("  ✓ S3 bucket: {}", bucket_str);
        }
    }

    if let Some(region) = s3.get("region") {
        if let Some(region_str) = region.as_str() {
            if region_str.is_empty() {
                return Err(anyhow::anyhow!("s3_archive.region cannot be empty"));
            }
            println!("  ✓ S3 region: {}", region_str);
        }
    }

    if let Some(interval) = s3.get("archive_interval_days") {
        if let Some(n) = interval.as_integer() {
            if n < 1 {
                return Err(anyhow::anyhow!(
                    "s3_archive.archive_interval_days must be >= 1"
                ));
            }
        }
    }

    if let Some(max_size) = s3.get("max_file_size_mb") {
        if let Some(n) = max_size.as_integer() {
            if n < 1 {
                return Err(anyhow::anyhow!("s3_archive.max_file_size_mb must be >= 1"));
            }
        }
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
        // S3 variations
        "s3",
        "s3_archive",
        // Performance
        "performance",
        // HTTP variations
        "http",
        "http_server",
    ];

    for key in config.keys() {
        if !valid_sections.contains(&key.as_str()) {
            eprintln!("  ⚠ Unknown configuration section: [{}]", key);
        }
    }

    // Check for dual sink configuration (both file and database)
    let has_file = config.contains_key("file") || config.contains_key("file_sink");
    let has_database = config.contains_key("database") || config.contains_key("db_config");

    if has_file && has_database {
        if let Some(file) = config
            .get("file")
            .or(config.get("file_sink"))
            .and_then(|t| t.as_table())
        {
            if let Some(enabled) = file.get("enabled").and_then(|v| v.as_bool()) {
                if !enabled {
                    eprintln!(
                        "  ⚠ Both file and database sinks enabled - logs will be written to both"
                    );
                }
            }
        }
    }

    Ok(())
}

fn parse_size(size_str: &str) -> Result<()> {
    let size_str = size_str.trim().to_uppercase();
    let (num_str, unit) = size_str.split_at(
        size_str
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(size_str.len()),
    );

    let _: f64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid number: {}", num_str))?;

    let valid_units = ["B", "KB", "MB", "GB", "TB"];
    if !valid_units.contains(&unit) {
        return Err(anyhow::anyhow!(
            "Invalid size unit '{}'. Valid units: {:?}",
            unit,
            valid_units
        ));
    }

    Ok(())
}

pub fn check_prerequisites() {
    println!("Checking prerequisites...\n");

    println!("  Rust version:");
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "not found".to_string());
    println!("    {}", output);

    println!("  Cargo version:");
    let output = Command::new("cargo")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "not found".to_string());
    println!("    {}", output);

    println!("\n  Optional dependencies:");
    if Command::new("openssl").arg("version").output().is_ok() {
        println!("    ✓ OpenSSL available");
    } else {
        eprintln!("    ⚠ OpenSSL not found (needed for encryption)");
    }

    if Command::new("zstd").arg("--version").output().is_ok() {
        println!("    ✓ zstd available");
    } else {
        eprintln!("    ⚠ zstd not found (for compression support)");
    }

    println!("\n  Configuration check:");

    let home_config = std::path::PathBuf::from("/etc/inklog/config.toml");
    let local_config = std::path::PathBuf::from("./inklog_config.toml");
    let config_example = std::path::PathBuf::from("./config.example.toml");

    if home_config.exists() {
        println!("    ✓ System config exists: {}", home_config.display());
    } else {
        eprintln!("    ⚠ System config not found: {}", home_config.display());
    }

    if local_config.exists() {
        println!("    ✓ Local config exists: {}", local_config.display());
    } else {
        eprintln!("    ⚠ Local config not found: {}", local_config.display());
    }

    if config_example.exists() {
        println!("    ✓ Config example exists: {}", config_example.display());
    }

    println!("\nPrerequisites check complete.");
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
    fn test_validate_config_with_s3_section() {
        // Covers get_table for "s3" section
        let content = r#"
[s3]
enabled = true
bucket = "my-bucket"
region = "us-east-1"
archive_interval_days = 7
max_file_size_mb = 100
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_with_s3_archive_naming() {
        // Covers get_table for "s3_archive" naming convention
        let content = r#"
[s3_archive]
enabled = true
bucket = "my-bucket"
region = "us-west-2"
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid log level"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid database driver"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid database URL"));
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

    #[test]
    fn test_validate_config_file_sink_encrypt_without_key_env() {
        let content = r#"
[file]
encrypt = true
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("encryption_key_env"));
    }

    #[test]
    fn test_validate_config_file_sink_encrypt_with_empty_key_env() {
        let content = r#"
[file]
encrypt = true
encryption_key_env = ""
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("encryption_key_env is empty"));
    }

    #[test]
    fn test_validate_config_file_sink_encrypt_with_valid_key_env() {
        let content = r#"
[file]
encrypt = true
encryption_key_env = "LOG_ENCRYPTION_KEY"
"#;
        let file = write_config(content);
        let result = validate_config(&file.path().to_path_buf());
        assert!(result.is_ok());
    }
}
