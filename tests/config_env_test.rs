use inklog::InklogConfig;
use serial_test::serial;

fn clear_all_inklog_env_vars() {
    // 清除所有可能的 INKLOG_* 环境变量
    for (key, _) in std::env::vars() {
        if key.starts_with("INKLOG_") {
            std::env::remove_var(&key);
        }
    }
}

#[test]
#[serial]
fn test_config_from_env_overrides() {
    clear_all_inklog_env_vars();

    std::env::set_var("INKLOG_LEVEL", "debug");
    std::env::set_var("INKLOG_FILE_ENABLED", "true");
    std::env::set_var("INKLOG_FILE_PATH", "/tmp/test_logs/app.log");
    std::env::set_var("INKLOG_FILE_MAX_SIZE", "50MB");
    std::env::set_var("INKLOG_FILE_COMPRESS", "true");

    let mut config = InklogConfig::default();
    config.apply_env_overrides();

    // 验证环境变量覆盖生效
    assert_eq!(config.global.level, "debug");

    assert!(config.file_sink.is_some());
    let file = config.file_sink.unwrap();
    assert!(file.enabled);
    assert_eq!(file.max_size, "50MB");
    assert!(file.compress);
}

#[test]
#[serial]
fn test_config_env_override_s3_encryption() {
    clear_all_inklog_env_vars();

    // 设置 S3 加密环境变量
    std::env::set_var("INKLOG_S3_ENABLED", "true");
    std::env::set_var("INKLOG_S3_BUCKET", "test-bucket");
    std::env::set_var("INKLOG_S3_REGION", "us-west-2");
    std::env::set_var("INKLOG_S3_ENCRYPTION_ALGORITHM", "awskms");
    std::env::set_var("INKLOG_S3_ENCRYPTION_KMS_KEY_ID", "test-key-id");
    std::env::set_var("INKLOG_ARCHIVE_FORMAT", "parquet");

    let mut config = InklogConfig::default();
    config.apply_env_overrides();

    // 验证 S3 归档配置
    assert!(config.s3_archive.is_some());
    let s3 = config.s3_archive.unwrap();
    assert!(s3.enabled);
    assert_eq!(s3.bucket, "test-bucket");
    assert_eq!(s3.region, "us-west-2");
    assert!(s3.encryption.is_some());
    match &s3.encryption.unwrap().algorithm {
        inklog::archive::EncryptionAlgorithm::AwsKms => {} // 正确
        _ => panic!("Expected AwsKms encryption"),
    }
    assert_eq!(s3.archive_format, "parquet");
}

#[test]
#[serial]
fn test_config_env_override_http_server() {
    clear_all_inklog_env_vars();

    std::env::set_var("INKLOG_HTTP_ENABLED", "true");
    std::env::set_var("INKLOG_HTTP_HOST", "127.0.0.1");
    std::env::set_var("INKLOG_HTTP_PORT", "9090");
    std::env::set_var("INKLOG_HTTP_METRICS_PATH", "/prometheus");
    std::env::set_var("INKLOG_HTTP_HEALTH_PATH", "/status");

    let mut config = InklogConfig::default();
    config.apply_env_overrides();

    assert!(config.http_server.is_some());
    let http = config.http_server.unwrap();
    assert!(http.enabled);
    assert_eq!(http.host, "127.0.0.1");
    assert_eq!(http.port, 9090);
    assert_eq!(http.metrics_path, "/prometheus");
    assert_eq!(http.health_path, "/status");
}

#[test]
#[serial]
fn test_config_env_override_performance() {
    clear_all_inklog_env_vars();

    std::env::set_var("INKLOG_WORKER_THREADS", "8");
    std::env::set_var("INKLOG_CHANNEL_CAPACITY", "20000");

    let mut config = InklogConfig::default();
    config.apply_env_overrides();

    assert_eq!(config.performance.worker_threads, 8);
    assert_eq!(config.performance.channel_capacity, 20000);
}
