// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 性能统计：百分位计算与吞吐量格式化。
//!
//! 从 `examples/src/bin/performance.rs` 提取纯计算逻辑（不依赖 sink）：
//!
//! - [`calculate_percentiles`]：计算 P50/P95/P99 延迟。
//! - [`format_throughput`]：把 `(duration, count)` 格式化为人类可读字符串。

use std::time::Duration;

/// 计算 P50/P95/P99 延迟，返回 `(p50, p95, p99)`。
///
/// - 输入为空时返回 `(Duration::ZERO, Duration::ZERO, Duration::ZERO)`。
/// - 输入仅 1 个元素时三者相同。
/// - 否则先排序，再按索引取百分位（与原 bin 的算法一致：
///   `p50 = sorted[n/2]`，`p95 = sorted[(n*0.95) as usize]`，`p99 = sorted[(n*0.99) as usize]`）。
///
/// 不修改入参（内部 clone 后排序）。
pub fn calculate_percentiles(mut latencies: Vec<Duration>) -> (Duration, Duration, Duration) {
    if latencies.is_empty() {
        return (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    }
    latencies.sort();
    let n = latencies.len();
    let p50 = latencies[n / 2];
    // 注意：n*0.95 转 usize 在 n=0 时为 0，但前面已处理空列表。
    // 用 min 防止 n=1 时 (1.0 * 0.95) as usize = 0 导致越界之外的偏移问题。
    let p95_idx = ((n as f64) * 0.95) as usize;
    let p99_idx = ((n as f64) * 0.99) as usize;
    let p95 = latencies[p95_idx.min(n - 1)];
    let p99 = latencies[p99_idx.min(n - 1)];
    (p50, p95, p99)
}

/// 格式化吞吐量为字符串：`{count} ops in {secs:.3}s → {throughput:.2} ops/sec`。
///
/// `duration` 为 0 时返回 `"N/A"`（避免除零）。
pub fn format_throughput(duration: Duration, count: usize) -> String {
    if duration.is_zero() {
        return format!("{} ops in 0.000s → N/A ops/sec (zero duration)", count);
    }
    let secs = duration.as_secs_f64();
    let throughput = count as f64 / secs;
    format!("{} ops in {:.3}s → {:.2} ops/sec", count, secs, throughput)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_percentiles_basic() {
        // 验证：100 个递增元素，P50/P95/P99 索引正确。
        let latencies: Vec<Duration> = (0..100).map(Duration::from_micros).collect();
        let (p50, p95, p99) = calculate_percentiles(latencies);
        // n=100, p50_idx=50, p95_idx=95, p99_idx=99
        assert_eq!(p50, Duration::from_micros(50));
        assert_eq!(p95, Duration::from_micros(95));
        assert_eq!(p99, Duration::from_micros(99));
    }

    #[test]
    fn test_calculate_percentiles_empty() {
        // 验证：空输入返回零时长。
        let (p50, p95, p99) = calculate_percentiles(vec![]);
        assert_eq!(p50, Duration::ZERO);
        assert_eq!(p95, Duration::ZERO);
        assert_eq!(p99, Duration::ZERO);
    }

    #[test]
    fn test_calculate_percentiles_single() {
        // 验证：单元素时三者相同。
        let d = Duration::from_micros(42);
        let (p50, p95, p99) = calculate_percentiles(vec![d]);
        assert_eq!(p50, d);
        assert_eq!(p95, d);
        assert_eq!(p99, d);
    }

    #[test]
    fn test_calculate_percentiles_unsorted_input() {
        // 验证：未排序输入也能正确计算（函数内部会排序）。
        let latencies = vec![
            Duration::from_micros(100),
            Duration::from_micros(10),
            Duration::from_micros(50),
            Duration::from_micros(20),
            Duration::from_micros(80),
        ];
        let (p50, _p95, _p99) = calculate_percentiles(latencies);
        // 排序后 [10, 20, 50, 80, 100]，n=5, p50_idx=2 → 50
        assert_eq!(p50, Duration::from_micros(50));
    }

    #[test]
    fn test_calculate_percentiles_does_not_modify_input() {
        // 验证：函数按值取参，调用方原始 Vec 不受影响（编译期保证，这里加显式测试）。
        let original = vec![
            Duration::from_micros(3),
            Duration::from_micros(1),
            Duration::from_micros(2),
        ];
        let snapshot = original.clone();
        let _ = calculate_percentiles(original);
        // original 已被 move，无法再访问；snapshot 应保持原顺序
        assert_eq!(snapshot[0], Duration::from_micros(3));
        assert_eq!(snapshot[1], Duration::from_micros(1));
        assert_eq!(snapshot[2], Duration::from_micros(2));
    }

    #[test]
    fn test_format_throughput_normal() {
        // 验证：正常输入的格式化输出包含 count、秒数、ops/sec。
        let s = format_throughput(Duration::from_secs(2), 1000);
        assert!(s.contains("1000 ops"), "应包含 count: {}", s);
        assert!(s.contains("2.000s"), "应包含 2.000s: {}", s);
        assert!(s.contains("500.00 ops/sec"), "应包含 500.00 ops/sec: {}", s);
    }

    #[test]
    fn test_format_throughput_zero_duration() {
        // 验证：零时长返回 N/A，避免除零。
        let s = format_throughput(Duration::ZERO, 100);
        assert!(s.contains("N/A"), "零时长应返回 N/A: {}", s);
    }

    #[test]
    fn test_format_throughput_subsecond() {
        // 验证：亚秒级时长的格式化（500ms = 0.5s，1000 ops → 2000 ops/sec）。
        let s = format_throughput(Duration::from_millis(500), 1000);
        assert!(s.contains("0.500s"), "应包含 0.500s: {}", s);
        assert!(
            s.contains("2000.00 ops/sec"),
            "应包含 2000.00 ops/sec: {}",
            s
        );
    }
}
