// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 复杂特性组合测试

#[cfg(all(feature = "aws", feature = "dbnexus"))]
#[cfg(test)]
mod complex_features_test {
    use inklog::{DatabaseSinkConfig, FileSinkConfig, InklogConfig, LoggerManager};
    use inklog::archive::CompressionType;
    use inklog::config::DatabaseDriver;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;
    use tokio::time::sleep;
    use std::time::Duration;

    #[tokio::test]
    #[serial]
    async fn test_encrypted_compressed_s3_database() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("complex_test.log.enc");
        let db_path = temp_dir.path().join("complex_test.db");
        
        // 设置加密密钥
        let encryption_key = "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=";
        env::set_var("INKLOG_ENCRYPTION_KEY", encryption_key);
        
        let config = InklogConfig {
            file_sink: Some(FileSinkConfig {
                enabled: true,
                path: log_path.clone(),
                max_size: "50MB".into(),
                compress: false,
                encrypt: true,
                encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
                ..Default::default()
            }),
            database_sink: Some(DatabaseSinkConfig {
                enabled: true,
                driver: DatabaseDriver::SQLite,
                url: format!("sqlite://{}", db_path.display()),
                pool_size: 3,
                batch_size: 50,
                flush_interval_ms: 1000,
                table_name: "logs".to_string(),
                ..Default::default()
            }),
            #[cfg(feature = "aws")]
            s3_archive: Some(inklog::S3ArchiveConfig {
                enabled: true,
                bucket: "test-bucket".to_string(),
                region: "us-east-1".to_string(),
                compression: CompressionType::Zstd,
                ..Default::default()
            }),
            ..Default::default()
        };

        let logger = LoggerManager::with_config(config).await.unwrap();
        
        // 写入加密数据
        for i in 0..500 {
            log::info!(target: "complex_test", "Encrypted message {}", i);
        }
        
        sleep(Duration::from_secs(2)).await;
        
        // 验证数据
        assert!(log_path.exists());
        assert!(db_path.exists());
        
        // 验证健康状态
        let health = logger.get_health_status();
        assert!(health.sinks.contains_key("file"));
        assert!(health.sinks.contains_key("database"));
        
        env::remove_var("INKLOG_ENCRYPTION_KEY");
    }
}
