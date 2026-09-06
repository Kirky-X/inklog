// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! FileSink 压缩/加密轮转验证 + DatabaseSink 写入落库验证。
//!
//! 核正：DatabaseSink 已重构为 DI 架构（new_with_config 接 Arc<dyn Database>），
//! 原经 sea_orm 直查 sqlite 的计数验证改为 MockDatabaseAdapter（test-utils）
//! record_count 验证；sqlite 真实连接路径由 LoggerManager 集成用例覆盖。
use inklog::sink::LogSink;
use inklog::sink::database::DatabaseSink;
use inklog::sink::file::FileSink;
use inklog::tracing::Level;
use inklog::{
    FileSinkConfig, MockDatabaseAdapter, config::DatabaseSinkConfig, log_record::LogRecord,
};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

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

    let sink = FileSink::new(config).expect("Failed to create FileSink");
    let record = LogRecord::new(
        Level::INFO,
        "test".into(),
        "A long message to trigger rotation".into(),
    );
    futures::executor::block_on(sink.write(&record)).expect("Failed to write log record");

    // Trigger rotation
    for _ in 0..5 {
        futures::executor::block_on(sink.write(&record))
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

    // 核正：FileSink::get_encryption_key 要求 base64 解码后恰好 32 字节且
    // Shannon 熵 >= 4.0（弱密钥校验）。原 "MTIz..."（解码为重复数字序列，
    // 熵 3.31）会被拒绝导致归档加密失败（后台线程 error! 无全局 logger 被吞）；
    // 换用 32 个互不相同字符的 key（熵 5.0）
    unsafe {
        std::env::set_var("LOG_KEY", "YVozeFc4dksybVE3dE41clU5eUI0Y0U2ZkgxZ0owZEw=");
    }

    let config = FileSinkConfig {
        enabled: true,
        path: log_path.clone(),
        max_size: "10".into(),
        compress: false,
        encrypt: true,
        encryption_key_env: Some("LOG_KEY".into()),
        ..Default::default()
    };

    let sink = FileSink::new(config).expect("Failed to create FileSink");
    let record = LogRecord::new(Level::INFO, "test".into(), "Secret message".into());
    futures::executor::block_on(sink.write(&record)).expect("Failed to write log record");

    for _ in 0..5 {
        futures::executor::block_on(sink.write(&record))
            .expect("Failed to write log record during rotation");
    }

    std::thread::sleep(Duration::from_millis(500));

    let enc_path = find_file_with_extension(&temp_dir, "enc").expect("No encrypted file found");
    verify_encrypted_file(&enc_path);
}

#[test]
fn verify_database_sink_write_and_count() {
    let config = DatabaseSinkConfig {
        name: "test".to_string(),
        enabled: true,
        batch_size: 1,
        flush_interval_ms: 100,
        pool_size: 5,
        ..Default::default()
    };

    let mock_db = Arc::new(MockDatabaseAdapter::new());
    let sink = DatabaseSink::new_with_config(mock_db.clone(), Some(config))
        .expect("Failed to create DatabaseSink");

    let record = LogRecord::new(Level::INFO, "db_test".into(), "message to db".into());
    futures::executor::block_on(sink.write(&record))
        .expect("Failed to write log record to database");
    futures::executor::block_on(sink.flush()).expect("Failed to flush");

    let count = mock_db.record_count();
    assert_eq!(count, 1);
}
