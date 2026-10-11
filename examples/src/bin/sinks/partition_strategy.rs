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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};

fn main() {
    print_separator(&tr("example-partition-strategy-inklog-database-partition"));

    show_partition_strategy_enum();
    show_database_config_with_partition();
    show_partition_comparison();
    show_toml_config();
    show_selection_guide();

    println!(
        "\n{}",
        tr("example-partition-strategy-all-partition-strategy-examples")
    );
}

/// 演示 PartitionStrategy 枚举值
fn show_partition_strategy_enum() {
    print_section(&tr(
        "example-partition-strategy-example-1-partitionstrategy",
    ));

    println!(
        "{}\n",
        tr("example-partition-strategy-partitionstrategy-supports-two")
    );

    let monthly = PartitionStrategy::Monthly;
    let yearly = PartitionStrategy::Yearly;
    let default = PartitionStrategy::default();

    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-monthly-default",
            &[("monthly", &monthly)]
        )
    );
    println!(
        "  {}",
        tr_args("example-partition-strategy-yearly", &[("yearly", &yearly)])
    );
    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-default",
            &[("default", &default)]
        )
    );

    println!("\n{}", tr("example-partition-strategy-string-parsing"));
    let from_str_monthly: PartitionStrategy = "monthly".parse().unwrap();
    let from_str_yearly: PartitionStrategy = "yearly".parse().unwrap();
    let from_str_month: PartitionStrategy = "month".parse().unwrap();
    let from_str_year: PartitionStrategy = "year".parse().unwrap();

    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-monthly",
            &[("from_str_monthly", &format!("{:?}", from_str_monthly))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-yearly-2",
            &[("from_str_yearly", &format!("{:?}", from_str_yearly))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-month-alias",
            &[("from_str_month", &format!("{:?}", from_str_month))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-year-alias",
            &[("from_str_year", &format!("{:?}", from_str_year))]
        )
    );

    println!(
        "\n{}",
        tr("example-partition-strategy-invalid-value-parsing")
    );
    let invalid: Result<PartitionStrategy, _> = "daily".parse();
    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-daily-returns-an-error",
            &[("value", &format!("{:?}", invalid.err()))]
        )
    );
}

/// 演示在 DatabaseSinkConfig 中使用分区策略
fn show_database_config_with_partition() {
    print_section(&tr("example-partition-strategy-example-2-partition"));

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

    println!(
        "{}",
        tr("example-partition-strategy-monthly-partition-configuration")
    );
    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-driver",
            &[("driver", &format!("{:?}", monthly_config.driver))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-partition",
            &[("partition", &monthly_config.partition)]
        )
    );
    println!(
        "  {}",
        tr("example-partition-strategy-table-name-pattern-logs-2026-01")
    );

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

    println!(
        "\n{}",
        tr("example-partition-strategy-yearly-partition-configuration")
    );
    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-driver",
            &[("driver", &format!("{:?}", yearly_config.driver))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-partition-strategy-partition",
            &[("partition", &yearly_config.partition)]
        )
    );
    println!(
        "  {}",
        tr("example-partition-strategy-table-name-pattern-logs-2026")
    );
}

/// 对比两种分区策略
fn show_partition_comparison() {
    print_section(&tr(
        "example-partition-strategy-example-3-partition-strategy",
    ));

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

    println!(
        "\n{}",
        tr_args(
            "example-partition-strategy-default-strategy",
            &[("value", &(PartitionStrategy::default()))]
        )
    );
}

/// 演示 TOML 配置文件格式
fn show_toml_config() {
    print_section(&tr(
        "example-partition-strategy-example-4-toml-configuration",
    ));

    println!(
        "{}",
        tr("example-partition-strategy-monthly-partition-default")
    );
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

    println!("\n{}", tr("example-partition-strategy-yearly-partition"));
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

    println!(
        "\n{}",
        tr("example-partition-strategy-note-when-the-partition-field")
    );
}

/// 演示分区策略选择指南
fn show_selection_guide() {
    print_section(&tr("example-partition-strategy-example-5-how-to-choose-a"));

    println!(
        "{}",
        tr("example-partition-strategy-when-to-choose-monthly")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-more-than-100k-log-records-per")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-you-archive-or-purge-history-by")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-queries-are-usually-limited-to")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-you-use-postgresql-partitioned")
    );

    println!(
        "\n{}",
        tr("example-partition-strategy-when-to-choose-yearly")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-fewer-than-10k-log-records-per")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-retention-is-measured-in-years")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-management-simplicity-comes")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-you-do-not-purge-history")
    );

    println!(
        "\n{}",
        tr("example-partition-strategy-partitioned-table-management")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-drop-expired-partitions")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-watch-per-partition-sizes-to")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-index-the-fields-that-queries")
    );
    println!(
        "  {}",
        tr("example-partition-strategy-archive-cold-data-as-parquet")
    );
}
