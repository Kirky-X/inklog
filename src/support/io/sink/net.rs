// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 网络转发 Sink（feature `net-sink`）。
//!
//! - [`TcpSink`]：NDJSON/TCP 转发（可选 TLS，rustls 客户端）；断线时写入
//!   进入有界内存缓冲（满则丢最旧），后续写入触发自动重连并按序补发缓冲；
//! - [`UdpSink`]：数据报转发（无连接，尽力而为）。
//!
//! 记录以一行 JSON（newline-delimited JSON）出站，下游 collector 直接可解析。
//! 实现走 `std::net` 阻塞 IO——sink worker 运行在 `spawn_blocking` 线程上。
//!
//! TLS 说明（MVP）：客户端 TLS 支持 CA PEM 或 `danger_accept_invalid_certs`
//! （仅限测试/内网自签场景）；TLS 握手集成测试需自签服务端（未含），本地
//! socket 集成测试覆盖明文 TCP/UDP 全链路。
//!
//! # Example
//! ```ignore
//! let tcp = TcpSink::new(TcpSinkConfig {
//!     addr: "127.0.0.1:5170".into(),
//!     tls: None,
//!     ..Default::default()
//! })?;
//! LoggerManager::builder().add_sink(Arc::new(tcp)).build().await?;
//! ```

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;

use crate::support::io::LogSink;
use crate::{InklogError, LogRecord};

/// 出站线格式（一行一条记录）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NetWireFormat {
    /// JSON lines（默认；下游直接可解析）
    #[default]
    JsonLine,
    /// 默认模板单行文本
    Text,
}

/// TLS 客户端配置。
#[derive(Debug, Clone, Default)]
pub struct TlsClientConfig {
    /// SNI 服务器名（必填）
    pub server_name: String,
    /// CA 证书 PEM 路径（与 `danger_accept_invalid_certs` 二选一）
    pub ca_pem_path: Option<PathBuf>,
    /// 危险：跳过服务端证书校验（仅测试/自签内网）
    pub danger_accept_invalid_certs: bool,
}

/// TCP sink 配置。
#[derive(Debug, Clone)]
pub struct TcpSinkConfig {
    /// 目标地址（`host:port`）
    pub addr: String,
    /// TLS（None = 明文）
    pub tls: Option<TlsClientConfig>,
    /// 连接超时（默认 3s）
    pub connect_timeout: Duration,
    /// 断线缓冲容量（条数，满则丢最旧；默认 10000）
    pub buffer_capacity: usize,
    /// 出站格式
    pub format: NetWireFormat,
}

impl Default for TcpSinkConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:5170".to_string(),
            tls: None,
            connect_timeout: Duration::from_secs(3),
            buffer_capacity: 10_000,
            format: NetWireFormat::JsonLine,
        }
    }
}

/// UDP sink 配置。
#[derive(Debug, Clone)]
pub struct UdpSinkConfig {
    /// 目标地址（`host:port`）
    pub addr: String,
    /// 出站格式
    pub format: NetWireFormat,
}

/// 传输流（明文 TCP 或 TLS-over-TCP，均为阻塞式）。
enum Wire {
    Plain(std::net::TcpStream),
    #[cfg(feature = "net-sink")]
    Tls(rustls::StreamOwned<rustls::ClientConnection, std::net::TcpStream>),
}

impl Wire {
    fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        match self {
            Wire::Plain(s) => std::io::Write::write_all(s, data),
            Wire::Tls(s) => std::io::Write::write_all(s, data),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Wire::Plain(s) => std::io::Write::flush(s),
            Wire::Tls(s) => std::io::Write::flush(s),
        }
    }
}

/// 连接状态（互斥保护；worker 串行调用 write，锁无竞争）。
struct TcpState {
    wire: Option<Wire>,
    buffer: VecDeque<Vec<u8>>,
}

/// TCP 转发 sink（断线缓冲 + 自动重连）。
///
/// 未调用 [`TcpSink::shutdown`] 即 drop 时，重发缓冲中未落盘的记录会被
/// 丢弃；需要 at-least-once 语义的调用方应先显式 shutdown。
pub struct TcpSink {
    config: TcpSinkConfig,
    tls_config: Option<rustls::ClientConfig>,
    state: Mutex<TcpState>,
}

impl TcpSink {
    /// 创建 TCP sink；地址/TLS 配置非法立即报错（连接惰性建立）。
    pub fn new(config: TcpSinkConfig) -> Result<Self, InklogError> {
        if config.addr.is_empty() || !config.addr.contains(':') {
            return Err(InklogError::ConfigError(format!(
                "invalid TCP sink address '{}': expected host:port",
                config.addr
            )));
        }
        let tls_config = match &config.tls {
            None => None,
            Some(tls) => Some(Self::build_tls_client_config(tls)?),
        };
        Ok(Self {
            config,
            tls_config,
            state: Mutex::new(TcpState {
                wire: None,
                buffer: VecDeque::new(),
            }),
        })
    }

    fn build_tls_client_config(tls: &TlsClientConfig) -> Result<rustls::ClientConfig, InklogError> {
        if tls.server_name.is_empty() {
            return Err(InklogError::ConfigError(
                "TLS client requires a server_name (SNI)".to_string(),
            ));
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .map_err(|e| InklogError::ConfigError(format!("TLS protocol versions: {e}")))?;
        let config = if tls.danger_accept_invalid_certs {
            builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(AcceptAnyVerifier {
                    provider: (*provider).clone(),
                }))
                .with_no_client_auth()
        } else {
            let Some(ca_path) = &tls.ca_pem_path else {
                return Err(InklogError::ConfigError(
                    "TLS client requires ca_pem_path (or danger_accept_invalid_certs for tests)"
                        .to_string(),
                ));
            };
            let mut roots = rustls::RootCertStore::empty();
            for cert in load_certs_pem(ca_path)? {
                roots
                    .add(cert)
                    .map_err(|e| InklogError::ConfigError(format!("CA cert rejected: {e}")))?;
            }
            builder.with_root_certificates(roots).with_no_client_auth()
        };
        Ok(config)
    }

    /// 建立连接（明文或 TLS 握手）。
    fn connect(&self) -> Result<Wire, InklogError> {
        let addr = self.config.addr.clone();
        let stream = std::net::TcpStream::connect_timeout(
            &addr
                .parse()
                .map_err(|e| InklogError::ConfigError(format!("bad addr '{addr}': {e}")))?,
            self.config.connect_timeout,
        )?;
        stream.set_nodelay(true).ok();
        match (&self.config.tls, &self.tls_config) {
            (None, _) => Ok(Wire::Plain(stream)),
            (Some(_), Some(client_config)) => {
                let server_name = rustls::pki_types::ServerName::try_from(
                    self.config
                        .tls
                        .as_ref()
                        .map(|t| t.server_name.clone())
                        .unwrap_or_default(),
                )
                .map_err(|e| InklogError::ConfigError(format!("bad SNI: {e}")))?;
                let conn =
                    rustls::ClientConnection::new(Arc::new(client_config.clone()), server_name)
                        .map_err(|e| {
                            InklogError::ConfigError(format!("TLS handshake setup: {e}"))
                        })?;
                Ok(Wire::Tls(rustls::StreamOwned::new(conn, stream)))
            }
            (Some(_), None) => Err(InklogError::ConfigError(
                "TLS requested but client config unavailable".to_string(),
            )),
        }
    }

    /// 单条记录 → 出站字节行。
    fn encode(&self, record: &LogRecord) -> Vec<u8> {
        let mut line = match self.config.format {
            NetWireFormat::JsonLine => serde_json::to_string(record).unwrap_or_else(|_| {
                // 序列化失败兜底：仅保留 message/level/target 的合法 JSON，
                // 维持 NDJSON 单行契约（Debug 格式化的字符串不是合法 JSON）
                serde_json::json!({
                    "message": record.message,
                    "level": record.level,
                    "target": record.target,
                })
                .to_string()
            }),
            NetWireFormat::Text => {
                crate::LogTemplate::new("{timestamp} [{level}] {target} - {message}").render(record)
            }
        };
        line.push('\n');
        line.into_bytes()
    }

    /// 写前健康探测：对端已 FIN/RST（半开连接）时返回 true——避免首笔写
    /// 静默落入死连接（TCP 写后知错的固有窗口，明文流可非阻塞 peek 检出）。
    fn connection_dead(wire: &mut Wire) -> bool {
        match wire {
            Wire::Plain(stream) => {
                let _ = stream.set_nonblocking(true);
                let mut probe = [0u8; 1];
                let verdict = match stream.peek(&mut probe) {
                    // 对端关闭：读到 EOF
                    Ok(0) => true,
                    // 仍有数据/暂无数据：视为存活（单向日志流不读应用数据）
                    Ok(_) => false,
                    Err(e) => matches!(
                        e.kind(),
                        std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::BrokenPipe
                            | std::io::ErrorKind::NotConnected
                    ),
                    // WouldBlock 等其他错误 = 存活
                };
                // 阻塞模式恢复失败则弃用连接：非阻塞残留会让后续
                // write_all/flush 全部 WouldBlock，sink 永久失效
                if stream.set_nonblocking(false).is_err() {
                    return true;
                }
                verdict
            }
            // TLS 流的半开探测需 rustls 状态机配合，依赖 write 错误回传
            Wire::Tls(_) => false,
        }
    }

    /// 入队断线缓冲（满则丢最旧）。
    fn enqueue(buffer: &mut VecDeque<Vec<u8>>, capacity: usize, line: Vec<u8>) {
        if buffer.len() >= capacity {
            buffer.pop_front();
        }
        buffer.push_back(line);
    }

    /// 尽力按序补发缓冲；失败即停止（保留剩余，下次继续）。
    fn drain_buffer(wire: &mut Wire, buffer: &mut VecDeque<Vec<u8>>) {
        while let Some(front) = buffer.front() {
            if wire.write_all(front).is_err() {
                return;
            }
            buffer.pop_front();
        }
        let _ = wire.flush();
    }
}

#[async_trait]
impl LogSink for TcpSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        let line = self.encode(record);
        let capacity = self.config.buffer_capacity;
        let TcpState { wire, buffer } = &mut *self
            .state
            .lock()
            .map_err(|_| InklogError::ConfigError("tcp sink state poisoned".to_string()))?;

        // 无连接：尝试建连
        if wire.is_none()
            && let Ok(new_wire) = self.connect()
        {
            *wire = Some(new_wire);
        }

        if let Some(conn) = wire.as_mut() {
            // 半开连接探测：对端已断开则视为写失败路径（进缓冲 + 重连）
            if Self::connection_dead(conn) {
                Self::enqueue(buffer, capacity, line);
                *wire = None;
            } else {
                // 先补发断线期间的缓冲（保持 FIFO），再写当前记录
                Self::drain_buffer(conn, buffer);
                if conn.write_all(&line).is_ok() && conn.flush().is_ok() {
                    return Ok(());
                }
                Self::enqueue(buffer, capacity, line);
            }
        } else {
            // 仍未连上：记录进缓冲（绝不无声丢弃）
            Self::enqueue(buffer, capacity, line);
        }
        // 自动重连：成功则连接会在下次 write 时补发缓冲
        if let Ok(new_wire) = self.connect() {
            *wire = Some(new_wire);
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), InklogError> {
        if let Ok(mut state) = self.state.lock()
            && let Some(wire) = state.wire.as_mut()
        {
            let _ = wire.flush();
        }
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        if let Ok(TcpState { wire, buffer }) = self.state.lock().as_deref_mut() {
            // 尽力补发剩余缓冲后关闭
            if let Some(conn) = wire.as_mut() {
                Self::drain_buffer(conn, buffer);
            }
            *wire = None;
            buffer.clear();
        }
        Ok(())
    }
}

/// UDP 转发 sink（数据报，尽力而为）。
pub struct UdpSink {
    socket: std::net::UdpSocket,
    format: NetWireFormat,
}

impl UdpSink {
    pub fn new(config: UdpSinkConfig) -> Result<Self, InklogError> {
        if config.addr.is_empty() || !config.addr.contains(':') {
            return Err(InklogError::ConfigError(format!(
                "invalid UDP sink address '{}': expected host:port",
                config.addr
            )));
        }
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
        socket.connect(config.addr.as_str()).map_err(|e| {
            InklogError::ConfigError(format!("UDP connect to '{}': {e}", config.addr))
        })?;
        Ok(Self {
            socket,
            format: config.format,
        })
    }

    fn encode(&self, record: &LogRecord) -> Vec<u8> {
        let mut line = match self.format {
            NetWireFormat::JsonLine => serde_json::to_string(record).unwrap_or_else(|_| {
                // 序列化失败兜底：合法 JSON 单行（同 TcpSink，NDJSON 契约）
                serde_json::json!({
                    "message": record.message,
                    "level": record.level,
                    "target": record.target,
                })
                .to_string()
            }),
            NetWireFormat::Text => {
                crate::LogTemplate::new("{timestamp} [{level}] {target} - {message}").render(record)
            }
        };
        line.push('\n');
        line.into_bytes()
    }
}

#[async_trait]
impl LogSink for UdpSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        let payload = self.encode(record);
        self.socket
            .send(&payload)
            .map_err(|e| InklogError::ConfigError(format!("UDP send failed: {e}")))?;
        Ok(())
    }

    async fn flush(&self) -> Result<(), InklogError> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        Ok(())
    }
}

/// 宽容 PEM 证书加载（BEGIN/END CERTIFICATE 块，base64 解码）。
#[cfg(feature = "net-sink")]
fn load_certs_pem(
    path: &PathBuf,
) -> Result<Vec<rustls::pki_types::CertificateDer<'static>>, InklogError> {
    use base64::Engine;
    let content = std::fs::read_to_string(path)?;
    let mut certs = Vec::new();
    let mut current: Option<String> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("-----BEGIN CERTIFICATE-----") {
            current = Some(String::new());
        } else if trimmed.starts_with("-----END CERTIFICATE-----") {
            if let Some(body) = current.take() {
                let der = base64::engine::general_purpose::STANDARD
                    .decode(body.replace(['\n', '\r'], ""))
                    .map_err(|e| {
                        InklogError::ConfigError(format!("invalid base64 in CA PEM: {e}"))
                    })?;
                certs.push(rustls::pki_types::CertificateDer::from(der));
            }
        } else if let Some(body) = current.as_mut() {
            body.push_str(trimmed);
        }
    }
    if certs.is_empty() {
        return Err(InklogError::ConfigError(format!(
            "no CERTIFICATE blocks found in '{}'",
            path.display()
        )));
    }
    Ok(certs)
}

/// 测试/自签场景的"接受任意证书"校验器。
#[cfg(feature = "net-sink")]
#[derive(Debug)]
struct AcceptAnyVerifier {
    provider: rustls::crypto::CryptoProvider,
}

#[cfg(feature = "net-sink")]
impl rustls::client::danger::ServerCertVerifier for AcceptAnyVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::sync::Arc;

    fn test_record(message: &str) -> LogRecord {
        LogRecord::new(
            tracing::Level::INFO,
            "net::test".to_string(),
            message.to_string(),
        )
    }

    /// 后台 TCP 服务端：接受连接、收集收到的行，可控制"接受后立即断开"。
    struct TcpTestServer {
        addr: std::net::SocketAddr,
        received: Arc<Mutex<Vec<String>>>,
        shutdown: Arc<std::sync::atomic::AtomicBool>,
        handle: std::thread::JoinHandle<()>,
    }

    impl TcpTestServer {
        /// `close_after_lines = Some(n)`：某连接收到 n 行后立即 shutdown——
        /// 模拟服务端主动断开（sink 端写失败 → 缓冲 → 重连重放）。
        fn spawn(close_after_lines: Option<usize>) -> Self {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let received = Arc::new(Mutex::new(Vec::new()));
            let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let shutdown_clone = shutdown.clone();
            let received_clone = received.clone();
            let handle = std::thread::spawn(move || {
                let mut accepted = 0usize;
                for stream in listener.incoming() {
                    if shutdown_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    let Ok(mut stream) = stream else { continue };
                    accepted += 1;
                    // 逐连接读至 EOF；仅首个连接在达到 close_after_lines 阈值后
                    // 立即 shutdown——模拟服务端主动断开（sink 端写失败→缓冲→
                    // 重连重放）。后续连接保持正常读取。
                    let kill_after = if accepted == 1 {
                        close_after_lines
                    } else {
                        None
                    };
                    let mut conn_lines = 0usize;
                    loop {
                        let mut buf = [0u8; 4096];
                        match stream.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                let text = String::from_utf8_lossy(&buf[..n]);
                                for line in text.lines() {
                                    received_clone.lock().unwrap().push(line.to_string());
                                    conn_lines += 1;
                                }
                                if let Some(limit) = kill_after
                                    && conn_lines >= limit
                                {
                                    let _ = stream.shutdown(std::net::Shutdown::Both);
                                    break;
                                }
                            }
                        }
                    }
                    drop(stream);
                    if shutdown_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                }
            });
            Self {
                addr,
                received,
                shutdown,
                handle,
            }
        }

        fn lines(&self) -> Vec<String> {
            self.received.lock().unwrap().clone()
        }

        fn shutdown(&self) {
            self.shutdown
                .store(true, std::sync::atomic::Ordering::Relaxed);
            // 打断 accept：连一次即释放
            let _ = std::net::TcpStream::connect(self.addr);
            self.handle.thread().unpark();
        }
    }

    #[tokio::test]
    async fn test_tcp_sink_delivers_json_lines() {
        let server = TcpTestServer::spawn(None);
        let sink = TcpSink::new(TcpSinkConfig {
            addr: server.addr.to_string(),
            tls: None,
            ..Default::default()
        })
        .unwrap();

        sink.write(&test_record("first message")).await.unwrap();
        sink.write(&test_record("second message")).await.unwrap();
        sink.flush().await.unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while server.lines().len() < 2 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let lines = server.lines();
        assert_eq!(lines.len(), 2, "both records must arrive");
        let parsed: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(parsed["message"], "first message");
        assert_eq!(parsed["target"], "net::test");
        server.shutdown();
    }

    #[tokio::test]
    async fn test_tcp_sink_reconnects_and_replays_buffer_after_disconnect() {
        // 服务端收到第 1 行后立即断开连接：sink 的后续写入经历
        // 写失败 → 进缓冲 → 自动重连 → 按序补发，全部 4 条必须按序送达。
        let server = TcpTestServer::spawn(Some(1));
        let sink = Arc::new(
            TcpSink::new(TcpSinkConfig {
                addr: server.addr.to_string(),
                tls: None,
                connect_timeout: Duration::from_secs(1),
                ..Default::default()
            })
            .unwrap(),
        );

        const MESSAGES: [&str; 4] = [
            "m0 first",
            "m1 buffered",
            "m2 buffered",
            "m3 after reconnect",
        ];
        for (i, m) in MESSAGES.iter().enumerate() {
            if i == 1 {
                // 服务端在收到 m0 后立即断开；稍候写入让 FIN/RST 先于
                // 写前探测可见，否则 m1 可能落入"TCP 写后知错"的固有窗口
                //（对端已关但本端写仍被内核接受，数据不可达且无从缓冲）。
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            sink.write(&test_record(m)).await.unwrap();
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while server.lines().len() < MESSAGES.len() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let lines = server.lines();
        assert_eq!(
            lines.len(),
            MESSAGES.len(),
            "all records must be delivered (buffered ones replayed), got: {lines:?}"
        );
        // FIFO 缓冲重放 + TCP 顺序保证 → 全局有序
        for (i, m) in MESSAGES.iter().enumerate() {
            assert!(
                lines[i].contains(m),
                "records must arrive in write order, expected {m} at {i}, got: {lines:?}"
            );
        }
        server.shutdown();
    }

    #[tokio::test]
    async fn test_tcp_sink_invalid_config_rejected() {
        assert!(
            TcpSink::new(TcpSinkConfig {
                addr: "no-port-here".to_string(),
                ..Default::default()
            })
            .is_err()
        );
        // TLS 缺 server_name / 缺 CA → 构造期报错
        assert!(
            TcpSink::new(TcpSinkConfig {
                addr: "127.0.0.1:5170".to_string(),
                tls: Some(TlsClientConfig {
                    server_name: String::new(),
                    ca_pem_path: None,
                    danger_accept_invalid_certs: false,
                }),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            TcpSink::new(TcpSinkConfig {
                addr: "127.0.0.1:5170".to_string(),
                tls: Some(TlsClientConfig {
                    server_name: "logs.internal".to_string(),
                    ca_pem_path: None,
                    danger_accept_invalid_certs: false,
                }),
                ..Default::default()
            })
            .is_err(),
            "TLS without CA and without danger opt-in must be rejected"
        );
    }

    #[tokio::test]
    async fn test_udp_sink_delivers_datagrams() {
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();

        let sink = UdpSink::new(UdpSinkConfig {
            addr: addr.to_string(),
            format: NetWireFormat::JsonLine,
        })
        .unwrap();
        sink.write(&test_record("udp hello")).await.unwrap();

        let mut buf = [0u8; 4096];
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let n = socket.recv(&mut buf).expect("datagram must arrive");
        let parsed: serde_json::Value = serde_json::from_slice(&buf[..n]).unwrap();
        assert_eq!(parsed["message"], "udp hello");
    }

    #[tokio::test]
    async fn test_udp_sink_invalid_config_rejected() {
        assert!(
            UdpSink::new(UdpSinkConfig {
                addr: "bad".to_string(),
                format: NetWireFormat::JsonLine,
            })
            .is_err()
        );
    }
}
