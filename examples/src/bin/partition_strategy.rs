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
    print_separator("inklog 数据库分区策略示例");

    show_partition_strategy_enum();
    show_database_config_with_partition();
    show_partition_comparison();
    show_toml_config();
    show_selection_guide();

    println!("\n所有分区策略示例展示完毕。");
}

/// 演示 PartitionStrategy 枚举值
fn show_partition_strategy_enum() {
    print_section("示例 1：PartitionStrategy 枚举值");

    println!("PartitionStrategy 支持两种分区策略：\n");

    let monthly = PartitionStrategy::Monthly;
    let yearly = PartitionStrategy::Yearly;
    let default = PartitionStrategy::default();

    println!("  Monthly (默认): {}", monthly);
    println!("  Yearly:         {}", yearly);
    println!("  Default:        {}", default);

    println!("\n字符串解析：");
    let from_str_monthly: PartitionStrategy = "monthly".parse().unwrap();
    let from_str_yearly: PartitionStrategy = "yearly".parse().unwrap();
    let from_str_month: PartitionStrategy = "month".parse().unwrap();
    let from_str_year: PartitionStrategy = "year".parse().unwrap();

    println!("  \"monthly\" → {:?}", from_str_monthly);
    println!("  \"yearly\"  → {:?}", from_str_yearly);
    println!("  \"month\"   → {:?} (别名)", from_str_month);
    println!("  \"year\"    → {:?} (别名)", from_str_year);

    println!("\n无效值解析：");
    let invalid: Result<PartitionStrategy, _> = "daily".parse();
    println!("  \"daily\" → {:?} (返回错误)", invalid.err());
}

/// 演示在 DatabaseSinkConfig 中使用分区策略
fn show_database_config_with_partition() {
    print_section("示例 2：DatabaseSinkConfig 中的分区配置");

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
        archive_format: "json".to_string(),
        ..Default::default()
    };

    println!("Monthly 分区配置：");
    println!("  driver    = {:?}", monthly_config.driver);
    println!("  partition = {}", monthly_config.partition);
    println!("  表名模式  = logs_2026_01, logs_2026_02, ...");

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
        archive_format: "json".to_string(),
        ..Default::default()
    };

    println!("\nYearly 分区配置：");
    println!("  driver    = {:?}", yearly_config.driver);
    println!("  partition = {}", yearly_config.partition);
    println!("  表名模式  = logs_2026, logs_2027, ...");
}

/// 对比两种分区策略
fn show_partition_comparison() {
    print_section("示例 3：分区策略对比");

    println!("{:<20} {:<25} {:<25}", "维度", "Monthly", "Yearly");
    println!("{}", "-".repeat(70));
    println!(
        "{:<20} {:<25} {:<25}",
        "表名格式", "logs_2026_01", "logs_2026"
    );
    println!("{:<20} {:<25} {:<25}", "年表数量", "~12/年", "1/年");
    println!("{:<20} {:<25} {:<25}", "单表数据量", "较小", "较大");
    println!("{:<20} {:<25} {:<25}", "查询效率", "高（分区裁剪）", "中");
    println!("{:<20} {:<25} {:<25}", "管理复杂度", "中", "低");
    println!(
        "{:<20} {:<25} {:<25}",
        "适用场景", "高流量/细粒度归档", "低流量/简单管理"
    );
    println!(
        "{:<20} {:<25} {:<25}",
        "推荐驱动", "PostgreSQL/MySQL", "PostgreSQL/MySQL"
    );

    println!("\n默认策略：{}", PartitionStrategy::default());
}

/// 演示 TOML 配置文件格式
fn show_toml_config() {
    print_section("示例 4：TOML 配置文件格式");

    println!("Monthly 分区（默认）：");
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

    println!("\nYearly 分区：");
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

    println!("\n注意：不指定 partition 字段时默认为 \"monthly\"。");
}

/// 演示分区策略选择指南
fn show_selection_guide() {
    print_section("示例 5：分区策略选择指南");

    println!("选择 Monthly 的场景：");
    println!("  - 日均日志量 > 10 万条");
    println!("  - 需要按月归档或清理历史数据");
    println!("  - 查询通常限定在特定月份范围");
    println!("  - 使用 PostgreSQL 分区表或 MySQL 分区");

    println!("\n选择 Yearly 的场景：");
    println!("  - 日均日志量 < 1 万条");
    println!("  - 数据保留周期以年为单位");
    println!("  - 管理简单性优先");
    println!("  - 不需要频繁清理历史数据");

    println!("\n分区表管理建议：");
    println!("  - 定期清理过期分区（DROP PARTITION）");
    println!("  - 监控各分区数据量，避免数据倾斜");
    println!("  - 对高频查询字段建立索引");
    println!("  - 使用 Parquet 归档冷数据（参见 parquet_archive 示例）");
}
