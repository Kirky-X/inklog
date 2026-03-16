// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.
//
// 微服务架构日志记录示例
// 展示在微服务环境中的完整配置和最佳实践

use inklog::{
    config::{ConsoleSinkConfig, DatabaseSinkConfig, FileSinkConfig, GlobalConfig, HttpServerConfig},
    InklogConfig, LoggerManager,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

/// 微服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MicroserviceConfig {
    service_name: String,
    service_version: String,
    environment: String,
    tracing_id: String,
    instance_id: String,
}

/// 请求追踪数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestTrace {
    trace_id: String,
    service_name: String,
    instance_id: String,
    timestamp: u64,
    method: String,
    path: String,
    user_id: Option<String>,
    session_id: Option<String>,
    request_size: u64,
    response_status: u16,
    response_time_ms: u64,
}

/// 性能指标数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerformanceMetrics {
    service_name: String,
    instance_id: String,
    timestamp: u64,
    request_count: u64,
    error_count: u64,
    avg_response_time_ms: f64,
    p95_response_time_ms: f64,
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
}

/// 错误详情数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorDetails {
    timestamp: u64,
    trace_id: String,
    service_name: String,
    instance_id: String,
    error_type: String,
    error_code: Option<String>,
    error_message: String,
    stack_trace: Option<String>,
    user_context: HashMap<String, String>,
    recovery_action: String,
}

/// 生成唯一追踪ID
fn generate_trace_id() -> String {
    format!("trace_{}", uuid::Uuid::new_v4())
}

/// 模拟微服务请求处理
async fn simulate_microservice_request(
    service_name: &str,
    instance_id: &str,
    method: &str,
    path: &str,
    user_id: Option<&str>,
    session_id: Option<&str>,
) -> (RequestTrace, Result<(), String>) {
    let trace_id = generate_trace_id();
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // 模拟请求处理时间（50-500ms）
    let processing_time = tokio::time::sleep(Duration::from_millis(
        rand::random::<u64>() % 450 + 50
    )).await;
    processing_time.as_secs();
    
    // 模拟95%的成功率和1-5%的错误率
    let (response_status, error_info) = if rand::random::<f64>() < 0.95 {
        (200, None)
    } else {
        let error_codes = vec!["E001", "E002", "E003", "E004", "E005"];
        let error_messages = vec![
            "Database connection timeout",
            "Service unavailable", 
            "Invalid request parameters",
            "Rate limit exceeded",
            "Internal server error"
        ];
        
        let error_idx = rand::random::<usize>() % error_codes.len();
        let error_message_idx = rand::random::<usize>() % error_messages.len();
        
        (
            500,
            Some(format!("{}: {}", error_codes[error_idx], error_messages[error_message_idx]))
        )
    };
    
    let trace = RequestTrace {
        trace_id,
        service_name: service_name.to_string(),
        instance_id: instance_id.to_string(),
        timestamp: start_time,
        method: method.to_string(),
        path: path.to_string(),
        user_id: user_id.map(String::from),
        session_id: session_id.map(String::from),
        request_size: rand::random::<u64>() % 10000 + 100, // 100-10100 bytes
        response_status: response_status as u16,
        response_time_ms: 100 + rand::random::<u64>() % 400, // 100-500ms
    };
    
    (trace, error_info)
}

/// 记录请求追踪日志
fn log_request_trace(trace: &RequestTrace) {
    log::info!(
        target: "microservice.request",
        "REQUEST_TRACE - trace_id={}, service={}, instance={}, method={}, path={}, user_id={:?}, session_id={:?}, size={}, status={}, time={}ms",
        trace.trace_id,
        trace.service_name,
        trace.instance_id,
        trace.method,
        trace.path,
        trace.user_id,
        trace.session_id,
        trace.request_size,
        trace.response_status,
        trace.response_time_ms
    );
}

/// 记录错误详情日志
fn log_error_details(error: &ErrorDetails) {
    log::error!(
        target: "microservice.error",
        "ERROR_DETAILS - trace_id={}, service={}, instance={}, type={}, code={}, message={}, stack_trace={:?}, user_context={:?}, recovery={}",
        error.trace_id,
        error.service_name,
        error.instance_id,
        error.error_type,
        error.error_code.as_deref(),
        error.error_message,
        error.stack_trace,
        error.user_context,
        error.recovery_action
    );
}

/// 记录性能指标日志
fn log_performance_metrics(metrics: &PerformanceMetrics) {
    log::info!(
        target: "microservice.metrics",
        "PERFORMANCE_METRICS - service={}, instance={}, timestamp={}, requests={}, errors={}, avg_time={:.1}ms, p95_time={:.1}ms, memory={:.1}MB, cpu={:.1}%",
        metrics.service_name,
        metrics.instance_id,
        metrics.timestamp,
        metrics.request_count,
        metrics.error_count,
        metrics.avg_response_time_ms,
        metrics.p95_response_time_ms,
        metrics.memory_usage_mb,
        metrics.cpu_usage_percent
    );
}

/// 记录业务事件日志
fn log_business_event(
    service_name: &str,
    instance_id: &str,
    event_type: &str,
    event_data: &serde_json::Value,
) {
    log::info!(
        target: "microservice.business",
        "BUSINESS_EVENT - service={}, instance={}, type={}, data={}",
        service_name,
        instance_id,
        event_type,
        serde_json::to_string_pretty(event_data)
    );
}

/// 获取系统资源指标
async fn get_system_metrics() -> (f64, f64) {
    // 在实际微服务中，这些应该从系统监控获取
    // 这里我们模拟一些合理的值
    let memory_usage_mb = 50.0 + (rand::random::<f64>() * 200.0); // 50-250MB
    let cpu_usage_percent = 10.0 + (rand::random::<f64>() * 80.0); // 10-90%
    
    (memory_usage_mb, cpu_usage_percent)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Inklog 微服务架构示例 ===\n");

    // 从环境变量读取微服务配置
    let service_config = MicroserviceConfig {
        service_name: env::var("SERVICE_NAME").unwrap_or_else(|_| "user-service".to_string()),
        service_version: env::var("SERVICE_VERSION").unwrap_or_else(|_| "1.0.0".to_string()),
        environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        tracing_id: env::var("TRACING_ID").unwrap_or_else(|_| "trace-all".to_string()),
        instance_id: env::var("INSTANCE_ID").unwrap_or_else(|_| format!("instance-{}", uuid::Uuid::new_v4())),
    };

    // 设置加密密钥（如果启用加密）
    if service_config.environment == "production" {
        let encryption_key = "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=";
        env::set_var("INKLOG_ENCRYPTION_KEY", encryption_key);
    }

    // 配置日志
    let config = InklogConfig {
        global: GlobalConfig {
            level: if service_config.environment == "production" { "info".to_string() } else { "debug".to_string() },
            format: "[{timestamp}] [{level:>5}] [{service}:{instance}] {target} - {message}".to_string(),
            masking_enabled: true, // 在生产环境启用数据掩码
            ..Default::default()
        },
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: format!("logs/{}.log", service_config.service_name),
            max_size: "500MB".into(),
            rotation_time: "hourly".into(),
            keep_files: 168, // 7天（每小时一个文件）
            batch_size: 500,
            flush_interval_ms: 2000,
            compress: true,
            compression_level: if service_config.environment == "production" { 6 } else { 3 },
            encrypt: service_config.environment == "production",
            encryption_key_env: if service_config.environment == "production" {
                Some("INKLOG_ENCRYPTION_KEY".into())
            } else {
                None
            },
            ..Default::default()
        }),
        database_sink: Some(DatabaseSinkConfig {
            enabled: true,
            driver: inklog::config::DatabaseDriver::PostgreSQL,
            url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                format!("postgresql://localhost:5432/{}", service_config.service_name)
            }),
            pool_size: 10,
            batch_size: 100,
            flush_interval_ms: 5000,
            table_name: "service_logs".to_string(),
            ..Default::default()
        }),
        console_sink: Some(ConsoleSinkConfig {
            enabled: true,
            colored: service_config.environment != "production",
            ..Default::default()
        }),
        http_server: Some(HttpServerConfig {
            enabled: true,
            host: "0.0.0.0".to_string(),
            port: 8080,
            metrics_path: "/metrics".to_string(),
            health_path: "/health".to_string(),
            ..Default::default()
        }),
        performance: inklog::config::PerformanceConfig {
            worker_threads: 8,
            channel_capacity: 50000,
            ..Default::default()
        },
        ..Default::default()
    };

    println!("微服务配置：");
    println!("  服务名称: {}", service_config.service_name);
    println!("  服务版本: {}", service_config.service_version);
    println!("  运行环境: {}", service_config.environment);
    println!("  实例ID: {}", service_config.instance_id);
    println!("  追踪ID: {}", service_config.tracing_id);

    let logger = LoggerManager::with_config(config).await?;
    let logger = Arc::new(logger);

    println!("\n=== 开始微服务日志记录演示 ===");

    // 模拟微服务运行，持续记录各种类型的日志
    let mut request_count = 0;
    let mut error_count = 0;
    let start_time = SystemTime::now();

    // 运行60秒的微服务模拟
    for cycle in 0..12 {
        println!("\n--- 模拟周期 {} ---", cycle + 1);

        // 模拟各种API请求
        let requests = vec![
            ("GET", "/api/users", Some("user123")),
            ("POST", "/api/orders", None),
            ("PUT", "/api/users/12345/profile", Some("user123")),
            ("DELETE", "/api/sessions/abc123", None),
        ];

        for (method, path, user_id) in requests {
            let (trace, error_info) = simulate_microservice_request(
                &service_config.service_name,
                &service_config.instance_id,
                method,
                path,
                user_id,
                None, // session_id
            ).await;

            log_request_trace(&trace);

            match error_info {
                None => {
                    request_count += 1;
                    
                    // 模拟业务事件
                    if method == "POST" && path.contains("orders") {
                        let event_data = serde_json::json!({
                            "order_id": format!("order_{}", rand::random::<u32>()),
                            "user_id": user_id.unwrap_or("anonymous"),
                            "amount": rand::random::<f64>() * 100.0 + 10.0,
                            "currency": "USD"
                        });
                        
                        log_business_event(
                            &service_config.service_name,
                            &service_config.instance_id,
                            "order_created",
                            &event_data
                        );
                    }
                }
                Some(error) => {
                    error_count += 1;
                    
                    let error_details = ErrorDetails {
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        trace_id: trace.trace_id.clone(),
                        service_name: service_config.service_name.clone(),
                        instance_id: service_config.instance_id.clone(),
                        error_type: "api_error".to_string(),
                        error_code: None,
                        error_message: error,
                        stack_trace: Some(format!("at {}:{}", path)),
                        user_context: {
                            "method".to_string() => method.to_string(),
                            "path".to_string() => path.to_string(),
                            "user_id".to_string() => user_id.unwrap_or("anonymous").to_string(),
                        },
                        recovery_action: "retry_request".to_string(),
                    };
                    
                    log_error_details(&error_details);
                }
            }

            // 模拟一些处理延迟
            sleep(Duration::from_millis(50)).await;
        }

        // 每5个周期记录性能指标
        if (cycle + 1) % 5 == 0 {
            let (memory_mb, cpu_percent) = get_system_metrics().await;
            let elapsed = start_time.elapsed().as_secs();
            
            let metrics = PerformanceMetrics {
                service_name: service_config.service_name.clone(),
                instance_id: service_config.instance_id.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                request_count,
                error_count,
                avg_response_time_ms: 250.0, // 平均响应时间
                p95_response_time_ms: 450.0, // P95响应时间
                memory_usage_mb,
                cpu_usage_percent,
            };
            
            log_performance_metrics(&metrics);
        }

        // 模拟健康检查日志
        log::info!(
            target: "microservice.health",
            "HEALTH_CHECK - service={}, instance={}, status=healthy, uptime={}s",
            service_config.service_name,
            service_config.instance_id,
            start_time.elapsed().as_secs()
        );

        // 短暂休息模拟真实服务
        sleep(Duration::from_millis(200)).await;
    }

    let total_elapsed = start_time.elapsed();
    
    println!("\n=== 微服务日志记录演示完成 ===");
    println!("运行时长: {:?}", total_elapsed);
    println!("总请求数: {}", request_count);
    println!("总错误数: {}", error_count);
    println!("成功率: {:.2}%", (request_count as f64 - error_count as f64) / request_count as f64 * 100.0);
    
    // 记录服务启动和关闭事件
    log_business_event(
        &service_config.service_name,
        &service_config.instance_id,
        "service_shutdown",
        &serde_json::json!({
            "service_name": service_config.service_name,
            "instance_id": service_config.instance_id,
            "uptime_seconds": total_elapsed.as_secs(),
            "total_requests": request_count,
            "total_errors": error_count,
            "success_rate": (request_count as f64 - error_count as f64) / request_count as f64 * 100.0
        })
    );

    // 清理环境变量（仅在生产环境）
    if service_config.environment == "production" {
        env::remove_var("INKLOG_ENCRYPTION_KEY");
    }

    // 优雅关闭日志服务
    logger.shutdown().map_err(|e| format!("Failed to shutdown logger: {:?}", e))?;
    
    println!("=== 示例完成 ===");
    Ok(())
}