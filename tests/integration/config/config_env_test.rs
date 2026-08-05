// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
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
