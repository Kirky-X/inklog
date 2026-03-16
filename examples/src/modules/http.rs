// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! HTTP 健康监控示例
//!
//! 演示如何配置和使用 HTTP 健康监控功能。

/// HTTP 服务器配置
pub fn http_config() {
	println!("=== HTTP 健康监控配置 ===\n");
	println!("配置示例:");
	println!("  [http_server]");
	println!("  enabled = true");
	println!("  host = \"127.0.0.1\"");
	println!("  port = 8080");
	println!("  health_path = \"/health\"");
	println!("  metrics_path = \"/metrics\"");
}

/// 健康端点
pub fn health_endpoint() {
	println!("\n=== 健康端点 ===\n");
	println!("GET /health 返回:");
	println!("  {{");
	println!("    \"status\": \"healthy\",");
	println!("    \"sinks\": {{");
	println!("      \"console\": {{ \"status\": \"operational\" }},");
	println!("      \"file\": {{ \"status\": \"operational\" }}");
	println!("    }}");
	println!("  }}");
}

/// Prometheus 指标
pub fn prometheus_metrics() {
	println!("\n=== Prometheus 指标 ===\n");
	println!("GET /metrics 返回:");
	println!("  # HELP inklog_logs_total Total logs processed");
	println!("  # TYPE inklog_logs_total counter");
	println!("  inklog_logs_total{{level=\"info\"}} 1234");
}

/// 错误处理模式
pub fn error_modes() {
	println!("\n=== 错误处理模式 ===\n");
	println!("支持的错误模式:");
	println!("  - warn: 记录警告并继续 (默认)");
	println!("  - strict: 返回错误");
}

/// 运行所有示例
pub fn run_all() {
	http_config();
	health_endpoint();
	prometheus_metrics();
	error_modes();
}
