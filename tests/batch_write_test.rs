use inklog::config::DatabaseDriver;
use inklog::sink::database::DatabaseSink;
use inklog::sink::LogSink;
use inklog::{log_record::LogRecord, DatabaseSinkConfig};
use std::time::Duration;
use tracing::Level;

#[test]
fn test_database_batch_write() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("batch_test.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let config = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: url.clone(),
        batch_size: 5,
        flush_interval_ms: 1000,
        ..Default::default()
    };

    let mut sink = DatabaseSink::new(config).expect("Failed to create DatabaseSink");

    // Write 3 records (buffer=3, not enough to trigger batch flush)
    for i in 0..3 {
        let record = LogRecord::new(Level::INFO, "batch_test".into(), format!("Message {}", i));
        sink.write(&record).expect("Failed to write log record");
    }

    // Wait for flush interval to pass
    std::thread::sleep(Duration::from_millis(1100));

    // Write 4th record - this triggers time-based flush (3 records flushed)
    let record = LogRecord::new(Level::INFO, "batch_test".into(), "Trigger flush".into());
    sink.write(&record).expect("Failed to write log record");

    // Wait for flush to complete
    std::thread::sleep(Duration::from_millis(200));

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let count_before = rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(&url)
            .await
            .expect("Failed to connect to database");
        let logs = Entity::find().all(&db).await.expect("Failed to query logs");
        logs.len() as i64
    });
    // After time-based flush, 4 records should be in DB
    assert_eq!(count_before, 4, "时间刷新应该写入4条记录");

    // Write 5 more records to trigger batch-based flush (batch_size=5)
    for i in 4..9 {
        let record = LogRecord::new(Level::INFO, "batch_test".into(), format!("Message {}", i));
        sink.write(&record).expect("Failed to write log record");
    }

    // Wait for batch flush to complete
    std::thread::sleep(Duration::from_millis(500));

    let count_after = rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(&url)
            .await
            .expect("Failed to connect to database");
        let logs = Entity::find().all(&db).await.expect("Failed to query logs");
        logs.len() as i64
    });
    // Total should be 4 (first flush) + 5 (batch flush) = 9
    assert_eq!(
        count_after, 9,
        "批次写入应该触发，当前记录数: {}",
        count_after
    );

    println!("批量写入测试通过！批次大小: 5, 实际写入: {}", count_after);
}

#[test]
fn test_database_timeout_flush() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("timeout_test.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let config = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: url.clone(),
        batch_size: 100,
        flush_interval_ms: 300,
        ..Default::default()
    };

    let mut sink = DatabaseSink::new(config).expect("Failed to create DatabaseSink");

    let record1 = LogRecord::new(Level::INFO, "timeout_test".into(), "First message".into());
    sink.write(&record1)
        .expect("Failed to write first log record");

    std::thread::sleep(Duration::from_millis(500));

    let record2 = LogRecord::new(Level::INFO, "timeout_test".into(), "Second message".into());
    sink.write(&record2)
        .expect("Failed to write second log record");

    std::thread::sleep(Duration::from_millis(500));

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let count = rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(&url)
            .await
            .expect("Failed to connect to database");
        let logs = Entity::find().all(&db).await.expect("Failed to query logs");
        logs.len() as i64
    });

    assert!(count >= 1, "超时刷新应该触发写入，当前记录数: {}", count);

    println!("超时刷新测试通过！刷新间隔: 300ms, 实际写入: {}", count);
}
