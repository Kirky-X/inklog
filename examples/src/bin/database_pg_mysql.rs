// Copyright (c) 2026 Kirky.X
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
    print_separator("inklog PostgreSQL/MySQL 数据库驱动示例");

    show_driver_variants();
    show_from_str_display();
    show_postgresql_config();
    show_mysql_config();
    show_driver_comparison();
    show_env_overrides();

    println!("\n所有 PostgreSQL/MySQL 驱动示例展示完毕。");
}

/// 演示 DatabaseDriver 三种变体
fn show_driver_variants() {
    print_section("示例 1：DatabaseDriver 三种变体");

    let postgres = DatabaseDriver::PostgreSQL;
    let mysql = DatabaseDriver::MySQL;
    let sqlite = DatabaseDriver::SQLite;

    println!("枚举变体：");
    println!("  {:?} - 企业级首选，功能最全", postgres);
    println!("  {:?} - 性能与简洁的平衡", mysql);
    println!("  {:?} - 嵌入式，零配置", sqlite);

    println!("\n默认驱动：");
    println!(
        "  DatabaseDriver::default() = {:?}",
        DatabaseDriver::default()
    );
}

/// 演示 FromStr 和 Display trait
fn show_from_str_display() {
    print_section("示例 2：FromStr / Display trait");

    println!("FromStr 解析（大小写不敏感）：");
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

    println!("\n无效输入：");
    for input in ["", "oracle", "mongodb", "invalid"] {
        let result: Result<DatabaseDriver, ()> = input.parse();
        match result {
            Ok(d) => println!("  {:>10}.parse() → Ok({:?})", input, d),
            Err(_) => println!("  {:>10}.parse() → Err(())", input),
        }
    }

    println!("\nDisplay 输出：");
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
    print_section("示例 3：PostgreSQL 连接配置");

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

    println!("PostgreSQL DatabaseSinkConfig：");
    println!("  driver            = {:?}", pg_config.driver);
    println!("  url               = \"{}\"", pg_config.url);
    println!("  table_name        = \"{}\"", pg_config.table_name);
    println!("  batch_size        = {}", pg_config.batch_size);
    println!("  flush_interval_ms = {}", pg_config.flush_interval_ms);
    println!("  pool_size         = {}", pg_config.pool_size);

    println!("\n连接 URL 格式：");
    println!("  postgres://USER:PASSWORD@HOST:PORT/DATABASE");
    println!();
    println!("  示例：");
    println!("  postgres://inklog:secret@db.example.com:5432/logs");
    println!("  postgres://postgres@localhost/inklog");
    println!("  postgres://user:pass@10.0.0.1:5432/prod_logs?sslmode=require");

    println!("\n对应 TOML 配置：");
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

    println!("\nPostgreSQL 优势：");
    println!("  - 功能最全：支持分区表（PARTITION BY RANGE）");
    println!("  - 性能优秀：高并发写入，MVCC 多版本并发控制");
    println!("  - 扩展性：支持 JSONB、全文搜索、地理数据");
    println!("  - 生产推荐：企业级日志存储首选");

    println!("\n表分区配置（高日志量场景）：");
    println!(
        r#"# 按月分区
[database_sink.partition]
strategy = "monthly"
# 或按年分区
# strategy = "yearly""#
    );
}

/// 演示 MySQL 连接配置
fn show_mysql_config() {
    print_section("示例 4：MySQL 连接配置");

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

    println!("MySQL DatabaseSinkConfig：");
    println!("  driver            = {:?}", mysql_config.driver);
    println!("  url               = \"{}\"", mysql_config.url);
    println!("  table_name        = \"{}\"", mysql_config.table_name);
    println!("  batch_size        = {}", mysql_config.batch_size);
    println!("  flush_interval_ms = {}", mysql_config.flush_interval_ms);
    println!("  pool_size         = {}", mysql_config.pool_size);

    println!("\n连接 URL 格式：");
    println!("  mysql://USER:PASSWORD@HOST:PORT/DATABASE");
    println!();
    println!("  示例：");
    println!("  mysql://root@localhost/inklog");
    println!("  mysql://inklog:secret@db.example.com:3306/logs");
    println!("  mysql://user:pass@10.0.0.1:3306/prod_logs?ssl-ca=/path/to/ca.pem");

    println!("\n对应 TOML 配置：");
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

    println!("\nMySQL 优势：");
    println!("  - 部署简单：默认配置即可使用");
    println!("  - 生态成熟：LAMP/LEMP 栈天然集成");
    println!("  - 性能平衡：读写性能均衡");
    println!("  - 分区支持：PARTITION BY RANGE");
}

/// 演示三种驱动对比
fn show_driver_comparison() {
    print_section("示例 5：三种驱动对比");

    println!(
        "{:<12} {:<15} {:<20} {:<15}",
        "驱动", "默认端口", "适用场景", "推荐度"
    );
    println!("{}", "-".repeat(65));
    println!(
        "{:<12} {:<15} {:<20} {:<15}",
        "PostgreSQL", "5432", "企业级生产", "★★★★★"
    );
    println!(
        "{:<12} {:<15} {:<20} {:<15}",
        "MySQL", "3306", "Web 应用", "★★★★"
    );
    println!(
        "{:<12} {:<15} {:<20} {:<15}",
        "SQLite", "无", "开发/测试/嵌入式", "★★★"
    );

    println!("\n功能对比：");
    println!(
        "{:<20} {:<12} {:<12} {:<12}",
        "特性", "PostgreSQL", "MySQL", "SQLite"
    );
    println!("{}", "-".repeat(56));
    println!("{:<20} {:<12} {:<12} {:<12}", "分区表", "✓", "✓", "✗");
    println!("{:<20} {:<12} {:<12} {:<12}", "JSONB", "✓", "✗ (JSON)", "✗");
    println!(
        "{:<20} {:<12} {:<12} {:<12}",
        "并发写入", "高 (MVCC)", "中", "低 (锁)"
    );
    println!(
        "{:<20} {:<12} {:<12} {:<12}",
        "连接池", "必需", "必需", "N/A"
    );
    println!("{:<20} {:<12} {:<12} {:<12}", "零配置", "✗", "✗", "✓");

    println!("\n选型建议：");
    println!("  - 开发/测试：SQLite（零配置，快速启动）");
    println!("  - 中小规模生产：MySQL（部署简单，性能均衡）");
    println!("  - 大规模/企业级：PostgreSQL（功能最全，性能最强）");
}

/// 演示环境变量覆盖
fn show_env_overrides() {
    print_section("示例 6：环境变量覆盖");

    println!("数据库驱动环境变量：");
    println!("  INKLOG_DATABASE_SINK_DRIVER=postgres   # 或 mysql / sqlite");
    println!("  INKLOG_DATABASE_SINK_URL=\"postgres://user:pass@host/db\"");

    println!("\n通过 load_with_env_overrides() 加载：");
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
    println!("  if config.database_sink.enabled {{");
    println!("      println!(\"Driver: {{:?}}, URL: {{}}\",");
    println!("          config.database_sink.driver,");
    println!("          config.database_sink.url);");
    println!("  }}");

    println!("\nDocker 部署示例：");
    println!("  docker run -e INKLOG_DATABASE_SINK_DRIVER=postgres \\");
    println!("             -e INKLOG_DATABASE_SINK_URL=\"postgres://...\" \\");
    println!("             my-app");

    println!("\nKubernetes ConfigMap 示例：");
    println!("  env:");
    println!("    - name: INKLOG_DATABASE_SINK_DRIVER");
    println!("      valueFrom:");
    println!("        configMapKeyRef:");
    println!("          name: inklog-config");
    println!("          key: db-driver");
}
