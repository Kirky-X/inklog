// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! HTTP server configuration for health checks and metrics.

use serde::{Deserialize, Serialize};
use std::net::{AddrParseError, SocketAddr};

// HttpServerConfig - HTTP health/metrics server settings

/// 解析 HTTP 服务的绑定地址：`host` + `port` → `SocketAddr`。
///
/// 回环判定（[`HttpServerConfig::is_loopback_host`]）与运行期绑定共用此解析：
/// 两处各按一套规则解析 host 会产生分歧——守卫放行而端点起不来。
///
/// 只接受能直接构成 socket 地址的 IP 字面量：IPv4 明文（`127.0.0.1`）、
/// IPv6 带方括号（`[::1]`）。`localhost` 等主机名与畸形方括号写法
/// （`[::1`、`[[::1]]`）无法在此判定，一律视为不可绑定。
pub(crate) fn bind_socket_addr(host: &str, port: u16) -> Result<SocketAddr, AddrParseError> {
    format!("{}:{}", host.trim(), port).parse()
}

/// 白名单条目是否放行全部地址（CIDR 前缀长度为 0，如 `0.0.0.0/0`）。
///
/// 这类条目对所有来源放行，不构成访问控制。
fn allows_all_addresses(entry: &str) -> bool {
    entry
        .parse::<ipnet::IpNet>()
        .is_ok_and(|net| net.prefix_len() == 0)
}

/// HTTP server configuration for health checks and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

impl HttpServerConfig {
    /// 绑定地址是否为回环（`127.0.0.0/8`、`[::1]`）。
    ///
    /// 回环绑定只有本机可达，是管理/观测端点的默认安全形态；非回环绑定
    /// （含 `0.0.0.0` / `[::]` 通配）会把端点暴露给网络上可达的任何对端。
    ///
    /// 判定与运行期绑定共用 `bind_socket_addr`：只有能被绑定成 socket 地址的
    /// 主机才算回环，`localhost` 等主机名与畸形方括号写法均判为非回环。
    pub fn is_loopback_host(&self) -> bool {
        bind_socket_addr(&self.host, self.port)
            .map(|addr| addr.ip().is_loopback())
            .unwrap_or(false)
    }

    /// 是否已配置访问控制（Bearer 鉴权或 IP 白名单）。
    ///
    /// 白名单含放行全部地址的条目（CIDR 前缀长度为 0，如 `0.0.0.0/0`）时不构成
    /// 访问控制：这类条目对所有来源放行，与不加白名单无异。
    pub fn has_access_control(&self) -> bool {
        self.auth.as_ref().is_some_and(|a| a.enabled)
            || self.ip_whitelist.as_ref().is_some_and(|w| {
                !w.is_empty() && !w.iter().any(|entry| allows_all_addresses(entry))
            })
    }

    /// 非回环绑定且无访问控制：管理/观测端点在网络上裸露。
    ///
    /// 该形态下 `/metrics`（写入/丢弃计数、延迟分位）与 `/health`（sink
    /// 拓扑、故障状态）对任何可达对端开放。Host 校验按对端地址触发，
    /// 只保护回环对端（DNS rebinding 后浏览器对端即回环），网络上的
    /// 其他对端不受 Host 约束。
    ///
    /// 绑不上的地址（主机名、畸形方括号）起不来端点，不属「裸露」——
    /// 那类配置由配置期的可绑定性校验单独报错。
    pub fn is_exposed_without_access_control(&self) -> bool {
        self.enabled
            && bind_socket_addr(&self.host, self.port).is_ok_and(|addr| !addr.ip().is_loopback())
            && !self.has_access_control()
    }
}

// HttpAuthConfig - HTTP authentication

/// HTTP authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

// TlsConfig - TLS configuration

/// TLS configuration for the HTTP server.
///
/// Specifies the paths to the PEM-encoded certificate and private key files
/// used for HTTPS connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
            return Err(crate::i18n::tr("config-tls_cert_path_empty"));
        }
        if self.key_path.is_empty() {
            return Err(crate::i18n::tr("config-tls_key_path_empty"));
        }
        if !std::path::Path::new(&self.cert_path).is_file() {
            return Err(crate::i18n::tr_args("config-tls_cert_path_missing", {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("path", &self.cert_path);
                args
            }));
        }
        if !std::path::Path::new(&self.key_path).is_file() {
            return Err(crate::i18n::tr_args("config-tls_key_path_missing", {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("path", &self.key_path);
                args
            }));
        }
        Ok(())
    }
}

// HttpErrorMode - HTTP server error handling mode

/// HTTP server error handling mode.
///
/// Controls how the HTTP server handles and reports errors.
/// - `Warn`: Log errors as warnings and continue operation.
/// - `Strict`: Return error responses to callers (default).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

    /// 回环判定：默认回环、通配与非回环地址均判为非回环。
    ///
    /// 主机名 `localhost` 判为非回环：它无法构成 socket 地址，运行期绑不上，
    /// 判为回环只会让守卫放行一个起不来的端点。
    #[test]
    fn test_is_loopback_host() {
        let mut cfg = HttpServerConfig::default();
        assert!(cfg.is_loopback_host());
        cfg.host = "localhost".to_string();
        assert!(!cfg.is_loopback_host());
        cfg.host = "127.0.0.5".to_string();
        assert!(cfg.is_loopback_host());
        cfg.host = "[::1]".to_string();
        assert!(cfg.is_loopback_host());
        cfg.host = "0.0.0.0".to_string();
        assert!(!cfg.is_loopback_host());
        cfg.host = "::".to_string();
        assert!(!cfg.is_loopback_host());
        cfg.host = "10.0.0.5".to_string();
        assert!(!cfg.is_loopback_host());
    }

    /// 回环判定与运行期绑定同源：判为回环的主机一定能绑出回环地址。
    ///
    /// 两处口径一旦分歧，守卫会放行一个起不来的端点——判为回环却绑不上。
    /// 逐个 host 断言「判定结果 == 运行期绑定出的地址是否回环」，两个方向
    /// 都必须成立。
    #[test]
    fn test_is_loopback_host_matches_runtime_binding() {
        let cases = [
            ("127.0.0.1", true),
            ("127.0.0.5", true),
            ("[::1]", true),
            // 首尾空白按配置值裁剪后判定
            (" 127.0.0.1 ", true),
            ("localhost", false),
            (" localhost ", false),
            ("LOCALHOST", false),
            ("localhost.", false),
            // IPv6 字面量必须带方括号，裸写无法构成 socket 地址
            ("::1", false),
            // 方括号必须成对且只包一层
            ("[[::1]]", false),
            ("[::1", false),
            ("[::1]]", false),
            ("0.0.0.0", false),
            ("[::]", false),
            ("10.0.0.5", false),
        ];
        for (host, expected_loopback) in cases {
            let cfg = HttpServerConfig {
                host: host.to_string(),
                port: 9090,
                ..Default::default()
            };
            assert_eq!(
                cfg.is_loopback_host(),
                expected_loopback,
                "回环判定不符: host={host:?}"
            );
            let bound_is_loopback = bind_socket_addr(&cfg.host, cfg.port)
                .map(|addr| addr.ip().is_loopback())
                .unwrap_or(false);
            assert_eq!(
                bound_is_loopback, expected_loopback,
                "运行期绑定结果与回环判定不一致: host={host:?}"
            );
        }
    }

    /// 放行全部地址的白名单条目不构成访问控制。
    ///
    /// `0.0.0.0/0` 对所有来源放行，与不加白名单无异——若算作访问控制，
    /// 守卫会放行一个实际裸露的端点。
    #[test]
    fn test_allow_all_whitelist_is_not_access_control() {
        let mut cfg = HttpServerConfig {
            enabled: true,
            host: "0.0.0.0".to_string(),
            ..Default::default()
        };
        cfg.ip_whitelist = Some(vec!["0.0.0.0/0".to_string()]);
        assert!(!cfg.has_access_control());
        assert!(cfg.is_exposed_without_access_control());

        // 有限网段的白名单仍构成访问控制
        cfg.ip_whitelist = Some(vec!["10.0.0.0/8".to_string()]);
        assert!(cfg.has_access_control());
        assert!(!cfg.is_exposed_without_access_control());
    }

    /// 暴露面判定：非回环绑定且无鉴权/白名单 → 裸露；有任一访问控制 → 放行。
    #[test]
    fn test_is_exposed_without_access_control() {
        let mut cfg = HttpServerConfig {
            enabled: true,
            host: "0.0.0.0".to_string(),
            ..Default::default()
        };
        assert!(cfg.is_exposed_without_access_control());

        // 启用鉴权后不再裸露
        cfg.auth = Some(HttpAuthConfig {
            enabled: true,
            ..Default::default()
        });
        assert!(!cfg.is_exposed_without_access_control());

        // 白名单同样构成访问控制
        cfg.auth = None;
        assert!(cfg.is_exposed_without_access_control());
        cfg.ip_whitelist = Some(vec!["10.0.0.0/8".to_string()]);
        assert!(!cfg.is_exposed_without_access_control());

        // 回环绑定本身即安全形态
        let mut cfg = HttpServerConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            ..Default::default()
        };
        assert!(!cfg.is_exposed_without_access_control());
        // 未启用的 HTTP 服务不算暴露
        cfg.enabled = false;
        cfg.host = "0.0.0.0".to_string();
        assert!(!cfg.is_exposed_without_access_control());
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
