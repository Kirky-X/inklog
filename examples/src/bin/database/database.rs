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
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use inklog_examples::common::{print_section, print_separator};

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
    print_separator("Example 1: SQLite in-memory database connection");

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

    println!("Database config:");
    println!("  driver: {:?}", config.driver);
    println!("  URL: {}", config.url);
    println!("  connection pool: {}", config.pool_size);
    println!("  batch size: {}", config.batch_size);
    println!("  flush interval: {}ms", config.flush_interval_ms);

    // 使用 dbnexus 直接连接数据库
    print_section("connecting to SQLite in-memory database");

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
                println!("✓ connection pool created");
                p
            }
            Err(e) => {
                println!("✗ failed to create connection pool: {:?}", e);
                return;
            }
        };

        let session: dbnexus::database::pool::Session = match pool.get_session("admin").await {
            Ok(s) => {
                println!("✓ session acquired");
                s
            }
            Err(e) => {
                println!("✗ failed to acquire session: {:?}", e);
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
            Ok(_) => println!("✓ logs table created"),
            Err(e) => println!("⚠ failed to create logs table (may already exist): {:?}", e),
        }

        print_section("write test log");

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
                Err(e) => println!("  ✗ write failed: {:?}", e),
            }
        }

        print_section("query verification");
        let result: dbnexus::foundation::error::DbResult<_> = session
            .execute_raw("SELECT COUNT(*) as count FROM logs")
            .await;

        match result {
            Ok(_) => println!("✓ query succeeded, table has data"),
            Err(e) => println!("✗ query failed: {:?}", e),
        }
    });

    println!("\n✓ In-memory database connection example completed\n");
    println!("  Note: SQLite in-memory data is lost when the connection closes");
    println!("  Each DatabaseSink instance creates its own connection\n");

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
    print_separator("Example 2: batch write demo");

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

    println!("Batch configuration:");
    println!("  batch size: {}", config.batch_size);
    println!("  flush interval: {}ms", config.flush_interval_ms);

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
                println!("✗ failed to create connection pool: {:?}", e);
                return;
            }
        };

        let session: dbnexus::database::pool::Session = match pool.get_session("admin").await {
            Ok(s) => s,
            Err(e) => {
                println!("✗ failed to acquire session: {:?}", e);
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

        print_section("batch write log");

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

        println!("writing {} log records...\n", levels.len() * 3);

        // 写入 3 轮日志，演示批量效果
        for round in 1..=3 {
            println!("--- write round {} ---", round);

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
                    Err(e) => println!("  ✗ [{}] write failed: {:?}", levels[i], e),
                }
            }

            println!("  Round {} done: {} / {} succeeded\n", round, success_count, levels.len());
        }

        let result: dbnexus::foundation::error::DbResult<_> = session
            .execute_raw("SELECT COUNT(*) as total FROM logs")
            .await;

        match result {
            Ok(_) => println!("✓ total log records in the database"),
            Err(e) => println!("✗ count failed: {:?}", e),
        }
    });

    println!("\n✓ Batch write demo completed\n");

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
    print_separator("Example 3: query demo");

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
                println!("✗ failed to create connection pool: {:?}", e);
                return;
            }
        };

        let session: dbnexus::database::pool::Session = match pool.get_session("admin").await {
            Ok(s) => s,
            Err(e) => {
                println!("✗ failed to acquire session: {:?}", e);
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

        print_section("prepare test data");
        println!("writing log records at different levels...\n");

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
                Err(e) => println!("  ⚠ INSERT failed: {:?}", e),
            }
        }
        println!("✓ wrote {} test records\n", test_data.len());

        print_section("Query 1: count by level");
        println!("SQL: SELECT level, COUNT(*) as count FROM logs GROUP BY level\n");

        println!("Expected result:");
        println!("  ┌───────┬───────┐");
        println!("  │ Level │ Count │");
        println!("  ├───────┼───────┤");
        println!("  │ TRACE │   2   │");
        println!("  │ DEBUG │   2   │");
        println!("  │ INFO  │   3   │");
        println!("  │ WARN  │   2   │");
        println!("  │ ERROR │   2   │");
        println!("  └───────┴───────┘");
        println!("  Total: 11 records\n");

        print_section("Query 2: count by target module");
        println!("SQL: SELECT target, COUNT(*) as count FROM logs GROUP BY target\n");

        println!("Expected result:");
        println!("  ┌────────────────────────┬───────┐");
        println!("  │ Target                 │ Count │");
        println!("  ├────────────────────────┼───────┤");
        println!("  │ database_example::api  │   3   │");
        println!("  │ database_example::query│   4   │");
        println!("  │ database_example::auth │   1   │");
        println!("  │ database_example::cache│   1   │");
        println!("  │ database_example::database│  2   │");
        println!("  └────────────────────────┴───────┘\n");

        // 查询 3: 按级别过滤 (ERROR/WARN)
        print_section("Query 3: filter by level (ERROR/WARN)");
        println!("SQL: SELECT * FROM logs WHERE level IN ('ERROR', 'WARN')\n");

        println!("Expected result:");
        println!("  [WARN]  database_example::auth     - auth token expiring soon");
        println!("  [WARN]  database_example::cache    - cache hit rate low: 45%");
        println!("  [ERROR] database_example::database  - database connection timeout");
        println!("  [ERROR] database_example::database  - query execution failed\n");

        print_section("Query 4: search log content");
        println!("SQL: SELECT * FROM logs WHERE message LIKE '%timeout%' OR message LIKE '%failed%'\n");

        println!("Expected result:");
        println!("  [ERROR] database_example::database  - database connection timeout");
        println!("  [ERROR] database_example::database  - query execution failed\n");

        print_section("Query 5: fetch recent logs");
        println!("SQL: SELECT * FROM logs ORDER BY id DESC LIMIT 5\n");

        println!("Expected result (IDs descending):");
        println!("  11. [ERROR] database_example::database - query execution failed");
        println!("  10. [ERROR] database_example::database - database connection timeout");
        println!("   9. [WARN]  database_example::cache   - cache hit rate low: 45%");
        println!("   8. [WARN]  database_example::auth    - auth token expiring soon");
        println!("   7. [INFO]  database_example::api    - response sent successfully\n");

        print_section("Query 6: time-range query");
        println!("SQL: SELECT * FROM logs WHERE timestamp > '2024-01-01'\n");

        println!("Expected result: all logs (all were written recently)\n");

        print_section("Query 7: combined multi-condition query");
        println!("SQL: SELECT level, target, message FROM logs");
        println!("       WHERE level IN ('WARN', 'ERROR') AND target LIKE '%database%'\n");

        println!("Expected result:");
        println!("  [ERROR] database_example::database  - database connection timeout");
        println!("  [ERROR] database_example::database  - query execution failed\n");
    });

    println!("✓ Query demo completed\n");
    println!("  Tip: use SQL clauses such as WHERE, GROUP BY, ORDER BY");
    println!("        enabling complex log queries and analysis\n");

    Ok(())
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog Database Sink example ===\n");
    println!("This example uses an SQLite in-memory database (:memory:)");
    println!("No file cleanup needed; data lives only in memory\n");

    // 示例1：内存数据库连接
    memory_database()?;

    // 示例2：批量写入
    batch_write()?;

    // 示例3：查询演示
    query_demo()?;

    println!("All examples completed!");
    println!("\nKey takeaways:");
    println!("  1. DatabaseSink supports SQLite, PostgreSQL, and MySQL");
    println!("  2. In-memory databases suit testing and demos; data is lost on shutdown");
    println!(
        "  3. Batched writes improve performance; batch size and flush interval are configurable"
    );
    println!("  4. Supports queries by level, time, target, and more");
    println!("  5. Automatically falls back to file output on failure");
    println!("\nSQL query tips:");
    println!("  - by level: WHERE level = 'ERROR'");
    println!("  - by module: WHERE target LIKE '%api%'");
    println!("  - by time: WHERE timestamp > '2024-01-01'");
    println!("  - aggregate stats: GROUP BY level");

    Ok(())
}

// 当未启用 dbnexus 特性时显示友好提示
#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
fn main() {
    eprintln!("error: enable a database feature (sqlite/postgres/mysql) to run this example");
    eprintln!();
    eprintln!("Run with one of the following commands:");
    eprintln!("  cargo run --bin database --features sqlite");
    eprintln!();
    eprintln!("or enable the sqlite feature in examples/Cargo.toml:");
    eprintln!("  inklog = {{ path = \"..\", features = [\"sqlite\"] }}");
}
