// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 归档格式示例（Layer 0 零依赖）
//!
//! 演示 `inklog::ArchiveFormat` 枚举的使用：
//!
//! 1. `ArchiveFormat::Json` — JSON Lines 格式归档（默认）
//! 2. `ArchiveFormat::Parquet` — Apache Parquet 列式归档
//! 3. `ArchiveFormat::Csv` — CSV 格式归档
//! 4. FromStr / Display trait 实现
//! 5. 在 DatabaseSinkConfig 中配置归档格式
//! 6. TOML 配置文件格式
//!
//! # 运行
//! ```bash
//! cargo run --bin archive_format
//! ```

use inklog::ArchiveFormat;
use inklog::config::{DatabaseDriver, DatabaseSinkConfig, PartitionStrategy};
use inklog_examples::common::{print_section, print_separator};
use std::str::FromStr;

fn main() {
    print_separator("inklog archive format examples");

    show_archive_format_variants();
    show_from_str_parsing();
    show_display_trait();
    show_database_config_with_archive();
    show_archive_format_comparison();
    show_toml_config();
    show_selection_guide();

    println!("\n✓ all archive format examples completed");
}

/// 展示 ArchiveFormat 枚举变体
fn show_archive_format_variants() {
    print_section("1. ArchiveFormat enum variants");

    let json = ArchiveFormat::Json;
    let parquet = ArchiveFormat::Parquet;
    let csv = ArchiveFormat::Csv;
    let default = ArchiveFormat::default();

    println!("ArchiveFormat::Json    = {:?}", json);
    println!("ArchiveFormat::Parquet = {:?}", parquet);
    println!("ArchiveFormat::Csv     = {:?}", csv);
    println!("ArchiveFormat::default() = {:?}", default);

    assert_eq!(default, ArchiveFormat::Json);
    println!("\nDefault format: {:?} (JSON Lines)", default);
}

/// 展示 FromStr 解析
fn show_from_str_parsing() {
    print_section("2. FromStr string parsing");

    println!("String → ArchiveFormat parsing:\n");

    let cases = [
        "json", "Json", "JSON", "parquet", "Parquet", "PARQUET", "csv", "Csv", "CSV",
    ];
    for s in &cases {
        match ArchiveFormat::from_str(s) {
            Ok(fmt) => println!("  {:?} → {:?} ✓", s, fmt),
            Err(e) => println!("  {:?} → error: {}", s, e),
        }
    }

    assert_eq!(
        ArchiveFormat::from_str("json").unwrap(),
        ArchiveFormat::Json
    );
    assert_eq!(
        ArchiveFormat::from_str("parquet").unwrap(),
        ArchiveFormat::Parquet
    );
    assert_eq!(ArchiveFormat::from_str("csv").unwrap(), ArchiveFormat::Csv);

    let invalid = ArchiveFormat::from_str("xml");
    assert!(invalid.is_err());
    println!(
        "\n  {:?} → error (invalid value): {}",
        "xml",
        invalid.unwrap_err()
    );
}

/// 展示 Display trait
fn show_display_trait() {
    print_section("3. Display trait output");

    println!("ArchiveFormat → string:\n");
    println!("  ArchiveFormat::Json    → \"{}\"", ArchiveFormat::Json);
    println!("  ArchiveFormat::Parquet → \"{}\"", ArchiveFormat::Parquet);
    println!("  ArchiveFormat::Csv     → \"{}\"", ArchiveFormat::Csv);

    assert_eq!(format!("{}", ArchiveFormat::Json), "json");
    assert_eq!(format!("{}", ArchiveFormat::Parquet), "parquet");
    assert_eq!(format!("{}", ArchiveFormat::Csv), "csv");
}

/// 展示在 DatabaseSinkConfig 中使用归档格式
fn show_database_config_with_archive() {
    print_section("4. Archive configuration in DatabaseSinkConfig");

    // JSON 归档（默认）
    let json_archive = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: "sqlite:///tmp/inklog.db".to_string(),
        table_name: "logs".to_string(),
        batch_size: 100,
        flush_interval_ms: 500,
        partition: PartitionStrategy::Monthly,
        archive_format: ArchiveFormat::Json,
        ..Default::default()
    };
    println!("JSON archive (default):");
    println!("  archive_format = {:?}", json_archive.archive_format);
    println!("  Output: .jsonl files (one JSON object per line)");

    // Parquet 归档
    let parquet_archive = DatabaseSinkConfig {
        archive_format: ArchiveFormat::Parquet,
        ..Default::default()
    };
    println!("\nParquet archive:");
    println!("  archive_format = {:?}", parquet_archive.archive_format);
    println!("  Output: .parquet files (columnar storage, high compression)");

    // CSV 归档
    let csv_archive = DatabaseSinkConfig {
        archive_format: ArchiveFormat::Csv,
        ..Default::default()
    };
    println!("\nCSV archive:");
    println!("  archive_format = {:?}", csv_archive.archive_format);
    println!("  Output: .csv files (comma separated, Excel compatible)");
}

/// 对比三种归档格式
fn show_archive_format_comparison() {
    print_section("5. Archive format comparison");

    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "aspect", "Json", "Parquet", "Csv"
    );
    println!("{}", "-".repeat(75));
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "file extension", ".jsonl", ".parquet", ".csv"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "storage type", "row (text)", "columnar (binary)", "row (text)"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "compression ratio", "medium (gzip 3-5x)", "high (5-10x)", "low (gzip 2-3x)"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "query tooling", "grep/jq", "DuckDB/Spark", "Excel/SQL"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "readability", "human readable", "binary", "human readable"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "write cost", "low", "medium-high", "low"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "use case", "log collection", "analytics / cold storage", "reports / Excel"
    );
}

/// 展示 TOML 配置文件格式
fn show_toml_config() {
    print_section("6. TOML configuration file format");

    println!("JSON archive (default):");
    println!(
        r#"[database_sink]
enabled = true
driver = "postgres"
url = "postgres://user:pass@localhost/inklog"
table_name = "logs"
batch_size = 100
flush_interval_ms = 500
archive_format = "json""#
    );

    println!("\nParquet archive (with the Parquet settings):");
    println!(
        r#"[database_sink]
enabled = true
driver = "postgres"
url = "postgres://user:pass@localhost/inklog"
table_name = "logs"
batch_size = 100
flush_interval_ms = 500
archive_format = "parquet"

[database_sink.parquet]
compression_level = 3
max_row_group_size = 100000
max_page_size = 1048576"#
    );

    println!("\nCSV archive:");
    println!(
        r#"[database_sink]
enabled = true
driver = "postgres"
url = "postgres://user:pass@localhost/inklog"
table_name = "logs"
batch_size = 100
flush_interval_ms = 500
archive_format = "csv""#
    );
}

/// 展示归档格式选择指南
fn show_selection_guide() {
    print_section("7. How to choose an archive format");

    println!("When to choose Json:");
    println!("  - You need human-readable archive files");
    println!("  - You query them with grep/jq");
    println!("  - A log shipper consumes them (Fluentd/Logstash)");
    println!("  - Fewer than 1M log records per day");

    println!("\nWhen to choose Parquet:");
    println!("  - A big-data analytics platform (Spark/DuckDB/BigQuery)");
    println!("  - You need a high ratio to save storage cost");
    println!("  - You run columnar queries (aggregate/filter by field)");
    println!("  - More than 1M log records per day");
    println!("  - Compliance retention (long-term cold storage)");

    println!("\nWhen to choose Csv:");
    println!("  - You import it into Excel or a BI tool");
    println!("  - Simple reports and data sharing");
    println!("  - Integrating with legacy systems (CSV parsers are widely supported)");

    println!("\nPerformance characteristics:");
    println!("  - Json: fast writes, large files, slow queries");
    println!("  - Parquet: slow writes (CPU intensive), small files, fast queries");
    println!("  - Csv: fast writes, large files, no nested structures");
    println!(
        "  - Archiving runs asynchronously in the background and does not affect the main log path"
    );
}
