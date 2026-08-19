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
                    if allowed.ends_with(".*") {
                        // 剥离 ".*" 后必须补回结尾点，否则 "192.168" 会
                        // 前缀匹配 "192.1681.x" / "10.01.x" 等越界地址被放行
                        // （diting MED-003 修复）
                        let prefix = format!("{}.", &allowed[..allowed.len() - 2]);
                        client_ip.starts_with(&prefix)
                    } else if allowed.contains('/') {
                        matches!(parse_cidr(allowed), Some(network) if network.contains(&addr.ip()))
                    } else {
                        client_ip == *allowed
                    }
                }) {
                    return (StatusCode::FORBIDDEN, "IP not in whitelist").into_response();
                }
            }

            next.run(request).await
        }

        fn subtle_constant_time_compare(a: &[u8], b: &[u8]) -> bool {
            a.ct_eq(b).unwrap_u8() == 1
        }

        fn parse_cidr(cidr: &str) -> Option<ipnet::IpNet> {
            cidr.parse().ok()
        }

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

        match http_server_handle.lock() {
            Ok(mut guard) => *guard = Some(handle),
            Err(e) => {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("err", e.to_string());
                tracing::error!(
                    "{}",
                    crate::i18n::tr_args("config-http_lock_poisoned", args)
                );
            }
        }

        info!("HTTP monitoring server configured on {}", addr);
        Ok(())
    }
}
