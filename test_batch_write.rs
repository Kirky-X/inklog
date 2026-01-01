use inklog::sink::database::DatabaseSink;
use inklog::sink::LogSink;
use inklog::{log_record::LogRecord, DatabaseSinkConfig};
use std::time::{Duration, Instant};
use tracing::Level;

#[test]
fn test_database_batch_write() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("batch_test.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

    // 配置批量写入：批次大小为5，刷新间隔1秒
    let config = DatabaseSinkConfig {
        enabled: true,
        driver: "sqlite".to_string(),
        url: url.clone(),
        batch_size: 5,
        flush_interval_ms: 1000, // 1秒
        ..Default::default()
    };

    let mut sink = DatabaseSink::new(config).unwrap();

    // 写入3条日志（小于批次大小）
    let start_time = Instant::now();
    for i in 0..3 {
        let record = LogRecord::new(
            Level::INFO,
            "batch_test".into(),
            format!("Message {}", i).into(),
        );
        sink.write(&record).unwrap();
    }

    // 此时不应该立即写入数据库（批次未满，时间未到）
    std::thread::sleep(Duration::from_millis(200));

    // 验证数据库中还没有数据
    let rt = tokio::runtime::Runtime::new().unwrap();
    let count_before = rt.block_on(async {
        use sea_orm::{ConnectionTrait, Database};
        let db = Database::connect(&url).await.unwrap();
        
        let res = db
            .query_one(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                "SELECT count(*) as count FROM logs".to_owned(),
            ))
            .await
            .unwrap();

        res.unwrap().try_get::<i64>("", "count").unwrap()
    });
    assert_eq!(count_before, 0, "数据不应该在批次未满时写入");

    // 再写入3条日志，超过批次大小（5）
    for i in 3..6 {
        let record = LogRecord::new(
            Level::INFO,
            "batch_test".into(),
            format!("Message {}", i).into(),
        );
        sink.write(&record).unwrap();
    }

    // 等待写入完成
    std::thread::sleep(Duration::from_millis(200));

    // 验证数据已写入数据库
    let count_after = rt.block_on(async {
        use sea_orm::{ConnectionTrait, Database};
        let db = Database::connect(&url).await.unwrap();
        
        let res = db
            .query_one(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                "SELECT count(*) as count FROM logs".to_owned(),
            ))
            .await
            .unwrap();

        res.unwrap().try_get::<i64>("", "count").unwrap()
    });
    
    // 应该至少有5条记录（批次大小）被写入
    assert!(count_after >= 5, "批次写入应该触发，当前记录数: {}", count_after);
    
    println!("批量写入测试通过！批次大小: 5, 实际写入: {}", count_after);
}

#[test]
fn test_database_timeout_flush() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("timeout_test.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

    // 配置：批次大小较大，但刷新间隔较短
    let config = DatabaseSinkConfig {
        enabled: true,
        driver: "sqlite".to_string(),
        url: url.clone(),
        batch_size: 100, // 较大的批次
        flush_interval_ms: 300, // 较短的刷新间隔：300ms
        ..Default::default()
    };

    let mut sink = DatabaseSink::new(config).unwrap();

    // 只写入2条日志（远小于批次大小）
    for i in 0..2 {
        let record = LogRecord::new(
            Level::INFO,
            "timeout_test".into(),
            format!("Message {}", i).into(),
        );
        sink.write(&record).unwrap();
    }

    // 等待超时刷新（300ms + 缓冲时间）
    std::thread::sleep(Duration::from_millis(500));

    // 验证数据已写入数据库（通过超时刷新）
    let rt = tokio::runtime::Runtime::new().unwrap();
    let count = rt.block_on(async {
        use sea_orm::{ConnectionTrait, Database};
        let db = Database::connect(&url).await.unwrap();
        
        let res = db
            .query_one(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                "SELECT count(*) as count FROM logs".to_owned(),
            ))
            .await
            .unwrap();

        res.unwrap().try_get::<i64>("", "count").unwrap()
    });
    
    assert_eq!(count, 2, "超时刷新应该触发写入，当前记录数: {}", count);
    
    println!("超时刷新测试通过！刷新间隔: 300ms, 实际写入: {}", count);
}