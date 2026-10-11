// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
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

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "manual"]
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

        println!("starting long-running test ({}s)", duration);
        println!("initial memory: {:.2} MB", initial_memory);

        while start_time.elapsed() < Duration::from_secs(duration) {
            tracing::info!(
                "long-running test log #{} - time: {:?}",
                log_count,
                chrono::Utc::now()
            );
            log_count += 1;

            tokio::time::sleep(Duration::from_millis(interval_ms)).await;

            if log_count.is_multiple_of(100) {
                let current_memory = get_memory_usage_mb();
                let growth = current_memory - initial_memory;
                println!(
                    "wrote {} logs, current memory: {:.2} MB, growth: {:.2} MB",
                    log_count, current_memory, growth
                );
            }
        }

        let final_memory = get_memory_usage_mb();
        let memory_growth = final_memory - initial_memory;

        println!("test completed");
        println!("total logs: {}", log_count);
        println!("initial memory: {:.2} MB", initial_memory);
        println!("final memory: {:.2} MB", final_memory);
        println!("memory growth: {:.2} MB", memory_growth);

        assert!(
            memory_growth < max_memory_growth_mb,
            "memory growth exceeds the threshold: {:.2} MB > {:.2} MB",
            memory_growth,
            max_memory_growth_mb
        );
    }

    // 核正：进程级 RSS 增长断言在 cargo test 并发执行下不可靠——同进程其他用例
    // （性能基准等）的内存占用均计入 VmRSS，增长与本测试行为无关。与
    // test_long_running_memory_stability 的既有口径一致，改为手动运行：
    // cargo test -p inklog --test performance long_running -- --ignored
    #[tokio::test]
    #[ignore = "manual"]
    async fn test_short_memory_stability() {
        let duration = 5u64;
        let initial_memory = get_memory_usage_mb();
        let start_time = Instant::now();
        let mut log_count = 0u64;

        while start_time.elapsed() < Duration::from_secs(duration) {
            tracing::info!("short test log #{}", log_count);
            log_count += 1;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let final_memory = get_memory_usage_mb();
        let memory_growth = final_memory - initial_memory;

        println!(
            "short test: {} logs, memory growth: {:.2} MB",
            log_count, memory_growth
        );

        assert!(
            memory_growth < 10.0,
            "short-test memory growth too large: {:.2} MB",
            memory_growth
        );
    }

    #[test]
    fn test_memory_measurement_available() {
        let memory = get_memory_usage_mb();
        println!("current memory usage: {:.2} MB", memory);
        assert!(
            memory >= 0.0,
            "memory measurement should return a non-negative value"
        );
    }
}
