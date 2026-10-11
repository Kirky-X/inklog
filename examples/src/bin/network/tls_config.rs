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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};

fn main() {
    print_separator(&tr("example-tls-config-inklog-tls-configuration"));

    show_tls_config_basic();
    show_http_with_tls();
    show_full_secure_tls_config();
    show_tls_toml_config();
    show_tls_best_practices();

    println!(
        "\n{}",
        tr("example-tls-config-all-tls-configuration-examples")
    );
}

/// 演示 TlsConfig 基础配置
fn show_tls_config_basic() {
    print_section(&tr("example-tls-config-example-1-tlsconfig-basics"));

    println!("{}", tr("example-tls-config-tlsconfig-fields"));
    println!("  {}", tr("example-tls-config-cert-path-path-to-the-pem"));
    println!("  {}", tr("example-tls-config-key-path-path-to-the-pem"));

    println!("\n{}", tr("example-tls-config-basic-tlsconfig"));
    let tls = TlsConfig {
        cert_path: "/etc/ssl/certs/inklog.crt".to_string(),
        key_path: "/etc/ssl/private/inklog.key".to_string(),
    };
    println!(
        "  {}",
        tr_args(
            "example-tls-config-cert-path",
            &[("cert_path", &tls.cert_path)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-key-path",
            &[("key_path", &tls.key_path)]
        )
    );

    println!(
        "\n{}",
        tr("example-tls-config-certificate-format-requirements")
    );
    println!(
        "  {}",
        tr("example-tls-config-certificate-x-509-pem-format")
    );
    println!(
        "  {}",
        tr("example-tls-config-private-key-pkcs-8-pem-format")
    );
    println!(
        "  {}",
        tr("example-tls-config-self-signed-certificates-work")
    );
    println!(
        "  {}",
        tr("example-tls-config-in-production-use-a-certificate")
    );
}

/// 演示启用 TLS 的 HTTP 服务器配置
fn show_http_with_tls() {
    print_section(&tr("example-tls-config-example-2-an-http-server-with"));

    println!("{}", tr("example-tls-config-default-configuration-no-tls"));
    let default_http = HttpServerConfig::default();
    println!(
        "  {}",
        tr_args(
            "example-tls-config-tls-none-tls-disabled",
            &[("tls", &format!("{:?}", default_http.tls))]
        )
    );

    println!(
        "\n{}",
        tr("example-tls-config-http-server-configuration-with")
    );
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
    println!(
        "  {}",
        tr_args(
            "example-tls-config-enabled",
            &[("enabled", &tls_http.enabled)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-host-port",
            &[("host", &tls_http.host), ("port", &tls_http.port)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-tls-cert",
            &[(
                "value",
                &(tls_http
                    .tls
                    .as_ref()
                    .map(|t| t.cert_path.as_str())
                    .unwrap_or(""))
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-tls-key",
            &[(
                "value",
                &(tls_http
                    .tls
                    .as_ref()
                    .map(|t| t.key_path.as_str())
                    .unwrap_or(""))
            )]
        )
    );

    println!("\n{}", tr("example-tls-config-how-access-changes"));
    println!("  {}", tr("example-tls-config-no-tls-http-localhost-9090"));
    println!(
        "  {}",
        tr("example-tls-config-with-tls-https-localhost-9443")
    );
    println!("  {}", tr("example-tls-config-curl-k-https-localhost-9443"));
}

/// 演示完整的生产级 TLS 安全配置
fn show_full_secure_tls_config() {
    print_section(&tr("example-tls-config-example-3-full-production-grade"));

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

    println!("{}", tr("example-tls-config-production-grade"));
    println!(
        "  {}",
        tr_args(
            "example-tls-config-enabled",
            &[("enabled", &secure_config.enabled)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-host-port",
            &[("host", &secure_config.host), ("port", &secure_config.port)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-error-mode",
            &[("error_mode", &format!("{:?}", secure_config.error_mode))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-auth-enabled",
            &[(
                "value",
                &(secure_config.auth.as_ref().is_some_and(|a| a.enabled))
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-ip-whitelist",
            &[("ip_whitelist", &format!("{:?}", secure_config.ip_whitelist))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-tls-cert",
            &[(
                "value",
                &(secure_config
                    .tls
                    .as_ref()
                    .map(|t| t.cert_path.as_str())
                    .unwrap_or(""))
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-tls-config-tls-key",
            &[(
                "value",
                &(secure_config
                    .tls
                    .as_ref()
                    .map(|t| t.key_path.as_str())
                    .unwrap_or(""))
            )]
        )
    );

    println!("\n{}", tr("example-tls-config-security-layers"));
    println!("  {}", tr("example-tls-config-1-tls-transport-encryption"));
    println!("  {}", tr("example-tls-config-2-bearer-token-auth-stops"));
    println!(
        "  {}",
        tr("example-tls-config-3-ip-whitelist-restricts-access")
    );
    println!(
        "  {}",
        tr("example-tls-config-4-strict-error-mode-fail-closed")
    );
}

/// 演示 TOML 配置文件格式
fn show_tls_toml_config() {
    print_section(&tr("example-tls-config-example-4-toml-configuration"));

    println!("{}", tr("example-tls-config-the-tls-configuration-inside"));
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

    println!("\n{}", tr("example-tls-config-load-the-configuration"));
    println!("  // Load from a TOML file (the tls field is parsed automatically)");
    println!("  let _logger = LoggerManager::from_file(\"inklog.toml\").await?;");
    println!();
    println!("  // Or search for the configuration file automatically");
    println!("  let _logger = LoggerManager::load().await?;");
}

/// 演示 TLS 最佳实践
fn show_tls_best_practices() {
    print_section(&tr("example-tls-config-example-5-tls-best-practices"));

    println!("{}", tr("example-tls-config-certificate-management"));
    println!("  {}", tr("example-tls-config-use-a-let-s-encrypt-or"));
    println!(
        "  {}",
        tr("example-tls-config-a-self-signed-certificate-is")
    );
    println!(
        "    {}",
        tr("example-tls-config-openssl-req-x509-newkey-rsa")
    );
    println!(
        "      {}",
        tr("example-tls-config-keyout-inklog-key-out-inklog")
    );
    println!("      {}", tr("example-tls-config-subj-cn-localhost"));
    println!(
        "  {}",
        tr("example-tls-config-rotate-certificates-regularly")
    );

    println!("\n{}", tr("example-tls-config-private-key-protection"));
    println!(
        "  {}",
        tr("example-tls-config-set-the-private-key-file-mode")
    );
    println!(
        "  {}",
        tr("example-tls-config-use-chmod-600-etc-ssl-private")
    );
    println!("  {}", tr("example-tls-config-consider-managing-keys-with"));

    println!("\n{}", tr("example-tls-config-port-selection"));
    println!(
        "  {}",
        tr("example-tls-config-standard-https-port-443-needs")
    );
    println!("  {}", tr("example-tls-config-custom-https-port-8443-9443"));
    println!(
        "  {}",
        tr("example-tls-config-do-not-share-it-with-the-http")
    );

    println!("\n{}", tr("example-tls-config-monitoring-and-alerting"));
    println!(
        "  {}",
        tr("example-tls-config-alert-on-certificate-expiry-30")
    );
    println!(
        "  {}",
        tr("example-tls-config-watch-the-tls-handshake-failure")
    );
    println!("  {}", tr("example-tls-config-use-the-health-endpoint-to"));
}
