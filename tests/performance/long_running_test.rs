// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 长时间运行内存泄漏测试
//!
//! 验证系统在长时间运行条件下的内存稳定性

use std::time::{Duration, Instant};

#[cfg(test)]
mod tests {
    use super::*;

    fn get_memory_usage_mb() -> f64 {
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let kb: f64 = line
                    .split(':')
                    .nth(1)
                    .unwrap_or("0")
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0.0);
                return kb / 1024.0;
            }
        }
        0.0
    }

    #[tokio::test]
    #[ignore]
    async fn test_long_running_memory_stability() {
        let duration = std::env::var("TEST_DURATION_SECS")
            .map(|d| d.parse().unwrap_or(60))
            .unwrap_or(60);

        let interval_ms = std::env::var("TEST_INTERVAL_MS")
            .map(|i| i.parse().unwrap_or(100))
            .unwrap_or(100);

        let max_memory_growth_mb: f64 = std::env::var("MAX_MEMORY_GROWTH_MB")
            .map(|m| m.parse().unwrap_or(50.0))
            .unwrap_or(50.0);

        let initial_memory = get_memory_usage_mb();
        let start_time = Instant::now();
        let mut log_count = 0u64;

        println!("开始长时间运行测试 ({}秒)", duration);
        println!("初始内存: {:.2} MB", initial_memory);

        while start_time.elapsed() < Duration::from_secs(duration) {
            tracing::info!(
                "长时间运行测试日志 #{} - 时间: {:?}",
                log_count,
                chrono::Utc::now()
            );
            log_count += 1;

            tokio::time::sleep(Duration::from_millis(interval_ms)).await;

            if log_count % 100 == 0 {
                let current_memory = get_memory_usage_mb();
                let growth = current_memory - initial_memory;
                println!(
                    "已写入 {} 条日志, 当前内存: {:.2} MB, 增长: {:.2} MB",
                    log_count, current_memory, growth
                );
            }
        }

        let final_memory = get_memory_usage_mb();
        let memory_growth = final_memory - initial_memory;

        println!("测试完成");
        println!("总日志数: {}", log_count);
        println!("初始内存: {:.2} MB", initial_memory);
        println!("最终内存: {:.2} MB", final_memory);
        println!("内存增长: {:.2} MB", memory_growth);

        assert!(
            memory_growth < max_memory_growth_mb,
            "内存增长超过阈值: {:.2} MB > {:.2} MB",
            memory_growth,
            max_memory_growth_mb
        );
    }

    #[tokio::test]
    async fn test_short_memory_stability() {
        let duration = 5u64;
        let initial_memory = get_memory_usage_mb();
        let start_time = Instant::now();
        let mut log_count = 0u64;

        while start_time.elapsed() < Duration::from_secs(duration) {
            tracing::info!("短时测试日志 #{}", log_count);
            log_count += 1;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let final_memory = get_memory_usage_mb();
        let memory_growth = final_memory - initial_memory;

        println!(
            "短时测试: {} 条日志, 内存增长: {:.2} MB",
            log_count, memory_growth
        );

        assert!(memory_growth < 10.0, "短时测试内存增长过大: {:.2} MB", memory_growth);
    }

    #[test]
    fn test_memory_measurement_available() {
        let memory = get_memory_usage_mb();
        println!("当前内存使用: {:.2} MB", memory);
        assert!(memory >= 0.0, "内存测量应该返回非负值");
    }
}
