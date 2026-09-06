// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// 复杂特性组合测试

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
#[cfg(test)]
mod complex_features {
    use inklog::config::DatabaseDriver;
    use inklog::tokio::time::sleep;
    use inklog::{DatabaseSinkConfig, FileSinkConfig, InklogConfig, LoggerManager};
    use serial_test::serial;
    use std::env;
    use std::time::Duration;
    use tempfile::TempDir;
    use tracing_subscriber::layer::SubscriberExt;

    #[tokio::test]
    #[serial]
    async fn test_encrypted_compressed_database() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("complex_test.log.enc");
        let db_path = temp_dir.path().join("complex_test.db");

        // 设置加密密钥
        let encryption_key = "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=";
        unsafe {
            env::set_var("INKLOG_ENCRYPTION_KEY", encryption_key);
        }

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
                // ?mode=rwc：sqlx 默认不创建缺失文件（本仓 sqlite 测试既定口径）
                url: format!("sqlite://{}?mode=rwc", db_path.display()),
                pool_size: 3,
                batch_size: 50,
                flush_interval_ms: 1000,
                table_name: "logs".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        // 核正：build_detached + 线程级 set_default（对齐 additional_tests 范本）——
        // with_config 的 set_global_default 为进程级单次语义，多用例下后装者的日志
        // 会流向首个 logger（已 shutdown）导致记录丢失
        let (logger, subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            None,
        )
        .await
        .unwrap();
        let _guard = tracing::subscriber::set_default(
            tracing_subscriber::registry().with(subscriber).with(filter),
        );

        // 写入加密数据
        for i in 0..500 {
            tracing::info!(target: "complex_test", "Encrypted message {}", i);
        }

        sleep(Duration::from_secs(2)).await;

        // 验证数据
        assert!(log_path.exists());
        assert!(db_path.exists());

        // 验证健康状态
        let health = logger.get_health_status();
        assert!(health.sinks.contains_key("file"));
        assert!(health.sinks.contains_key("database"));

        unsafe {
            env::remove_var("INKLOG_ENCRYPTION_KEY");
        }
    }
}
