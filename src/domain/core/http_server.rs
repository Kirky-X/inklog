// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! HTTP server for health checks and metrics.

use super::LoggerManager;
#[cfg(feature = "http")]
use crate::InklogError;
#[cfg(feature = "http")]
use crate::LogRecord;
#[cfg(feature = "http")]
use crate::Metrics;
#[cfg(feature = "http")]
use crossbeam_channel::Sender;
#[cfg(feature = "http")]
use std::sync::atomic::AtomicUsize;
#[cfg(feature = "http")]
use std::sync::{Arc, Mutex};

impl LoggerManager {
    /// 启动HTTP监控服务器
    ///
    /// 提供健康检查和Prometheus指标端点
    /// 支持 Bearer Token 认证和 IP 白名单
    #[cfg(feature = "http")]
    pub(crate) async fn start_http_server(
        metrics: Arc<Metrics>,
        sender: Sender<Arc<LogRecord>>,
        effective_capacity: Arc<AtomicUsize>,
        http_server_handle: &Mutex<Option<tokio::task::JoinHandle<()>>>,
        config: &crate::HttpServerConfig,
    ) -> Result<(), InklogError> {
        use axum::{
            Router,
            extract::{ConnectInfo, State},
            http::{Request, StatusCode, header},
            middleware::{self, Next},
            response::{IntoResponse, Response},
            routing::get,
        };
        use std::net::SocketAddr;
        use subtle::ConstantTimeEq;
        use tracing::info;

        let health_path = config.health_path.clone();
        let metrics_path = config.metrics_path.clone();

        let health_status_getter = {
            let sender = sender.clone();
            let effective_capacity = effective_capacity.clone();
            let metrics_clone = metrics.clone();
            move || {
                let channel_len = sender.len();
                let channel_cap = effective_capacity.load(std::sync::atomic::Ordering::Relaxed);
                metrics_clone.get_status(channel_len, channel_cap)
            }
        };

        /// vuln-0003 修复：HttpAuthState 在启动时一次性读取 token 值并缓存，
        /// auth_middleware 不再调用 `std::env::var`。这杜绝了运行时环境变量
        /// 被篡改对后续请求鉴权的影响（fail-closed at startup）。
        #[derive(Clone)]
        struct HttpAuthState {
            auth_enabled: bool,
            /// 启动时一次性读取的 token 值。`None` 表示未配置或读取失败。
            /// 当 `auth_enabled=true` 且 `token_value=None` 时，启动直接失败。
            token_value: Option<String>,
            ip_whitelist: Option<Vec<String>>,
        }

        // vuln-0003: 在启动时（而非请求时）读取 token 值。若 auth 启用但
        // token 未配置或读取失败，直接 fail-closed 拒绝启动。
        let (auth_enabled, token_value) = match config.auth.as_ref() {
            Some(a) if a.enabled => {
                let token_env = if a.token_env.is_empty() {
                    "INKLOG_HTTP_AUTH_TOKEN"
                } else {
                    a.token_env.as_str()
                };
                match std::env::var(token_env) {
                    Ok(t) if !t.is_empty() => (true, Some(t)),
                    Ok(_) => {
                        let mut args = fluent_bundle::FluentArgs::new();
                        args.set("env", token_env);
                        return Err(InklogError::ConfigError(crate::i18n::tr_args(
                            "config-http_auth_token_empty",
                            args,
                        )));
                    }
                    Err(_) => {
                        let mut args = fluent_bundle::FluentArgs::new();
                        args.set("env", token_env);
                        return Err(InklogError::ConfigError(crate::i18n::tr_args(
                            "config-http_auth_token_not_set",
                            args,
                        )));
                    }
                }
            }
            Some(_) => (false, None),
            None => (false, None),
        };

        let auth_state = HttpAuthState {
            auth_enabled,
            token_value,
            ip_whitelist: config.ip_whitelist.clone(),
        };

        async fn auth_middleware(
            State(state): State<HttpAuthState>,
            ConnectInfo(addr): ConnectInfo<SocketAddr>,
            request: Request<axum::body::Body>,
            next: Next,
        ) -> Response {
            // vuln-0003: 使用启动时缓存的 token_value，不再读取环境变量。
            // 若 auth_enabled=true 则 token_value 一定为 Some（启动时已校验）。
            if state.auth_enabled
                && let Some(ref expected_token) = state.token_value
            {
                let auth_header = request
                    .headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());

                match auth_header {
                    Some(h) if h.starts_with("Bearer ") => {
                        let token = &h[7..];
                        if !subtle_constant_time_compare(
                            token.as_bytes(),
                            expected_token.as_bytes(),
                        ) {
                            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
                        }
                    }
                    _ => {
                        return (
                            StatusCode::UNAUTHORIZED,
                            "Missing or invalid Authorization header",
                        )
                            .into_response();
                    }
                }
            }

            if let Some(ref whitelist) = state.ip_whitelist {
                let client_ip = addr.ip().to_string();
                if !whitelist.iter().any(|allowed| {
                    whitelist_entry_matches(
                        allowed,
                        &client_ip,
                        addr.ip(),
                        &INVALID_WHITELIST_WARNED,
                    )
                }) {
                    return (StatusCode::FORBIDDEN, "IP not in whitelist").into_response();
                }
            }

            next.run(request).await
        }

        fn subtle_constant_time_compare(a: &[u8], b: &[u8]) -> bool {
            a.ct_eq(b).unwrap_u8() == 1
        }

        // 进程级一次性告警标志：坏白名单条目在请求热路径上只告警一次，
        // 主防线是配置期校验（InklogConfig::validate）
        static INVALID_WHITELIST_WARNED: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);

        let app = Router::new()
            .route(
                &health_path,
                get(|| async move {
                    let status = health_status_getter();
                    match serde_json::to_value(&status) {
                        Ok(v) => axum::Json(v),
                        Err(e) => {
                            let mut args = fluent_bundle::FluentArgs::new();
                            args.set("err", e.to_string());
                            tracing::error!(
                                "{}",
                                crate::i18n::tr_args("config-http_serialize_failed", args)
                            );
                            axum::Json(serde_json::json!({"error": "serialization failed"}))
                        }
                    }
                }),
            )
            .route(
                &metrics_path,
                get(move || async move { metrics.export_prometheus() }),
            )
            .layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            ))
            .with_state(auth_state);

        let addr: std::net::SocketAddr = format!("{}:{}", config.host, config.port)
            .parse()
            .map_err(|e: std::net::AddrParseError| {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("addr", format!("{}:{}", config.host, config.port));
                args.set("err", e.to_string());
                InklogError::ConfigError(crate::i18n::tr_args("config-invalid_http_address", args))
            })?;

        let auth_enabled = config.auth.as_ref().map(|a| a.enabled).unwrap_or(false);
        let ip_whitelist = config.ip_whitelist.clone();

        let tls_config = config.tls.clone();

        let handle = tokio::spawn(async move {
            let make_svc = app.into_make_service_with_connect_info::<SocketAddr>();

            if let Some(ref tls) = tls_config {
                // TLS mode via axum-server + rustls
                use axum_server::tls_rustls::RustlsConfig;

                let rustls_config =
                    match RustlsConfig::from_pem_file(&tls.cert_path, &tls.key_path).await {
                        Ok(c) => c,
                        Err(e) => {
                            let mut args = fluent_bundle::FluentArgs::new();
                            args.set("err", e.to_string());
                            tracing::error!(
                                "{}",
                                crate::i18n::tr_args("config-https_server_error", args)
                            );
                            return;
                        }
                    };
                info!(
                    "HTTPS server started on {} (auth: {}, ip_whitelist: {:?})",
                    addr, auth_enabled, ip_whitelist
                );
                if let Err(e) = axum_server::tls_rustls::bind_rustls(addr, rustls_config)
                    .serve(make_svc)
                    .await
                {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("err", e.to_string());
                    tracing::error!(
                        "{}",
                        crate::i18n::tr_args("config-https_server_error", args)
                    );
                }
            } else {
                // Plain TCP mode
                let listener = match tokio::net::TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        let mut args = fluent_bundle::FluentArgs::new();
                        args.set("addr", addr.to_string());
                        args.set("err", e.to_string());
                        tracing::error!(
                            "{}",
                            crate::i18n::tr_args("config-http_bind_failed", args)
                        );
                        return;
                    }
                };
                info!(
                    "HTTP server started on {} (auth: {}, ip_whitelist: {:?})",
                    addr, auth_enabled, ip_whitelist
                );
                match axum::serve(listener, make_svc).await {
                    Ok(_) => info!("HTTP server stopped"),
                    Err(e) => {
                        let mut args = fluent_bundle::FluentArgs::new();
                        args.set("err", e.to_string());
                        tracing::error!(
                            "{}",
                            crate::i18n::tr_args("config-http_server_error", args)
                        );
                    }
                }
            }
        });

        // 此锁仅保护 JoinHandle 槽位，无复合不变量：毒化时恢复出守卫继续写入，
        // 消除"仅记日志但 handle 未存入"的路径
        let mut handle_guard = http_server_handle
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *handle_guard = Some(handle);

        info!("HTTP monitoring server configured on {}", addr);
        Ok(())
    }
}

/// 单条 IP 白名单匹配：`IP` 精确、`前缀.*` 通配（补结尾点防越界前缀匹配，
/// diting MED-003 修复）、`CIDR` 经 [`parse_cidr`]。
///
/// CIDR 解析失败的条目 fail-closed（永不匹配），并经 `invalid_warned`
/// 进程级标志仅首次输出 `tracing::warn`——配置期校验是主防线，此处为
/// 覆盖直接构造 HttpServer 路径的运行期兜底。
#[cfg(feature = "http")]
fn whitelist_entry_matches(
    allowed: &str,
    client_ip: &str,
    ip: std::net::IpAddr,
    invalid_warned: &std::sync::atomic::AtomicBool,
) -> bool {
    use std::sync::atomic::Ordering;

    if let Some(prefix_body) = allowed.strip_suffix(".*") {
        // 剥离 ".*" 后必须补回结尾点，否则 "192.168" 会
        // 前缀匹配 "192.1681.x" / "10.01.x" 等越界地址被放行
        let prefix = format!("{prefix_body}.");
        client_ip.starts_with(&prefix)
    } else if allowed.contains('/') {
        match parse_cidr(allowed) {
            Some(network) => network.contains(&ip),
            None => {
                if !invalid_warned.swap(true, Ordering::Relaxed) {
                    tracing::warn!(
                        entry = %allowed,
                        "ip_whitelist entry is not a valid IP or CIDR; it never matches (fail-closed)"
                    );
                }
                false
            }
        }
    } else {
        client_ip == allowed
    }
}

#[cfg(feature = "http")]
fn parse_cidr(cidr: &str) -> Option<ipnet::IpNet> {
    cidr.parse().ok()
}

#[cfg(all(test, feature = "http"))]
mod tests {
    use super::whitelist_entry_matches;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;

    fn ip(s: &str) -> std::net::IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn test_invalid_cidr_entry_fails_closed_and_raises_warn_flag() {
        let flag = AtomicBool::new(false);
        // 非法 CIDR：不匹配任何 IP（fail-closed）
        assert!(!whitelist_entry_matches(
            "10.0.0.0/33",
            "10.0.0.1",
            ip("10.0.0.1"),
            &flag
        ));
        // 首次失败置位告警标志
        assert!(flag.load(Ordering::Relaxed));
        // 后续请求不再重复置位（swap 返回 true = 已告警过，跳过第二次日志）
        assert!(!whitelist_entry_matches(
            "10.0.0.0/33",
            "10.0.0.2",
            ip("10.0.0.2"),
            &flag
        ));
        assert!(flag.load(Ordering::Relaxed));
    }

    #[test]
    fn test_valid_entries_match_without_touching_warn_flag() {
        let flag = AtomicBool::new(false);

        // 精确匹配
        assert!(whitelist_entry_matches(
            "192.168.1.1",
            "192.168.1.1",
            ip("192.168.1.1"),
            &flag
        ));
        // 通配：补结尾点防越界前缀匹配
        assert!(whitelist_entry_matches("10.*", "10.1.2.3", ip("10.1.2.3"), &flag));
        assert!(!whitelist_entry_matches(
            "10.*",
            "110.1.2.3",
            ip("110.1.2.3"),
            &flag
        ));
        // CIDR 包含判定
        assert!(whitelist_entry_matches(
            "192.168.0.0/24",
            "192.168.0.99",
            ip("192.168.0.99"),
            &flag
        ));
        assert!(!whitelist_entry_matches(
            "192.168.0.0/24",
            "192.168.1.1",
            ip("192.168.1.1"),
            &flag
        ));

        // 合法条目全程不应触发告警标志
        assert!(!flag.load(Ordering::Relaxed));
    }
}
