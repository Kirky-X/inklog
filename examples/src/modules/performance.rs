// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 性能配置示例
//!
//! 演示如何优化日志系统性能。

/// Channel 策略
pub fn channel_strategy() {
	println!("=== Channel 策略 ===\n");
	println!("支持的策略:");
	println!("  - blocking: 阻塞等待 (默认)");
	println!("  - drop_oldest: 丢弃最旧日志");
	println!("  - drop_newest: 丢弃最新日志");
	println!("\n配置示例:");
	println!("  [performance]");
	println!("  channel_capacity = 10000");
	println!("  channel_strategy = \"drop_oldest\"");
}

/// Worker 线程配置
pub fn worker_config() {
	println!("\n=== Worker 线程配置 ===\n");
	println!("配置参数:");
	println!("  worker_threads = 4  // 工作线程数");
	println!("  batch_size = 100    // 批量处理大小");
}

/// 性能监控
pub fn performance_monitoring() {
	println!("\n=== 性能监控 ===\n");
	println!("关键指标:");
	println!("  - channel_usage: channel 使用率");
	println!("  - logs_per_second: 每秒日志数");
	println!("  - avg_latency: 平均延迟");
}

/// 运行所有示例
pub fn run_all() {
	channel_strategy();
	worker_config();
	performance_monitoring();
}
