// SPDX-License-Identifier: MIT
//! TLS 配置示例
//!
//! 演示 `inklog::TlsConfig` 和 `HttpServerConfig.tls` 的配置：
//!
//! 1. `TlsConfig` 证书和密钥路径配置
//! 2. 启用 TLS 的 HTTP 服务器配置
//! 3. 完整的生产级 TLS 安全配置
//! 4. TOML 配置文件格式
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin tls_config
//! ```

use inklog::config::{HttpServerConfig, TlsConfig};
use inklog_examples::common::{print_section, print_separator};

fn main() {
    print_separator("inklog TLS 配置示例");

    show_tls_config_basic();
    show_http_with_tls();
    show_full_secure_tls_config();
    show_tls_toml_config();
    show_tls_best_practices();

    println!("\n所有 TLS 配置示例展示完毕。");
}

/// 演示 TlsConfig 基础配置
fn show_tls_config_basic() {
    print_section("示例 1：TlsConfig 基础配置");

    println!("TlsConfig 字段说明：");
    println!("  cert_path: PEM 编码证书文件路径");
    println!("  key_path:  PEM 编码私钥文件路径");

    println!("\n基础 TlsConfig：");
    let tls = TlsConfig {
        cert_path: "/etc/ssl/certs/inklog.crt".to_string(),
        key_path: "/etc/ssl/private/inklog.key".to_string(),
    };
    println!("  cert_path = \"{}\"", tls.cert_path);
    println!("  key_path  = \"{}\"", tls.key_path);

    println!("\n证书格式要求：");
    println!("  - 证书: X.509 PEM 格式 (.crt / .pem)");
    println!("  - 私钥: PKCS#8 PEM 格式 (.key / .pem)");
    println!("  - 支持自签名证书（开发/测试环境）");
    println!("  - 生产环境建议使用 Let's Encrypt 或商业 CA 签发的证书");
}

/// 演示启用 TLS 的 HTTP 服务器配置
fn show_http_with_tls() {
    print_section("示例 2：启用 TLS 的 HTTP 服务器");

    println!("默认配置（无 TLS）：");
    let default_http = HttpServerConfig::default();
    println!("  tls = {:?} (None = 不启用 TLS)", default_http.tls);

    println!("\n启用 TLS 的 HTTP 服务器配置：");
    let tls_http = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9443,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        tls: Some(TlsConfig {
            cert_path: "/etc/ssl/certs/inklog.crt".to_string(),
            key_path: "/etc/ssl/private/inklog.key".to_string(),
        }),
        ..Default::default()
    };
    println!("  enabled      = {}", tls_http.enabled);
    println!("  host:port    = {}:{}", tls_http.host, tls_http.port);
    println!(
        "  tls.cert     = \"{}\"",
        tls_http
            .tls
            .as_ref()
            .map(|t| t.cert_path.as_str())
            .unwrap_or("")
    );
    println!(
        "  tls.key      = \"{}\"",
        tls_http
            .tls
            .as_ref()
            .map(|t| t.key_path.as_str())
            .unwrap_or("")
    );

    println!("\n访问方式变化：");
    println!("  无 TLS: http://localhost:9090/health");
    println!("  有 TLS: https://localhost:9443/health");
    println!("  curl -k https://localhost:9443/metrics");
}

/// 演示完整的生产级 TLS 安全配置
fn show_full_secure_tls_config() {
    print_section("示例 3：完整生产级 TLS 安全配置");

    use inklog::config::{HttpAuthConfig, HttpErrorMode};

    let secure_config = HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 9443,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Strict,
        auth: Some(HttpAuthConfig {
            enabled: true,
            token_env: "INKLOG_HTTP_AUTH_TOKEN".to_string(),
        }),
        ip_whitelist: Some(vec!["10.0.0.0/8".to_string(), "127.0.0.1".to_string()]),
        tls: Some(TlsConfig {
            cert_path: "/etc/ssl/certs/inklog.crt".to_string(),
            key_path: "/etc/ssl/private/inklog.key".to_string(),
        }),
    };

    println!("生产级 HttpServerConfig（TLS + 认证 + 白名单）：");
    println!("  enabled      = {}", secure_config.enabled);
    println!(
        "  host:port    = {}:{}",
        secure_config.host, secure_config.port
    );
    println!("  error_mode   = {:?}", secure_config.error_mode);
    println!(
        "  auth.enabled = {}",
        secure_config.auth.as_ref().is_some_and(|a| a.enabled)
    );
    println!("  ip_whitelist = {:?}", secure_config.ip_whitelist);
    println!(
        "  tls.cert     = \"{}\"",
        secure_config
            .tls
            .as_ref()
            .map(|t| t.cert_path.as_str())
            .unwrap_or("")
    );
    println!(
        "  tls.key      = \"{}\"",
        secure_config
            .tls
            .as_ref()
            .map(|t| t.key_path.as_str())
            .unwrap_or("")
    );

    println!("\n安全层次：");
    println!("  1. TLS 加密传输 → 防止中间人攻击");
    println!("  2. Bearer Token 认证 → 防止未授权访问");
    println!("  3. IP 白名单 → 限制访问来源");
    println!("  4. Strict 错误模式 → 失败即拒绝");
}

/// 演示 TOML 配置文件格式
fn show_tls_toml_config() {
    print_section("示例 4：TOML 配置文件格式");

    println!("inklog.toml 中的 TLS 配置：");
    println!(
        r#"[http_server]
enabled = true
host = "0.0.0.0"
port = 9443
metrics_path = "/metrics"
health_path = "/health"
error_mode = "strict"
ip_whitelist = ["10.0.0.0/8", "127.0.0.1"]

[http_server.auth]
enabled = true
token_env = "INKLOG_HTTP_AUTH_TOKEN"

[http_server.tls]
cert_path = "/etc/ssl/certs/inklog.crt"
key_path = "/etc/ssl/private/inklog.key""#
    );

    println!("\n加载配置：");
    println!("  // 从 TOML 文件加载（自动解析 tls 字段）");
    println!("  let _logger = LoggerManager::from_file(\"inklog.toml\").await?;");
    println!();
    println!("  // 或自动搜索配置文件");
    println!("  let _logger = LoggerManager::load().await?;");
}

/// 演示 TLS 最佳实践
fn show_tls_best_practices() {
    print_section("示例 5：TLS 最佳实践");

    println!("证书管理：");
    println!("  - 生产环境使用 Let's Encrypt 或商业 CA 证书");
    println!("  - 开发环境可使用自签名证书：");
    println!("    openssl req -x509 -newkey rsa:4096 -days 365 \\");
    println!("      -keyout inklog.key -out inklog.crt -nodes \\");
    println!("      -subj '/CN=localhost'");
    println!("  - 定期轮换证书（建议 90 天）");

    println!("\n私钥保护：");
    println!("  - 私钥文件权限设置为 600（仅 owner 可读写）");
    println!("  - 使用 chmod 600 /etc/ssl/private/inklog.key");
    println!("  - 考虑使用 Kubernetes Secrets 或 Vault 管理密钥");

    println!("\n端口选择：");
    println!("  - 标准 HTTPS 端口: 443（需 root 权限）");
    println!("  - 自定义 HTTPS 端口: 8443 / 9443（推荐）");
    println!("  - 避免与 HTTP 端口共用（9090 vs 9443）");

    println!("\n监控与告警：");
    println!("  - 监控证书过期时间（建议 30 天前告警）");
    println!("  - 监控 TLS 握手失败率");
    println!("  - 使用 /health 端点验证服务可用性");
}
