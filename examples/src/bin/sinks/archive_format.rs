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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::str::FromStr;

fn main() {
    print_separator(&tr("example-archive-format-inklog-archive-format-examples"));

    show_archive_format_variants();
    show_from_str_parsing();
    show_display_trait();
    show_database_config_with_archive();
    show_archive_format_comparison();
    show_toml_config();
    show_selection_guide();

    println!(
        "\n{}",
        tr("example-archive-format-all-archive-format-examples")
    );
}

/// 展示 ArchiveFormat 枚举变体
fn show_archive_format_variants() {
    print_section(&tr("example-archive-format-1-archiveformat-enum-variants"));

    let json = ArchiveFormat::Json;
    let parquet = ArchiveFormat::Parquet;
    let csv = ArchiveFormat::Csv;
    let default = ArchiveFormat::default();

    println!(
        "{}",
        tr_args(
            "example-archive-format-archiveformat-json",
            &[("json", &format!("{:?}", json))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-archive-format-archiveformat-parquet",
            &[("parquet", &format!("{:?}", parquet))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-archive-format-archiveformat-csv",
            &[("csv", &format!("{:?}", csv))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-archive-format-archiveformat-default",
            &[("default", &format!("{:?}", default))]
        )
    );

    assert_eq!(default, ArchiveFormat::Json);
    println!(
        "\n{}",
        tr_args(
            "example-archive-format-default-format-json-lines",
            &[("default", &format!("{:?}", default))]
        )
    );
}

/// 展示 FromStr 解析
fn show_from_str_parsing() {
    print_section(&tr("example-archive-format-2-fromstr-string-parsing"));

    println!(
        "{}\n",
        tr("example-archive-format-string-archiveformat-parsing")
    );

    let cases = [
        "json", "Json", "JSON", "parquet", "Parquet", "PARQUET", "csv", "Csv", "CSV",
    ];
    for s in &cases {
        match ArchiveFormat::from_str(s) {
            Ok(fmt) => println!("  {:?} → {:?} ✓", s, fmt),
            Err(e) => println!(
                "  {}",
                tr_args(
                    "example-archive-format-error",
                    &[("s", &format!("{:?}", s)), ("e", &e)]
                )
            ),
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
        "\n  {}",
        tr_args(
            "example-archive-format-error-invalid-value",
            &[
                ("value", &format!("{:?}", "xml")),
                ("value2", &(invalid.unwrap_err()))
            ]
        )
    );
}

/// 展示 Display trait
fn show_display_trait() {
    print_section(&tr("example-archive-format-3-display-trait-output"));

    println!("{}\n", tr("example-archive-format-archiveformat-string"));
    println!(
        "  {}",
        tr_args(
            "example-archive-format-archiveformat-json-2",
            &[("value", &(ArchiveFormat::Json))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-archive-format-archiveformat-parquet-2",
            &[("value", &(ArchiveFormat::Parquet))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-archive-format-archiveformat-csv-2",
            &[("value", &(ArchiveFormat::Csv))]
        )
    );

    assert_eq!(format!("{}", ArchiveFormat::Json), "json");
    assert_eq!(format!("{}", ArchiveFormat::Parquet), "parquet");
    assert_eq!(format!("{}", ArchiveFormat::Csv), "csv");
}

/// 展示在 DatabaseSinkConfig 中使用归档格式
fn show_database_config_with_archive() {
    print_section(&tr("example-archive-format-4-archive-configuration-in"));

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
    println!("{}", tr("example-archive-format-json-archive-default"));
    println!(
        "  {}",
        tr_args(
            "example-archive-format-archive-format",
            &[(
                "archive_format",
                &format!("{:?}", json_archive.archive_format)
            )]
        )
    );
    println!(
        "  {}",
        tr("example-archive-format-output-jsonl-files-one-json")
    );

    // Parquet 归档
    let parquet_archive = DatabaseSinkConfig {
        archive_format: ArchiveFormat::Parquet,
        ..Default::default()
    };
    println!("\n{}", tr("example-archive-format-parquet-archive"));
    println!(
        "  {}",
        tr_args(
            "example-archive-format-archive-format",
            &[(
                "archive_format",
                &format!("{:?}", parquet_archive.archive_format)
            )]
        )
    );
    println!(
        "  {}",
        tr("example-archive-format-output-parquet-files-columnar")
    );

    // CSV 归档
    let csv_archive = DatabaseSinkConfig {
        archive_format: ArchiveFormat::Csv,
        ..Default::default()
    };
    println!("\n{}", tr("example-archive-format-csv-archive"));
    println!(
        "  {}",
        tr_args(
            "example-archive-format-archive-format",
            &[(
                "archive_format",
                &format!("{:?}", csv_archive.archive_format)
            )]
        )
    );
    println!("  {}", tr("example-archive-format-output-csv-files-comma"));
}

/// 对比三种归档格式
fn show_archive_format_comparison() {
    print_section(&tr("example-archive-format-5-archive-format-comparison"));

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
    print_section(&tr(
        "example-archive-format-6-toml-configuration-file-format",
    ));

    println!("{}", tr("example-archive-format-json-archive-default"));
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

    println!(
        "\n{}",
        tr("example-archive-format-parquet-archive-with-the")
    );
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

    println!("\n{}", tr("example-archive-format-csv-archive"));
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
    print_section(&tr("example-archive-format-7-how-to-choose-an-archive"));

    println!("{}", tr("example-archive-format-when-to-choose-json"));
    println!(
        "  {}",
        tr("example-archive-format-you-need-human-readable-archive")
    );
    println!(
        "  {}",
        tr("example-archive-format-you-query-them-with-grep-jq")
    );
    println!(
        "  {}",
        tr("example-archive-format-a-log-shipper-consumes-them")
    );
    println!(
        "  {}",
        tr("example-archive-format-fewer-than-1m-log-records-per")
    );

    println!("\n{}", tr("example-archive-format-when-to-choose-parquet"));
    println!(
        "  {}",
        tr("example-archive-format-a-big-data-analytics-platform")
    );
    println!(
        "  {}",
        tr("example-archive-format-you-need-a-high-ratio-to-save")
    );
    println!(
        "  {}",
        tr("example-archive-format-you-run-columnar-queries")
    );
    println!(
        "  {}",
        tr("example-archive-format-more-than-1m-log-records-per-day")
    );
    println!(
        "  {}",
        tr("example-archive-format-compliance-retention-long-term")
    );

    println!("\n{}", tr("example-archive-format-when-to-choose-csv"));
    println!(
        "  {}",
        tr("example-archive-format-you-import-it-into-excel-or-a")
    );
    println!(
        "  {}",
        tr("example-archive-format-simple-reports-and-data-sharing")
    );
    println!(
        "  {}",
        tr("example-archive-format-integrating-with-legacy-systems")
    );

    println!(
        "\n{}",
        tr("example-archive-format-performance-characteristics")
    );
    println!(
        "  {}",
        tr("example-archive-format-json-fast-writes-large-files")
    );
    println!("  {}", tr("example-archive-format-parquet-slow-writes-cpu"));
    println!(
        "  {}",
        tr("example-archive-format-csv-fast-writes-large-files-no")
    );
    println!(
        "  - Archiving runs asynchronously in the background and does not affect the main log path"
    );
}
