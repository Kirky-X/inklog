// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! OTLP 日志导出 MVP（feature `otlp`）。
//!
//! [`OtlpSink`] 把记录编码为 OTLP/HTTP JSON（`resourceLogs`）批量 POST 到
//! OpenTelemetry Collector。传输层为手写 HTTP/1.1（std::net 阻塞 IO，与
//! sink worker 的 `spawn_blocking` + `block_on` 模式匹配；MVP 仅支持
//! `http://` 明文 endpoint，HTTPS 需 TLS 传输层后续演进）。
//!
//! 批量语义：`write` 聚合到内存批次，达到 `max_batch_size` 或收到 `flush`
//! （worker 空闲 tick）时发送；`shutdown` 尽力发完剩余批次。
//!
//! # Example
//! ```ignore
//! let sink = OtlpSink::new(OtlpConfig {
//!     endpoint: "http://otel-collector:4318/v1/logs".into(),
//!     ..Default::default()
//! })?;
//! LoggerManager::builder().add_sink(Arc::new(sink)).build().await?;
//! ```

use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;

use crate::support::io::LogSink;
use crate::{InklogError, LogRecord};
use std::sync::Arc;

/// OTLP sink 配置。
#[derive(Debug, Clone)]
pub struct OtlpConfig {
    /// OTLP/HTTP logs endpoint（如 `http://127.0.0.1:4318/v1/logs`）
    pub endpoint: String,
    /// 单批最大记录数（达到即发送；默认 100）
    pub max_batch_size: usize,
    /// HTTP 超时（默认 5s）
    pub timeout: Duration,
    /// Service name（OTel 语义资源属性 `service.name`）
    pub service_name: String,
    /// 导出前 PII 掩码开关（默认开启；OTLP 常出进程边界，默认不得裸奔）
    pub masking_enabled: bool,
    /// https endpoint 的 CA 证书 PEM 路径（与 `danger_accept_invalid_certs`
    /// 二选一；仅 net-sink feature 下生效，语义与 TcpSink 的 TlsClientConfig 一致）
    pub ca_pem_path: Option<std::path::PathBuf>,
    /// https endpoint 跳过证书校验（仅测试用途）
    pub danger_accept_invalid_certs: bool,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:4318/v1/logs".to_string(),
            max_batch_size: 100,
            timeout: Duration::from_secs(5),
            service_name: "inklog".to_string(),
            masking_enabled: true,
            ca_pem_path: None,
            danger_accept_invalid_certs: false,
        }
    }
}

/// severityText → OTLP severityNumber（Logs Data Model 标准映射）。
fn severity_number(level: &str) -> u8 {
    match level.to_ascii_uppercase().as_str() {
        "TRACE" | "DEBUG" => 5,
        "INFO" => 9,
        "WARN" => 13,
        "ERROR" => 17,
        "FATAL" => 21,
        _ => 0,
    }
}

/// 单条记录的 OTLP LogRecord JSON 片段。
///
/// `traceId`/`spanId` 来自 LogRecord 顶层字段（16/32 hex），使 Collector
/// 侧可以做日志↔trace 关联；无上下文时缺省（不输出空串字段）。
fn encode_log_record(record: &LogRecord) -> serde_json::Value {
    let mut attributes: Vec<serde_json::Value> = record
        .fields
        .iter()
        .map(|(k, v)| {
            serde_json::json!({
                "key": k,
                "value": { "stringValue": v.to_string() },
            })
        })
        .collect();
    attributes.push(serde_json::json!({
        "key": "target",
        "value": { "stringValue": record.target },
    }));
    let mut encoded = serde_json::json!({
        "timeUnixNano": record.timestamp.timestamp_nanos_opt().unwrap_or_default(),
        "severityText": record.level,
        "severityNumber": severity_number(&record.level),
        "body": { "stringValue": record.message },
        "attributes": attributes,
    });
    if let (Some(trace_id), Some(span_id)) = (&record.trace_id, &record.span_id) {
        encoded["traceId"] = serde_json::Value::String(trace_id.clone());
        encoded["spanId"] = serde_json::Value::String(span_id.clone());
    }
    encoded
}

/// 组装 OTLP/HTTP JSON 请求体（导出供下游复用同一编码）。
pub fn encode_otlp_body(records: &[LogRecord], service_name: &str) -> String {
    let log_records: Vec<serde_json::Value> = records.iter().map(encode_log_record).collect();
    serde_json::json!({
        "resourceLogs": [{
            "resource": {
                "attributes": [{
                    "key": "service.name",
                    "value": { "stringValue": service_name },
                }],
            },
            "scopeLogs": [{
                "logRecords": log_records,
            }],
        }],
    })
    .to_string()
}

/// 手写 HTTP/1.1 POST（明文 http；https 走 https_post_json；worker 阻塞线程执行）。
fn http_post_json(endpoint: &str, body: &str, timeout: Duration) -> Result<(), InklogError> {
    let (scheme, host, port, path) = parse_endpoint(endpoint)?;
    if scheme == Scheme::Https {
        return Err(InklogError::ConfigError(
            "OTLP https endpoint requires the 'net-sink' feature (rustls transport); \
             either use http:// or rebuild with --features net-sink"
                .to_string(),
        ));
    }
    // CR/LF 注入防护：host/path 直接内插进原始请求头，出现换行控制符
    // 即可注入额外头部/拆分响应
    if host.contains(['\r', '\n']) || path.contains(['\r', '\n']) {
        return Err(InklogError::ConfigError(format!(
            "OTLP endpoint contains CR/LF in host or path: '{endpoint}'"
        )));
    }
    let addr = format!("{host}:{port}");
    let mut stream = std::net::TcpStream::connect(&addr).map_err(|e| {
        InklogError::IoError(std::io::Error::other(format!(
            "OTLP connect to '{endpoint}': {e}"
        )))
    })?;
    stream
        .set_write_timeout(Some(timeout))
        .and_then(|_| stream.set_read_timeout(Some(timeout)))
        .map_err(|e| {
            InklogError::IoError(std::io::Error::other(format!("OTLP timeout setup: {e}")))
        })?;

    use std::io::Write;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| InklogError::IoError(std::io::Error::other(format!("OTLP send: {e}"))))?;
    stream.flush().ok();

    // 读状态行，2xx 视为成功
    use std::io::Read as _;
    let mut response = Vec::new();
    stream
        .take(8192)
        .read_to_end(&mut response)
        .map_err(|e| InklogError::IoError(std::io::Error::other(format!("OTLP read: {e}"))))?;
    let text = String::from_utf8_lossy(&response);
    let status = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);
    if !(200..300).contains(&status) {
        // 传输层/远端状态错误与配置错误分型：调用方的重试/熔断依据错误类别区分
        return Err(InklogError::RuntimeError(format!(
            "OTLP export to '{endpoint}' failed with HTTP status {status}"
        )));
    }
    Ok(())
}

/// https 传输：rustls StreamOwned 包装 TcpStream（复用 net-sink 的连接语义）。
/// `tls_config` 来自构造期校验（`build_tls_config`），此处不再重复解析 CA。
#[cfg(feature = "net-sink")]
fn https_post_json(
    endpoint: &str,
    host: &str,
    port: u16,
    path: &str,
    body: &str,
    timeout: Duration,
    tls_config: Arc<rustls::ClientConfig>,
) -> Result<(), InklogError> {
    let addr = format!("{host}:{port}");
    let stream = std::net::TcpStream::connect(&addr).map_err(|e| {
        InklogError::IoError(std::io::Error::other(format!(
            "OTLP connect to '{endpoint}': {e}"
        )))
    })?;
    stream
        .set_write_timeout(Some(timeout))
        .and_then(|_| stream.set_read_timeout(Some(timeout)))
        .map_err(|e| {
            InklogError::IoError(std::io::Error::other(format!("OTLP timeout setup: {e}")))
        })?;

    // server_name 取 host（不含端口）
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|e| InklogError::ConfigError(format!("OTLP TLS server name '{host}': {e}")))?;
    let conn = rustls::ClientConnection::new(tls_config, server_name)
        .map_err(|e| InklogError::IoError(std::io::Error::other(format!("OTLP TLS: {e}"))))?;
    let mut tls_stream = rustls::StreamOwned::new(conn, stream);

    use std::io::Write;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    tls_stream
        .write_all(request.as_bytes())
        .map_err(|e| InklogError::IoError(std::io::Error::other(format!("OTLP send: {e}"))))?;
    tls_stream.flush().ok();

    // 读状态行，2xx 视为成功
    use std::io::Read as _;
    let mut response = Vec::new();
    tls_stream
        .take(8192)
        .read_to_end(&mut response)
        .map_err(|e| InklogError::IoError(std::io::Error::other(format!("OTLP read: {e}"))))?;
    let text = String::from_utf8_lossy(&response);
    let status = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);
    if !(200..300).contains(&status) {
        return Err(InklogError::RuntimeError(format!(
            "OTLP export to '{endpoint}' failed with HTTP status {status}"
        )));
    }
    Ok(())
}

/// endpoint scheme。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scheme {
    Http,
    Https,
}

/// 解析 `http(s)://host[:port]/path`，https 缺省端口 443、http 缺省 80。
fn parse_endpoint(endpoint: &str) -> Result<(Scheme, String, u16, String), InklogError> {
    let (scheme, rest) = if let Some(rest) = endpoint.strip_prefix("https://") {
        (Scheme::Https, rest)
    } else if let Some(rest) = endpoint.strip_prefix("http://") {
        (Scheme::Http, rest)
    } else {
        return Err(InklogError::ConfigError(format!(
            "OTLP endpoint '{endpoint}' must start with http:// or https://"
        )));
    };
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let default_port = match scheme {
        Scheme::Http => 80,
        Scheme::Https => 443,
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (
            h.to_string(),
            p.parse::<u16>()
                .map_err(|_| InklogError::ConfigError(format!("bad OTLP port in '{endpoint}'")))?,
        ),
        None => (authority.to_string(), default_port),
    };
    Ok((scheme, host, port, path.to_string()))
}

/// OTLP 日志导出 sink（批量 + 手写 HTTP/1.1 传输）。
pub struct OtlpSink {
    config: OtlpConfig,
    batch: Mutex<Vec<LogRecord>>,
    /// None = masking_enabled=false（导出原样）
    masker: Option<crate::DataMasker>,
    /// https 传输配置（构造期由 build_tls_config 校验生成；http 为 None）
    #[cfg(feature = "net-sink")]
    tls: Option<Arc<rustls::ClientConfig>>,
}

impl OtlpSink {
    pub fn new(config: OtlpConfig) -> Result<Self, InklogError> {
        let (scheme, ..) = parse_endpoint(&config.endpoint)?;
        if scheme == Scheme::Https {
            // https 需要 net-sink feature（复用其 rustls 依赖与传输模式）
            #[cfg(not(feature = "net-sink"))]
            {
                let _ = &config;
                return Err(InklogError::ConfigError(
                    "OTLP https endpoint requires the 'net-sink' feature (rustls transport); \
                     either use http:// or rebuild with --features net-sink"
                        .to_string(),
                ));
            }
            #[cfg(feature = "net-sink")]
            Self::build_tls_config(&config)?;
        }
        let masker = config.masking_enabled.then(crate::DataMasker::new);
        #[cfg(feature = "net-sink")]
        let tls = match scheme {
            Scheme::Https => Some(Arc::new(Self::build_tls_config(&config)?)),
            Scheme::Http => None,
        };
        Ok(Self {
            config,
            batch: Mutex::new(Vec::new()),
            masker,
            #[cfg(feature = "net-sink")]
            tls,
        })
    }

    /// https 传输的 rustls 客户端配置（构造期校验，避免发送期才发现 CA 问题）。
    #[cfg(all(feature = "net-sink"))]
    fn build_tls_config(config: &OtlpConfig) -> Result<rustls::ClientConfig, InklogError> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .map_err(|e| InklogError::ConfigError(format!("TLS protocol versions: {e}")))?;
        if config.danger_accept_invalid_certs {
            return Ok(builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(super::net::AcceptAnyVerifier {
                    provider: (*provider).clone(),
                }))
                .with_no_client_auth());
        }
        let Some(ca_path) = &config.ca_pem_path else {
            return Err(InklogError::ConfigError(
                "OTLP https endpoint requires ca_pem_path (or danger_accept_invalid_certs for tests)"
                    .to_string(),
            ));
        };
        let mut roots = rustls::RootCertStore::empty();
        for cert in super::net::load_certs_pem(ca_path)? {
            roots
                .add(cert)
                .map_err(|e| InklogError::ConfigError(format!("CA cert rejected: {e}")))?;
        }
        Ok(builder.with_root_certificates(roots).with_no_client_auth())
    }

    /// 注入自定义 masker（覆盖 masking_enabled 的默认构造）。
    pub fn with_masker(mut self, masker: crate::DataMasker) -> Self {
        self.masker = Some(masker);
        self
    }

    /// 入批前脱敏：message 过值掩码，fields 过键值遍历掩码。
    fn mask_record(&self, record: &LogRecord) -> LogRecord {
        match self.masker.as_ref() {
            Some(masker) => {
                let mut fields = record.fields.clone();
                masker.mask_hashmap(&mut fields);
                LogRecord {
                    message: masker.mask(&record.message),
                    fields,
                    ..record.clone()
                }
            }
            None => record.clone(),
        }
    }

    /// 当前批内记录数（诊断）。
    pub fn pending(&self) -> usize {
        self.batch.lock().map(|b| b.len()).unwrap_or(0)
    }

    /// 立即发送当前批次。
    fn send_now(&self, batch: &mut Vec<LogRecord>) -> Result<(), InklogError> {
        if batch.is_empty() {
            return Ok(());
        }
        let body = encode_otlp_body(batch, &self.config.service_name);
        batch.clear();
        let (scheme, ..) = parse_endpoint(&self.config.endpoint)?;
        match scheme {
            Scheme::Http => http_post_json(&self.config.endpoint, &body, self.config.timeout),
            #[cfg(feature = "net-sink")]
            Scheme::Https => {
                let (_, host, port, path) = parse_endpoint(&self.config.endpoint)?;
                let tls = Arc::clone(self.tls.as_ref().ok_or_else(|| {
                    InklogError::ConfigError(
                        "OTLP TLS config missing for https endpoint".to_string(),
                    )
                })?);
                https_post_json(
                    &self.config.endpoint,
                    &host,
                    port,
                    &path,
                    &body,
                    self.config.timeout,
                    tls,
                )
            }
            #[cfg(not(feature = "net-sink"))]
            Scheme::Https => Err(InklogError::ConfigError(
                "OTLP https endpoint requires the 'net-sink' feature (rustls transport); \
                 either use http:// or rebuild with --features net-sink"
                    .to_string(),
            )),
        }
    }
}

#[async_trait]
impl LogSink for OtlpSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        let should_send = {
            let mut batch = self
                .batch
                .lock()
                .map_err(|_| InklogError::ConfigError("otlp batch poisoned".to_string()))?;
            batch.push(self.mask_record(record));
            batch.len() >= self.config.max_batch_size
        };
        if should_send {
            let mut batch = self
                .batch
                .lock()
                .map_err(|_| InklogError::ConfigError("otlp batch poisoned".to_string()))?;
            self.send_now(&mut batch)?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), InklogError> {
        let mut batch = self
            .batch
            .lock()
            .map_err(|_| InklogError::ConfigError("otlp batch poisoned".to_string()))?;
        self.send_now(&mut batch)
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        self.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::sync::Arc;

    fn record(message: &str) -> LogRecord {
        let mut r = LogRecord::new(
            tracing::Level::ERROR,
            "otlp::test".to_string(),
            message.to_string(),
        );
        r.fields.insert("k".to_string(), serde_json::json!("v"));
        r
    }

    #[test]
    fn test_encode_otlp_body_shape() {
        let body = encode_otlp_body(&[record("hello otlp")], "svc-1");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let resource_logs = json["resourceLogs"].as_array().unwrap();
        assert_eq!(resource_logs.len(), 1);
        assert_eq!(
            resource_logs[0]["resource"]["attributes"][0]["key"],
            "service.name"
        );
        let log_records = resource_logs[0]["scopeLogs"][0]["logRecords"]
            .as_array()
            .unwrap();
        assert_eq!(log_records.len(), 1);
        assert_eq!(log_records[0]["severityText"], "ERROR");
        assert_eq!(log_records[0]["body"]["stringValue"], "hello otlp");
        // target 附加为属性
        let attrs = log_records[0]["attributes"].as_array().unwrap();
        assert!(
            attrs
                .iter()
                .any(|a| a["key"] == "target" && a["value"]["stringValue"] == "otlp::test")
        );
    }

    #[test]
    fn test_parse_endpoint_schemes_and_defaults() {
        assert!(parse_endpoint("not-a-url").is_err());
        assert!(parse_endpoint("ftp://host/x").is_err());
        assert!(parse_endpoint("http://host:99999").is_err());

        let (scheme, host, port, path) = parse_endpoint("http://127.0.0.1:4318/v1/logs").unwrap();
        assert_eq!(scheme, Scheme::Http);
        assert_eq!(
            (host.as_str(), port, path.as_str()),
            ("127.0.0.1", 4318, "/v1/logs")
        );

        let (scheme, host, port, path) = parse_endpoint("https://collector/v1/logs").unwrap();
        assert_eq!(scheme, Scheme::Https);
        assert_eq!(
            (host.as_str(), port, path.as_str()),
            ("collector", 443, "/v1/logs")
        );
    }

    /// https 无 net-sink feature：构造期即报错并提示 feature。
    #[cfg(not(feature = "net-sink"))]
    #[test]
    fn test_https_endpoint_rejected_without_net_sink_feature() {
        let err = match OtlpSink::new(OtlpConfig {
            endpoint: "https://collector:4318/v1/logs".to_string(),
            ..Default::default()
        }) {
            Err(e) => e,
            Ok(_) => panic!("https must be rejected without net-sink feature"),
        };
        assert!(
            err.to_string().contains("net-sink"),
            "error must mention the net-sink feature, got: {err}"
        );
    }

    /// https 有 feature 但缺 CA 且未显式放行：构造期报 CA 配置错误。
    #[cfg(feature = "net-sink")]
    #[test]
    fn test_https_endpoint_requires_ca_or_danger_flag() {
        let err = match OtlpSink::new(OtlpConfig {
            endpoint: "https://collector:4318/v1/logs".to_string(),
            ..Default::default()
        }) {
            Err(e) => e,
            Ok(_) => panic!("https without CA and without danger flag must fail"),
        };
        assert!(
            err.to_string().contains("ca_pem_path"),
            "error must mention CA configuration, got: {err}"
        );

        // danger 标志放行：构造成功（不发起连接）
        let sink = OtlpSink::new(OtlpConfig {
            endpoint: "https://collector:4318/v1/logs".to_string(),
            danger_accept_invalid_certs: true,
            ..Default::default()
        });
        assert!(sink.is_ok());
    }

    /// mock collector：接受一次 POST，捕获请求体并回 200。

    /// mock collector 标准行为：完整读取请求（头 + Content-Length 体）后响应 `status`。
    fn serve_one_request(
        listener: std::net::TcpListener,
        captured: Arc<Mutex<Vec<u8>>>,
        response: &'static [u8],
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            // 读头
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            let header_end;
            loop {
                let n = stream.read(&mut chunk).expect("read");
                assert!(n > 0, "client closed before sending headers");
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
                    header_end = pos + 4;
                    break;
                }
            }
            // 按 Content-Length 读完 body
            let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
            let content_length: usize = head
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    k.eq_ignore_ascii_case("content-length")
                        .then(|| v.trim().parse().ok())
                })
                .flatten()
                .unwrap_or(0);
            while buf.len() < header_end + content_length {
                let n = stream.read(&mut chunk).expect("read body");
                assert!(n > 0, "client closed before sending body");
                buf.extend_from_slice(&chunk[..n]);
            }
            *captured.lock().unwrap() = buf;
            let _ = std::io::Write::write_all(&mut stream, response);
        })
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    #[tokio::test]
    async fn test_otlp_sink_exports_to_mock_collector() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
        let server = serve_one_request(
            listener,
            captured.clone(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );

        let sink = OtlpSink::new(OtlpConfig {
            endpoint: format!("http://{addr}/v1/logs"),
            max_batch_size: 10,
            timeout: Duration::from_secs(5),
            service_name: "inklog-test".to_string(),
            masking_enabled: true,
            ca_pem_path: None,
            danger_accept_invalid_certs: false,
        })
        .unwrap();

        sink.write(&record("exported record")).await.unwrap();
        assert_eq!(
            sink.pending(),
            1,
            "batching: below max_batch_size stays buffered"
        );
        sink.flush().await.unwrap();

        server.join().unwrap();
        let raw = String::from_utf8_lossy(&captured.lock().unwrap()).to_string();
        assert!(
            raw.starts_with("POST /v1/logs HTTP/1.1"),
            "must be an OTLP/HTTP POST"
        );
        assert!(raw.contains("Content-Type: application/json"));
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or("");
        let json: serde_json::Value = serde_json::from_str(body).expect("body must be JSON");
        assert_eq!(
            json["resourceLogs"][0]["resource"]["attributes"][0]["value"]["stringValue"],
            "inklog-test"
        );
        assert_eq!(
            json["resourceLogs"][0]["scopeLogs"][0]["logRecords"][0]["body"]["stringValue"],
            "exported record"
        );
    }

    /// 批量触发：达到 max_batch_size 自动发送，无需显式 flush。
    #[tokio::test]
    async fn test_otlp_sink_auto_flush_on_batch_full() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
        let server = serve_one_request(
            listener,
            captured.clone(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );

        let sink = OtlpSink::new(OtlpConfig {
            endpoint: format!("http://{addr}/v1/logs"),
            max_batch_size: 2,
            timeout: Duration::from_secs(5),
            service_name: "inklog-test".to_string(),
            masking_enabled: true,
            ca_pem_path: None,
            danger_accept_invalid_certs: false,
        })
        .unwrap();

        sink.write(&record("one")).await.unwrap();
        assert_eq!(sink.pending(), 1);
        sink.write(&record("two")).await.unwrap();
        // 第 2 条达到 max_batch_size → 自动发送
        assert_eq!(sink.pending(), 0, "batch must auto-flush when full");

        server.join().unwrap();
        let raw = String::from_utf8_lossy(&captured.lock().unwrap()).to_string();
        assert!(raw.contains("\"one\"") && raw.contains("\"two\""));
    }

    /// collector 5xx → 写入报错（worker 按既有重试/降级语义处理）。
    #[tokio::test]
    async fn test_otlp_sink_surfaces_collector_error() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = serve_one_request(
            listener,
            Arc::new(Mutex::new(Vec::new())),
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );

        let sink = OtlpSink::new(OtlpConfig {
            endpoint: format!("http://{addr}/v1/logs"),
            max_batch_size: 1,
            timeout: Duration::from_secs(5),
            service_name: "inklog-test".to_string(),
            masking_enabled: true,
            ca_pem_path: None,
            danger_accept_invalid_certs: false,
        })
        .unwrap();

        let err = sink.write(&record("x")).await.unwrap_err();
        assert!(
            err.to_string().contains("503"),
            "collector failure must surface, got {err}"
        );
        server.join().unwrap();
    }

    #[test]
    fn test_encode_log_record_includes_trace_fields_and_severity_number() {
        let mut rec = record("with context");
        // W3C 规范文档中的示例 ID（非凭据）—— pragma 放行 detect-secrets 高熵启发式
        rec.trace_id = Some("0af7651916cd43dd8448eb211c80319c".to_string()); // pragma: allowlist secret
        rec.span_id = Some("b7ad6b7169203331".to_string()); // pragma: allowlist secret
        let encoded = encode_log_record(&rec);
        assert_eq!(
            encoded["traceId"],
            "0af7651916cd43dd8448eb211c80319c", // pragma: allowlist secret
            "32-hex trace id must be exported for log-trace correlation"
        );
        assert_eq!(encoded["spanId"], "b7ad6b7169203331"); // pragma: allowlist secret
        assert_eq!(encoded["severityNumber"], 17, "ERROR maps to 17");

        // 无上下文：字段缺省而不是空串
        let plain = encode_log_record(&record("no context"));
        assert!(plain.get("traceId").is_none());
        assert!(plain.get("spanId").is_none());
        // 级别映射表
        assert_eq!(severity_number("INFO"), 9);
        assert_eq!(severity_number("WARN"), 13);
        assert_eq!(severity_number("FATAL"), 21);
        assert_eq!(severity_number("debug"), 5);
    }

    /// 出站掩码：message 与 fields 中的明文 PII 不得进入导出请求体。
    #[tokio::test]
    async fn test_otlp_sink_masks_pii_before_export() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
        let server = serve_one_request(
            listener,
            captured.clone(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );

        let sink = OtlpSink::new(OtlpConfig {
            endpoint: format!("http://{addr}/v1/logs"),
            max_batch_size: 10,
            timeout: Duration::from_secs(5),
            service_name: "inklog-test".to_string(),
            masking_enabled: true,
            ca_pem_path: None,
            danger_accept_invalid_certs: false,
        })
        .unwrap();

        let mut rec = record("user phone 13812345678 login");
        rec.fields
            .insert("phone".to_string(), serde_json::json!("13912345678"));
        sink.write(&rec).await.unwrap();
        sink.flush().await.unwrap();
        server.join().unwrap();

        let raw = String::from_utf8_lossy(&captured.lock().unwrap()).to_string();
        assert!(
            !raw.contains("13812345678") && !raw.contains("13912345678"),
            "export body must not carry plaintext PII"
        );
    }
}
