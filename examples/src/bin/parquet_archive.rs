// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Parquet 归档示例
//!
//! 演示 `inklog::ParquetConfig` 和 `convert_logs_to_parquet()` 的使用：
//!
//! 1. `ParquetConfig` 配置字段（compression_level, max_row_group_size 等）
//! 2. `convert_logs_to_parquet()` 将 LogRecord 转为 Parquet 字节
//! 3. 数据库 Sink 的 Parquet 归档配置
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin parquet_archive --features sqlite
//! ```

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use inklog::config::{DatabaseSinkConfig, ParquetConfig};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use inklog::sink::database::convert_logs_to_parquet;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use inklog::LogRecord;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use inklog_examples::common::{print_section, print_separator};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use tracing::Level;

#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
fn main() {
    eprintln!("本示例需要 sqlite feature: cargo run --bin parquet_archive --features sqlite");
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn main() {
    print_separator("inklog Parquet 归档示例");

    show_parquet_config();
    show_convert_logs_to_parquet();
    show_database_sink_with_parquet();
    show_archive_workflow();

    println!("\n所有 Parquet 归档示例展示完毕。");
}

/// 演示 ParquetConfig 配置字段
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn show_parquet_config() {
    print_section("示例 1：ParquetConfig 配置字段");

    let default = ParquetConfig::default();
    println!("默认 ParquetConfig：");
    println!(
        "  compression_level  = {} (Zstandard 0-10)",
        default.compression_level
    );
    println!("  encoding           = \"{}\"", default.encoding);
    println!(
        "  max_row_group_size = {} (行/行组)",
        default.max_row_group_size
    );
    println!("  max_page_size      = {} (字节/页)", default.max_page_size);

    println!("\n字段语义：");
    println!("  compression_level: Zstandard 压缩级别");
    println!("    0 = 无压缩, 3 = 平衡(推荐), 10 = 最大压缩(较慢)");
    println!("  max_row_group_size: 每个行组的最大行数");
    println!("    较小值: 利于选择性查询");
    println!("    较大值: 更好压缩比");
    println!("  max_page_size: 每页最大字节数，控制读取时内存使用");

    println!("\n自定义配置（高压缩）：");
    let high_compression = ParquetConfig {
        compression_level: 10,
        encoding: "DELTA_BINARY_PACKED".to_string(),
        max_row_group_size: 50000,
        max_page_size: 2 * 1024 * 1024, // 2MB
        ..Default::default()
    };
    println!(
        "  compression_level  = {} (最大压缩)",
        high_compression.compression_level
    );
    println!("  encoding           = \"{}\"", high_compression.encoding);
    println!(
        "  max_row_group_size = {} (更大行组)",
        high_compression.max_row_group_size
    );
    println!(
        "  max_page_size      = {} (2MB)",
        high_compression.max_page_size
    );

    println!("\n自定义配置（低延迟查询）：");
    let low_latency = ParquetConfig {
        compression_level: 1,
        encoding: "PLAIN".to_string(),
        max_row_group_size: 5000,
        max_page_size: 512 * 1024, // 512KB
        ..Default::default()
    };
    println!(
        "  compression_level  = {} (快速)",
        low_latency.compression_level
    );
    println!("  encoding           = \"{}\"", low_latency.encoding);
    println!(
        "  max_row_group_size = {} (更小行组)",
        low_latency.max_row_group_size
    );
    println!(
        "  max_page_size      = {} (512KB)",
        low_latency.max_page_size
    );
}

/// 演示 convert_logs_to_parquet() 实际调用
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn show_convert_logs_to_parquet() {
    print_section("示例 2：convert_logs_to_parquet() 实际调用");

    println!("创建测试 LogRecord...");

    let records = vec![
        LogRecord::new(Level::INFO, "module_a".to_string(), "服务启动".to_string()),
        LogRecord::new(
            Level::WARN,
            "module_a".to_string(),
            "配置项缺失，使用默认值".to_string(),
        ),
        LogRecord::new(
            Level::ERROR,
            "module_b".to_string(),
            "数据库连接失败".to_string(),
        ),
        LogRecord::new(
            Level::INFO,
            "module_c".to_string(),
            "请求处理完成".to_string(),
        ),
    ];
    println!("  ✓ 创建 {} 条 LogRecord", records.len());

    let config = ParquetConfig::default();
    println!("\n调用 convert_logs_to_parquet()...");
    match convert_logs_to_parquet(&records, &config) {
        Ok(parquet_bytes) => {
            println!("  ✓ Parquet 转换成功");
            println!("  输入: {} 条 LogRecord", records.len());
            println!("  输出: {} 字节 Parquet 数据", parquet_bytes.len());
            println!(
                "  压缩比: {:.2} bytes/record",
                parquet_bytes.len() as f64 / records.len() as f64
            );

            // 验证 Parquet magic number (PAR1)
            if parquet_bytes.len() >= 4 {
                let magic = &parquet_bytes[..4];
                println!(
                    "  Parquet magic: {:?} (期望: [80, 65, 82, 49] = \"PAR1\")",
                    magic
                );
            }
        }
        Err(e) => {
            println!("  ✗ 转换失败: {}", e);
        }
    }

    // 空记录测试
    println!("\n空记录测试：");
    let empty: Vec<LogRecord> = vec![];
    match convert_logs_to_parquet(&empty, &config) {
        Ok(bytes) => println!("  ✓ 空记录转换成功: {} 字节", bytes.len()),
        Err(e) => println!("  ✗ 空记录转换失败: {}", e),
    }
}

/// 演示 DatabaseSinkConfig 中的 Parquet 配置
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn show_database_sink_with_parquet() {
    print_section("示例 3：DatabaseSinkConfig 集成 Parquet");

    let config = DatabaseSinkConfig {
        enabled: true,
        driver: inklog::config::DatabaseDriver::SQLite,
        url: "sqlite:///tmp/inklog_parquet.db".to_string(),
        table_name: "logs".to_string(),
        batch_size: 1000,
        flush_interval_ms: 5000,
        archive_format: "parquet".to_string(),
        parquet_config: ParquetConfig {
            compression_level: 6,
            max_row_group_size: 20000,
            ..Default::default()
        },
        ..Default::default()
    };

    println!("启用 Parquet 归档的 DatabaseSinkConfig：");
    println!("  enabled          = {}", config.enabled);
    println!("  driver           = {:?}", config.driver);
    println!("  url              = \"{}\"", config.url);
    println!("  table_name       = \"{}\"", config.table_name);
    println!("  batch_size       = {}", config.batch_size);
    println!("  flush_interval_ms= {}", config.flush_interval_ms);
    println!(
        "  archive_format   = \"{}\" ← 启用 Parquet 归档",
        config.archive_format
    );
    println!(
        "  parquet_config.compression_level  = {}",
        config.parquet_config.compression_level
    );
    println!(
        "  parquet_config.max_row_group_size = {}",
        config.parquet_config.max_row_group_size
    );

    println!("\n对应 TOML 配置：");
    println!(
        r#"[database_sink]
enabled = true
driver = "sqlite"
url = "sqlite:///tmp/inklog_parquet.db"
table_name = "logs"
batch_size = 1000
flush_interval_ms = 5000
archive_format = "parquet"

[database_sink.parquet]
compression_level = 6
max_row_group_size = 20000"#
    );
}

/// 演示 Parquet 归档工作流
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn show_archive_workflow() {
    print_section("示例 4：Parquet 归档工作流");

    println!("Parquet 归档工作流：\n");

    println!("1. 日志写入数据库");
    println!("   tracing::info!(\"服务启动\");");
    println!("   → LogRecord → DatabaseSink → INSERT INTO logs");

    println!("\n2. 定期导出为 Parquet（后台异步）");
    println!("   flush_interval_ms 触发 → convert_logs_to_parquet()");
    println!("   → 生成 Parquet 字节流");

    println!("\n3. Parquet 文件存储");
    println!("   → 写入本地文件 /tmp/inklog_archive_20260101.parquet");
    println!("   → 或上传到对象存储/冷存储（由用户自行集成）");

    println!("\n4. 分析查询（外部工具）");
    println!("   # 使用 DuckDB 查询 Parquet");
    println!("   duckdb -c \"SELECT level, COUNT(*) FROM 'archive.parquet' GROUP BY level\"");
    println!();
    println!("   # 使用 Apache Arrow Python");
    println!("   import pyarrow.parquet as pq");
    println!("   table = pq.read_table('archive.parquet')");

    println!("\n适用场景：");
    println!("  - 分析：加载到 Snowflake/BigQuery/Redshift");
    println!("  - 归档：对象存储冷存储（由用户集成）");
    println!("  - 合规：高效存储满足监管留存要求");

    println!("\n性能特点：");
    println!("  - 异步后台导出，不影响主日志路径");
    println!("  - 列式存储，高压缩比（典型 5-10x）");
    println!("  - 支持谓词下推，查询效率高");
}
