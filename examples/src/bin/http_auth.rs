// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! HTTP 认证与 IP 白名单示例
//!
//! 演示 `inklog::HttpAuthConfig` 和 `HttpServerConfig.ip_whitelist` 的配置：
//!
//! 1. `HttpAuthConfig` Bearer Token 认证配置
//! 2. `ip_whitelist` IP 白名单配置（支持精确/CIDR/通配符）
//! 3. `HttpErrorMode` 错误处理模式（Warn vs Strict）
//! 4. 完整的安全配置示例
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin http_auth
//! ```

use inklog::config::{HttpAuthConfig, HttpErrorMode, HttpServerConfig};
use inklog_examples::common::{print_section, print_separator};

fn main() {
    print_separator("inklog HTTP 认证与 IP 白名单示例");

    show_auth_config();
    show_ip_whitelist();
    show_error_modes();
    show_full_secure_config();
    show_env_overrides();

    println!("\n所有 HTTP 认证示例展示完毕。");
}

/// 演示 HttpAuthConfig Bearer Token 认证
fn show_auth_config() {
    print_section("示例 1：HttpAuthConfig Bearer Token 认证");

    let default_auth = HttpAuthConfig::default();
    println!("默认 HttpAuthConfig：");
    println!("  enabled   = {} (默认禁用)", default_auth.enabled);
    println!("  token_env = \"{}\" (环境变量名)", default_auth.token_env);

    println!("\n启用认证：");
    let enabled_auth = HttpAuthConfig {
        enabled: true,
        token_env: "INKLOG_HTTP_AUTH_TOKEN".to_string(),
    };
    println!("  enabled   = {}", enabled_auth.enabled);
    println!("  token_env = \"{}\"", enabled_auth.token_env);

    println!("\n工作流程：");
    println!("  1. 客户端发送请求：curl -H \"Authorization: Bearer $TOKEN\" http://localhost:9090/metrics");
    println!(
        "  2. 服务端从环境变量 {} 读取期望的 token",
        enabled_auth.token_env
    );
    println!("  3. 比对 Authorization header 中的 Bearer token");
    println!("  4. 匹配则返回数据，不匹配则根据 HttpErrorMode 处理");

    println!("\n自定义 token 环境变量：");
    let custom_auth = HttpAuthConfig {
        enabled: true,
        token_env: "MY_APP_AUTH_TOKEN".to_string(),
    };
    println!("  enabled   = {}", custom_auth.enabled);
    println!("  token_env = \"{}\" (自定义)", custom_auth.token_env);
}

/// 演示 IP 白名单配置
fn show_ip_whitelist() {
    print_section("示例 2：IP 白名单（ip_whitelist）");

    println!("支持的格式：");
    println!("  1. 精确 IP:     \"192.168.1.100\"");
    println!("  2. CIDR 网段:   \"10.0.0.0/8\"");
    println!("  3. 通配符:      \"192.168.*.*\"");

    println!("\n配置示例 1：仅允许内网访问");
    let internal_only = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9090,
        ip_whitelist: Some(vec![
            "10.0.0.0/8".to_string(),
            "192.168.0.0/16".to_string(),
            "127.0.0.1".to_string(),
        ]),
        ..Default::default()
    };
    println!("  ip_whitelist = {:?}", internal_only.ip_whitelist);

    println!("\n配置示例 2：仅允许特定监控主机");
    let monitoring_only = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9090,
        ip_whitelist: Some(vec!["10.0.1.50".to_string(), "10.0.1.51".to_string()]),
        ..Default::default()
    };
    println!("  ip_whitelist = {:?}", monitoring_only.ip_whitelist);

    println!("\n配置示例 3：禁用白名单（允许所有 IP）");
    let open = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9090,
        ip_whitelist: None,
        ..Default::default()
    };
    println!("  ip_whitelist = {:?} (None = 允许所有)", open.ip_whitelist);
}

/// 演示 HttpErrorMode 错误处理模式
fn show_error_modes() {
    print_section("示例 3：HttpErrorMode 错误处理模式");

    println!("两种模式对比：");
    println!("{:<15} {:<30} {:<30}", "场景", "Warn (默认)", "Strict");
    println!("{}", "-".repeat(75));
    println!("{:<15} {:<30} {:<30}", "有效认证", "返回数据", "返回数据");
    println!(
        "{:<15} {:<30} {:<30}",
        "缺失认证", "返回数据 + 警告 header", "返回 401 Unauthorized"
    );
    println!(
        "{:<15} {:<30} {:<30}",
        "无效认证", "返回数据 + 警告 header", "返回 401 Unauthorized"
    );
    println!(
        "{:<15} {:<30} {:<30}",
        "IP 不在白名单", "返回数据 + 警告 header", "返回 403 Forbidden"
    );

    println!("\n默认模式：");
    println!(
        "  HttpErrorMode::default() = {:?}",
        HttpErrorMode::default()
    );

    println!("\n推荐场景：");
    println!("  开发/测试:    Warn  (便于调试，不破坏监控)");
    println!("  预发布:       Warn  (测试认证配置，不破坏监控)");
    println!("  生产环境:     Strict (安全最佳实践，失败即拒绝)");
}

/// 演示完整的安全配置
fn show_full_secure_config() {
    print_section("示例 4：完整生产级安全配置");

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
    };

    println!("生产级 HttpServerConfig：");
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

    println!("\n对应的 TOML 配置：");
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

    println!("\n启动前需设置环境变量：");
    println!("  export INKLOG_HTTP_AUTH_TOKEN=\"your-secret-token-here\"");
}

/// 演示环境变量覆盖
fn show_env_overrides() {
    print_section("示例 5：环境变量覆盖");

    println!("认证相关环境变量：");
    println!("  INKLOG_HTTP_SERVER_AUTH_ENABLED=true");
    println!("  INKLOG_HTTP_SERVER_AUTH_TOKEN_ENV=\"MY_AUTH_VAR\"");

    println!("\nIP 白名单环境变量（逗号分隔）：");
    println!("  INKLOG_HTTP_SERVER_IP_WHITELIST=\"10.0.0.0/8,192.168.0.0/16\"");

    println!("\n错误模式环境变量：");
    println!("  INKLOG_HTTP_SERVER_ERROR_MODE=strict  # 或 warn");

    println!("\n通过 load_with_env_overrides() 加载：");
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
}
