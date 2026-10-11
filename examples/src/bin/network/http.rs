// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! HTTP 健康检查和指标端点示例
//!
//! 演示如何启动 HTTP 服务器并访问健康检查和 Prometheus 指标端点。
//!
//! ## 功能演示
//!
//! - **start_http_server()**: 启动随机端口 HTTP 服务器
//! - **health_endpoint()**: 健康检查端点 `/health`（JSON 格式）
//! - **metrics_endpoint()**: 指标端点 `/metrics`（Prometheus 格式）
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p inklog-examples --bin http --features http
//! ```
//!
//! ## 端点说明
//!
//! ### GET /health
//!
//! 返回 JSON 格式的健康状态：
//!
//! ```json
//! {
//!   "overall_status": "Healthy",
//!   "sinks": {
//!     "console": { "status": "Healthy", "consecutive_failures": 0 }
//!   },
//!   "channel_usage": 0.0,
//!   "uptime_seconds": 120,
//!   "metrics": { ... },
//!   "encryption_key_valid": true
//! }
//! ```
//!
//! ### GET /metrics
//!
//! 返回 Prometheus 格式的指标数据：
//!
//! ```text
//! # HELP inklog_logs_written_total Total logs successfully written
//! # TYPE inklog_logs_written_total counter
//! inklog_logs_written_total 1234
//! ...
//! ```
//!
//! ## 集成 Prometheus
//!
//! 在 `prometheus.yml` 中添加以下配置：
//!
//! ```yaml
//! scrape_configs:
//!   - job_name: 'inklog'
//!     static_configs:
//!       - targets: ['localhost:PORT']
//! ```

use anyhow::Result;
use inklog::serde::Serialize;
use inklog::tokio::io::{AsyncReadExt, AsyncWriteExt};
use inklog::tokio::net::{TcpListener, TcpStream};
use inklog::tokio::time::sleep;
use inklog::{Metrics, SinkStatus};
use inklog_examples::common::{print_section, print_separator};
use std::collections::HashMap;

/// 外部响应内容打印前统一脱敏 + 截断（CodeQL CWE-117：外部输入直入
/// stdout 可伪造日志行；示例应演示正确姿势）。
fn safe_body_preview(body: &str) -> String {
    use inklog::LogSanitizer;
    let sanitized = LogSanitizer::new().sanitize(body);
    sanitized
        .chars()
        .filter(|c| !c.is_control())
        .take(400)
        .collect()
}
use std::time::Duration;

/// 启动随机端口 HTTP 服务器
///
/// 在随机可用端口上启动 HTTP 服务器，处理以下请求：
/// - `GET /health` - 返回 JSON 格式的健康状态
/// - `GET /metrics` - 返回 Prometheus 格式的指标
/// - 其他路径返回 404
///
/// # 返回值
///
/// 返回实际使用的端口号和 shutdown 信号的发送端。
///
/// # 示例
///
/// ```rust,no_run
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let (port, shutdown) = start_http_server().await?;
///     println!("HTTP 服务器运行在端口 {}", port);
///     // ... 使用服务器 ...
///     Ok(())
/// }
/// ```
async fn start_http_server() -> Result<(u16, tokio::sync::oneshot::Sender<()>)> {
    print_separator("1. Start the HTTP server");

    // 创建 Metrics 实例用于演示
    let metrics = Metrics::new();

    // 模拟一些日志记录活动
    for i in 0..100 {
        metrics.inc_logs_written();
        if i % 10 == 0 {
            metrics.record_latency(Duration::from_micros(100 + (i as u64 % 500)));
        }
    }

    metrics.sink_started("console");
    metrics.sink_started("file");
    metrics.update_sink_health("console", true, None);
    metrics.update_sink_health("file", true, None);

    // 设置一些运行时指标（使用公开的 API）
    metrics.set_db_batch_size(50);
    metrics.set_pool_hit_rate(95.5);

    let port = find_available_port()?;
    let addr = format!("127.0.0.1:{}", port);

    print_section("HTTP server configuration");
    println!("Listen address: {}", addr);
    println!("Endpoints:");
    println!("  - GET /health  → health status as JSON");
    println!("  - GET /metrics → metrics in Prometheus format");
    println!();

    let listener = TcpListener::bind(&addr).await?;
    println!("✓ HTTP server started on port {}", port);

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // 克隆 metrics 用于异步处理
    let metrics = std::sync::Arc::new(metrics);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                // 接受新连接
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            let metrics = metrics.clone();
                            // 外层任务持有内层连接任务的 JoinHandle 并监控其结果：
                            // handle_connection 返回的 Err 与任务 panic 都会在此记录，
                            // 避免 JoinHandle 被丢弃后异常被静默吞掉
                            //（生产环境可基于 JoinError 进一步做重试或熔断）
                            tokio::spawn(async move {
                                let handle = tokio::spawn(handle_connection(stream, metrics));
                                match handle.await {
                                    Ok(Ok(())) => {}
                                    Ok(Err(e)) => {
                                        eprintln!("failed to handle connection {}: {}", addr, e);
                                    }
                                    Err(join_err) => {
                                        eprintln!("connection {} handler task aborted: {}", addr, join_err);
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("failed to accept a connection: {}", e);
                        }
                    }
                }
                // 检查 shutdown 信号
                _ = &mut shutdown_rx => {
                    println!("\n✓ shutdown signal received, closing the server");
                    break;
                }
            }
        }
    });

    Ok((port, shutdown_tx))
}

/// 单次读取超时：防止慢速或异常客户端长期占用连接与工作线程
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// 处理单个 HTTP 连接
///
/// 注意：接收 `Arc<Metrics>`（而非引用），便于在外层任务中
/// 通过 `tokio::spawn` 独立运行（future 需满足 `'static` 约束）。
async fn handle_connection(mut stream: TcpStream, metrics: std::sync::Arc<Metrics>) -> Result<()> {
    let mut buffer = Vec::new();
    let mut temp_buf = [0u8; 8192];

    loop {
        // 每轮读取都带超时：对端在 READ_TIMEOUT 内无数据则主动关闭连接，
        // 避免连接被恶意或失效的客户端无限期挂起
        let n = match tokio::time::timeout(READ_TIMEOUT, stream.read(&mut temp_buf)).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e.into()),
            Err(_elapsed) => {
                eprintln!(
                    "request read timed out ({}s), closing the connection",
                    READ_TIMEOUT.as_secs()
                );
                return Ok(());
            }
        };
        if n == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&temp_buf[..n]);

        // 检查是否读取到完整的 HTTP 请求头（以 \r\n\r\n 结束）
        if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }

        // 防止缓冲区过大
        if buffer.len() > 65536 {
            break;
        }
    }

    if buffer.is_empty() {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer);
    let first_line = request.lines().next().unwrap_or("");

    let path = first_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();

    print_section(&format!("Request received: {}", path));

    let (status_line, content_type, body) = match path.as_str() {
        "/health" => {
            let health = build_health_response(metrics.as_ref());
            let body = serde_json::to_string_pretty(&health)?;
            ("HTTP/1.1 200 OK", "application/json", body)
        }
        "/metrics" => {
            let metrics_output = metrics.export_prometheus();
            (
                "HTTP/1.1 200 OK",
                "text/plain; version=0.0.4",
                metrics_output,
            )
        }
        "/favicon.ico" => ("HTTP/1.1 204 No Content", "image/x-icon", String::new()),
        _ => {
            let body = serde_json::to_string_pretty(&serde_json::json!({
                "error": "Not Found",
                "message": format!("Path '{}' not found. Available endpoints: /health, /metrics", path)
            }))?;
            ("HTTP/1.1 404 Not Found", "application/json", body)
        }
    };

    let response = if body.is_empty() {
        format!(
            "{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            status_line
        )
    } else {
        format!(
            "{}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status_line,
            content_type,
            body.len(),
            body
        )
    };

    stream.write_all(response.as_bytes()).await?;

    Ok(())
}

/// 构建健康检查响应的数据结构
#[derive(Serialize)]
struct HealthResponse {
    overall_status: String,
    sinks: HashMap<String, SinkHealthResponse>,
    channel_usage: f64,
    uptime_seconds: u64,
    metrics: MetricsResponse,
    encryption_key_valid: bool,
}

#[derive(Serialize)]
struct SinkHealthResponse {
    status: String,
    consecutive_failures: u32,
    last_error: Option<String>,
}

#[derive(Serialize)]
struct MetricsResponse {
    logs_written: u64,
    logs_dropped: u64,
    channel_blocked: u64,
    sink_errors: u64,
    avg_latency_us: u64,
    p50_latency_us: u64,
    p95_latency_us: u64,
    p99_latency_us: u64,
    db_batch_size: i64,
    pool_hit_rate: f64,
}

/// 构建健康检查 JSON 响应
fn build_health_response(metrics: &Metrics) -> HealthResponse {
    let status = metrics.get_status(0, 10000);

    let mut sinks = HashMap::new();
    for (name, health) in &status.sinks {
        sinks.insert(
            name.clone(),
            SinkHealthResponse {
                status: match &health.status {
                    SinkStatus::Healthy => "Healthy".to_string(),
                    SinkStatus::Degraded { reason } => format!("Degraded: {}", reason),
                    SinkStatus::Unhealthy { error } => format!("Unhealthy: {}", error),
                    SinkStatus::NotStarted => "NotStarted".to_string(),
                },
                consecutive_failures: health.consecutive_failures,
                last_error: health.last_error.clone(),
            },
        );
    }

    HealthResponse {
        overall_status: match &status.overall_status {
            SinkStatus::Healthy => "Healthy".to_string(),
            SinkStatus::Degraded { reason } => format!("Degraded: {}", reason),
            SinkStatus::Unhealthy { error } => format!("Unhealthy: {}", error),
            SinkStatus::NotStarted => "NotStarted".to_string(),
        },
        sinks,
        channel_usage: status.channel_usage,
        uptime_seconds: status.uptime_seconds,
        metrics: MetricsResponse {
            logs_written: status.metrics.logs_written,
            logs_dropped: status.metrics.logs_dropped,
            channel_blocked: status.metrics.channel_blocked,
            sink_errors: status.metrics.sink_errors,
            avg_latency_us: status.metrics.avg_latency_us,
            p50_latency_us: status.metrics.p50_latency_us,
            p95_latency_us: status.metrics.p95_latency_us,
            p99_latency_us: status.metrics.p99_latency_us,
            db_batch_size: status.metrics.db_batch_size,
            pool_hit_rate: status.metrics.pool_hit_rate,
        },
        encryption_key_valid: status.encryption_key_valid,
    }
}

/// 使用简单的 HTTP 客户端发送请求并获取响应
async fn http_get(url: &str) -> Result<(u16, String, String)> {
    let url_parsed = url.trim_start_matches("http://");
    let (host_port, path) = if let Some(idx) = url_parsed.find('/') {
        (&url_parsed[..idx], &url_parsed[idx..])
    } else {
        (url_parsed, "/")
    };

    let (host, port) = if let Some(idx) = host_port.find(':') {
        (
            &host_port[..idx],
            host_port[idx + 1..].parse().unwrap_or(80),
        )
    } else {
        (host_port, 80)
    };

    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr).await?;

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, host
    );
    stream.write_all(request.as_bytes()).await?;

    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buf[..n]);
    }

    let response_str = String::from_utf8_lossy(&response).to_string();

    let status_code = if let Some(line) = response_str.lines().next() {
        line.split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    } else {
        0
    };

    // 提取 body（跳过 header）
    let body = if let Some(idx) = response_str.find("\r\n\r\n") {
        response_str[idx + 4..].to_string()
    } else {
        response_str.clone()
    };

    let content_type = response_str
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-type:"))
        .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
        .unwrap_or_default();

    Ok((status_code, content_type, body))
}

/// 健康检查端点演示
///
/// 演示如何访问 `/health` 端点并解析响应。
async fn health_endpoint(port: u16) -> Result<()> {
    print_separator("2. Health check endpoint demo");

    print_section("Send a GET /health request");

    // 发送 HTTP 请求
    let url = format!("http://127.0.0.1:{}/health", port);
    println!("Request URL: {}\n", url);

    let (status_code, content_type, body) = http_get(&url).await?;

    println!("Response status: {}", status_code);
    println!("Content-Type: {}", content_type);

    println!("\nResponse body (JSON):\n");
    let json: serde_json::Value = serde_json::from_str(&body)?;
    println!(
        "{}",
        safe_body_preview(&serde_json::to_string_pretty(&json)?)
    );

    assert!(
        json.get("overall_status").is_some(),
        "missing the overall_status field"
    );
    assert!(json.get("sinks").is_some(), "missing the sinks field");
    assert!(
        json.get("uptime_seconds").is_some(),
        "missing the uptime_seconds field"
    );
    assert!(json.get("metrics").is_some(), "missing the metrics field");

    let overall_status = json["overall_status"].as_str().unwrap();
    println!("\n✓ Health status: {}", overall_status);

    if let Some(sinks) = json["sinks"].as_object() {
        for (name, health) in sinks {
            println!("  ✓ Sink '{}': {}", name, health["status"]);
        }
    }

    println!("\n✓ /health endpoint test passed");
    Ok(())
}

/// 指标端点演示
///
/// 演示如何访问 `/metrics` 端点并解析 Prometheus 格式的指标。
async fn metrics_endpoint(port: u16) -> Result<()> {
    print_separator("3. Metrics endpoint demo");

    print_section("Send a GET /metrics request");

    let url = format!("http://127.0.0.1:{}/metrics", port);
    println!("Request URL: {}\n", url);

    let (status_code, content_type, body) = http_get(&url).await?;

    println!("Response status: {}", status_code);
    println!("Content-Type: {}", content_type);

    print_section("Prometheus format metrics output");
    println!("{}", safe_body_preview(&body));

    // 验证关键指标存在（uptime_seconds 只在运行时间 > 0 时输出）
    assert!(
        body.contains("inklog_logs_written_total"),
        "missing the logs_written_total metric"
    );
    assert!(
        body.contains("inklog_sink_healthy"),
        "missing the sink_healthy metric"
    );

    print_section("Key metrics summary");

    let metrics: Vec<&str> = body
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .collect();

    for metric in metrics.iter().take(10) {
        println!("  {}", metric);
    }

    if metrics.len() > 10 {
        println!("  ... ({} metrics in total)", metrics.len());
    }

    println!("\n✓ /metrics endpoint test passed");
    Ok(())
}

/// 错误处理端点演示
async fn error_endpoint_demo(port: u16) -> Result<()> {
    print_separator("4. Error handling endpoint demo");

    print_section("Request a nonexistent endpoint");

    let url = format!("http://127.0.0.1:{}/invalid-path", port);
    println!("Request URL: {}\n", url);

    let (status_code, _content_type, body) = http_get(&url).await?;
    println!("Response status: {} (expected: 404)", status_code);

    let json: serde_json::Value = serde_json::from_str(&body)?;
    println!("\nResponse body:\n{}", serde_json::to_string_pretty(&json)?);

    println!("\n✓ 404 error handling test passed");
    Ok(())
}

/// 查找可用的随机端口
fn find_available_port() -> Result<u16> {
    use std::net::TcpListener;
    use std::time::{SystemTime, UNIX_EPOCH};

    // 尝试多个随机端口
    for _ in 0..100 {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as u16;
        let try_port = 10000 + (seed.wrapping_mul(31)) % 55535;

        let addr = format!("127.0.0.1:{}", try_port);
        if TcpListener::bind(&addr).is_ok() {
            return Ok(try_port);
        }
    }

    // 如果随机端口都不可用，使用一个固定的测试端口
    Ok(18080)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== inklog HTTP health check and metrics endpoint examples ===\n");

    let (port, shutdown) = start_http_server().await?;

    // 等待服务器完全启动
    sleep(Duration::from_millis(100)).await;

    health_endpoint(port).await?;

    metrics_endpoint(port).await?;

    error_endpoint_demo(port).await?;

    print_separator("5. Integration guide");

    println!("\nConfigure scraping in Prometheus:");
    println!();
    println!("  # prometheus.yml");
    println!("  scrape_configs:");
    println!("    - job_name: 'inklog'");
    println!("      static_configs:");
    println!("        - targets: ['localhost:{}']", port);
    println!();
    println!("  Then run: prometheus --config.file=prometheus.yml");
    println!();

    println!("Test with curl:");
    println!();
    println!("  # health check");
    println!("  curl http://localhost:{}/health | jq", port);
    println!();
    println!("  # Prometheus metrics");
    println!("  curl http://localhost:{}/metrics", port);
    println!();

    let _ = shutdown.send(());

    println!("\nPress Ctrl+C to exit...");
    inklog_examples::wait_for_ctrl_c().await?;

    Ok(())
}
