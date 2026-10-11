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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};

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
    print_separator(&tr(
        "example-parquet-archive-inklog-parquet-archive-examples",
    ));

    show_parquet_config();
    show_convert_logs_to_parquet();
    show_database_sink_with_parquet();
    show_archive_workflow();

    println!(
        "\n{}",
        tr("example-parquet-archive-all-parquet-archive-examples")
    );
}

/// 演示 ParquetConfig 配置字段
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn show_parquet_config() {
    print_section(&tr(
        "example-parquet-archive-example-1-parquetconfig-fields",
    ));

    let default = ParquetConfig::default();
    println!("{}", tr("example-parquet-archive-default-parquetconfig"));
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-compression-level-zstandard-0-10",
            &[("compression_level", &default.compression_level)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-encoding",
            &[("encoding", &default.encoding)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-max-row-group-size-rows-per-row",
            &[("max_row_group_size", &default.max_row_group_size)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-max-page-size-bytes-per-page",
            &[("max_page_size", &default.max_page_size)]
        )
    );

    println!("\n{}", tr("example-parquet-archive-field-semantics"));
    println!(
        "  {}",
        tr("example-parquet-archive-compression-level-the-zstandard")
    );
    println!(
        "    {}",
        tr("example-parquet-archive-0-none-3-balanced-recommended")
    );
    println!(
        "  {}",
        tr("example-parquet-archive-max-row-group-size-the-maximum")
    );
    println!(
        "    {}",
        tr("example-parquet-archive-a-smaller-value-helps-selective")
    );
    println!(
        "    {}",
        tr("example-parquet-archive-a-larger-value-gives-a-better")
    );
    println!(
        "  {}",
        tr("example-parquet-archive-max-page-size-the-maximum-bytes")
    );

    println!(
        "\n{}",
        tr("example-parquet-archive-custom-configuration-high")
    );
    let high_compression = ParquetConfig {
        compression_level: 10,
        encoding: "DELTA_BINARY_PACKED".to_string(),
        max_row_group_size: 50000,
        max_page_size: 2 * 1024 * 1024, // 2MB
        ..Default::default()
    };
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-compression-level-maximum",
            &[("compression_level", &high_compression.compression_level)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-encoding",
            &[("encoding", &high_compression.encoding)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-max-row-group-size-larger-row",
            &[("max_row_group_size", &high_compression.max_row_group_size)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-max-page-size-2mb",
            &[("max_page_size", &high_compression.max_page_size)]
        )
    );

    println!(
        "\n{}",
        tr("example-parquet-archive-custom-configuration-low")
    );
    let low_latency = ParquetConfig {
        compression_level: 1,
        encoding: "PLAIN".to_string(),
        max_row_group_size: 5000,
        max_page_size: 512 * 1024, // 512KB
        ..Default::default()
    };
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-compression-level-fast",
            &[("compression_level", &low_latency.compression_level)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-encoding",
            &[("encoding", &low_latency.encoding)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-max-row-group-size-smaller-row",
            &[("max_row_group_size", &low_latency.max_row_group_size)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-max-page-size-512kb",
            &[("max_page_size", &low_latency.max_page_size)]
        )
    );
}

/// 演示 convert_logs_to_parquet() 实际调用
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn show_convert_logs_to_parquet() {
    print_section(&tr(
        "example-parquet-archive-example-2-calling-convert-logs",
    ));

    println!(
        "{}",
        tr("example-parquet-archive-create-test-logrecord-values")
    );

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
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-created-logrecord-values",
            &[("value", &(records.len()))]
        )
    );

    let config = ParquetConfig::default();
    println!(
        "\n{}",
        tr("example-parquet-archive-call-convert-logs-to-parquet")
    );
    match convert_logs_to_parquet(&records, &config) {
        Ok(parquet_bytes) => {
            println!(
                "  {}",
                tr("example-parquet-archive-parquet-conversion-succeeded")
            );
            println!(
                "  {}",
                tr_args(
                    "example-parquet-archive-input-logrecord-values",
                    &[("value", &(records.len()))]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-parquet-archive-output-bytes-of-parquet-data",
                    &[("value", &(parquet_bytes.len()))]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-parquet-archive-ratio-bytes-record",
                    &[(
                        "value",
                        &format!("{:.2}", parquet_bytes.len() as f64 / records.len() as f64)
                    )]
                )
            );

            // 验证 Parquet magic number (PAR1)
            if parquet_bytes.len() >= 4 {
                let magic = &parquet_bytes[..4];
                println!(
                    "  {}",
                    tr_args(
                        "example-parquet-archive-parquet-magic-expected-80-65-82",
                        &[("magic", &format!("{:?}", magic))]
                    )
                );
            }
        }
        Err(e) => {
            println!(
                "  {}",
                tr_args("example-parquet-archive-conversion-failed", &[("e", &e)])
            );
        }
    }

    println!("\n{}", tr("example-parquet-archive-empty-record-test"));
    let empty: Vec<LogRecord> = vec![];
    match convert_logs_to_parquet(&empty, &config) {
        Ok(bytes) => println!(
            "  {}",
            tr_args(
                "example-parquet-archive-empty-record-conversion",
                &[("value", &(bytes.len()))]
            )
        ),
        Err(e) => println!(
            "  {}",
            tr_args(
                "example-parquet-archive-empty-record-conversion-failed",
                &[("e", &e)]
            )
        ),
    }
}

/// 演示 DatabaseSinkConfig 中的 Parquet 配置
#[cfg(all(
    feature = "parquet",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn show_database_sink_with_parquet() {
    print_section(&tr("example-parquet-archive-example-3-databasesinkconfig"));

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

    println!(
        "{}",
        tr("example-parquet-archive-a-databasesinkconfig-with")
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-enabled",
            &[("enabled", &config.enabled)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-driver",
            &[("driver", &format!("{:?}", config.driver))]
        )
    );
    println!(
        "  {}",
        tr_args("example-parquet-archive-url", &[("url", &config.url)])
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-table-name",
            &[("table_name", &config.table_name)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-batch-size",
            &[("batch_size", &config.batch_size)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-flush-interval-ms",
            &[("flush_interval_ms", &config.flush_interval_ms)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-archive-format-enables-the",
            &[("archive_format", &config.archive_format)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-parquet-config-compression-level",
            &[(
                "compression_level",
                &config.parquet_config.compression_level
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-parquet-archive-parquet-config-max-row-group",
            &[(
                "max_row_group_size",
                &config.parquet_config.max_row_group_size
            )]
        )
    );

    println!("\n{}", tr("example-parquet-archive-the-corresponding-toml"));
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
    print_section(&tr("example-parquet-archive-example-4-the-parquet-archive"));

    println!(
        "{}\n",
        tr("example-parquet-archive-the-parquet-archive-workflow")
    );

    println!(
        "{}",
        tr("example-parquet-archive-1-logs-are-written-into-the")
    );
    println!("   tracing::info!(\"service started\");");
    println!(
        "   {}",
        tr("example-parquet-archive-logrecord-databasesink-insert")
    );

    println!(
        "\n{}",
        tr("example-parquet-archive-2-periodically-export-to")
    );
    println!(
        "   {}",
        tr("example-parquet-archive-flush-interval-ms-triggers")
    );
    println!(
        "   {}",
        tr("example-parquet-archive-produce-the-parquet-byte-stream")
    );

    println!("\n{}", tr("example-parquet-archive-3-parquet-file-storage"));
    println!(
        "   {}",
        tr("example-parquet-archive-write-the-local-file-tmp-inklog")
    );
    println!(
        "   {}",
        tr("example-parquet-archive-or-upload-to-object-cold")
    );

    println!(
        "\n{}",
        tr("example-parquet-archive-4-analytical-queries-external")
    );
    println!(
        "   {}",
        tr("example-parquet-archive-query-parquet-with-duckdb")
    );
    println!(
        "   {}",
        tr("example-parquet-archive-duckdb-c-select-level-count")
    );
    println!();
    println!(
        "   {}",
        tr("example-parquet-archive-use-apache-arrow-from-python")
    );
    println!(
        "   {}",
        tr("example-parquet-archive-import-pyarrow-parquet-as-pq")
    );
    println!(
        "   {}",
        tr("example-parquet-archive-table-pq-read-table-archive")
    );

    println!("\n{}", tr("example-parquet-archive-use-cases"));
    println!(
        "  {}",
        tr("example-parquet-archive-analytics-load-into-snowflake")
    );
    println!(
        "  {}",
        tr("example-parquet-archive-archive-object-storage-cold")
    );
    println!(
        "  {}",
        tr("example-parquet-archive-compliance-efficient-storage")
    );

    println!(
        "\n{}",
        tr("example-parquet-archive-performance-characteristics")
    );
    println!(
        "  {}",
        tr("example-parquet-archive-asynchronous-background-export")
    );
    println!(
        "  {}",
        tr("example-parquet-archive-columnar-storage-with-a-high")
    );
    println!(
        "  {}",
        tr("example-parquet-archive-predicate-pushdown-keeps")
    );
}
