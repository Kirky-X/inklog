// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Database Sink 示例
//!
//! 演示 inklog 数据库输出的核心功能：
//!
//! 1. **memory_database**: SQLite 内存数据库连接
//! 2. **batch_write**: 批量写入演示
//! 3. **query_demo**: 查询功能演示
//!
//! ## 运行
//!
//! ```bash
//! cargo run --bin database --features sqlite
//! ```
//!
//! ## 核心特性
//!
//! - **内存数据库**: 使用 SQLite `:memory:` 模式，无需文件清理
//! - **批量写入**: 可配置批次大小和刷新间隔
//! - **查询功能**: 支持按级别、时间范围、目标过滤
//! - **自动降级**: 数据库故障时自动降级到文件输出
//!
//! ## 数据库表结构
//!
//! | 字段 | 类型 | 说明 |
//! |------|------|------|
//! | id | BIGINT | 自增主键 |
//! | timestamp | TIMESTAMP | 日志时间戳 |
//! | level | VARCHAR | 日志级别 (TRACE/DEBUG/INFO/WARN/ERROR) |
//! | target | VARCHAR | 日志目标模块 |
//! | message | TEXT | 日志消息 |
//! | fields | TEXT | 额外字段 (JSON) |
//! | file | VARCHAR | 来源文件 |
//! | line | INT | 来源行号 |
//! | thread_id | VARCHAR | 线程 ID |
//! | module_path | VARCHAR | 模块路径 |
//! | metadata | TEXT | 元数据 |

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use inklog::chrono::Utc;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use inklog::config::{DatabaseDriver, DatabaseSinkConfig};
use inklog_examples::common::tr;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use inklog_examples::common::{print_section, print_separator, tr_args};

/// 转义 SQL 字符串字面量，防止 SQL 注入。
///
/// dbnexus 的 `execute_raw` 不支持占位符参数绑定，
/// 因此与库内 `DatabaseSink`（src/integrations/infra/database.rs）的做法一致：
/// 所有拼入 SQL 的字符串必须先经此函数转义。
/// - ANSI 后端（SQLite/PostgreSQL/DuckDB）：将单引号 `'` 双写为 `''`
/// - MySQL：额外转义反斜杠（MySQL 默认把 `\` 视为转义符）
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn escape_sql_string(s: &str, driver: &DatabaseDriver) -> String {
    match driver {
        DatabaseDriver::MySQL => s.replace('\\', "\\\\").replace('\'', "''"),
        _ => s.replace('\'', "''"),
    }
}

/// 创建临时权限配置文件
///
/// 创建一个包含全权限配置的临时 YAML 文件，用于 dbnexus 权限系统。
/// 临时文件会在程序退出时自动清理。
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn create_temp_permission_file() -> Result<tempfile::NamedTempFile, Box<dyn std::error::Error>> {
    use std::io::Write;

    let mut temp_file = tempfile::NamedTempFile::new()?;

    // 写入权限配置 YAML
    // 注意：PermissionAction 使用 snake_case 序列化，所以使用 select/insert/update/delete
    let yaml_content = r#"roles:
  admin:
    tables:
      - name: "*"
        operations:
          - select
          - insert
          - update
          - delete
"#;

    temp_file.write_all(yaml_content.as_bytes())?;
    temp_file.flush()?;

    Ok(temp_file)
}

/// SQLite 内存数据库连接示例
///
/// 演示如何配置 DatabaseSink 使用 SQLite 内存数据库。
/// 内存数据库特点：
/// - 数据存储在内存中，断电后丢失
/// - 无需文件清理，非常适合测试和演示
/// - 连接字符串为 `sqlite::memory:`
///
/// 注意：SQLite 内存数据库每个连接是独立的，
/// 需要使用共享连接模式或同一连接来保持数据。
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn memory_database() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-database-example-1-sqlite-in-memory"));

    let temp_perm_file = create_temp_permission_file()?;
    let perm_path = temp_perm_file.path().to_str().ok_or("Invalid temp path")?;

    // 创建 DatabaseSink 配置
    let config = DatabaseSinkConfig {
        name: "memory_example".to_string(),
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: "sqlite::memory:".to_string(),
        pool_size: 1,           // 内存数据库必须用 1，避免多连接隔离
        batch_size: 10,         // 批次大小
        flush_interval_ms: 100, // 刷新间隔 100ms
        partition: inklog::config::PartitionStrategy::Monthly,
        table_name: "logs".to_string(),
        archive_format: inklog::ArchiveFormat::default(),
        parquet_config: Default::default(),
        permissions_path: Some(perm_path.to_string()),
        admin_role: "admin".to_string(),
        secret_scan_enabled: false,
    };

    println!("{}", tr("example-database-database-config"));
    println!(
        "  {}",
        tr_args(
            "example-database-driver",
            &[("driver", &format!("{:?}", config.driver))]
        )
    );
    println!(
        "  {}",
        tr_args("example-database-url", &[("url", &config.url)])
    );
    println!(
        "  {}",
        tr_args(
            "example-database-connection-pool",
            &[("pool_size", &config.pool_size)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-batch-size",
            &[("batch_size", &config.batch_size)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-flush-interval-ms",
            &[("flush_interval_ms", &config.flush_interval_ms)]
        )
    );

    // 使用 dbnexus 直接连接数据库
    print_section(&tr("example-database-connecting-to-sqlite-in-memory"));

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        // 创建 DbConfig（带权限配置）
        let db_config = dbnexus::foundation::config::DbConfig {
            url: config.url.clone(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: config.pool_size,
                min_connections: 1,
                ..Default::default()
            },
            permissions_path: Some(perm_path.to_string()),
            admin_role: "admin".to_string(),
            ..Default::default()
        };

        let pool = match dbnexus::DbPool::with_config(db_config).await {
            Ok(p) => {
                println!("{}", tr("example-database-connection-pool-created"));
                p
            }
            Err(e) => {
                println!("{}", tr_args("example-database-failed-to-create-connection-pool", &[("e", &format!("{:?}", e))]));
                return;
            }
        };

        let session: dbnexus::database::pool::Session = match pool.get_session("admin").await {
            Ok(s) => {
                println!("{}", tr("example-database-session-acquired"));
                s
            }
            Err(e) => {
                println!("{}", tr_args("example-database-failed-to-acquire-session", &[("e", &format!("{:?}", e))]));
                return;
            }
        };

        let create_result: dbnexus::foundation::error::DbResult<_> = session.execute_raw_ddl(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                target TEXT NOT NULL,
                message TEXT NOT NULL,
                fields TEXT,
                file TEXT,
                line INTEGER,
                thread_id TEXT NOT NULL,
                module_path TEXT,
                metadata TEXT
            )"
        ).await;

        match create_result {
            Ok(_) => println!("{}", tr("example-database-logs-table-created")),
            Err(e) => println!("{}", tr_args("example-database-failed-to-create-logs-table-may", &[("e", &format!("{:?}", e))])),
        }

        print_section(&tr("example-database-write-test-log"));

        let test_logs = vec![
            ("INFO", "database_example::memory", "in-memory database connection test"),
            ("DEBUG", "database_example::memory", "config parameters validated"),
            ("TRACE", "database_example::memory", "connection pool init trace"),
        ];

        for (level, target, message) in test_logs {
            // 所有插值字符串必须转义后再拼入 SQL，防止 SQL 注入
            let sql = format!(
                "INSERT INTO logs (timestamp, level, target, message, thread_id) VALUES ('{}', '{}', '{}', '{}', '{}')",
                escape_sql_string(&Utc::now().to_rfc3339(), &config.driver),
                escape_sql_string(level, &config.driver),
                escape_sql_string(target, &config.driver),
                escape_sql_string(message, &config.driver),
                escape_sql_string("main", &config.driver)
            );
            match session.execute_raw(&sql).await {
                Ok(_) => println!("  ✓ [{}] {}", level, message),
                Err(e) => println!("  {}", tr_args("example-database-write-failed", &[("e", &format!("{:?}", e))])),
            }
        }

        print_section(&tr("example-database-query-verification"));
        let result: dbnexus::foundation::error::DbResult<_> = session
            .execute_raw("SELECT COUNT(*) as count FROM logs")
            .await;

        match result {
            Ok(_) => println!("{}", tr("example-database-query-succeeded-table-has-data")),
            Err(e) => println!("{}", tr_args("example-database-query-failed", &[("e", &format!("{:?}", e))])),
        }
    });

    println!(
        "\n{}\n",
        tr("example-database-in-memory-database-connection")
    );
    println!("  {}", tr("example-database-note-sqlite-in-memory-data-is"));
    println!("  {}\n", tr("example-database-each-databasesink-instance"));

    Ok(())
}

/// 批量写入演示
///
/// 演示数据库的批量写入功能：
/// - 配置小批次大小触发频繁刷新
/// - 写入多种级别的日志
/// - 展示批量写入的性能优势
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn batch_write() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-database-example-2-batch-write-demo"));

    let temp_perm_file = create_temp_permission_file()?;
    let perm_path = temp_perm_file.path().to_str().ok_or("Invalid temp path")?;

    let config = DatabaseSinkConfig {
        name: "batch_example".to_string(),
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: "sqlite::memory:".to_string(),
        pool_size: 1,          // 内存数据库必须用 1
        batch_size: 5,         // 小批次大小
        flush_interval_ms: 50, // 快速刷新
        partition: inklog::config::PartitionStrategy::Monthly,
        table_name: "logs".to_string(),
        archive_format: inklog::ArchiveFormat::default(),
        parquet_config: Default::default(),
        permissions_path: Some(perm_path.to_string()),
        admin_role: "admin".to_string(),
        secret_scan_enabled: false,
    };

    println!("{}", tr("example-database-batch-configuration"));
    println!(
        "  {}",
        tr_args(
            "example-database-batch-size",
            &[("batch_size", &config.batch_size)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-database-flush-interval-ms",
            &[("flush_interval_ms", &config.flush_interval_ms)]
        )
    );

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        // 创建 DbConfig（带权限配置）
        let db_config = dbnexus::foundation::config::DbConfig {
            url: config.url.clone(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: config.pool_size,
                min_connections: 1,
                ..Default::default()
            },
            permissions_path: Some(perm_path.to_string()),
            admin_role: "admin".to_string(),
            ..Default::default()
        };

        let pool = match dbnexus::DbPool::with_config(db_config).await {
            Ok(p) => p,
            Err(e) => {
                println!("{}", tr_args("example-database-failed-to-create-connection-pool", &[("e", &format!("{:?}", e))]));
                return;
            }
        };

        let session: dbnexus::database::pool::Session = match pool.get_session("admin").await {
            Ok(s) => s,
            Err(e) => {
                println!("{}", tr_args("example-database-failed-to-acquire-session", &[("e", &format!("{:?}", e))]));
                return;
            }
        };

        let _: dbnexus::foundation::error::DbResult<_> = session.execute_raw_ddl(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                target TEXT NOT NULL,
                message TEXT NOT NULL,
                fields TEXT,
                file TEXT,
                line INTEGER,
                thread_id TEXT NOT NULL,
                module_path TEXT,
                metadata TEXT
            )"
        ).await;

        print_section(&tr("example-database-batch-write-log"));

        let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
        let messages = [
            "system startup initialization",
            "config parameters loaded",
            "database connection pool created",
            "config change detected",
            "request processing timed out",
        ];
        let targets = [
            "batch_example::startup",
            "batch_example::config",
            "batch_example::database",
            "batch_example::monitor",
            "batch_example::handler",
        ];

        println!("{}\n", tr_args("example-database-writing-log-records", &[("value", &(levels.len() * 3))]));

        // 写入 3 轮日志，演示批量效果
        for round in 1..=3 {
            println!("{}", tr_args("example-database-write-round", &[("round", &round)]));

            let mut success_count = 0;
            for i in 0..levels.len() {
                // 所有插值字符串必须转义后再拼入 SQL，防止 SQL 注入
                let fields_json =
                    format!(r#"{{"round":"{}","sequence":"{}"}}"#, round, i + 1);
                let sql = format!(
                    "INSERT INTO logs (timestamp, level, target, message, fields, thread_id) VALUES ('{}', '{}', '{}', '{}', '{}', '{}')",
                    escape_sql_string(&Utc::now().to_rfc3339(), &config.driver),
                    escape_sql_string(levels[i], &config.driver),
                    escape_sql_string(targets[i], &config.driver),
                    escape_sql_string(messages[i], &config.driver),
                    escape_sql_string(&fields_json, &config.driver),
                    escape_sql_string(&format!("worker-{}", round), &config.driver)
                );

                match session.execute_raw(&sql).await {
                    Ok(_) => {
                        success_count += 1;
                        println!("  ✓ [{}] {}", levels[i], messages[i]);
                    }
                    Err(e) => println!("  {}", tr_args("example-database-write-failed-2", &[("value", &(levels[i])), ("e", &format!("{:?}", e))])),
                }
            }

            println!("  {}\n", tr_args("example-database-round-done-succeeded", &[("round", &round), ("success_count", &success_count), ("value", &(levels.len()))]));
        }

        let result: dbnexus::foundation::error::DbResult<_> = session
            .execute_raw("SELECT COUNT(*) as total FROM logs")
            .await;

        match result {
            Ok(_) => println!("{}", tr("example-database-total-log-records-in-the")),
            Err(e) => println!("{}", tr_args("example-database-count-failed", &[("e", &format!("{:?}", e))])),
        }
    });

    println!("\n{}\n", tr("example-database-batch-write-demo-completed"));

    Ok(())
}

/// 查询功能演示
///
/// 演示如何使用 SQL 查询日志数据：
/// - 按级别过滤
/// - 按时间范围过滤
/// - 按目标模块过滤
/// - 聚合统计
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn query_demo() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-database-example-3-query-demo"));

    let temp_perm_file = create_temp_permission_file()?;
    let perm_path = temp_perm_file.path().to_str().ok_or("Invalid temp path")?;

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        // 创建 DbConfig（带权限配置）
        let db_config = dbnexus::foundation::config::DbConfig {
            url: "sqlite::memory:".to_string(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: 2,
                min_connections: 1,
                ..Default::default()
            },
            permissions_path: Some(perm_path.to_string()),
            admin_role: "admin".to_string(),
            ..Default::default()
        };

        let pool = match dbnexus::DbPool::with_config(db_config).await {
            Ok(p) => p,
            Err(e) => {
                println!("{}", tr_args("example-database-failed-to-create-connection-pool", &[("e", &format!("{:?}", e))]));
                return;
            }
        };

        let session: dbnexus::database::pool::Session = match pool.get_session("admin").await {
            Ok(s) => s,
            Err(e) => {
                println!("{}", tr_args("example-database-failed-to-acquire-session", &[("e", &format!("{:?}", e))]));
                return;
            }
        };

        let _: dbnexus::foundation::error::DbResult<_> = session.execute_raw_ddl(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                target TEXT NOT NULL,
                message TEXT NOT NULL,
                fields TEXT,
                file TEXT,
                line INTEGER,
                thread_id TEXT NOT NULL,
                module_path TEXT,
                metadata TEXT
            )"
        ).await;

        print_section(&tr("example-database-prepare-test-data"));
        println!("{}\n", tr("example-database-writing-log-records-at"));

        let test_data = vec![
            ("TRACE", "database_example::query", "trace: entry point check"),
            ("TRACE", "database_example::query", "trace: parameter validation passed"),
            ("DEBUG", "database_example::query", "debug: SQL statement construction"),
            ("DEBUG", "database_example::query", "debug: acquiring database connection"),
            ("INFO", "database_example::api", "API request processing started"),
            ("INFO", "database_example::api", "data processing finished"),
            ("INFO", "database_example::api", "response sent successfully"),
            ("WARN", "database_example::auth", "auth token expiring soon"),
            ("WARN", "database_example::cache", "cache hit rate low: 45%"),
            ("ERROR", "database_example::database", "database connection timeout"),
            ("ERROR", "database_example::database", "query execution failed"),
        ];

        for (level, target, message) in &test_data {
            // 本示例硬编码 SQLite，所有插值字符串转义后再拼入 SQL
            let sql = format!(
                "INSERT INTO logs (timestamp, level, target, message, thread_id) VALUES ('{}', '{}', '{}', '{}', '{}')",
                escape_sql_string(&Utc::now().to_rfc3339(), &DatabaseDriver::SQLite),
                escape_sql_string(level, &DatabaseDriver::SQLite),
                escape_sql_string(target, &DatabaseDriver::SQLite),
                escape_sql_string(message, &DatabaseDriver::SQLite),
                escape_sql_string("query-demo", &DatabaseDriver::SQLite)
            );
            match session.execute_raw(&sql).await {
                Ok(_) => {}
                Err(e) => println!("  {}", tr_args("example-database-insert-failed", &[("e", &format!("{:?}", e))])),
            }
        }
        println!("{}\n", tr_args("example-database-wrote-test-records", &[("value", &(test_data.len()))]));

        print_section(&tr("example-database-query-1-count-by-level"));
        println!("{}\n", tr("example-database-sql-select-level-count-as-count"));

        println!("{}", tr("example-database-expected-result"));
        println!("  ┌───────┬───────┐");
        println!("  {}", tr("example-database-level-count"));
        println!("  ├───────┼───────┤");
        println!("  {}", tr("example-database-trace-2"));
        println!("  {}", tr("example-database-debug-2"));
        println!("  {}", tr("example-database-info-3"));
        println!("  {}", tr("example-database-warn-2"));
        println!("  {}", tr("example-database-error-2"));
        println!("  └───────┴───────┘");
        println!("  {}\n", tr("example-database-total-11-records"));

        print_section(&tr("example-database-query-2-count-by-target-module"));
        println!("{}\n", tr("example-database-sql-select-target-count-as"));

        println!("{}", tr("example-database-expected-result"));
        println!("  ┌────────────────────────┬───────┐");
        println!("  {}", tr("example-database-target-count"));
        println!("  ├────────────────────────┼───────┤");
        println!("  {}", tr("example-database-database-example-api-3"));
        println!("  {}", tr("example-database-database-example-query-4"));
        println!("  {}", tr("example-database-database-example-auth-1"));
        println!("  {}", tr("example-database-database-example-cache-1"));
        println!("  {}", tr("example-database-database-example-database-2"));
        println!("  └────────────────────────┴───────┘\n");

        // 查询 3: 按级别过滤 (ERROR/WARN)
        print_section(&tr("example-database-query-3-filter-by-level-error"));
        println!("{}\n", tr("example-database-sql-select-from-logs-where"));

        println!("{}", tr("example-database-expected-result"));
        println!("  {}", tr("example-database-warn-database-example-auth-auth"));
        println!("  {}", tr("example-database-warn-database-example-cache"));
        println!("  {}", tr("example-database-error-database-example-database"));
        println!("  {}\n", tr("example-database-error-database-example-database-2"));

        print_section(&tr("example-database-query-4-search-log-content"));
        println!("{}\n", tr("example-database-sql-select-from-logs-where-2"));

        println!("{}", tr("example-database-expected-result"));
        println!("  {}", tr("example-database-error-database-example-database"));
        println!("  {}\n", tr("example-database-error-database-example-database-2"));

        print_section(&tr("example-database-query-5-fetch-recent-logs"));
        println!("{}\n", tr("example-database-sql-select-from-logs-order-by"));

        println!("{}", tr("example-database-expected-result-ids-descending"));
        println!("  {}", tr("example-database-11-error-database-example"));
        println!("  {}", tr("example-database-10-error-database-example"));
        println!("   {}", tr("example-database-9-warn-database-example-cache"));
        println!("   {}", tr("example-database-8-warn-database-example-auth"));
        println!("   {}\n", tr("example-database-7-info-database-example-api"));

        print_section(&tr("example-database-query-6-time-range-query"));
        println!("{}\n", tr("example-database-sql-select-from-logs-where-3"));

        println!("{}\n", tr("example-database-expected-result-all-logs-all"));

        print_section(&tr("example-database-query-7-combined-multi"));
        println!("{}", tr("example-database-sql-select-level-target-message"));
        println!("       {}\n", tr("example-database-where-level-in-warn-error-and"));

        println!("{}", tr("example-database-expected-result"));
        println!("  {}", tr("example-database-error-database-example-database"));
        println!("  {}\n", tr("example-database-error-database-example-database-2"));
    });

    println!("{}\n", tr("example-database-query-demo-completed"));
    println!("  {}", tr("example-database-tip-use-sql-clauses-such-as"));
    println!(
        "        {}\n",
        tr("example-database-enabling-complex-log-queries")
    );

    Ok(())
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}\n", tr("example-database-inklog-database-sink-example"));
    println!("{}", tr("example-database-this-example-uses-an-sqlite-in"));
    println!("{}\n", tr("example-database-no-file-cleanup-needed-data"));

    // 示例1：内存数据库连接
    memory_database()?;

    // 示例2：批量写入
    batch_write()?;

    // 示例3：查询演示
    query_demo()?;

    println!("{}", tr("example-database-all-examples-completed"));
    println!("\n{}", tr("example-database-key-takeaways"));
    println!(
        "  {}",
        tr("example-database-1-databasesink-supports-sqlite")
    );
    println!("  {}", tr("example-database-2-in-memory-databases-suit"));
    println!(
        "  3. Batched writes improve performance; batch size and flush interval are configurable"
    );
    println!("  {}", tr("example-database-4-supports-queries-by-level"));
    println!("  {}", tr("example-database-5-automatically-falls-back-to"));
    println!("\n{}", tr("example-database-sql-query-tips"));
    println!("  {}", tr("example-database-by-level-where-level-error"));
    println!(
        "  {}",
        tr("example-database-by-module-where-target-like-api")
    );
    println!(
        "  {}",
        tr("example-database-by-time-where-timestamp-2024-01")
    );
    println!(
        "  {}",
        tr("example-database-aggregate-stats-group-by-level")
    );

    Ok(())
}

// 当未启用 dbnexus 特性时显示友好提示
#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
fn main() {
    eprintln!("{}", tr("example-database-error-enable-a-database-feature"));
    eprintln!();
    eprintln!("{}", tr("example-database-run-with-one-of-the-following"));
    eprintln!(
        "  {}",
        tr("example-database-cargo-run-bin-database-features")
    );
    eprintln!();
    eprintln!("{}", tr("example-database-or-enable-the-sqlite-feature-in"));
    eprintln!("  inklog = {{ path = \"..\", features = [\"sqlite\"] }}");
}
