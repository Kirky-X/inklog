// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! PostgreSQL/MySQL 数据库驱动示例
//!
//! 演示 `inklog::DatabaseDriver` 枚举和 PostgreSQL/MySQL 数据库 Sink 配置：
//!
//! 1. `DatabaseDriver` 三种驱动（PostgreSQL/MySQL/SQLite）对比
//! 2. PostgreSQL 连接配置
//! 3. MySQL 连接配置
//! 4. FromStr / Display trait 实现
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin database_pg_mysql
//! ```

use inklog::config::{DatabaseDriver, DatabaseSinkConfig};
use inklog_examples::common::{print_section, print_separator};
use std::str::FromStr;

fn main() {
    print_separator("inklog PostgreSQL/MySQL database driver example");

    show_driver_variants();
    show_from_str_display();
    show_postgresql_config();
    show_mysql_config();
    show_driver_comparison();
    show_env_overrides();

    println!("\nAll PostgreSQL/MySQL driver examples completed.");
}

/// 演示 DatabaseDriver 三种变体
fn show_driver_variants() {
    print_section("Example 1: the three DatabaseDriver variants");

    let postgres = DatabaseDriver::PostgreSQL;
    let mysql = DatabaseDriver::MySQL;
    let sqlite = DatabaseDriver::SQLite;

    println!("Enum variants:");
    println!(
        "  {:?} - enterprise first choice, the most complete feature set",
        postgres
    );
    println!("  {:?} - a balance of performance and simplicity", mysql);
    println!("  {:?} - embedded, zero configuration", sqlite);

    println!("\nDefault driver:");
    println!(
        "  DatabaseDriver::default() = {:?}",
        DatabaseDriver::default()
    );
}

/// 演示 FromStr 和 Display trait
fn show_from_str_display() {
    print_section("Example 2: FromStr / Display traits");

    println!("FromStr parsing (case-insensitive):");
    let inputs = [
        "postgres",
        "postgresql",
        "PostgreSQL",
        "mysql",
        "MySQL",
        "sqlite",
        "sqlite3",
        "SQLite",
    ];
    for input in inputs {
        match DatabaseDriver::from_str(input) {
            Ok(driver) => println!("  {:>12}.parse() → Ok({:?})", input, driver),
            Err(_) => println!("  {:>12}.parse() → Err", input),
        }
    }

    println!("\nInvalid input:");
    for input in ["", "oracle", "mongodb", "invalid"] {
        let result: Result<DatabaseDriver, String> = input.parse();
        match result {
            Ok(d) => println!("  {:>10}.parse() → Ok({:?})", input, d),
            Err(e) => println!("  {:>10}.parse() → Err(\"{}\")", input, e),
        }
    }

    println!("\nDisplay output:");
    for driver in [
        DatabaseDriver::PostgreSQL,
        DatabaseDriver::MySQL,
        DatabaseDriver::SQLite,
    ] {
        println!("  format!(\"{{}}\", {:?}) → \"{}\"", driver, driver);
    }
}

/// 演示 PostgreSQL 连接配置
fn show_postgresql_config() {
    print_section("Example 3: PostgreSQL connection config");

    let pg_config = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::PostgreSQL,
        url: "postgres://inklog:password@localhost:5432/inklog_logs".to_string(),
        table_name: "logs".to_string(),
        batch_size: 1000,
        flush_interval_ms: 5000,
        pool_size: 10,
        ..Default::default()
    };

    println!("PostgreSQL DatabaseSinkConfig:");
    println!("  driver            = {:?}", pg_config.driver);
    println!("  url               = \"{}\"", pg_config.url);
    println!("  table_name        = \"{}\"", pg_config.table_name);
    println!("  batch_size        = {}", pg_config.batch_size);
    println!("  flush_interval_ms = {}", pg_config.flush_interval_ms);
    println!("  pool_size         = {}", pg_config.pool_size);

    println!("\nConnection URL format:");
    println!("  postgres://USER:PASSWORD@HOST:PORT/DATABASE");
    println!();
    println!("  Example:");
    println!("  postgres://inklog:secret@db.example.com:5432/logs");
    println!("  postgres://postgres@localhost/inklog");
    println!("  postgres://user:pass@10.0.0.1:5432/prod_logs?sslmode=require");

    println!("\nCorresponding TOML config:");
    println!(
        r#"[database_sink]
enabled = true
driver = "postgres"
url = "postgres://inklog:password@localhost:5432/inklog_logs"
table_name = "logs"
batch_size = 1000
flush_interval_ms = 5000
pool_size = 10"#
    );

    println!("\nPostgreSQL strengths:");
    println!("  - Richest features: partitioned tables (PARTITION BY RANGE)");
    println!("  - Great performance: high-concurrency writes, MVCC");
    println!("  - Extensibility: JSONB, full-text search, geospatial data");
    println!("  - Production pick: the first choice for enterprise log storage");

    println!("\nTable partitioning (high log volume):");
    println!(
        r#"# Partition by month
[database_sink.partition]
strategy = "monthly"
# Or partition by year
# strategy = "yearly""#
    );
}

/// 演示 MySQL 连接配置
fn show_mysql_config() {
    print_section("Example 4: MySQL connection config");

    let mysql_config = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::MySQL,
        url: "mysql://inklog:password@localhost:3306/inklog_logs".to_string(),
        table_name: "logs".to_string(),
        batch_size: 500,
        flush_interval_ms: 3000,
        pool_size: 20,
        ..Default::default()
    };

    println!("MySQL DatabaseSinkConfig:");
    println!("  driver            = {:?}", mysql_config.driver);
    println!("  url               = \"{}\"", mysql_config.url);
    println!("  table_name        = \"{}\"", mysql_config.table_name);
    println!("  batch_size        = {}", mysql_config.batch_size);
    println!("  flush_interval_ms = {}", mysql_config.flush_interval_ms);
    println!("  pool_size         = {}", mysql_config.pool_size);

    println!("\nConnection URL format:");
    println!("  mysql://USER:PASSWORD@HOST:PORT/DATABASE");
    println!();
    println!("  Example:");
    println!("  mysql://root@localhost/inklog");
    println!("  mysql://inklog:secret@db.example.com:3306/logs");
    println!("  mysql://user:pass@10.0.0.1:3306/prod_logs?ssl-ca=/path/to/ca.pem");

    println!("\nCorresponding TOML config:");
    println!(
        r#"[database_sink]
enabled = true
driver = "mysql"
url = "mysql://inklog:password@localhost:3306/inklog_logs"
table_name = "logs"
batch_size = 500
flush_interval_ms = 3000
pool_size = 20"#
    );

    println!("\nMySQL strengths:");
    println!("  - Easy deployment: works with default settings");
    println!("  - Mature ecosystem: native fit with LAMP/LEMP stacks");
    println!("  - Balanced performance: even read/write throughput");
    println!("  - Partitioning: PARTITION BY RANGE");
}

/// 演示三种驱动对比
fn show_driver_comparison() {
    print_section("Example 5: three-driver comparison");

    println!(
        "{:<12} {:<15} {:<20} {:<15}",
        "Driver", "Default port", "Use case", "Rating"
    );
    println!("{}", "-".repeat(65));
    println!(
        "{:<12} {:<15} {:<20} {:<15}",
        "PostgreSQL", "5432", "Enterprise", "★★★★★"
    );
    println!(
        "{:<12} {:<15} {:<20} {:<15}",
        "MySQL", "3306", "Web apps", "★★★★"
    );
    println!(
        "{:<12} {:<15} {:<20} {:<15}",
        "SQLite", "none", "Dev/test/embedded", "★★★"
    );

    println!("\nFeature comparison:");
    println!(
        "{:<20} {:<12} {:<12} {:<12}",
        "Feature", "PostgreSQL", "MySQL", "SQLite"
    );
    println!("{}", "-".repeat(56));
    println!("{:<20} {:<12} {:<12} {:<12}", "Partitioning", "✓", "✓", "✗");
    println!("{:<20} {:<12} {:<12} {:<12}", "JSONB", "✓", "✗ (JSON)", "✗");
    println!(
        "{:<20} {:<12} {:<12} {:<12}",
        "Concurrent writes", "High (MVCC)", "Medium", "Low (locks)"
    );
    println!(
        "{:<20} {:<12} {:<12} {:<12}",
        "Connection pool", "Required", "Required", "N/A"
    );
    println!("{:<20} {:<12} {:<12} {:<12}", "Zero-config", "✗", "✗", "✓");

    println!("\nSelection advice:");
    println!("  - Dev/test: SQLite (zero-config, fast startup)");
    println!("  - Small/medium production: MySQL (easy to deploy, balanced performance)");
    println!("  - Large/enterprise: PostgreSQL (richest features, strongest performance)");
}

/// 演示环境变量覆盖
fn show_env_overrides() {
    print_section("Example 6: env-var overrides");

    println!("Database driver env vars:");
    println!("  INKLOG_DATABASE_SINK_DRIVER=postgres   # or mysql / sqlite");
    println!("  INKLOG_DATABASE_SINK_URL=\"postgres://user:pass@host/db\"");

    println!("\nLoading via load_with_env_overrides():");
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
    println!("  if config.database_sink.enabled {{");
    println!("      println!(\"Driver: {{:?}}, URL: {{}}\",");
    println!("          config.database_sink.driver,");
    println!("          config.database_sink.url);");
    println!("  }}");

    println!("\nDocker deployment example:");
    println!("  docker run -e INKLOG_DATABASE_SINK_DRIVER=postgres \\");
    println!("             -e INKLOG_DATABASE_SINK_URL=\"postgres://...\" \\");
    println!("             my-app");

    println!("\nKubernetes ConfigMap example:");
    println!("  env:");
    println!("    - name: INKLOG_DATABASE_SINK_DRIVER");
    println!("      valueFrom:");
    println!("        configMapKeyRef:");
    println!("          name: inklog-config");
    println!("          key: db-driver");
}
