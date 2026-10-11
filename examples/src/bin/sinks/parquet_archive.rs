// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Parquet 归档示例
//!
//! 演示 `inklog::ParquetConfig` 和 `convert_logs_to_parquet()` 的使用：
//!
//! 1. `ParquetConfig` 配置字段（compression_level, max_row_group_size 等）
//! 2. `convert_logs_to_parquet()` 将 LogRecord 转为 Parquet 字节
//! 3. 数据库 Sink 的 Parquet 归档配置
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin parquet_archive --features sqlite,parquet
//! ```

#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
use inklog::LogRecord;
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
use inklog::config::{DatabaseSinkConfig, ParquetConfig};
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
use inklog::sink::database::convert_logs_to_parquet;
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
use inklog::tracing::Level;
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
use inklog_examples::common::{print_section, print_separator};

#[cfg(not(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
)))]
fn main() {
    // convert_logs_to_parquet 的实际实现按 inklog feature = "parquet" 门控，
    // 仅启用数据库 feature 时调用必然失败，因此本示例要求同时启用两者
    eprintln!(
        "this example needs the parquet + database features: cargo run --bin parquet_archive --features sqlite,parquet"
    );
}

#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn main() {
    print_separator("inklog Parquet archive examples");

    show_parquet_config();
    show_convert_logs_to_parquet();
    show_database_sink_with_parquet();
    show_archive_workflow();

    println!("\nAll Parquet archive examples shown.");
}

/// 演示 ParquetConfig 配置字段
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn show_parquet_config() {
    print_section("Example 1: ParquetConfig fields");

    let default = ParquetConfig::default();
    println!("Default ParquetConfig:");
    println!(
        "  compression_level  = {} (Zstandard 0-10)",
        default.compression_level
    );
    println!("  encoding           = \"{}\"", default.encoding);
    println!(
        "  max_row_group_size = {} (rows per row group)",
        default.max_row_group_size
    );
    println!(
        "  max_page_size      = {} (bytes per page)",
        default.max_page_size
    );

    println!("\nField semantics:");
    println!("  compression_level: the Zstandard compression level");
    println!("    0 = none, 3 = balanced (recommended), 10 = maximum (slower)");
    println!("  max_row_group_size: the maximum rows per row group");
    println!("    a smaller value helps selective queries");
    println!("    a larger value gives a better ratio");
    println!("  max_page_size: the maximum bytes per page; controls read memory use");

    println!("\nCustom configuration (high compression):");
    let high_compression = ParquetConfig {
        compression_level: 10,
        encoding: "DELTA_BINARY_PACKED".to_string(),
        max_row_group_size: 50000,
        max_page_size: 2 * 1024 * 1024, // 2MB
        ..Default::default()
    };
    println!(
        "  compression_level  = {} (maximum compression)",
        high_compression.compression_level
    );
    println!("  encoding           = \"{}\"", high_compression.encoding);
    println!(
        "  max_row_group_size = {} (larger row groups)",
        high_compression.max_row_group_size
    );
    println!(
        "  max_page_size      = {} (2MB)",
        high_compression.max_page_size
    );

    println!("\nCustom configuration (low-latency queries):");
    let low_latency = ParquetConfig {
        compression_level: 1,
        encoding: "PLAIN".to_string(),
        max_row_group_size: 5000,
        max_page_size: 512 * 1024, // 512KB
        ..Default::default()
    };
    println!(
        "  compression_level  = {} (fast)",
        low_latency.compression_level
    );
    println!("  encoding           = \"{}\"", low_latency.encoding);
    println!(
        "  max_row_group_size = {} (smaller row groups)",
        low_latency.max_row_group_size
    );
    println!(
        "  max_page_size      = {} (512KB)",
        low_latency.max_page_size
    );
}

/// 演示 convert_logs_to_parquet() 实际调用
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn show_convert_logs_to_parquet() {
    print_section("Example 2: calling convert_logs_to_parquet() for real");

    println!("Create test LogRecord values...");

    let records = vec![
        LogRecord::new(
            Level::INFO,
            "module_a".to_string(),
            "service started".to_string(),
        ),
        LogRecord::new(
            Level::WARN,
            "module_a".to_string(),
            "a config item is missing; using the default".to_string(),
        ),
        LogRecord::new(
            Level::ERROR,
            "module_b".to_string(),
            "database connection failed".to_string(),
        ),
        LogRecord::new(
            Level::INFO,
            "module_c".to_string(),
            "request completed".to_string(),
        ),
    ];
    println!("  ✓ created {} LogRecord values", records.len());

    let config = ParquetConfig::default();
    println!("\nCall convert_logs_to_parquet()...");
    match convert_logs_to_parquet(&records, &config) {
        Ok(parquet_bytes) => {
            println!("  ✓ Parquet conversion succeeded");
            println!("  Input: {} LogRecord values", records.len());
            println!("  Output: {} bytes of Parquet data", parquet_bytes.len());
            println!(
                "  Ratio: {:.2} bytes/record",
                parquet_bytes.len() as f64 / records.len() as f64
            );

            // 验证 Parquet magic number (PAR1)
            if parquet_bytes.len() >= 4 {
                let magic = &parquet_bytes[..4];
                println!(
                    "  Parquet magic: {:?} (expected: [80, 65, 82, 49] = \"PAR1\")",
                    magic
                );
            }
        }
        Err(e) => {
            println!("  ✗ conversion failed: {}", e);
        }
    }

    println!("\nEmpty record test:");
    let empty: Vec<LogRecord> = vec![];
    match convert_logs_to_parquet(&empty, &config) {
        Ok(bytes) => println!(
            "  ✓ empty record conversion succeeded: {} bytes",
            bytes.len()
        ),
        Err(e) => println!("  ✗ empty record conversion failed: {}", e),
    }
}

/// 演示 DatabaseSinkConfig 中的 Parquet 配置
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn show_database_sink_with_parquet() {
    print_section("Example 3: DatabaseSinkConfig Parquet integration");

    let config = DatabaseSinkConfig {
        enabled: true,
        driver: inklog::config::DatabaseDriver::SQLite,
        url: "sqlite:///tmp/inklog_parquet.db".to_string(),
        table_name: "logs".to_string(),
        batch_size: 1000,
        flush_interval_ms: 5000,
        archive_format: inklog::ArchiveFormat::Parquet,
        parquet_config: ParquetConfig {
            compression_level: 6,
            max_row_group_size: 20000,
            ..Default::default()
        },
        ..Default::default()
    };

    println!("A DatabaseSinkConfig with Parquet archiving enabled:");
    println!("  enabled          = {}", config.enabled);
    println!("  driver           = {:?}", config.driver);
    println!("  url              = \"{}\"", config.url);
    println!("  table_name       = \"{}\"", config.table_name);
    println!("  batch_size       = {}", config.batch_size);
    println!("  flush_interval_ms= {}", config.flush_interval_ms);
    println!(
        "  archive_format   = \"{}\" ← enables the Parquet archive",
        config.archive_format
    );
    println!(
        "  parquet_config.compression_level  = {}",
        config.parquet_config.compression_level
    );
    println!(
        "  parquet_config.max_row_group_size = {}",
        config.parquet_config.max_row_group_size
    );

    println!("\nThe corresponding TOML configuration:");
    println!(
        r#"[database_sink]
enabled = true
driver = "sqlite"
url = "sqlite:///tmp/inklog_parquet.db"
table_name = "logs"
batch_size = 1000
flush_interval_ms = 5000
archive_format = "parquet"

[database_sink.parquet]
compression_level = 6
max_row_group_size = 20000"#
    );
}

/// 演示 Parquet 归档工作流
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn show_archive_workflow() {
    print_section("Example 4: the Parquet archive workflow");

    println!("The Parquet archive workflow:\n");

    println!("1. Logs are written into the database");
    println!("   tracing::info!(\"service started\");");
    println!("   → LogRecord → DatabaseSink → INSERT INTO logs");

    println!("\n2. Periodically export to Parquet (in the background)");
    println!("   flush_interval_ms triggers → convert_logs_to_parquet()");
    println!("   → produce the Parquet byte stream");

    println!("\n3. Parquet file storage");
    println!("   → write the local file /tmp/inklog_archive_20260101.parquet");
    println!("   → or upload to object/cold storage (integration is up to you)");

    println!("\n4. Analytical queries (external tools)");
    println!("   # query Parquet with DuckDB");
    println!("   duckdb -c \"SELECT level, COUNT(*) FROM 'archive.parquet' GROUP BY level\"");
    println!();
    println!("   # use Apache Arrow from Python");
    println!("   import pyarrow.parquet as pq");
    println!("   table = pq.read_table('archive.parquet')");

    println!("\nUse cases:");
    println!("  - Analytics: load into Snowflake/BigQuery/Redshift");
    println!("  - Archive: object storage cold tier (integration is up to you)");
    println!("  - Compliance: efficient storage that meets regulatory retention");

    println!("\nPerformance characteristics:");
    println!("  - Asynchronous background export that does not affect the main log path");
    println!("  - Columnar storage with a high ratio (typically 5-10x)");
    println!("  - Predicate pushdown keeps queries efficient");
}
