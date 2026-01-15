use inklog::config::DatabaseDriver;
use inklog::sink::database::DatabaseSink;
use inklog::sink::file::FileSink;
use inklog::sink::LogSink;
use inklog::{log_record::LogRecord, DatabaseSinkConfig, FileSinkConfig};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tracing::Level;

// ============ Database Helper Functions ============

fn get_log_count(url: &str) -> i64 {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(url)
            .await
            .expect("Failed to connect to database");
        let logs = Entity::find().all(&db).await.expect("Failed to query logs");
        logs.len() as i64
    })
}

// ============ File Helper Functions ============

/// Finds a file with the specified extension in a directory
fn find_file_with_extension(dir: &TempDir, extension: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir.path()).expect("Failed to read temp directory");
    entries
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|ext| ext == extension))
}

/// Verifies that a file is compressed with Zstandard
fn verify_zstd_compression(file_path: &PathBuf) {
    let mut file = File::open(file_path).expect("Failed to open compressed file");
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .expect("Failed to read file magic bytes");
    // Zstd magic: 0xFD2FB528 (LE: 28 B5 2F FD)
    assert_eq!(magic, [0x28, 0xB5, 0x2F, 0xFD]);
}

/// Verifies that a file is encrypted (has nonce + ciphertext)
fn verify_encrypted_file(file_path: &PathBuf) {
    let metadata = std::fs::metadata(file_path).expect("Failed to get file metadata");
    assert!(
        metadata.len() > 12,
        "Encrypted file should have nonce (12 bytes) + ciphertext"
    );
}

// ============ Verification Tests ============

#[test]
fn verify_file_sink_compression() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let log_path = temp_dir.path().join("test.log");

    let config = FileSinkConfig {
        enabled: true,
        path: log_path.clone(),
        max_size: "10".into(),
        compress: true,
        encrypt: false,
        ..Default::default()
    };

    let mut sink = FileSink::new(config).expect("Failed to create FileSink");
    let record = LogRecord::new(
        Level::INFO,
        "test".into(),
        "A long message to trigger rotation".into(),
    );
    sink.write(&record).expect("Failed to write log record");

    // Trigger rotation
    for _ in 0..5 {
        sink.write(&record)
            .expect("Failed to write log record during rotation");
    }

    // Wait for background compression
    std::thread::sleep(Duration::from_millis(1000));

    let zst_path = find_file_with_extension(&temp_dir, "zst").expect("No compressed file found");
    verify_zstd_compression(&zst_path);
}

#[test]
fn verify_file_sink_encryption() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
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

    let mut sink = FileSink::new(config).expect("Failed to create FileSink");
    let record = LogRecord::new(Level::INFO, "test".into(), "Secret message".into());
    sink.write(&record).expect("Failed to write log record");

    for _ in 0..5 {
        sink.write(&record)
            .expect("Failed to write log record during rotation");
    }

    std::thread::sleep(Duration::from_millis(500));

    let enc_path = find_file_with_extension(&temp_dir, "enc").expect("No encrypted file found");
    verify_encrypted_file(&enc_path);
}

#[test]
fn verify_database_sink_sqlite() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
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

    let mut sink = DatabaseSink::new(config).expect("Failed to create DatabaseSink");

    let record = LogRecord::new(Level::INFO, "db_test".into(), "message to db".into());
    sink.write(&record)
        .expect("Failed to write log record to database");

    std::thread::sleep(Duration::from_millis(500));

    let count = get_log_count(&url);
    assert_eq!(count, 1);
}
