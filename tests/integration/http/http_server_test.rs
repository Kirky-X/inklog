// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! HTTP 服务器配置与错误模式测试。
//!
//! 核正：`HttpServerConfig` 新增 `auth`/`ip_whitelist`/`tls` 字段（以
//! `..Default::default()` 补齐）；`HttpErrorMode` 仅剩 `Warn`/`Strict`
//! （`Panic` 变体已移除，默认 `Strict`）；环境变量覆盖经公开入口
//! `InklogConfig::load_with_env_overrides()`，变量名为 `INKLOG_HTTP_SERVER_*`。
use inklog::InklogConfig;
use inklog::config::{HttpErrorMode, HttpServerConfig};
use serial_test::serial;

fn clear_inklog_env() {
    // 先收集再删除，避免在 env::vars() 迭代期间修改环境变量（UB 隐患）
    let keys: Vec<String> = std::env::vars()
        .map(|(key, _)| key)
        .filter(|key| key.starts_with("INKLOG_"))
        .collect();
    for key in keys {
        unsafe {
            std::env::remove_var(&key);
        }
    }
}

/// 将配置搜索路径固定到空 TOML 文件，避免读到宿主机已有 inklog 配置
fn isolate_config_paths() {
    let dir = std::env::temp_dir().join("inklog_test_config_isolation");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("empty_config.toml");
    if !path.exists() {
        std::fs::write(&path, "").unwrap();
    }
    unsafe {
        std::env::set_var("INKLOG_CONFIG_PATH", path.to_str().unwrap());
    }
}

#[tokio::test]
#[serial]
async fn test_http_server_startup_with_default_config() {
    clear_inklog_env();

    let port = 18080
        + std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u16
            % 10000;

    let config = HttpServerConfig {
        enabled: true,
        host: "127.0.0.1".to_string(),
        port,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Strict,
        ..Default::default()
    };

    let inklog_config = InklogConfig {
        http_server: Some(config),
        ..Default::default()
    };

    assert!(inklog_config.http_server.is_some());
    let http = inklog_config.http_server.unwrap();
    assert!(http.enabled);
    assert_eq!(http.port, port);
}

#[tokio::test]
#[serial]
async fn test_http_server_error_mode_default_strict() {
    clear_inklog_env();

    // HttpErrorMode 仅 Warn/Strict（Panic 已移除），#[default] 为 Strict
    let config = HttpServerConfig::default();
    assert!(
        matches!(config.error_mode, HttpErrorMode::Strict),
        "默认错误模式应为 Strict"
    );
}

#[tokio::test]
#[serial]
async fn test_http_server_error_mode_warn() {
    clear_inklog_env();

    let config = HttpServerConfig {
        enabled: true,
        host: "127.0.0.1".to_string(),
        port: 18082,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Warn,
        ..Default::default()
    };

    match config.error_mode {
        HttpErrorMode::Warn => {}
        _ => panic!("Expected Warn mode"),
    }
}

#[tokio::test]
#[serial]
async fn test_http_server_error_mode_strict() {
    clear_inklog_env();

    let config = HttpServerConfig {
        enabled: true,
        host: "127.0.0.1".to_string(),
        port: 18083,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Strict,
        ..Default::default()
    };

    match config.error_mode {
        HttpErrorMode::Strict => {}
        _ => panic!("Expected Strict mode"),
    }
}

#[tokio::test]
#[serial]
async fn test_http_server_with_logger_manager() {
    clear_inklog_env();
    isolate_config_paths();

    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
    }
    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_HOST", "127.0.0.1");
    }
    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_PORT", "18084");
    }
    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "warn");
    }

    let config = InklogConfig::load_with_env_overrides().expect("加载配置失败");

    let http = config.http_server.expect("http_server 应被环境变量启用");
    assert!(http.enabled);
    assert_eq!(http.host, "127.0.0.1");
    assert_eq!(http.port, 18084);
    match http.error_mode {
        HttpErrorMode::Warn => {}
        _ => panic!("Expected Warn mode from env"),
    }
}

#[tokio::test]
#[serial]
async fn test_http_metrics_path_configuration() {
    clear_inklog_env();
    isolate_config_paths();

    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
    }
    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_METRICS_PATH", "/prometheus/metrics");
    }
    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_HEALTH_PATH", "/status");
    }

    let config = InklogConfig::load_with_env_overrides().expect("加载配置失败");

    let http = config
        .http_server
        .expect("http_server should be Some after setting INKLOG_HTTP_SERVER_ENABLED");
    assert_eq!(http.metrics_path, "/prometheus/metrics");
    assert_eq!(http.health_path, "/status");
}

#[tokio::test]
#[serial]
async fn test_http_server_disabled_by_default() {
    clear_inklog_env();
    isolate_config_paths();

    let config = InklogConfig::load_with_env_overrides().expect("加载配置失败");

    assert!(
        config.http_server.is_none(),
        "INKLOG_HTTP_SERVER_ENABLED should not be set"
    );
}
