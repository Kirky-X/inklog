// Copyright (c) 2026 Kirky.X🌠
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
                        let mut args = crate::i18n::MsgArgs::new();
                        args.set("env", token_env);
                        return Err(InklogError::ConfigError(crate::i18n::tr_args(
                            "config-http_auth_token_empty",
                            args,
                        )));
                    }
                    Err(_) => {
                        let mut args = crate::i18n::MsgArgs::new();
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
            // DNS rebinding 防护：对端为回环时只接受回环 Host——恶意站点可把
            // 自有域名 rebind 到 127.0.0.1，此时 Origin 与目标同源，跨源
            // 读取不再被同源策略阻断，只能由服务端校验 Host 拦住。
            if let Some(response) = loopback_host_check(addr.ip(), &request) {
                return response;
            }

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
                            return (
                                StatusCode::UNAUTHORIZED,
                                crate::i18n::tr("config-http_invalid_token"),
                            )
                                .into_response();
                        }
                    }
                    _ => {
                        return (
                            StatusCode::UNAUTHORIZED,
                            crate::i18n::tr("config-http_missing_auth_header"),
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
                    return (
                        StatusCode::FORBIDDEN,
                        crate::i18n::tr("config-http_ip_not_whitelisted"),
                    )
                        .into_response();
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
                            let mut args = crate::i18n::MsgArgs::new();
                            args.set("err", e.to_string());
                            tracing::error!(
                                "{}",
                                crate::i18n::tr_args("config-http_serialize_failed", args)
                            );
                            axum::Json(
                                serde_json::json!({"error": crate::i18n::tr("config-http_serialization_failed")}),
                            )
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

        // 绑定地址解析与配置期回环判定共用同一函数：两处各按一套规则解析 host
        // 会让守卫放行一个起不来的端点（判为回环却绑不上）。
        let addr: std::net::SocketAddr = crate::domain::config::http::bind_socket_addr(
            &config.host,
            config.port,
        )
        .map_err(|e: std::net::AddrParseError| {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("addr", format!("{}:{}", config.host, config.port));
            args.set("err", e.to_string());
            InklogError::ConfigError(crate::i18n::tr_args("config-invalid_http_address", args))
        })?;

        let auth_enabled = config.auth.as_ref().map(|a| a.enabled).unwrap_or(false);
        let ip_whitelist = config.ip_whitelist.clone();
        let error_mode = config.error_mode.clone();

        let tls_config = config.tls.clone();

        let make_svc = app.into_make_service_with_connect_info::<SocketAddr>();

        // 修复（HttpErrorMode::Strict 形同虚设）：端口 bind 与 TLS 配置构建
        // 从 spawn 出去的任务移到本函数内同步执行——bind 成功后才 spawn 仅含 serve
        // 循环的任务。Strict 模式下失败直接返回 Err（启动失败）；Warn 模式下仅告警
        // 并返回 Ok、不启动服务器（决策见 [`bind_failure_outcome`]）。
        let handle = if let Some(ref tls) = tls_config {
            use axum_server::tls_rustls::RustlsConfig;

            // TLS 证书/密钥解析同步完成，失败走同一 error_mode 决策
            let rustls_config =
                match RustlsConfig::from_pem_file(&tls.cert_path, &tls.key_path).await {
                    Ok(c) => c,
                    Err(e) => return bind_failure_outcome(&error_mode, addr, &e, true),
                };
            // axum-server 0.8 的 `bind_rustls` 是惰性 bind（serve 阶段才绑端口，
            // 失败只会落在 spawn 任务内）。故先自行 bind TCP 端口以同步观测失败，
            // 再经 `from_tcp_rustls` 把 listener 交还 axum-server，serve 语义不变。
            let tcp_listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => return bind_failure_outcome(&error_mode, addr, &e, true),
            };
            let std_listener = match tcp_listener.into_std() {
                Ok(l) => l,
                Err(e) => return bind_failure_outcome(&error_mode, addr, &e, true),
            };
            // from_tcp_rustls 内部经 TcpListener::from_std 接管 fd，要求非阻塞模式
            if let Err(e) = std_listener.set_nonblocking(true) {
                return bind_failure_outcome(&error_mode, addr, &e, true);
            }
            let server = match axum_server::tls_rustls::from_tcp_rustls(std_listener, rustls_config)
            {
                Ok(s) => s,
                Err(e) => return bind_failure_outcome(&error_mode, addr, &e, true),
            };
            let mut args = crate::i18n::MsgArgs::new();
            args.set("addr", addr);
            args.set("auth", auth_enabled);
            args.set("whitelist", format!("{:?}", ip_whitelist));
            info!("{}", crate::i18n::tr_args("log-https_server_started", args));
            tokio::spawn(async move {
                if let Err(e) = server.serve(make_svc).await {
                    let mut args = crate::i18n::MsgArgs::new();
                    args.set("err", e.to_string());
                    tracing::error!(
                        "{}",
                        crate::i18n::tr_args("config-https_server_error", args)
                    );
                }
            })
        } else {
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => return bind_failure_outcome(&error_mode, addr, &e, false),
            };
            let mut args = crate::i18n::MsgArgs::new();
            args.set("addr", addr);
            args.set("auth", auth_enabled);
            args.set("whitelist", format!("{:?}", ip_whitelist));
            info!("{}", crate::i18n::tr_args("log-http_server_started", args));
            tokio::spawn(async move {
                match axum::serve(listener, make_svc).await {
                    Ok(_) => info!("{}", crate::i18n::tr("log-http_server_stopped")),
                    Err(e) => {
                        let mut args = crate::i18n::MsgArgs::new();
                        args.set("err", e.to_string());
                        tracing::error!(
                            "{}",
                            crate::i18n::tr_args("config-http_server_error", args)
                        );
                    }
                }
            })
        };

        // 此锁仅保护 JoinHandle 槽位，无复合不变量：毒化时恢复出守卫继续写入，
        // 消除"仅记日志但 handle 未存入"的路径
        let mut handle_guard = http_server_handle.lock().unwrap_or_else(|e| e.into_inner());
        *handle_guard = Some(handle);

        let mut args = crate::i18n::MsgArgs::new();
        args.set("addr", addr);
        info!(
            "{}",
            crate::i18n::tr_args("log-http_monitoring_configured", args)
        );
        Ok(())
    }
}

/// DNS rebinding 防护判定：对端为回环时，目标主机必须是回环主机。
///
/// 放行返回 `None`；拒绝返回 403 响应。
///
/// 触发条件按**对端地址**判定而非绑定地址：通配绑定（`0.0.0.0` / `::`）
/// 下浏览器经 rebinding 连到的对端同样是回环地址，必须校验 Host；而 LAN
/// 对端不是回环，不受 Host 限制（否则正常 LAN 抓取会被误杀）。
#[cfg(feature = "http")]
fn loopback_host_check<B>(
    peer_ip: std::net::IpAddr,
    request: &axum::http::Request<B>,
) -> Option<axum::response::Response> {
    use axum::response::IntoResponse;

    if !peer_ip.is_loopback() {
        return None;
    }
    let (host_header, authority) = request_host_values(request);
    if request_host_allowed(host_header, authority) {
        return None;
    }
    Some(
        (
            axum::http::StatusCode::FORBIDDEN,
            crate::i18n::tr("config-http_invalid_host"),
        )
            .into_response(),
    )
}

/// 取请求的目标主机来源：`Host` 头与 URI authority。
///
/// HTTP/2 请求不带 `Host` 头（`:authority` 伪头落到 URI authority 上），
/// 故两者都要读。
#[cfg(feature = "http")]
fn request_host_values<B>(request: &axum::http::Request<B>) -> (Option<&str>, Option<&str>) {
    use axum::http::header;

    let host_header = request
        .headers()
        .get(header::HOST)
        .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());
    let authority = request.uri().authority().map(|a| a.as_str());
    (host_header, authority)
}

/// 目标主机判定：`Host` 头优先，缺失时回退 URI authority，两者并存必须一致。
///
/// HTTP/1.1 请求带 `Host` 头；HTTP/2 请求只带 `:authority`（映射到 URI
/// authority、没有 `Host` 头）。两者都缺失按 fail-closed 拒绝；两者并存
/// 但指向不同主机时视为歧义输入并拒绝——不能「各自判定后取与」，那会让
/// 不一致的输入退化成单边判定。
#[cfg(feature = "http")]
fn request_host_allowed(host_header: Option<&str>, authority: Option<&str>) -> bool {
    match (host_header, authority) {
        (None, None) => false,
        (Some(host), None) => host_header_allowed(Some(host)),
        (None, Some(authority)) => host_header_allowed(Some(authority)),
        (Some(host), Some(authority)) => {
            match (parse_host_value(host), parse_host_value(authority)) {
                (Some(host), Some(authority)) => host == authority && is_loopback_host_name(&host),
                _ => false,
            }
        }
    }
}

/// Host 头校验（DNS rebinding 防护）：只接受回环主机。
///
/// 缺失 Host 头按 fail-closed 拒绝。
#[cfg(feature = "http")]
fn host_header_allowed(host_header: Option<&str>) -> bool {
    let Some(raw) = host_header else {
        return false;
    };
    parse_host_value(raw).is_some_and(|host| is_loopback_host_name(&host))
}

/// 严格解析 Host/authority 值为规范化主机名（小写、去端口、去 IPv6 方括号）。
///
/// 标准 authority 解析有三处宽松点会掩盖真实目标：userinfo（`@`）被剥离
/// 后给出另一个主机、非数字端口被静默丢弃、方括号后的残留字符被忽略。
/// 这三类一律拒绝；其余畸形值（非法 URI 字符、多重端口）由标准解析挡住。
#[cfg(feature = "http")]
fn parse_host_value(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.contains('@') {
        return None;
    }
    // 标准 authority 解析：非法 URI 字符、多重端口等畸形值在此被拒
    raw.parse::<axum::http::uri::Authority>().ok()?;
    let (host, port) = split_host_port(raw)?;
    if let Some(port) = port
        && (port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    if host.is_empty() {
        return None;
    }
    Some(host.to_ascii_lowercase())
}

/// 拆出主机部分与端口部分（端口为 `None` 表示未带端口）。
///
/// 方括号形式（IPv6 字面量）的 `]` 之后只允许 `:端口` 或空串，其余视为畸形。
#[cfg(feature = "http")]
fn split_host_port(raw: &str) -> Option<(&str, Option<&str>)> {
    if let Some(rest) = raw.strip_prefix('[') {
        let end = rest.find(']')?;
        let tail = &rest[end + 1..];
        let port = if tail.is_empty() {
            None
        } else {
            Some(tail.strip_prefix(':')?)
        };
        Some((&rest[..end], port))
    } else {
        match raw.split_once(':') {
            Some((host, port)) => Some((host, Some(port))),
            None => Some((raw, None)),
        }
    }
}

/// 主机名是否为回环主机：`localhost`（大小写不敏感）或回环 IP 字面量。
#[cfg(feature = "http")]
fn is_loopback_host_name(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<std::net::IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

/// bind / TLS 初始化失败后按 [`crate::HttpErrorMode`] 决策启动行为
/// （修复：此前失败被吞在 spawn 任务内，Strict 形同虚设）。
///
/// - [`crate::HttpErrorMode::Strict`]：启动失败，返回 `Err`（配置要求失败即拒）。
/// - [`crate::HttpErrorMode::Warn`]：降级——记录 warn 后返回 `Ok`，但 HTTP 服务器
///   不会启动（对调用方无感，仅日志可见）。
///
/// `https` 仅选择 i18n 文案：HTTPS 路径复用 `config-https_server_error`，
/// TCP 路径复用 `config-http_bind_failed`（与运行期 serve 错误口径一致）。
#[cfg(feature = "http")]
fn bind_failure_outcome(
    error_mode: &crate::HttpErrorMode,
    addr: std::net::SocketAddr,
    err: &std::io::Error,
    https: bool,
) -> Result<(), InklogError> {
    let mut args = crate::i18n::MsgArgs::new();
    args.set("addr", addr.to_string());
    args.set("err", err.to_string());
    let message_key = if https {
        "config-https_server_error"
    } else {
        "config-http_bind_failed"
    };
    match error_mode {
        crate::HttpErrorMode::Strict => Err(InklogError::ConfigError(crate::i18n::tr_args(
            message_key,
            args,
        ))),
        crate::HttpErrorMode::Warn => {
            tracing::warn!("{}", crate::i18n::tr_args(message_key, args));
            Ok(())
        }
    }
}

/// 单条 IP 白名单匹配：`IP` 精确、`前缀.*` 通配（补结尾点防越界前缀匹配）、`CIDR` 经 [`parse_cidr`]。
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
                        "{}",
                        crate::i18n::tr("config-http_ip_whitelist_entry_invalid")
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
    use super::bind_failure_outcome;
    use super::host_header_allowed;
    use super::loopback_host_check;
    use super::request_host_allowed;
    use super::request_host_values;
    use super::whitelist_entry_matches;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;

    fn ip(s: &str) -> std::net::IpAddr {
        s.parse().unwrap()
    }

    /// Host 头校验：回环主机放行，rebind 用的非回环 Host 拒绝。
    #[test]
    fn test_host_header_allowed_only_loopback() {
        assert!(host_header_allowed(Some("127.0.0.1:9090")));
        assert!(host_header_allowed(Some("127.0.0.1")));
        assert!(host_header_allowed(Some("localhost:9090")));
        assert!(host_header_allowed(Some("LOCALHOST")));
        assert!(host_header_allowed(Some("[::1]:9090")));
        // rebind 形态：域名解析到 127.0.0.1，但 Host 头是攻击者的域名
        assert!(!host_header_allowed(Some("evil.example.com")));
        assert!(!host_header_allowed(Some("evil.example.com:9090")));
        assert!(!host_header_allowed(Some("192.168.1.10:9090")));
        // 缺失/空 Host：fail-closed
        assert!(!host_header_allowed(None));
        assert!(!host_header_allowed(Some("")));
    }

    /// 畸形 authority 必须拒绝：userinfo、非数字端口、方括号后残留字符、多重端口。
    #[test]
    fn test_request_host_allowed_rejects_malformed_authority() {
        for raw in [
            "[::1]@evil",
            "localhost:9090@evil",
            "127.0.0.1:9090@evil",
            "[::1]junk",
            "127.0.0.1:not-a-port",
            "127.0.0.1:80:99",
        ] {
            assert!(
                !request_host_allowed(Some(raw), None),
                "malformed host value must be rejected: {raw}"
            );
        }
        // 合法值仍然放行（含 IPv6 回环与文档列出的允许值）
        for raw in ["[::1]:9090", "[::1]", "127.0.0.1:9090", "localhost:9090"] {
            assert!(
                request_host_allowed(Some(raw), None),
                "valid host value must pass: {raw}"
            );
        }
    }

    /// 目标主机判定口径：Host 头优先、缺失回退 URI authority、并存必须一致。
    #[test]
    fn test_request_host_allowed_combines_host_and_authority() {
        // 只有 Host 头（HTTP/1.1 形态）
        assert!(request_host_allowed(Some("127.0.0.1:9090"), None));
        // 只有 URI authority（HTTP/2 形态：`:authority` 落到 URI，无 Host 头）
        assert!(request_host_allowed(None, Some("127.0.0.1:9090")));
        // 两者都缺失：fail-closed
        assert!(!request_host_allowed(None, None));
        // 两者并存且指向同一主机 → 放行（端口不同不影响主机一致性）
        assert!(request_host_allowed(
            Some("127.0.0.1:9090"),
            Some("127.0.0.1:9090")
        ));
        assert!(request_host_allowed(
            Some("127.0.0.1:9090"),
            Some("127.0.0.1")
        ));
        // 两者并存但主机不一致 → 拒绝（任一方向都不放行）
        assert!(!request_host_allowed(
            Some("evil.example.com"),
            Some("127.0.0.1")
        ));
        assert!(!request_host_allowed(
            Some("127.0.0.1"),
            Some("evil.example.com")
        ));
    }

    /// HTTP/2 请求形态回归：`:authority` 伪头落到 URI authority，请求里没有
    /// `Host` 头——只看 Host 头会把这类合法回环请求判成非法主机。
    #[test]
    fn test_loopback_host_check_accepts_http2_request_without_host_header() {
        let request = axum::http::Request::builder()
            .method("GET")
            .uri("http://127.0.0.1:9090/metrics")
            .body(())
            .unwrap();
        // h2 请求不带 Host 头（RFC 7540 禁止），目标主机只体现在 URI authority
        assert!(request.headers().get(axum::http::header::HOST).is_none());
        let (host_header, authority) = request_host_values(&request);
        assert!(host_header.is_none());
        assert_eq!(authority, Some("127.0.0.1:9090"));
        assert!(
            loopback_host_check(ip("127.0.0.1"), &request).is_none(),
            "HTTP/2 request with loopback authority must pass the host check"
        );
    }

    /// Host 头与 authority 并存但指向不同主机 → 403。
    #[test]
    fn test_loopback_host_check_rejects_inconsistent_host_and_authority() {
        let evil_header = axum::http::Request::builder()
            .method("GET")
            .uri("http://127.0.0.1:9090/metrics")
            .header(axum::http::header::HOST, "evil.example.com")
            .body(())
            .unwrap();
        let response = loopback_host_check(ip("127.0.0.1"), &evil_header)
            .expect("Host 头与 authority 不一致必须拒绝");
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);

        let evil_authority = axum::http::Request::builder()
            .method("GET")
            .uri("http://evil.example.com/metrics")
            .header(axum::http::header::HOST, "127.0.0.1:9090")
            .body(())
            .unwrap();
        let response = loopback_host_check(ip("127.0.0.1"), &evil_authority)
            .expect("Host 头与 authority 不一致必须拒绝");
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
    }

    /// Host 校验按对端地址触发：回环对端必须过校验，通配绑定下的 LAN 对端不受限。
    #[test]
    fn test_loopback_host_check_applies_to_loopback_peer_only() {
        let rebind = axum::http::Request::builder()
            .method("GET")
            .uri("http://127.0.0.1:9090/metrics")
            .header(axum::http::header::HOST, "evil.example.com")
            .body(())
            .unwrap();
        // rebinding 后浏览器对端是回环，非回环 Host 必须 403
        assert_eq!(
            loopback_host_check(ip("127.0.0.1"), &rebind).map(|r| r.status()),
            Some(axum::http::StatusCode::FORBIDDEN),
            "loopback peer with non-loopback Host must be rejected"
        );

        // LAN 对端带自身 Host：不受 Host 校验限制（不误伤正常 LAN 访问）
        let lan = axum::http::Request::builder()
            .method("GET")
            .uri("/metrics")
            .header(axum::http::header::HOST, "10.0.0.5:9090")
            .body(())
            .unwrap();
        assert!(
            loopback_host_check(ip("10.0.0.5"), &lan).is_none(),
            "LAN peer must not be subject to the loopback Host check"
        );
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
        assert!(whitelist_entry_matches(
            "10.*",
            "10.1.2.3",
            ip("10.1.2.3"),
            &flag
        ));
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

    // 修复（Strict 形同虚设）：bind 失败按 HttpErrorMode 传播

    #[test]
    fn test_bind_failure_outcome_strict_propagates_error() {
        let addr: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let err = std::io::Error::new(std::io::ErrorKind::AddrInUse, "address in use");

        // Strict 模式：bind 失败必须向上传播为 Err（启动失败）
        let result = bind_failure_outcome(&crate::HttpErrorMode::Strict, addr, &err, false);
        let msg = match result {
            Err(crate::InklogError::ConfigError(msg)) => msg,
            other => panic!("expected ConfigError, got {other:?}"),
        };
        // 错误消息应包含 addr 与 err（i18n 渲染）
        assert!(msg.contains("127.0.0.1"), "addr should appear: {msg}");
        assert!(msg.contains("address in use"), "err should appear: {msg}");
    }

    #[test]
    fn test_bind_failure_outcome_warn_degrades_to_ok() {
        let addr: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let err = std::io::Error::new(std::io::ErrorKind::AddrInUse, "address in use");

        // Warn（Lenient）模式：降级为告警，返回 Ok 但服务器不启动
        let result = bind_failure_outcome(&crate::HttpErrorMode::Warn, addr, &err, false);
        assert!(result.is_ok(), "Warn mode must degrade bind failure to Ok");
    }

    #[tokio::test]
    async fn test_bind_conflict_propagates_per_error_mode() {
        // 真实 bind 冲突：先占用一个 OS 分配的端口，再对同一地址二次 bind
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = occupied.local_addr().unwrap();
        let bind_err = tokio::net::TcpListener::bind(addr)
            .await
            .expect_err("binding an occupied port must fail");

        // 同一 bind 失败在两种模式下的传播路径
        assert!(
            bind_failure_outcome(&crate::HttpErrorMode::Strict, addr, &bind_err, false).is_err(),
            "Strict mode must surface bind conflict as Err"
        );
        assert!(
            bind_failure_outcome(&crate::HttpErrorMode::Warn, addr, &bind_err, false).is_ok(),
            "Warn mode must degrade bind conflict to Ok"
        );
    }
}
