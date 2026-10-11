// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! HTTP 认证与 IP 白名单示例
//!
//! 演示 `inklog::HttpAuthConfig` 和 `HttpServerConfig.ip_whitelist` 的配置：
//!
//! 1. `HttpAuthConfig` Bearer Token 认证配置
//! 2. `ip_whitelist` IP 白名单配置（支持精确 IP / CIDR / 末尾前缀通配符）
//! 3. `HttpErrorMode` 启动失败处理模式（Warn vs Strict）
//! 4. 完整的安全配置示例
//!
//! # 运行
//!
//! ```bash
//! cargo run --package inklog-examples --bin http_auth
//! ```

use inklog::InklogConfig;
use inklog::config::{HttpAuthConfig, HttpErrorMode, HttpServerConfig};
use inklog_examples::common::{print_section, print_separator};

fn main() {
    print_separator("inklog HTTP auth and IP whitelist examples");

    show_auth_config();
    show_ip_whitelist();
    show_error_modes();
    show_full_secure_config();
    show_env_overrides();

    println!("\nAll HTTP auth examples shown.");
}

/// 演示 HttpAuthConfig Bearer Token 认证
fn show_auth_config() {
    print_section("Example 1: HttpAuthConfig bearer token auth");

    let default_auth = HttpAuthConfig::default();
    println!("Default HttpAuthConfig:");
    println!(
        "  enabled   = {} (disabled by default)",
        default_auth.enabled
    );
    println!(
        "  token_env = \"{}\" (environment variable name)",
        default_auth.token_env
    );

    println!("\nEnable auth:");
    let enabled_auth = HttpAuthConfig {
        enabled: true,
        token_env: "INKLOG_HTTP_AUTH_TOKEN".to_string(),
    };
    println!("  enabled   = {}", enabled_auth.enabled);
    println!("  token_env = \"{}\"", enabled_auth.token_env);

    println!("\nHow it works:");
    println!(
        "  1. The client sends: curl -H \"Authorization: Bearer $TOKEN\" http://localhost:9090/metrics"
    );
    println!(
        "  2. The server reads the expected token from environment variable {}",
        enabled_auth.token_env
    );
    println!("  3. It compares the bearer token in the Authorization header");
    println!("  4. A match returns data, a mismatch returns 401 Unauthorized");

    println!("\nCustom token environment variable:");
    let custom_auth = HttpAuthConfig {
        enabled: true,
        token_env: "MY_APP_AUTH_TOKEN".to_string(),
    };
    println!("  enabled   = {}", custom_auth.enabled);
    println!("  token_env = \"{}\" (custom)", custom_auth.token_env);
}

/// 演示 IP 白名单配置
fn show_ip_whitelist() {
    print_section("Example 2: IP whitelist (ip_whitelist)");

    println!("Supported formats:");
    println!("  1. Exact IP:    \"192.168.1.100\"");
    println!("  2. CIDR range:  \"10.0.0.0/8\"");
    println!(
        "  3. Prefix wildcard: \"192.168.*\" (a trailing .* only, with 1-3 numeric segments before it)"
    );
    println!(
        "     Note: \"192.168.*.*\" is not valid — a wildcard is allowed only in the last segment"
    );

    println!("\nConfig example 1: internal network only");
    let internal_only = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9090,
        ip_whitelist: Some(vec![
            "10.0.0.0/8".to_string(),
            "192.168.0.0/16".to_string(),
            "192.168.*".to_string(), // 前缀通配符：仅末尾一段 .*
            "127.0.0.1".to_string(),
        ]),
        ..Default::default()
    };
    println!("  ip_whitelist = {:?}", internal_only.ip_whitelist);

    // 用公开校验 API 检查白名单条目：坏条目在配置期直接报错，
    // 不会等到运行期静默 fail-closed（拒绝所有请求）
    InklogConfig {
        http_server: Some(internal_only.clone()),
        ..Default::default()
    }
    .validate()
    .expect("the whitelist entries should pass config validation");
    println!("  ✓ InklogConfig::validate() passed");

    // 反例：多段通配（"192.168.*.*"）不是受支持格式，配置期即被拒绝
    let invalid = HttpServerConfig {
        enabled: true,
        ip_whitelist: Some(vec!["192.168.*.*".to_string()]),
        ..Default::default()
    };
    let err = InklogConfig {
        http_server: Some(invalid),
        ..Default::default()
    }
    .validate()
    .expect_err("a multi-segment wildcard should be rejected at config time");
    println!("  ✓ the invalid \"192.168.*.*\" was rejected: {}", err);

    println!("\nConfig example 2: specific monitoring hosts only");
    let monitoring_only = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9090,
        ip_whitelist: Some(vec!["10.0.1.50".to_string(), "10.0.1.51".to_string()]),
        ..Default::default()
    };
    println!("  ip_whitelist = {:?}", monitoring_only.ip_whitelist);

    println!("\nConfig example 3: whitelist disabled (all IPs allowed)");
    let open = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9090,
        ip_whitelist: None,
        ..Default::default()
    };
    println!(
        "  ip_whitelist = {:?} (None = allow all)",
        open.ip_whitelist
    );
}

/// 演示 HttpErrorMode 错误处理模式
fn show_error_modes() {
    print_section("Example 3: HttpErrorMode error handling modes");

    println!("A comparison of the two modes (they apply when the HTTP server fails to start):");
    println!(
        "{:<20} {:<24} {:<24}",
        "scenario", "Warn", "Strict (default)"
    );
    println!("{}", "-".repeat(70));
    println!(
        "{:<20} {:<24} {:<24}",
        "port in use", "warn and skip startup", "startup fails with an error"
    );
    println!(
        "{:<20} {:<24} {:<24}",
        "bad TLS cert/key", "warn and skip startup", "startup fails with an error"
    );

    println!(
        "\nRequest-time auth/whitelist rejections are not affected by error_mode and are always rejected:"
    );
    println!("  missing or invalid auth → 401 Unauthorized");
    println!("  IP not in the whitelist → 403 Forbidden");

    println!("\nDefault mode:");
    println!(
        "  HttpErrorMode::default() = {:?}",
        HttpErrorMode::default()
    );

    println!("\nRecommended usage:");
    println!("  Dev/test:    Warn  (a busy port or TLS error only warns)");
    println!(
        "  Production:  Strict (startup failure surfaces immediately; security best practice)"
    );
}

/// 演示完整的安全配置
fn show_full_secure_config() {
    print_section("Example 4: full production-grade security config");

    let secure_config = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9090,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Strict,
        auth: Some(HttpAuthConfig {
            enabled: true,
            token_env: "INKLOG_HTTP_AUTH_TOKEN".to_string(),
        }),
        ip_whitelist: Some(vec!["10.0.0.0/8".to_string(), "127.0.0.1".to_string()]),
        tls: None,
    };

    println!("Production-grade HttpServerConfig:");
    println!("  enabled      = {}", secure_config.enabled);
    println!(
        "  host:port    = {}:{}",
        secure_config.host, secure_config.port
    );
    println!("  metrics_path = \"{}\"", secure_config.metrics_path);
    println!("  health_path  = \"{}\"", secure_config.health_path);
    println!("  error_mode   = {:?}", secure_config.error_mode);
    println!(
        "  auth.enabled = {}",
        secure_config
            .auth
            .as_ref()
            .map(|a| a.enabled)
            .unwrap_or(false)
    );
    println!(
        "  auth.token_env = \"{}\"",
        secure_config
            .auth
            .as_ref()
            .map(|a| a.token_env.as_str())
            .unwrap_or("")
    );
    println!("  ip_whitelist = {:?}", secure_config.ip_whitelist);

    println!("\nThe corresponding TOML configuration:");
    println!(
        r#"[http_server]
enabled = true
host = "0.0.0.0"
port = 9090
metrics_path = "/metrics"
health_path = "/health"
error_mode = "strict"
ip_whitelist = ["10.0.0.0/8", "127.0.0.1"]

[http_server.auth]
enabled = true
token_env = "INKLOG_HTTP_AUTH_TOKEN""#
    );

    println!("\nEnvironment variables to set before starting:");
    println!("  export INKLOG_HTTP_AUTH_TOKEN=\"<your-secret-token>\"");
}

/// 演示环境变量覆盖
fn show_env_overrides() {
    print_section("Example 5: environment variable overrides");

    println!("Environment variables supported by the HTTP server:");
    println!("  INKLOG_HTTP_SERVER_ENABLED=true");
    println!("  INKLOG_HTTP_SERVER_HOST=0.0.0.0");
    println!("  INKLOG_HTTP_SERVER_PORT=9090");
    println!("  INKLOG_HTTP_SERVER_METRICS_PATH=/metrics");
    println!("  INKLOG_HTTP_SERVER_HEALTH_PATH=/health");
    println!("  INKLOG_HTTP_SERVER_ERROR_MODE=strict  # or warn");

    println!("\nNote: auth.enabled / auth.token_env / ip_whitelist have no environment overrides;");
    println!(
        "configure them through TOML or code; the token value comes from the variable named by token_env."
    );

    println!("\nLoad with load_with_env_overrides():");
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
}
