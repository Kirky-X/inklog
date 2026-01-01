use inklog::config::DatabaseDriver;
use inklog::sink::database::DatabaseSink;
use inklog::sink::file::FileSink;
use inklog::sink::LogSink;
use inklog::{log_record::LogRecord, DatabaseSinkConfig, FileSinkConfig};
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use tempfile::TempDir;
use tracing::Level;

fn get_log_count(url: &str) -> i64 {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(url).await.unwrap();
        let logs = Entity::find().all(&db).await.unwrap();
        logs.len() as i64
    })
}

#[test]
fn verify_file_sink_compression() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let config = FileSinkConfig {
        enabled: true,
        path: log_path.clone(),
        max_size: "10".into(), // 10 bytes
        compress: true,
        encrypt: false,
        ..Default::default()
    };

    let mut sink = FileSink::new(config).unwrap();
    let record = LogRecord::new(
        Level::INFO,
        "test".into(),
        "A long message to trigger rotation".into(),
    );
    sink.write(&record).unwrap();

    // Trigger rotation
    for _ in 0..5 {
        sink.write(&record).unwrap();
    }

    // Wait for background compression
    std::thread::sleep(Duration::from_millis(1000));

    let entries = std::fs::read_dir(temp_dir.path()).unwrap();
    let mut found_zst = false;
    for entry in entries {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "zst") {
            found_zst = true;
            let mut file = File::open(path).unwrap();
            let mut magic = [0u8; 4];
            file.read_exact(&mut magic).unwrap();
            // Zstd magic: 0xFD2FB528 (LE: 28 B5 2F FD)
            assert_eq!(magic, [0x28, 0xB5, 0x2F, 0xFD]);
        }
    }
    assert!(found_zst, "No compressed file found");
}

#[test]
fn verify_file_sink_encryption() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("enc.log");

    // Use a proper base64-encoded 32-byte key (44 characters)
    std::env::set_var("LOG_KEY", "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=");

    let config = FileSinkConfig {
        enabled: true,
        path: log_path.clone(),
        max_size: "10".into(),
        compress: false,
        encrypt: true,
        encryption_key_env: Some("LOG_KEY".into()),
        ..Default::default()
    };

    let mut sink = FileSink::new(config).unwrap();
    let record = LogRecord::new(Level::INFO, "test".into(), "Secret message".into());
    sink.write(&record).unwrap();

    for _ in 0..5 {
        sink.write(&record).unwrap();
    }

    std::thread::sleep(Duration::from_millis(500));

    let entries = std::fs::read_dir(temp_dir.path()).unwrap();
    let mut found_enc = false;
    for entry in entries {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "enc") {
            found_enc = true;
            let metadata = std::fs::metadata(path).unwrap();
            assert!(
                metadata.len() > 12,
                "Encrypted file should have nonce (12 bytes) + ciphertext"
            );
        }
    }
    assert!(found_enc, "No encrypted file found");
}

#[test]
fn verify_database_sink_sqlite() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("logs.db");

    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let config = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: url.clone(),
        batch_size: 1,
        flush_interval_ms: 100,
        ..Default::default()
    };

    let mut sink = DatabaseSink::new(config).unwrap();

    let record = LogRecord::new(Level::INFO, "db_test".into(), "message to db".into());
    sink.write(&record).unwrap();

    std::thread::sleep(Duration::from_millis(500));

    // Verify
    let count = get_log_count(&url);
    assert_eq!(count, 1);
}
