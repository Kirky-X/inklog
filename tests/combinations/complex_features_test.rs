// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// 复杂特性组合测试

#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
#[cfg(test)]
mod complex_features {
    use inklog::config::DatabaseDriver;
    use inklog::tokio::time::sleep;
    use inklog::{
        ConsoleSinkConfig, DatabaseSinkConfig, FileSinkConfig, InklogConfig, LoggerManager,
    };
    use serial_test::serial;
    use std::env;
    use std::time::Duration;
    use tempfile::TempDir;
    use tracing_subscriber::layer::SubscriberExt;

    #[tokio::test]
    #[serial]
    async fn test_encrypted_compressed_database() {
        let temp_dir = TempDir::new().unwrap();
        // 活跃写入文件恒为明文（encrypt 仅作用于轮转归档），命名不带 .enc
        let log_path = temp_dir.path().join("complex_test.log");
        let db_path = temp_dir.path().join("complex_test.db");

        // 设置加密密钥：32 字节互异字符（熵 5.0），满足 get_encryption_key 的
        // 弱熵校验（>= 4.0）；与 comprehensive_validation_test 共用同一夹具 key
        let encryption_key = "YVozeFc4dksybVE3dE41clU5eUI0Y0U2ZkgxZ0owZEw=";
        unsafe {
            env::set_var("INKLOG_ENCRYPTION_KEY", encryption_key);
        }

        let config = InklogConfig {
            file_sink: Some(FileSinkConfig {
                enabled: true,
                path: log_path.clone(),
                // 500 条消息约 50KB：阈值压到 16KB 强制触发尺寸轮转，
                // 使 rotate_inner 的归档加密路径真实执行（50MB 恒不触发）
                max_size: "16KB".into(),
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
            // worker 线程的 stdout 直写不在测试 harness 捕获范围内，禁用以免
            // 泄漏千行日志污染进程 stdout（本用例不校验 console 输出）
            console_sink: Some(ConsoleSinkConfig {
                enabled: false,
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

        // 轮转归档由后台线程异步加密：轮询等待归档产物出现并稳定，
        // 最多 10s（flush_interval + 后台加密线程的调度裕量）
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut encrypted_archives: Vec<std::path::PathBuf> = Vec::new();
        while std::time::Instant::now() < deadline {
            encrypted_archives = std::fs::read_dir(temp_dir.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    name.starts_with("complex_test_") && name.ends_with(".enc")
                })
                .collect();
            if !encrypted_archives.is_empty() {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }

        // 验证数据
        assert!(log_path.exists());
        assert!(db_path.exists());

        // 加密路径真实验证：轮转归档必须存在且为密文（nonce + AES-256-GCM，
        // 明文消息不得以任何形式出现——含压缩前明文的残留）
        assert!(
            !encrypted_archives.is_empty(),
            "16KB threshold must have triggered size rotation with encrypted archives"
        );
        for archive in &encrypted_archives {
            let bytes = std::fs::read(archive).unwrap();
            // v2 最小合法密文 = magic(8)+version(2)+algo(2)+salt(16)+nonce(12)
            // +GCM tag(16) = 56 字节；下界不足会让窗口检查空真通过
            assert!(
                bytes.len() >= 56,
                "encrypted archive must hold a full v2 header + nonce + tag (>= 56 bytes): {}",
                archive.display()
            );
            assert!(
                !bytes
                    .windows(b"Encrypted message".len())
                    .any(|w| w == b"Encrypted message"),
                "rotated archive must be ciphertext, got plaintext in: {}",
                archive.display()
            );
        }

        // 验证健康状态
        let health = logger.get_health_status();
        assert!(health.sinks.contains_key("file"));
        assert!(health.sinks.contains_key("database"));

        unsafe {
            env::remove_var("INKLOG_ENCRYPTION_KEY");
        }
    }
}
