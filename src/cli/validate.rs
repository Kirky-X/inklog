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

    if let Some(console) = config.get("console_sink").and_then(|t| t.as_table()) {
        validate_console_sink(console)?;
    }

    if let Some(file) = config.get("file_sink").and_then(|t| t.as_table()) {
        validate_file_sink(file)?;
    }

    if let Some(perf) = config.get("performance").and_then(|t| t.as_table()) {
        validate_performance(perf)?;
    }

    if let Some(db) = config.get("database_sink").and_then(|t| t.as_table()) {
        validate_database_sink(db)?;
    }

    if let Some(http) = config.get("http_server").and_then(|t| t.as_table()) {
        validate_http_server(http)?;
    }

    if let Some(s3) = config.get("s3_archive").and_then(|t| t.as_table()) {
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
            return Err(anyhow::anyhow!("database_sink.enabled must be a boolean"));
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
                return Err(anyhow::anyhow!("database_sink.url cannot be empty"));
            }
            validate_database_url(url_str)?;
            println!("  ✓ Database URL: {} bytes", url_str.len());
        }
    }

    if let Some(pool_size) = db.get("pool_size") {
        if let Some(n) = pool_size.as_integer() {
            if !(1..=100).contains(&n) {
                return Err(anyhow::anyhow!(
                    "database_sink.pool_size must be between 1 and 100"
                ));
            }
        }
    }

    if let Some(batch_size) = db.get("batch_size") {
        if let Some(n) = batch_size.as_integer() {
            if n < 1 {
                return Err(anyhow::anyhow!("database_sink.batch_size must be >= 1"));
            }
        }
    }

    if let Some(table_name) = db.get("table_name") {
        if let Some(name) = table_name.as_str() {
            if name.is_empty() {
                return Err(anyhow::anyhow!("database_sink.table_name cannot be empty"));
            }
            if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(anyhow::anyhow!(
                    "database_sink.table_name must contain only alphanumeric characters and underscores"
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
    let valid_sections = [
        "global",
        "console_sink",
        "file_sink",
        "database_sink",
        "s3_archive",
        "performance",
        "http_server",
    ];

    for key in config.keys() {
        if !valid_sections.contains(&key.as_str()) {
            eprintln!("  ⚠ Unknown configuration section: [{}]", key);
        }
    }

    if config.contains_key("file_sink") && config.contains_key("database_sink") {
        if let Some(file) = config.get("file_sink").and_then(|t| t.as_table()) {
            if let Some(enabled) = file.get("enabled").and_then(|v| v.as_bool()) {
                if !enabled {
                    eprintln!("  ⚠ Both file_sink and database_sink enabled - logs will be written to both");
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
