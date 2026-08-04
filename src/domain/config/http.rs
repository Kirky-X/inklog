// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! HTTP server configuration for health checks and metrics.

use serde::{Deserialize, Serialize};

// ============================================================================
// HttpServerConfig - HTTP health/metrics server settings
// ============================================================================

/// HTTP server configuration for health checks and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_http_host")]
    pub host: String,
    #[serde(default = "default_http_port")]
    pub port: u16,
    #[serde(default = "default_http_metrics_path")]
    pub metrics_path: String,
    #[serde(default = "default_http_health_path")]
    pub health_path: String,
    #[serde(default)]
    pub error_mode: HttpErrorMode,
    #[serde(default)]
    pub auth: Option<HttpAuthConfig>,
    #[serde(default)]
    pub ip_whitelist: Option<Vec<String>>,
    /// Optional TLS configuration. When set, the HTTP server starts with TLS.
    #[serde(default)]
    pub tls: Option<TlsConfig>,
}

fn default_http_host() -> String {
    "127.0.0.1".to_string()
}
fn default_http_port() -> u16 {
    9090
}
fn default_http_metrics_path() -> String {
    "/metrics".to_string()
}
fn default_http_health_path() -> String {
    "/health".to_string()
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_http_host(),
            port: default_http_port(),
            metrics_path: default_http_metrics_path(),
            health_path: default_http_health_path(),
            error_mode: HttpErrorMode::default(),
            auth: None,
            ip_whitelist: None,
            tls: None,
        }
    }
}

// ============================================================================
// HttpAuthConfig - HTTP authentication
// ============================================================================

/// HTTP authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAuthConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_http_auth_token_env")]
    pub token_env: String,
}

fn default_http_auth_token_env() -> String {
    "INKLOG_HTTP_AUTH_TOKEN".to_string()
}

impl Default for HttpAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_env: default_http_auth_token_env(),
        }
    }
}

// ============================================================================
// TlsConfig - TLS configuration
// ============================================================================

/// TLS configuration for the HTTP server.
///
/// Specifies the paths to the PEM-encoded certificate and private key files
/// used for HTTPS connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to the PEM-encoded certificate file.
    pub cert_path: String,
    /// Path to the PEM-encoded private key file.
    pub key_path: String,
}

// ============================================================================
// HttpErrorMode - HTTP server error handling mode
// ============================================================================

/// HTTP server error handling mode.
///
/// Controls how the HTTP server handles and reports errors.
/// - `Warn`: Log errors as warnings and continue operation.
/// - `Strict`: Return error responses to callers (default).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HttpErrorMode {
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "strict")]
    #[default]
    Strict,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_server_config_default() {
        let cfg = HttpServerConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 9090);
        assert_eq!(cfg.metrics_path, "/metrics");
        assert_eq!(cfg.health_path, "/health");
        assert!(matches!(cfg.error_mode, HttpErrorMode::Strict));
        assert!(cfg.auth.is_none());
        assert!(cfg.ip_whitelist.is_none());
        assert!(cfg.tls.is_none());
    }

    #[test]
    fn test_http_auth_config_default() {
        let cfg = HttpAuthConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.token_env, "INKLOG_HTTP_AUTH_TOKEN");
    }

    #[test]
    fn test_http_error_mode_default() {
        let mode = HttpErrorMode::default();
        assert!(matches!(mode, HttpErrorMode::Strict));
    }
}
