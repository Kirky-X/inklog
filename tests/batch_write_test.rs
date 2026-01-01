use inklog::config::DatabaseDriver;
use inklog::sink::database::DatabaseSink;
use inklog::sink::LogSink;
use inklog::{log_record::LogRecord, DatabaseSinkConfig};
use std::time::Duration;
use tracing::Level;

#[test]
fn test_database_batch_write() {
    let temp_dir = tempfile::TempDir::new().unwrap();
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

    let mut sink = DatabaseSink::new(config).unwrap();

    for i in 0..3 {
        let record = LogRecord::new(Level::INFO, "batch_test".into(), format!("Message {}", i));
        sink.write(&record).unwrap();
    }

    std::thread::sleep(Duration::from_millis(200));

    let rt = tokio::runtime::Runtime::new().unwrap();
    let count_before = rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(&url).await.unwrap();
        let logs = Entity::find().all(&db).await.unwrap();
        logs.len() as i64
    });
    assert_eq!(count_before, 0, "数据不应该在批次未满时写入");

    for i in 3..6 {
        let record = LogRecord::new(Level::INFO, "batch_test".into(), format!("Message {}", i));
        sink.write(&record).unwrap();
    }

    std::thread::sleep(Duration::from_millis(500));

    let count_after = rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(&url).await.unwrap();
        let logs = Entity::find().all(&db).await.unwrap();
        logs.len() as i64
    });
    assert!(
        count_after >= 5,
        "批次写入应该触发，当前记录数: {}",
        count_after
    );

    println!("批量写入测试通过！批次大小: 5, 实际写入: {}", count_after);
}

#[test]
fn test_database_timeout_flush() {
    let temp_dir = tempfile::TempDir::new().unwrap();
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

    let mut sink = DatabaseSink::new(config).unwrap();

    let record1 = LogRecord::new(Level::INFO, "timeout_test".into(), "First message".into());
    sink.write(&record1).unwrap();

    std::thread::sleep(Duration::from_millis(500));

    let record2 = LogRecord::new(Level::INFO, "timeout_test".into(), "Second message".into());
    sink.write(&record2).unwrap();

    std::thread::sleep(Duration::from_millis(500));

    let rt = tokio::runtime::Runtime::new().unwrap();
    let count = rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(&url).await.unwrap();
        let logs = Entity::find().all(&db).await.unwrap();
        logs.len() as i64
    });

    assert!(count >= 1, "超时刷新应该触发写入，当前记录数: {}", count);

    println!("超时刷新测试通过！刷新间隔: 300ms, 实际写入: {}", count);
}
