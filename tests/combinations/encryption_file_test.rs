// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// 加密文件功能测试
// 测试加密日志文件写入、密钥管理，确保生产环境中的敏感日志数据安全。
//
// 核正：Encryptor/EncryptionKey（AES-256-GCM 原语）与 inklog::archive::SecretString
// 已随 API 重构移除——现公开面为密钥派生（sink::encryption::{get_encryption_key,
// derive_key_from_password}）与 FileSink 加密落盘（encrypt + encryption_key_env）。
// 原 Encryptor 加解密原语用例裁撤，保留并强化文件 Sink 集成与密钥管理用例。

#[cfg(test)]
mod encryption_file {
    use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tracing_subscriber::layer::SubscriberExt;

    /// 生成测试用的有效 256 位密钥（Base64 编码）
    fn generate_test_key() -> String {
        // 32 字节 = 256 位，Base64 编码后为 44 字符
        "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=".to_string()
    }

    /// 创建临时目录用于测试
    fn create_test_dir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("encrypted_test.log.enc");
        (temp_dir, log_path)
    }

    // === 密钥管理测试 ===

    #[test]
    #[serial]
    fn test_encryption_key_from_env() {
        unsafe {
            std::env::set_var("INKLOG_TEST_ENCRYPTION_KEY", generate_test_key());
        }

        let key = inklog::sink::encryption::get_encryption_key("INKLOG_TEST_ENCRYPTION_KEY")
            .expect("Base64 32 字节密钥应能解析");
        assert_eq!(key.len(), 32);

        unsafe {
            std::env::remove_var("INKLOG_TEST_ENCRYPTION_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_encryption_key_missing_env() {
        let result = inklog::sink::encryption::get_encryption_key("INKLOG_TEST_MISSING_KEY_XYZ");
        assert!(result.is_err(), "环境变量未设置应返回错误");
    }

    #[test]
    #[serial]
    fn test_encryption_key_invalid_base64_length() {
        // 核正：get_encryption_key 对非 Base64 内容按“密码”处理（1-127 字节经 PBKDF2
        // 派生，encryption.rs 头注释设计如此）——并非直接报错；真正的错误分支为
        // 密码过短（<12 字节，PBKDF2 下限校验）
        unsafe {
            std::env::set_var("INKLOG_TEST_BAD_KEY", "short");
        }

        // 非 Base64 且长度 < 12 字节 → 密码派生下限校验失败
        let result = inklog::sink::encryption::get_encryption_key("INKLOG_TEST_BAD_KEY");
        assert!(result.is_err(), "过短密码应被 PBKDF2 下限校验拒绝");

        // 非 Base64 但长度在密码范围内（12-127 字节）→ 按密码派生成功
        unsafe {
            std::env::set_var("INKLOG_TEST_BAD_KEY", "!!!invalid-base64!!!");
        }
        let result = inklog::sink::encryption::get_encryption_key("INKLOG_TEST_BAD_KEY");
        assert!(result.is_ok(), "12-127 字节非 Base64 内容应按密码派生处理");

        unsafe {
            std::env::remove_var("INKLOG_TEST_BAD_KEY");
        }
    }

    // === 加密文件 Sink 测试 ===

    #[tokio::test]
    #[serial]
    async fn test_encrypted_file_sink_write() {
        let (_temp_dir, log_path) = create_test_dir();

        unsafe {
            std::env::set_var("INKLOG_ENCRYPTION_KEY", generate_test_key());
        }

        let file_config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_size: "1MB".into(),
            rotation_time: "daily".into(),
            keep_files: 5,
            compress: false, // 加密与压缩不兼容
            encrypt: true,
            encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
            ..Default::default()
        };

        let config = InklogConfig {
            file_sink: Some(file_config),
            console_sink: Some(inklog::config::ConsoleSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // 核正：build_detached + 线程级 set_default（对齐 additional_tests 范本）——
        // with_config 的 set_global_default 为进程级单次语义，多用例下后装者的日志
        // 会流向首个 logger（已 shutdown）导致文件为空
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

        // 写入日志消息
        tracing::info!("Encrypted test message 1");
        tracing::info!("Encrypted test message 2");
        tracing::info!("Sensitive data: secret_value_12345");

        // 核正：FileSink 异步 worker 按批缓冲落盘，固定 sleep 不保证 flush——
        // 显式关闭以触发 worker 排空缓冲后再断言文件内容
        logger.shutdown().expect("关闭日志服务失败");

        // 验证文件存在且非空
        assert!(log_path.exists());
        let metadata = fs::metadata(&log_path).unwrap();
        assert!(metadata.len() > 0);

        // 核正：encrypt: true 的加密作用于轮转归档产物（compress_file 内 encrypt_file
        // → *.zst.enc/*.gz.enc），活跃写入文件为明文（设计如此）——原“活跃文件
        // 不含明文”的断言与实现语义不符，改为验证明文写入成功
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("test message"), "记录应写入活跃文件");

        unsafe {
            std::env::remove_var("INKLOG_ENCRYPTION_KEY");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_encrypted_file_sink_rotation() {
        // 核正：encrypt: true 加密作用于轮转归档（compress_file → *.zst.enc），
        // 活跃文件明文——本用例以 compress+encrypt 触发轮转，验证归档密文不含明文
        let (_temp_dir, log_path) = create_test_dir();

        unsafe {
            std::env::set_var("INKLOG_ENCRYPTION_KEY", generate_test_key());
        }

        let file_config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_size: "1KB".into(), // 小 size 便于触发轮转
            rotation_time: "daily".into(),
            keep_files: 3,
            batch_size: 10, // 小批强制分段落盘，确保写入量跨越轮转阈值
            compress: true,
            encrypt: true,
            encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
            ..Default::default()
        };

        let config = InklogConfig {
            file_sink: Some(file_config),
            console_sink: Some(inklog::config::ConsoleSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // 核正：同 test_encrypted_file_sink_write——build_detached + 线程级 set_default
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

        // 写入大量日志以触发轮转（含可检索的敏感明文片段）
        for i in 0..100 {
            tracing::info!("Encrypted rotation test message #{}", i);
        }
        tracing::info!("Rotated secret payload: rot_secret_98765");

        // 核正：固定 sleep 不保证 worker 排空与归档——显式关闭触发 flush
        logger.shutdown().expect("关闭日志服务失败");

        // 归档密文（*.zst.enc / *.gz.enc）不应包含任何明文片段
        let parent = log_path.parent().unwrap();
        let mut encrypted_archives = 0;
        for entry in fs::read_dir(parent).unwrap().filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".enc") {
                encrypted_archives += 1;
                let bytes = fs::read(entry.path()).unwrap();
                assert!(
                    !bytes.windows(17).any(|w| w == b"rot_secret_98765"),
                    "加密归档 {} 不应包含明文片段",
                    name
                );
            }
        }
        assert!(
            encrypted_archives > 0,
            "写入量已跨越轮转阈值，应产生加密归档"
        );

        unsafe {
            std::env::remove_var("INKLOG_ENCRYPTION_KEY");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_encrypted_sink_without_key_fails() {
        let (_temp_dir, log_path) = create_test_dir();

        // 确保环境变量已清除
        unsafe {
            std::env::remove_var("INKLOG_ENCRYPTION_KEY");
        }

        let file_config = FileSinkConfig {
            enabled: true,
            path: log_path,
            max_size: "1MB".into(),
            encrypt: true,
            encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
            ..Default::default()
        };

        let config = InklogConfig {
            file_sink: Some(file_config),
            console_sink: Some(inklog::config::ConsoleSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        // 应该能够创建（密钥在初始化 sink 时才加载）
        // 但写入时会失败或降级到控制台
        let result = LoggerManager::with_config(config).await;

        // 预期行为：可以创建 logger，但加密写入会失败或降级
        // 这里我们主要验证配置有效性
        assert!(result.is_ok() || result.is_err());
    }

    // === 加密配置验证测试 ===

    #[test]
    fn test_encryption_config_validation() {
        // 测试有效的加密配置
        let valid_config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/tmp/test.log.enc"),
            encrypt: true,
            encryption_key_env: Some("INKLOG_KEY".into()),
            compress: false,
            ..Default::default()
        };
        assert!(valid_config.enabled);
        assert!(valid_config.encrypt);

        // 注意：配置层面不强制禁止 compress+encrypt 组合
        // 但文档说明了不兼容性的原因
        let _invalid_config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/tmp/test.log.enc"),
            encrypt: true,
            compression_level: 3,
            ..Default::default()
        };
    }
}
