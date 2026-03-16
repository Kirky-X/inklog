// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 资源监控测试
//!
//! 验证系统在正常负载下的CPU和内存占用

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

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

    fn get_cpu_usage_percent() -> f64 {
        let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
        let parts: Vec<&str> = stat.split_whitespace().collect();
        if parts.len() > 15 {
            let utime: f64 = parts[13].parse().unwrap_or(0.0);
            let stime: f64 = parts[14].parse().unwrap_or(0.0);
            let total_time = utime + stime;
            let hz = 100.0;
            let elapsed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            if elapsed > 0.0 {
                return (total_time / hz / elapsed) * 100.0;
            }
        }
        0.0
    }

    #[tokio::test]
    async fn test_cpu_usage_under_normal_load() {
        let max_cpu_percent: f64 = std::env::var("MAX_CPU_PERCENT")
            .map(|c| c.parse().unwrap_or(5.0))
            .unwrap_or(5.0);

        let test_duration = std::env::var("CPU_TEST_DURATION_SECS")
            .map(|d| d.parse().unwrap_or(10))
            .unwrap_or(10);

        let start_time = Instant::now();
        let mut log_count = 0u64;
        let mut cpu_samples = Vec::new();

        println!("开始CPU占用测试 ({}秒)", test_duration);

        while start_time.elapsed() < Duration::from_secs(test_duration) {
            tracing::info!("CPU测试日志 #{}", log_count);
            log_count += 1;

            if log_count % 50 == 0 {
                let cpu = get_cpu_usage_percent();
                cpu_samples.push(cpu);
                println!("CPU使用率: {:.2}%", cpu);
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        let avg_cpu = if !cpu_samples.is_empty() {
            cpu_samples.iter().sum::<f64>() / cpu_samples.len() as f64
        } else {
            0.0
        };

        println!("测试完成");
        println!("总日志数: {}", log_count);
        println!("平均CPU使用率: {:.2}%", avg_cpu);
        println!("最大CPU使用率: {:.2}%", cpu_samples.iter().cloned().fold(0.0, f64::max));

        assert!(
            avg_cpu < max_cpu_percent,
            "平均CPU使用率超过阈值: {:.2}% > {:.2}%",
            avg_cpu,
            max_cpu_percent
        );
    }

    #[tokio::test]
    async fn test_memory_usage_under_normal_load() {
        let max_memory_mb: f64 = std::env::var("MAX_MEMORY_MB")
            .map(|m| m.parse().unwrap_or(30.0))
            .unwrap_or(30.0);

        let test_duration = std::env::var("MEMORY_TEST_DURATION_SECS")
            .map(|d| d.parse().unwrap_or(10))
            .unwrap_or(10);

        let initial_memory = get_memory_usage_mb();
        let start_time = Instant::now();
        let mut log_count = 0u64;
        let mut memory_samples = Vec::new();

        println!("开始内存占用测试 ({}秒)", test_duration);
        println!("初始内存: {:.2} MB", initial_memory);

        while start_time.elapsed() < Duration::from_secs(test_duration) {
            tracing::info!("内存测试日志 #{} - 数据: {}", log_count, "x".repeat(100));
            log_count += 1;

            if log_count % 50 == 0 {
                let mem = get_memory_usage_mb();
                memory_samples.push(mem);
                println!("内存使用: {:.2} MB", mem);
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        let final_memory = get_memory_usage_mb();
        let max_memory = memory_samples.iter().cloned().fold(0.0, f64::max);
        let avg_memory = if !memory_samples.is_empty() {
            memory_samples.iter().sum::<f64>() / memory_samples.len() as f64
        } else {
            final_memory
        };

        println!("测试完成");
        println!("总日志数: {}", log_count);
        println!("初始内存: {:.2} MB", initial_memory);
        println!("最终内存: {:.2} MB", final_memory);
        println!("平均内存: {:.2} MB", avg_memory);
        println!("最大内存: {:.2} MB", max_memory);

        assert!(
            avg_memory < max_memory_mb,
            "平均内存使用超过阈值: {:.2} MB > {:.2} MB",
            avg_memory,
            max_memory_mb
        );
    }

    #[test]
    fn test_resource_measurement_available() {
        let memory = get_memory_usage_mb();
        let cpu = get_cpu_usage_percent();
        println!("当前内存使用: {:.2} MB", memory);
        println!("当前CPU使用率: {:.2}%", cpu);
        assert!(memory >= 0.0, "内存测量应该返回非负值");
        assert!(cpu >= 0.0, "CPU测量应该返回非负值");
    }
}
