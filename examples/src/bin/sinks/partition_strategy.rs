// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 数据库分区策略示例
//!
//! 演示 `inklog::PartitionStrategy` 的配置和使用：
//!
//! 1. `PartitionStrategy` 枚举值（Monthly / Yearly）
//! 2. 在 `DatabaseSinkConfig` 中配置分区策略
//! 3. 分区策略的选择建议
//! 4. TOML 配置文件格式
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin partition_strategy
//! ```

use inklog::config::{DatabaseDriver, DatabaseSinkConfig, PartitionStrategy};
use inklog_examples::common::{print_section, print_separator};

fn main() {
    print_separator("inklog database partition strategy examples");

    show_partition_strategy_enum();
    show_database_config_with_partition();
    show_partition_comparison();
    show_toml_config();
    show_selection_guide();

    println!("\nAll partition strategy examples shown.");
}

/// 演示 PartitionStrategy 枚举值
fn show_partition_strategy_enum() {
    print_section("Example 1: PartitionStrategy enum values");

    println!("PartitionStrategy supports two partition strategies:\n");

    let monthly = PartitionStrategy::Monthly;
    let yearly = PartitionStrategy::Yearly;
    let default = PartitionStrategy::default();

    println!("  Monthly (default): {}", monthly);
    println!("  Yearly:         {}", yearly);
    println!("  Default:        {}", default);

    println!("\nString parsing:");
    let from_str_monthly: PartitionStrategy = "monthly".parse().unwrap();
    let from_str_yearly: PartitionStrategy = "yearly".parse().unwrap();
    let from_str_month: PartitionStrategy = "month".parse().unwrap();
    let from_str_year: PartitionStrategy = "year".parse().unwrap();

    println!("  \"monthly\" → {:?}", from_str_monthly);
    println!("  \"yearly\"  → {:?}", from_str_yearly);
    println!("  \"month\"   → {:?} (alias)", from_str_month);
    println!("  \"year\"    → {:?} (alias)", from_str_year);

    println!("\nInvalid value parsing:");
    let invalid: Result<PartitionStrategy, _> = "daily".parse();
    println!("  \"daily\" → {:?} (returns an error)", invalid.err());
}

/// 演示在 DatabaseSinkConfig 中使用分区策略
fn show_database_config_with_partition() {
    print_section("Example 2: partition configuration in DatabaseSinkConfig");

    // Monthly 分区配置
    let monthly_config = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::PostgreSQL,
        url: "postgres://user:pass@localhost/inklog".to_string(),
        pool_size: 10,
        batch_size: 100,
        flush_interval_ms: 500,
        partition: PartitionStrategy::Monthly,
        table_name: "logs".to_string(),
        archive_format: inklog::ArchiveFormat::default(),
        ..Default::default()
    };

    println!("Monthly partition configuration:");
    println!("  driver    = {:?}", monthly_config.driver);
    println!("  partition = {}", monthly_config.partition);
    println!("  Table name pattern = logs_2026_01, logs_2026_02, ...");

    // Yearly 分区配置
    let yearly_config = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::PostgreSQL,
        url: "postgres://user:pass@localhost/inklog".to_string(),
        pool_size: 10,
        batch_size: 100,
        flush_interval_ms: 500,
        partition: PartitionStrategy::Yearly,
        table_name: "logs".to_string(),
        archive_format: inklog::ArchiveFormat::default(),
        ..Default::default()
    };

    println!("\nYearly partition configuration:");
    println!("  driver    = {:?}", yearly_config.driver);
    println!("  partition = {}", yearly_config.partition);
    println!("  Table name pattern = logs_2026, logs_2027, ...");
}

/// 对比两种分区策略
fn show_partition_comparison() {
    print_section("Example 3: partition strategy comparison");

    println!("{:<20} {:<25} {:<25}", "aspect", "Monthly", "Yearly");
    println!("{}", "-".repeat(70));
    println!(
        "{:<20} {:<25} {:<25}",
        "table name format", "logs_2026_01", "logs_2026"
    );
    println!(
        "{:<20} {:<25} {:<25}",
        "tables per year", "~12/year", "1/year"
    );
    println!(
        "{:<20} {:<25} {:<25}",
        "rows per table", "smaller", "larger"
    );
    println!(
        "{:<20} {:<25} {:<25}",
        "query efficiency", "high (partition pruning)", "medium"
    );
    println!(
        "{:<20} {:<25} {:<25}",
        "management complexity", "medium", "low"
    );
    println!(
        "{:<20} {:<25} {:<25}",
        "use case", "high throughput / fine-grained archive", "low throughput / simple management"
    );
    println!(
        "{:<20} {:<25} {:<25}",
        "recommended driver", "PostgreSQL/MySQL", "PostgreSQL/MySQL"
    );

    println!("\nDefault strategy: {}", PartitionStrategy::default());
}

/// 演示 TOML 配置文件格式
fn show_toml_config() {
    print_section("Example 4: TOML configuration file format");

    println!("Monthly partition (default):");
    println!(
        r#"[database_sink]
enabled = true
driver = "postgres"
url = "postgres://user:pass@localhost/inklog"
table_name = "logs"
batch_size = 100
flush_interval_ms = 500
partition = "monthly""#
    );

    println!("\nYearly partition:");
    println!(
        r#"[database_sink]
enabled = true
driver = "postgres"
url = "postgres://user:pass@localhost/inklog"
table_name = "logs"
batch_size = 100
flush_interval_ms = 500
partition = "yearly""#
    );

    println!("\nNote: when the partition field is omitted it defaults to \"monthly\".");
}

/// 演示分区策略选择指南
fn show_selection_guide() {
    print_section("Example 5: how to choose a partition strategy");

    println!("When to choose Monthly:");
    println!("  - More than 100k log records per day");
    println!("  - You archive or purge history by month");
    println!("  - Queries are usually limited to specific months");
    println!("  - You use PostgreSQL partitioned tables or MySQL partitions");

    println!("\nWhen to choose Yearly:");
    println!("  - Fewer than 10k log records per day");
    println!("  - Retention is measured in years");
    println!("  - Management simplicity comes first");
    println!("  - You do not purge history frequently");

    println!("\nPartitioned table management advice:");
    println!("  - Drop expired partitions periodically (DROP PARTITION)");
    println!("  - Watch per-partition sizes to avoid data skew");
    println!("  - Index the fields that queries hit most");
    println!("  - Archive cold data as Parquet (see the parquet_archive example)");
}
