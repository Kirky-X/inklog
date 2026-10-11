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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};

fn main() {
    print_separator(&tr("example-http-auth-inklog-http-auth-and-ip"));

    show_auth_config();
    show_ip_whitelist();
    show_error_modes();
    show_full_secure_config();
    show_env_overrides();

    println!("\n{}", tr("example-http-auth-all-http-auth-examples-shown"));
}

/// 演示 HttpAuthConfig Bearer Token 认证
fn show_auth_config() {
    print_section(&tr("example-http-auth-example-1-httpauthconfig-bearer"));

    let default_auth = HttpAuthConfig::default();
    println!("{}", tr("example-http-auth-default-httpauthconfig"));
    println!(
        "  {}",
        tr_args(
            "example-http-auth-enabled-disabled-by-default",
            &[("enabled", &default_auth.enabled)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-token-env-environment-variable",
            &[("token_env", &default_auth.token_env)]
        )
    );

    println!("\n{}", tr("example-http-auth-enable-auth"));
    let enabled_auth = HttpAuthConfig {
        enabled: true,
        token_env: "INKLOG_HTTP_AUTH_TOKEN".to_string(),
    };
    println!(
        "  {}",
        tr_args(
            "example-http-auth-enabled",
            &[("enabled", &enabled_auth.enabled)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-token-env",
            &[("token_env", &enabled_auth.token_env)]
        )
    );

    println!("\n{}", tr("example-http-auth-how-it-works"));
    println!(
        "  1. The client sends: curl -H \"Authorization: Bearer $TOKEN\" http://localhost:9090/metrics"
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-2-the-server-reads-the-expected",
            &[("token_env", &enabled_auth.token_env)]
        )
    );
    println!(
        "  {}",
        tr("example-http-auth-3-it-compares-the-bearer-token")
    );
    println!("  {}", tr("example-http-auth-4-a-match-returns-data-a"));

    println!("\n{}", tr("example-http-auth-custom-token-environment"));
    let custom_auth = HttpAuthConfig {
        enabled: true,
        token_env: "MY_APP_AUTH_TOKEN".to_string(),
    };
    println!(
        "  {}",
        tr_args(
            "example-http-auth-enabled",
            &[("enabled", &custom_auth.enabled)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-token-env-custom",
            &[("token_env", &custom_auth.token_env)]
        )
    );
}

/// 演示 IP 白名单配置
fn show_ip_whitelist() {
    print_section(&tr("example-http-auth-example-2-ip-whitelist-ip"));

    println!("{}", tr("example-http-auth-supported-formats"));
    println!("  {}", tr("example-http-auth-1-exact-ip-192-168-1-100"));
    println!("  {}", tr("example-http-auth-2-cidr-range-10-0-0-0-8"));
    println!(
        "  3. Prefix wildcard: \"192.168.*\" (a trailing .* only, with 1-3 numeric segments before it)"
    );
    println!(
        "     Note: \"192.168.*.*\" is not valid — a wildcard is allowed only in the last segment"
    );

    println!("\n{}", tr("example-http-auth-config-example-1-internal"));
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
    println!(
        "  {}",
        tr_args(
            "example-http-auth-ip-whitelist",
            &[("ip_whitelist", &format!("{:?}", internal_only.ip_whitelist))]
        )
    );

    // 用公开校验 API 检查白名单条目：坏条目在配置期直接报错，
    // 不会等到运行期静默 fail-closed（拒绝所有请求）
    InklogConfig {
        http_server: Some(internal_only.clone()),
        ..Default::default()
    }
    .validate()
    .expect("the whitelist entries should pass config validation");
    println!("  {}", tr("example-http-auth-inklogconfig-validate-passed"));

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
    println!(
        "  {}",
        tr_args(
            "example-http-auth-the-invalid-192-168-was-rejected",
            &[("err", &err)]
        )
    );

    println!("\n{}", tr("example-http-auth-config-example-2-specific"));
    let monitoring_only = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9090,
        ip_whitelist: Some(vec!["10.0.1.50".to_string(), "10.0.1.51".to_string()]),
        ..Default::default()
    };
    println!(
        "  {}",
        tr_args(
            "example-http-auth-ip-whitelist",
            &[(
                "ip_whitelist",
                &format!("{:?}", monitoring_only.ip_whitelist)
            )]
        )
    );

    println!("\n{}", tr("example-http-auth-config-example-3-whitelist"));
    let open = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9090,
        ip_whitelist: None,
        ..Default::default()
    };
    println!(
        "  {}",
        tr_args(
            "example-http-auth-ip-whitelist-none-allow-all",
            &[("ip_whitelist", &format!("{:?}", open.ip_whitelist))]
        )
    );
}

/// 演示 HttpErrorMode 错误处理模式
fn show_error_modes() {
    print_section(&tr("example-http-auth-example-3-httperrormode-error"));

    println!("{}", tr("example-http-auth-a-comparison-of-the-two-modes"));
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
    println!("  {}", tr("example-http-auth-missing-or-invalid-auth-401"));
    println!("  {}", tr("example-http-auth-ip-not-in-the-whitelist-403"));

    println!("\n{}", tr("example-http-auth-default-mode"));
    println!(
        "  {}",
        tr_args(
            "example-http-auth-httperrormode-default",
            &[("value", &format!("{:?}", HttpErrorMode::default()))]
        )
    );

    println!("\n{}", tr("example-http-auth-recommended-usage"));
    println!("  {}", tr("example-http-auth-dev-test-warn-a-busy-port-or"));
    println!(
        "  Production:  Strict (startup failure surfaces immediately; security best practice)"
    );
}

/// 演示完整的安全配置
fn show_full_secure_config() {
    print_section(&tr("example-http-auth-example-4-full-production-grade"));

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

    println!("{}", tr("example-http-auth-production-grade"));
    println!(
        "  {}",
        tr_args(
            "example-http-auth-enabled-2",
            &[("enabled", &secure_config.enabled)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-host-port",
            &[("host", &secure_config.host), ("port", &secure_config.port)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-metrics-path",
            &[("metrics_path", &secure_config.metrics_path)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-health-path",
            &[("health_path", &secure_config.health_path)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-error-mode",
            &[("error_mode", &format!("{:?}", secure_config.error_mode))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-auth-enabled",
            &[(
                "value",
                &(secure_config
                    .auth
                    .as_ref()
                    .map(|a| a.enabled)
                    .unwrap_or(false))
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-auth-token-env",
            &[(
                "value",
                &(secure_config
                    .auth
                    .as_ref()
                    .map(|a| a.token_env.as_str())
                    .unwrap_or(""))
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-http-auth-ip-whitelist",
            &[("ip_whitelist", &format!("{:?}", secure_config.ip_whitelist))]
        )
    );

    println!("\n{}", tr("example-http-auth-the-corresponding-toml"));
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

    println!("\n{}", tr("example-http-auth-environment-variables-to-set"));
    println!("  export INKLOG_HTTP_AUTH_TOKEN=\"<your-secret-token>\"");
}

/// 演示环境变量覆盖
fn show_env_overrides() {
    print_section(&tr("example-http-auth-example-5-environment-variable"));

    println!(
        "{}",
        tr("example-http-auth-environment-variables-supported")
    );
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

    println!("\n{}", tr("example-http-auth-load-with-load-with-env"));
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
}
