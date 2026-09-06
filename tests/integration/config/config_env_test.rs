// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 环境变量覆盖配置测试。
//!
//! 核正：`apply_env_overrides` 为私有关联函数，公开入口是
//! `InklogConfig::load_with_env_overrides()`；且实际读取的环境变量名为
//! `INKLOG_GLOBAL_*` / `INKLOG_FILE_SINK_*` / `INKLOG_HTTP_SERVER_*` /
//! `INKLOG_PERFORMANCE_*` 前缀（与实现一致），非旧用例假设的短前缀名。
use inklog::InklogConfig;
use serial_test::serial;

fn clear_all_inklog_env_vars() {
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

#[test]
#[serial]
fn test_config_from_env_overrides() {
    clear_all_inklog_env_vars();
    isolate_config_paths();

    unsafe {
        std::env::set_var("INKLOG_GLOBAL_LEVEL", "debug");
    }
    unsafe {
        std::env::set_var("INKLOG_FILE_SINK_ENABLED", "true");
    }
    unsafe {
        std::env::set_var("INKLOG_FILE_SINK_PATH", "/tmp/inklog_test_logs/app.log");
    }
    unsafe {
        std::env::set_var("INKLOG_FILE_SINK_MAX_SIZE", "50MB");
    }

    let config = InklogConfig::load_with_env_overrides().expect("加载配置失败");

    // 验证环境变量覆盖生效
    assert_eq!(config.global.level, "debug");

    let file = config.file_sink.expect("file_sink 应被环境变量启用");
    assert!(file.enabled);
    assert_eq!(file.max_size, "50MB");
}

#[test]
#[serial]
fn test_config_env_override_http_server() {
    clear_all_inklog_env_vars();
    isolate_config_paths();

    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
    }
    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_HOST", "127.0.0.1");
    }
    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_PORT", "9090");
    }
    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_METRICS_PATH", "/prometheus");
    }
    unsafe {
        std::env::set_var("INKLOG_HTTP_SERVER_HEALTH_PATH", "/status");
    }

    let config = InklogConfig::load_with_env_overrides().expect("加载配置失败");

    let http = config.http_server.expect("http_server 应被环境变量启用");
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
    isolate_config_paths();

    unsafe {
        std::env::set_var("INKLOG_PERFORMANCE_WORKER_THREADS", "8");
    }
    unsafe {
        std::env::set_var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY", "20000");
    }

    let config = InklogConfig::load_with_env_overrides().expect("加载配置失败");

    assert_eq!(config.performance.worker_threads, 8);
    assert_eq!(config.performance.channel_capacity, 20000);
}
