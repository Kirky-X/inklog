// Copyright (c) 2026 Kirky.X🌠
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
    print_separator("inklog TLS configuration examples");

    show_tls_config_basic();
    show_http_with_tls();
    show_full_secure_tls_config();
    show_tls_toml_config();
    show_tls_best_practices();

    println!("\nAll TLS configuration examples shown.");
}

/// 演示 TlsConfig 基础配置
fn show_tls_config_basic() {
    print_section("Example 1: TlsConfig basics");

    println!("TlsConfig fields:");
    println!("  cert_path: path to the PEM encoded certificate file");
    println!("  key_path:  path to the PEM encoded private key file");

    println!("\nBasic TlsConfig:");
    let tls = TlsConfig {
        cert_path: "/etc/ssl/certs/inklog.crt".to_string(),
        key_path: "/etc/ssl/private/inklog.key".to_string(),
    };
    println!("  cert_path = \"{}\"", tls.cert_path);
    println!("  key_path  = \"{}\"", tls.key_path);

    println!("\nCertificate format requirements:");
    println!("  - Certificate: X.509 PEM format (.crt / .pem)");
    println!("  - Private key: PKCS#8 PEM format (.key / .pem)");
    println!("  - Self-signed certificates work (development/test)");
    println!("  - In production use a certificate from Let's Encrypt or a commercial CA");
}

/// 演示启用 TLS 的 HTTP 服务器配置
fn show_http_with_tls() {
    print_section("Example 2: an HTTP server with TLS enabled");

    println!("Default configuration (no TLS):");
    let default_http = HttpServerConfig::default();
    println!("  tls = {:?} (None = TLS disabled)", default_http.tls);

    println!("\nHTTP server configuration with TLS:");
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

    println!("\nHow access changes:");
    println!("  No TLS: http://localhost:9090/health");
    println!("  With TLS: https://localhost:9443/health");
    println!("  curl -k https://localhost:9443/metrics");
}

/// 演示完整的生产级 TLS 安全配置
fn show_full_secure_tls_config() {
    print_section("Example 3: full production-grade TLS config");

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

    println!("Production-grade HttpServerConfig (TLS + auth + whitelist):");
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

    println!("\nSecurity layers:");
    println!("  1. TLS transport encryption → stops man-in-the-middle attacks");
    println!("  2. Bearer token auth → stops unauthorized access");
    println!("  3. IP whitelist → restricts access sources");
    println!("  4. Strict error mode → fail closed");
}

/// 演示 TOML 配置文件格式
fn show_tls_toml_config() {
    print_section("Example 4: TOML configuration file format");

    println!("The TLS configuration inside inklog.toml:");
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

    println!("\nLoad the configuration:");
    println!("  // Load from a TOML file (the tls field is parsed automatically)");
    println!("  let _logger = LoggerManager::from_file(\"inklog.toml\").await?;");
    println!();
    println!("  // Or search for the configuration file automatically");
    println!("  let _logger = LoggerManager::load().await?;");
}

/// 演示 TLS 最佳实践
fn show_tls_best_practices() {
    print_section("Example 5: TLS best practices");

    println!("Certificate management:");
    println!("  - Use a Let's Encrypt or commercial CA certificate in production");
    println!("  - A self-signed certificate is fine in development:");
    println!("    openssl req -x509 -newkey rsa:4096 -days 365 \\");
    println!("      -keyout inklog.key -out inklog.crt -nodes \\");
    println!("      -subj '/CN=localhost'");
    println!("  - Rotate certificates regularly (90 days is a good target)");

    println!("\nPrivate key protection:");
    println!("  - Set the private key file mode to 600 (owner only)");
    println!("  - Use chmod 600 /etc/ssl/private/inklog.key");
    println!("  - Consider managing keys with Kubernetes Secrets or Vault");

    println!("\nPort selection:");
    println!("  - Standard HTTPS port: 443 (needs root)");
    println!("  - Custom HTTPS port: 8443 / 9443 (recommended)");
    println!("  - Do not share it with the HTTP port (9090 vs 9443)");

    println!("\nMonitoring and alerting:");
    println!("  - Alert on certificate expiry (30 days ahead is recommended)");
    println!("  - Watch the TLS handshake failure rate");
    println!("  - Use the /health endpoint to verify availability");
}
