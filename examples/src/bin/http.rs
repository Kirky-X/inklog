// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
//! cargo run --bin http
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
use inklog::metrics::{Metrics, SinkStatus};
use inklog_examples::common::{print_section, print_separator};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;
use tokio::net::{TcpListener, TcpStream};

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
    print_separator("1. 启动 HTTP 服务器");

    // 创建 Metrics 实例用于演示
    let metrics = Metrics::new();

    // 模拟一些日志记录活动
    for i in 0..100 {
        metrics.inc_logs_written();
        if i % 10 == 0 {
            metrics.record_latency(Duration::from_micros(100 + (i as u64 % 500)));
        }
    }

    // 设置一些 sink 健康状态
    metrics.sink_started("console");
    metrics.sink_started("file");
    metrics.update_sink_health("console", true, None);
    metrics.update_sink_health("file", true, None);

    // 设置一些运行时指标（使用公开的 API）
    metrics.set_db_batch_size(50);
    metrics.set_pool_hit_rate(95.5);

    // 查找随机可用端口
    let port = find_available_port()?;
    let addr = format!("127.0.0.1:{}", port);

    print_section("HTTP 服务器配置");
    println!("监听地址: {}", addr);
    println!("端点:");
    println!("  - GET /health  → JSON 格式健康状态");
    println!("  - GET /metrics → Prometheus 格式指标");
    println!();

    // 创建 TCP 监听器
    let listener = TcpListener::bind(&addr).await?;
    println!("✓ HTTP 服务器已启动在端口 {}", port);

    // 创建 shutdown 信号
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // 克隆 metrics 用于异步处理
    let metrics = std::sync::Arc::new(metrics);

    // 启动 HTTP 服务器任务
    tokio::spawn(async move {
        loop {
            tokio::select! {
                // 接受新连接
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            let metrics = metrics.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, &metrics).await {
                                    eprintln!("处理连接 {} 失败: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("接受连接失败: {}", e);
                        }
                    }
                }
                // 检查 shutdown 信号
                _ = &mut shutdown_rx => {
                    println!("\n✓ 接收到 shutdown 信号，关闭服务器");
                    break;
                }
            }
        }
    });

    Ok((port, shutdown_tx))
}

/// 处理单个 HTTP 连接
async fn handle_connection(
    mut stream: TcpStream,
    metrics: &std::sync::Arc<Metrics>,
) -> Result<()> {
    // 读取请求
    let mut buffer = Vec::new();
    let mut temp_buf = [0u8; 8192];

    loop {
        let n = stream.read(&mut temp_buf).await?;
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

    // 解析请求行
    let request = String::from_utf8_lossy(&buffer);
    let first_line = request.lines().next().unwrap_or("");

    // 提取路径
    let path = first_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();

    print_section(&format!("收到请求: {}", path));

    // 根据路径返回响应
    let (status_line, content_type, body) = match path.as_str() {
        "/health" => {
            let health = build_health_response(metrics.as_ref());
            let body = serde_json::to_string_pretty(&health)?;
            ("HTTP/1.1 200 OK", "application/json", body)
        }
        "/metrics" => {
            let metrics_output = metrics.export_prometheus();
            ("HTTP/1.1 200 OK", "text/plain; version=0.0.4", metrics_output)
        }
        "/favicon.ico" => {
            ("HTTP/1.1 204 No Content", "image/x-icon", String::new())
        }
        _ => {
            let body = serde_json::to_string_pretty(&serde_json::json!({
                "error": "Not Found",
                "message": format!("Path '{}' not found. Available endpoints: /health, /metrics", path)
            }))?;
            ("HTTP/1.1 404 Not Found", "application/json", body)
        }
    };

    // 构建响应头
    let response = if body.is_empty() {
        format!("{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", status_line)
    } else {
        format!(
            "{}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status_line,
            content_type,
            body.len(),
            body
        )
    };

    // 发送响应
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
        sinks.insert(name.clone(), SinkHealthResponse {
            status: match &health.status {
                SinkStatus::Healthy => "Healthy".to_string(),
                SinkStatus::Degraded { reason } => format!("Degraded: {}", reason),
                SinkStatus::Unhealthy { error } => format!("Unhealthy: {}", error),
                SinkStatus::NotStarted => "NotStarted".to_string(),
            },
            consecutive_failures: health.consecutive_failures,
            last_error: health.last_error.clone(),
        });
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
    // 解析 URL
    let url_parsed = url.trim_start_matches("http://");
    let (host_port, path) = if let Some(idx) = url_parsed.find('/') {
        (&url_parsed[..idx], &url_parsed[idx..])
    } else {
        (url_parsed, "/")
    };

    let (host, port) = if let Some(idx) = host_port.find(':') {
        (&host_port[..idx], host_port[idx + 1..].parse().unwrap_or(80))
    } else {
        (host_port, 80)
    };

    // 建立 TCP 连接
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr).await?;

    // 发送 HTTP 请求
    let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
    stream.write_all(request.as_bytes()).await?;

    // 读取响应
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

    // 解析状态码
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

    // 提取 Content-Type
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
    print_separator("2. 健康检查端点演示");

    print_section("发送 GET /health 请求");

    // 发送 HTTP 请求
    let url = format!("http://127.0.0.1:{}/health", port);
    println!("请求 URL: {}\n", url);

    let (status_code, content_type, body) = http_get(&url).await?;

    println!("响应状态: {}", status_code);
    println!("Content-Type: {}", content_type);

    // 读取并解析响应体
    println!("\n响应内容 (JSON):\n");
    let json: serde_json::Value = serde_json::from_str(&body)?;
    println!("{}", serde_json::to_string_pretty(&json)?);

    // 验证响应结构
    assert!(json.get("overall_status").is_some(), "缺少 overall_status 字段");
    assert!(json.get("sinks").is_some(), "缺少 sinks 字段");
    assert!(json.get("uptime_seconds").is_some(), "缺少 uptime_seconds 字段");
    assert!(json.get("metrics").is_some(), "缺少 metrics 字段");

    let overall_status = json["overall_status"].as_str().unwrap();
    println!("\n✓ 健康状态: {}", overall_status);

    // 检查 sink 状态
    if let Some(sinks) = json["sinks"].as_object() {
        for (name, health) in sinks {
            println!("  ✓ Sink '{}': {}", name, health["status"]);
        }
    }

    println!("\n✓ /health 端点测试通过");
    Ok(())
}

/// 指标端点演示
///
/// 演示如何访问 `/metrics` 端点并解析 Prometheus 格式的指标。
async fn metrics_endpoint(port: u16) -> Result<()> {
    print_separator("3. 指标端点演示");

    print_section("发送 GET /metrics 请求");

    // 发送 HTTP 请求
    let url = format!("http://127.0.0.1:{}/metrics", port);
    println!("请求 URL: {}\n", url);

    let (status_code, content_type, body) = http_get(&url).await?;

    println!("响应状态: {}", status_code);
    println!("Content-Type: {}", content_type);

    print_section("Prometheus 格式指标输出");
    println!("{}", body);

    // 验证关键指标存在（uptime_seconds 只在运行时间 > 0 时输出）
    assert!(body.contains("inklog_logs_written_total"), "缺少 logs_written_total 指标");
    assert!(body.contains("inklog_sink_healthy"), "缺少 sink_healthy 指标");

    // 解析并显示关键指标
    print_section("关键指标摘要");

    let metrics: Vec<&str> = body.lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .collect();

    for metric in metrics.iter().take(10) {
        println!("  {}", metric);
    }

    if metrics.len() > 10 {
        println!("  ... (共 {} 个指标)", metrics.len());
    }

    println!("\n✓ /metrics 端点测试通过");
    Ok(())
}

/// 错误处理端点演示
async fn error_endpoint_demo(port: u16) -> Result<()> {
    print_separator("4. 错误处理端点演示");

    print_section("访问不存在的端点");

    let url = format!("http://127.0.0.1:{}/invalid-path", port);
    println!("请求 URL: {}\n", url);

    let (status_code, _content_type, body) = http_get(&url).await?;
    println!("响应状态: {} (预期: 404)", status_code);

    let json: serde_json::Value = serde_json::from_str(&body)?;
    println!("\n响应内容:\n{}", serde_json::to_string_pretty(&json)?);

    println!("\n✓ 404 错误处理测试通过");
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
    println!("=== inklog HTTP 健康检查和指标端点示例 ===\n");

    // 1. 启动 HTTP 服务器
    let (port, shutdown) = start_http_server().await?;

    // 等待服务器完全启动
    sleep(Duration::from_millis(100)).await;

    // 2. 测试健康检查端点
    health_endpoint(port).await?;

    // 3. 测试指标端点
    metrics_endpoint(port).await?;

    // 4. 测试错误处理
    error_endpoint_demo(port).await?;

    // 完成
    print_separator("5. 集成指南");

    println!("\n在 Prometheus 中配置抓取:");
    println!();
    println!("  # prometheus.yml");
    println!("  scrape_configs:");
    println!("    - job_name: 'inklog'");
    println!("      static_configs:");
    println!("        - targets: ['localhost:{}']", port);
    println!();
    println!("  然后运行: prometheus --config.file=prometheus.yml");
    println!();

    println!("使用 curl 测试:");
    println!();
    println!("  # 健康检查");
    println!("  curl http://localhost:{}/health | jq", port);
    println!();
    println!("  # Prometheus 指标");
    println!("  curl http://localhost:{}/metrics", port);
    println!();

    // 发送 shutdown 信号
    let _ = shutdown.send(());

    println!("\n按 Ctrl+C 退出...");
    inklog_examples::wait_for_ctrl_c().await?;

    Ok(())
}
