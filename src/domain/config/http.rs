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

impl TlsConfig {
    /// Validate TLS settings.
    ///
    /// Both paths must be non-empty and point to existing files, so a
    /// misconfigured HTTPS server fails at configuration time instead of at
    /// first connection.
    pub fn validate(&self) -> Result<(), String> {
        if self.cert_path.is_empty() {
            return Err("tls.cert_path is empty".to_string());
        }
        if self.key_path.is_empty() {
            return Err("tls.key_path is empty".to_string());
        }
        if !std::path::Path::new(&self.cert_path).is_file() {
            return Err(format!(
                "tls.cert_path \"{}\" does not exist",
                self.cert_path
            ));
        }
        if !std::path::Path::new(&self.key_path).is_file() {
            return Err(format!("tls.key_path \"{}\" does not exist", self.key_path));
        }
        Ok(())
    }
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

    #[test]
    fn test_tls_config_validate_rejects_empty_paths() {
        let tls = TlsConfig {
            cert_path: String::new(),
            key_path: "/tmp/key.pem".to_string(),
        };
        let err = tls.validate().unwrap_err();
        assert!(err.contains("cert_path"), "unexpected error: {err}");

        let tls = TlsConfig {
            cert_path: "/tmp/cert.pem".to_string(),
            key_path: String::new(),
        };
        let err = tls.validate().unwrap_err();
        assert!(err.contains("key_path"), "unexpected error: {err}");
    }

    #[test]
    fn test_tls_config_validate_rejects_missing_files() {
        let tls = TlsConfig {
            cert_path: "/nonexistent/cert.pem".to_string(),
            key_path: "/nonexistent/key.pem".to_string(),
        };
        let err = tls.validate().unwrap_err();
        assert!(err.contains("cert_path"), "unexpected error: {err}");

        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        std::fs::write(&cert, "-----BEGIN CERTIFICATE-----").unwrap();

        // 证书存在但密钥缺失
        let tls = TlsConfig {
            cert_path: cert.to_string_lossy().to_string(),
            key_path: dir.path().join("missing.pem").to_string_lossy().to_string(),
        };
        let err = tls.validate().unwrap_err();
        assert!(err.contains("key_path"), "unexpected error: {err}");
    }

    #[test]
    fn test_tls_config_validate_accepts_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, "-----BEGIN CERTIFICATE-----").unwrap();
        std::fs::write(&key, "-----BEGIN PRIVATE KEY-----").unwrap();

        let tls = TlsConfig {
            cert_path: cert.to_string_lossy().to_string(),
            key_path: key.to_string_lossy().to_string(),
        };
        assert!(tls.validate().is_ok());
    }
}
