use inklog::config::{HttpErrorMode, HttpServerConfig};
use inklog::InklogConfig;
use serial_test::serial;

fn clear_inklog_env() {
    for (key, _) in std::env::vars() {
        if key.starts_with("INKLOG_") {
            std::env::remove_var(&key);
        }
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
        error_mode: HttpErrorMode::Panic,
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
async fn test_http_server_error_mode_panic() {
    clear_inklog_env();

    let config = HttpServerConfig {
        enabled: true,
        host: "127.0.0.1".to_string(),
        port: 18081,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Panic,
    };

    match config.error_mode {
        HttpErrorMode::Panic => {}
        _ => panic!("Expected Panic mode"),
    }
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

    std::env::set_var("INKLOG_HTTP_ENABLED", "true");
    std::env::set_var("INKLOG_HTTP_HOST", "127.0.0.1");
    std::env::set_var("INKLOG_HTTP_PORT", "18084");
    std::env::set_var("INKLOG_HTTP_ERROR_MODE", "warn");

    let mut config = InklogConfig::default();
    config.apply_env_overrides();

    assert!(config.http_server.is_some());
    let http = config.http_server.unwrap();
    assert!(http.enabled);
    assert_eq!(http.host, "127.0.0.1");
    assert_eq!(http.port, 18084);
    match http.error_mode {
        HttpErrorMode::Warn => {}
        _ => panic!("Expected Warn mode from env"),
    }

    std::env::remove_var("INKLOG_HTTP_ENABLED");
    std::env::remove_var("INKLOG_HTTP_HOST");
    std::env::remove_var("INKLOG_HTTP_PORT");
    std::env::remove_var("INKLOG_HTTP_ERROR_MODE");
}

#[tokio::test]
#[serial]
async fn test_http_metrics_path_configuration() {
    clear_inklog_env();

    std::env::set_var("INKLOG_HTTP_ENABLED", "true");
    std::env::set_var("INKLOG_HTTP_METRICS_PATH", "/prometheus/metrics");
    std::env::set_var("INKLOG_HTTP_HEALTH_PATH", "/status");

    let mut config = InklogConfig::default();
    config.apply_env_overrides();

    let http = config
        .http_server
        .expect("http_server should be Some after setting INKLOG_HTTP_ENABLED");
    assert_eq!(http.metrics_path, "/prometheus/metrics");
    assert_eq!(http.health_path, "/status");
}

#[tokio::test]
#[serial]
async fn test_http_server_disabled_by_default() {
    clear_inklog_env();

    let mut config = InklogConfig::default();
    config.apply_env_overrides();

    assert!(
        config.http_server.is_none(),
        "INKLOG_HTTP_ENABLED should not be set"
    );
}
