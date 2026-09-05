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
    print_separator("inklog 归档格式示例");

    show_archive_format_variants();
    show_from_str_parsing();
    show_display_trait();
    show_database_config_with_archive();
    show_archive_format_comparison();
    show_toml_config();
    show_selection_guide();

    println!("\n✓ 所有归档格式示例演示完成");
}

/// 展示 ArchiveFormat 枚举变体
fn show_archive_format_variants() {
    print_section("1. ArchiveFormat 枚举变体");

    let json = ArchiveFormat::Json;
    let parquet = ArchiveFormat::Parquet;
    let csv = ArchiveFormat::Csv;
    let default = ArchiveFormat::default();

    println!("ArchiveFormat::Json    = {:?}", json);
    println!("ArchiveFormat::Parquet = {:?}", parquet);
    println!("ArchiveFormat::Csv     = {:?}", csv);
    println!("ArchiveFormat::default() = {:?}", default);

    assert_eq!(default, ArchiveFormat::Json);
    println!("\n默认格式: {:?}（JSON Lines）", default);
}

/// 展示 FromStr 解析
fn show_from_str_parsing() {
    print_section("2. FromStr 字符串解析");

    println!("字符串 → ArchiveFormat 解析：\n");

    let cases = [
        "json", "Json", "JSON", "parquet", "Parquet", "PARQUET", "csv", "Csv", "CSV",
    ];
    for s in &cases {
        match ArchiveFormat::from_str(s) {
            Ok(fmt) => println!("  {:?} → {:?} ✓", s, fmt),
            Err(e) => println!("  {:?} → 错误: {}", s, e),
        }
    }

    // 验证解析结果
    assert_eq!(
        ArchiveFormat::from_str("json").unwrap(),
        ArchiveFormat::Json
    );
    assert_eq!(
        ArchiveFormat::from_str("parquet").unwrap(),
        ArchiveFormat::Parquet
    );
    assert_eq!(ArchiveFormat::from_str("csv").unwrap(), ArchiveFormat::Csv);

    // 无效值
    let invalid = ArchiveFormat::from_str("xml");
    assert!(invalid.is_err());
    println!("\n  {:?} → 错误（无效值）: {}", "xml", invalid.unwrap_err());
}

/// 展示 Display trait
fn show_display_trait() {
    print_section("3. Display trait 输出");

    println!("ArchiveFormat → 字符串：\n");
    println!("  ArchiveFormat::Json    → \"{}\"", ArchiveFormat::Json);
    println!("  ArchiveFormat::Parquet → \"{}\"", ArchiveFormat::Parquet);
    println!("  ArchiveFormat::Csv     → \"{}\"", ArchiveFormat::Csv);

    assert_eq!(format!("{}", ArchiveFormat::Json), "json");
    assert_eq!(format!("{}", ArchiveFormat::Parquet), "parquet");
    assert_eq!(format!("{}", ArchiveFormat::Csv), "csv");
}

/// 展示在 DatabaseSinkConfig 中使用归档格式
fn show_database_config_with_archive() {
    print_section("4. DatabaseSinkConfig 中的归档配置");

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
    println!("JSON 归档（默认）：");
    println!("  archive_format = {:?}", json_archive.archive_format);
    println!("  输出: .jsonl 文件（每行一个 JSON 对象）");

    // Parquet 归档
    let parquet_archive = DatabaseSinkConfig {
        archive_format: ArchiveFormat::Parquet,
        ..Default::default()
    };
    println!("\nParquet 归档：");
    println!("  archive_format = {:?}", parquet_archive.archive_format);
    println!("  输出: .parquet 文件（列式存储，高压缩比）");

    // CSV 归档
    let csv_archive = DatabaseSinkConfig {
        archive_format: ArchiveFormat::Csv,
        ..Default::default()
    };
    println!("\nCSV 归档：");
    println!("  archive_format = {:?}", csv_archive.archive_format);
    println!("  输出: .csv 文件（逗号分隔，Excel 兼容）");
}

/// 对比三种归档格式
fn show_archive_format_comparison() {
    print_section("5. 归档格式对比");

    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "维度", "Json", "Parquet", "Csv"
    );
    println!("{}", "-".repeat(75));
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "文件扩展名", ".jsonl", ".parquet", ".csv"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "存储类型", "行式（文本）", "列式（二进制）", "行式（文本）"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "压缩比", "中（gzip 3-5x）", "高（5-10x）", "低（gzip 2-3x）"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "查询能力", "grep/jq", "DuckDB/Spark", "Excel/SQL"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "可读性", "人类可读", "二进制", "人类可读"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "写入开销", "低", "中-高", "低"
    );
    println!(
        "{:<15} {:<20} {:<20} {:<20}",
        "适用场景", "日志采集", "分析/冷存储", "报表/Excel"
    );
}

/// 展示 TOML 配置文件格式
fn show_toml_config() {
    print_section("6. TOML 配置文件格式");

    println!("JSON 归档（默认）：");
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

    println!("\nParquet 归档（含 Parquet 配置）：");
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

    println!("\nCSV 归档：");
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
    print_section("7. 归档格式选择指南");

    println!("选择 Json 的场景：");
    println!("  - 需要人类可读的归档文件");
    println!("  - 使用 grep/jq 进行简单查询");
    println!("  - 日志采集系统（Fluentd/Logstash）");
    println!("  - 日均日志量 < 100 万条");

    println!("\n选择 Parquet 的场景：");
    println!("  - 大数据分析平台（Spark/DuckDB/BigQuery）");
    println!("  - 需要高压缩比节省存储成本");
    println!("  - 列式查询（按字段聚合/过滤）");
    println!("  - 日均日志量 > 100 万条");
    println!("  - 合规留存（长期冷存储）");

    println!("\n选择 Csv 的场景：");
    println!("  - 需要导入 Excel 或 BI 工具");
    println!("  - 简单报表和数据分享");
    println!("  - 与旧系统集成（CSV 解析器广泛支持）");

    println!("\n性能特点：");
    println!("  - Json: 写入快，文件大，查询慢");
    println!("  - Parquet: 写入慢（CPU密集），文件小，查询快");
    println!("  - Csv: 写入快，文件大，无嵌套结构支持");
    println!("  - 归档在后台异步执行，不影响主日志路径");
}
