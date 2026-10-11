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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::str::FromStr;

fn main() {
    print_separator(&tr("example-database-pg-mysql-inklog-postgresql-mysql"));

    show_driver_variants();
    show_from_str_display();
    show_postgresql_config();
    show_mysql_config();
    show_driver_comparison();
    show_env_overrides();

    println!(
        "\n{}",
        tr("example-database-pg-mysql-all-postgresql-mysql-driver")
    );
}

/// 演示 DatabaseDriver 三种变体
fn show_driver_variants() {
    print_section(&tr("example-database-pg-mysql-example-1-the-three"));

    let postgres = DatabaseDriver::PostgreSQL;
    let mysql = DatabaseDriver::MySQL;
    let sqlite = DatabaseDriver::SQLite;

    println!("{}", tr("example-database-pg-mysql-enum-variants"));
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-enterprise-first-choice-the",
            &[("postgres", &format!("{:?}", postgres))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-a-balance-of-performance-and",
            &[("mysql", &format!("{:?}", mysql))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-embedded-zero-configuration",
            &[("sqlite", &format!("{:?}", sqlite))]
        )
    );

    println!("\n{}", tr("example-database-pg-mysql-default-driver"));
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-databasedriver-default",
            &[("value", &format!("{:?}", DatabaseDriver::default()))]
        )
    );
}

/// 演示 FromStr 和 Display trait
fn show_from_str_display() {
    print_section(&tr(
        "example-database-pg-mysql-example-2-fromstr-display-traits",
    ));

    println!(
        "{}",
        tr("example-database-pg-mysql-fromstr-parsing-case-insensitive")
    );
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
            Ok(driver) => println!(
                "  {}",
                tr_args(
                    "example-database-pg-mysql-parse-ok",
                    &[
                        ("input", &format!("{:>12}", input)),
                        ("driver", &format!("{:?}", driver))
                    ]
                )
            ),
            Err(_) => println!(
                "  {}",
                tr_args(
                    "example-database-pg-mysql-parse-err",
                    &[("input", &format!("{:>12}", input))]
                )
            ),
        }
    }

    println!("\n{}", tr("example-database-pg-mysql-invalid-input"));
    for input in ["", "oracle", "mongodb", "invalid"] {
        let result: Result<DatabaseDriver, String> = input.parse();
        match result {
            Ok(d) => println!(
                "  {}",
                tr_args(
                    "example-database-pg-mysql-parse-ok-2",
                    &[
                        ("input", &format!("{:>10}", input)),
                        ("d", &format!("{:?}", d))
                    ]
                )
            ),
            Err(e) => println!(
                "  {}",
                tr_args(
                    "example-database-pg-mysql-parse-err-2",
                    &[("input", &format!("{:>10}", input)), ("e", &e)]
                )
            ),
        }
    }

    println!("\n{}", tr("example-database-pg-mysql-display-output"));
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
    print_section(&tr(
        "example-database-pg-mysql-example-3-postgresql-connection",
    ));

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

    println!(
        "{}",
        tr("example-database-pg-mysql-postgresql-databasesinkconfig")
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-driver",
            &[("driver", &format!("{:?}", pg_config.driver))]
        )
    );
    println!(
        "  {}",
        tr_args("example-database-pg-mysql-url", &[("url", &pg_config.url)])
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-table-name",
            &[("table_name", &pg_config.table_name)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-batch-size",
            &[("batch_size", &pg_config.batch_size)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-flush-interval-ms",
            &[("flush_interval_ms", &pg_config.flush_interval_ms)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-pool-size",
            &[("pool_size", &pg_config.pool_size)]
        )
    );

    println!(
        "\n{}",
        tr("example-database-pg-mysql-connection-url-format")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-postgres-user-password-host")
    );
    println!();
    println!("  {}", tr("example-database-pg-mysql-example"));
    println!(
        "  {}",
        tr("example-database-pg-mysql-postgres-inklog-secret-db")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-postgres-postgres-localhost")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-postgres-user-pass-10-0-0-1")
    );

    println!(
        "\n{}",
        tr("example-database-pg-mysql-corresponding-toml-config")
    );
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

    println!("\n{}", tr("example-database-pg-mysql-postgresql-strengths"));
    println!(
        "  {}",
        tr("example-database-pg-mysql-richest-features-partitioned")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-great-performance-high")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-extensibility-jsonb-full-text")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-production-pick-the-first")
    );

    println!(
        "\n{}",
        tr("example-database-pg-mysql-table-partitioning-high-log")
    );
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
    print_section(&tr("example-database-pg-mysql-example-4-mysql-connection"));

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

    println!(
        "{}",
        tr("example-database-pg-mysql-mysql-databasesinkconfig")
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-driver",
            &[("driver", &format!("{:?}", mysql_config.driver))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-url",
            &[("url", &mysql_config.url)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-table-name",
            &[("table_name", &mysql_config.table_name)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-batch-size",
            &[("batch_size", &mysql_config.batch_size)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-flush-interval-ms",
            &[("flush_interval_ms", &mysql_config.flush_interval_ms)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-pg-mysql-pool-size",
            &[("pool_size", &mysql_config.pool_size)]
        )
    );

    println!(
        "\n{}",
        tr("example-database-pg-mysql-connection-url-format")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-mysql-user-password-host-port")
    );
    println!();
    println!("  {}", tr("example-database-pg-mysql-example"));
    println!(
        "  {}",
        tr("example-database-pg-mysql-mysql-root-localhost-inklog")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-mysql-inklog-secret-db-example")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-mysql-user-pass-10-0-0-1-3306")
    );

    println!(
        "\n{}",
        tr("example-database-pg-mysql-corresponding-toml-config")
    );
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

    println!("\n{}", tr("example-database-pg-mysql-mysql-strengths"));
    println!(
        "  {}",
        tr("example-database-pg-mysql-easy-deployment-works-with")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-mature-ecosystem-native-fit")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-balanced-performance-even-read")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-partitioning-partition-by-range")
    );
}

/// 演示三种驱动对比
fn show_driver_comparison() {
    print_section(&tr("example-database-pg-mysql-example-5-three-driver"));

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

    println!("\n{}", tr("example-database-pg-mysql-feature-comparison"));
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

    println!("\n{}", tr("example-database-pg-mysql-selection-advice"));
    println!(
        "  {}",
        tr("example-database-pg-mysql-dev-test-sqlite-zero-config")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-small-medium-production-mysql")
    );
    println!(
        "  {}",
        tr("example-database-pg-mysql-large-enterprise-postgresql")
    );
}

/// 演示环境变量覆盖
fn show_env_overrides() {
    print_section(&tr("example-database-pg-mysql-example-6-env-var-overrides"));

    println!(
        "{}",
        tr("example-database-pg-mysql-database-driver-env-vars")
    );
    println!("  INKLOG_DATABASE_SINK_DRIVER=postgres   # or mysql / sqlite");
    println!("  INKLOG_DATABASE_SINK_URL=\"postgres://user:pass@host/db\"");

    println!(
        "\n{}",
        tr("example-database-pg-mysql-loading-via-load-with-env")
    );
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
    println!("  if config.database_sink.enabled {{");
    println!("      println!(\"Driver: {{:?}}, URL: {{}}\",");
    println!(
        "          {}",
        tr("example-database-pg-mysql-config-database-sink-driver")
    );
    println!("          config.database_sink.url);");
    println!("  }}");

    println!(
        "\n{}",
        tr("example-database-pg-mysql-docker-deployment-example")
    );
    println!("  docker run -e INKLOG_DATABASE_SINK_DRIVER=postgres \\");
    println!("             -e INKLOG_DATABASE_SINK_URL=\"postgres://...\" \\");
    println!("             {}", tr("example-database-pg-mysql-my-app"));

    println!(
        "\n{}",
        tr("example-database-pg-mysql-kubernetes-configmap-example")
    );
    println!("  {}", tr("example-database-pg-mysql-env"));
    println!(
        "    {}",
        tr("example-database-pg-mysql-name-inklog-database-sink-driver")
    );
    println!("      {}", tr("example-database-pg-mysql-valuefrom"));
    println!(
        "        {}",
        tr("example-database-pg-mysql-configmapkeyref")
    );
    println!(
        "          {}",
        tr("example-database-pg-mysql-name-inklog-config")
    );
    println!(
        "          {}",
        tr("example-database-pg-mysql-key-db-driver")
    );
}
