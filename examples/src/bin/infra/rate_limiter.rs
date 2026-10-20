// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 速率限制器示例（Layer 0 零依赖）
//!
//! 演示 `inklog::RateLimiter` 令牌桶算法的使用：
//!
//! Example 1: creating the rate limiter（指定每秒令牌数）
//! 2. `try_acquire` 获取令牌（允许/拒绝）
//! 3. `dropped_count` 统计被限流的日志数
//! Example 4: automatic token refill（等待后恢复）
//! 5. 与日志写入结合的限流模式
//!
//! # 运行
//! ```bash
//! cargo run --bin rate_limiter
//! ```

use inklog::RateLimiter;
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::thread;
use std::time::Duration;

fn main() {
    print_separator(&tr("example-rate-limiter-inklog-rate-limiter-example"));

    show_basic_creation();
    show_rate_limiting();
    show_dropped_count();
    show_token_refill();
    show_log_rate_limiting_pattern();
    show_zero_rate();
    show_best_practices();

    println!("\n{}", tr("example-rate-limiter-all-rate-limiter-examples"));
}

/// 展示限流器创建与基础属性
fn show_basic_creation() {
    print_section(&tr("example-rate-limiter-example-1-creating-the-rate"));

    let limiter = RateLimiter::new(1000);
    println!("{}", tr("example-rate-limiter-ratelimiter-new-1000-1000"));
    println!("  {}", tr("example-rate-limiter-initial-state-bucket-full"));
    println!(
        "  {}",
        tr("example-rate-limiter-refill-rate-1000-tokens-sec")
    );

    assert!(limiter.try_acquire());
    println!(
        "  {}",
        tr("example-rate-limiter-try-acquire-true-bucket-full")
    );
}

/// 展示速率限制行为
fn show_rate_limiting() {
    print_section(&tr("example-rate-limiter-2-rate-limiting-behavior"));

    // 低速率便于演示：每秒 5 个令牌
    let limiter = RateLimiter::new(5);
    println!(
        "{}\n",
        tr("example-rate-limiter-ratelimiter-new-5-5-tokens-per")
    );

    println!(
        "{}",
        tr("example-rate-limiter-rapid-consecutive-try-acquire")
    );
    let mut allowed = 0;
    let mut rejected = 0;
    for i in 1..=10 {
        let ok = limiter.try_acquire();
        let status = if ok {
            allowed += 1;
            "✓ allowed"
        } else {
            rejected += 1;
            "✗ rejected"
        };
        println!("  #{:<3} {}", i, status);
    }

    println!(
        "\n{}",
        tr_args(
            "example-rate-limiter-result-allowed-rejected",
            &[("allowed", &allowed), ("rejected", &rejected)]
        )
    );
    assert!(allowed > 0, "should allow a few through");
    assert!(rejected > 0, "should reject once the bucket is drained");
    println!("  {}", tr("example-rate-limiter-token-bucket-rate-limits"));
}

/// 展示 dropped_count 统计
fn show_dropped_count() {
    print_section(&tr("example-rate-limiter-example-3-dropped-count"));

    let limiter = RateLimiter::new(2);
    println!(
        "{}\n",
        tr("example-rate-limiter-ratelimiter-new-2-2-tokens-per")
    );

    assert!(limiter.try_acquire());
    assert!(limiter.try_acquire());
    println!(
        "{}",
        tr_args(
            "example-rate-limiter-after-consuming-2-tokens",
            &[("value", &(limiter.dropped_count()))]
        )
    );
    assert_eq!(limiter.dropped_count(), 0);

    // 后续请求被拒绝，dropped_count 递增
    assert!(!limiter.try_acquire());
    assert!(!limiter.try_acquire());
    assert!(!limiter.try_acquire());
    println!(
        "{}",
        tr_args(
            "example-rate-limiter-after-3-rejections-dropped-count",
            &[("value", &(limiter.dropped_count()))]
        )
    );
    assert_eq!(limiter.dropped_count(), 3);

    println!(
        "\n  {}",
        tr("example-rate-limiter-dropped-count-correctly-counts")
    );
}

/// 展示令牌自动补充
fn show_token_refill() {
    print_section(&tr("example-rate-limiter-example-4-automatic-token-refill"));

    let limiter = RateLimiter::new(100);
    println!(
        "{}\n",
        tr("example-rate-limiter-ratelimiter-new-100-100-tokens")
    );

    for _ in 0..100 {
        limiter.try_acquire();
    }
    println!("{}", tr("example-rate-limiter-after-draining-100-tokens"));
    assert!(!limiter.try_acquire());
    println!(
        "  {}",
        tr("example-rate-limiter-try-acquire-false-bucket-empty")
    );

    // 等待 50ms，应补充约 5 个令牌（100 tokens/sec × 0.05s）
    thread::sleep(Duration::from_millis(50));
    println!(
        "\n{}",
        tr("example-rate-limiter-after-waiting-50ms-5-tokens")
    );
    let mut refilled = 0;
    for _ in 0..10 {
        if limiter.try_acquire() {
            refilled += 1;
        }
    }
    println!(
        "  {}",
        tr_args(
            "example-rate-limiter-acquired-again-allowed-expected",
            &[("refilled", &refilled)]
        )
    );
    assert!(refilled > 0, "tokens should be available after waiting");
    println!(
        "  {}",
        tr("example-rate-limiter-tokens-refill-automatically-at")
    );
}

/// 展示日志限流的实际应用模式
fn show_log_rate_limiting_pattern() {
    print_section(&tr("example-rate-limiter-example-5-log-rate-limiting"));

    let limiter = RateLimiter::new(10);
    println!(
        "{}\n",
        tr("example-rate-limiter-scenario-cap-log-output-at-10")
    );

    println!(
        "{}",
        tr("example-rate-limiter-simulating-a-burst-of-logs-20")
    );
    let mut written = 0;
    let mut dropped = 0;
    for i in 1..=20 {
        if limiter.try_acquire() {
            println!(
                "  {}",
                tr_args("example-rate-limiter-info-log-written", &[("i", &i)])
            );
            written += 1;
        } else {
            dropped += 1;
        }
    }

    println!("\n{}", tr("example-rate-limiter-statistics"));
    println!(
        "  {}",
        tr_args("example-rate-limiter-written", &[("written", &written)])
    );
    println!(
        "  {}",
        tr_args("example-rate-limiter-dropped", &[("dropped", &dropped)])
    );
    println!(
        "  {}",
        tr_args(
            "example-rate-limiter-total-dropped",
            &[("value", &(limiter.dropped_count()))]
        )
    );
    assert_eq!(written + dropped, 20);
    assert_eq!(limiter.dropped_count(), dropped as u64);
    println!(
        "\n  {}",
        tr("example-rate-limiter-rate-limiting-works-protects")
    );
}

/// 展示零速率限流器
fn show_zero_rate() {
    print_section(&tr("example-rate-limiter-example-6-zero-rate-limiter"));

    let limiter = RateLimiter::new(0);
    println!(
        "{}\n",
        tr("example-rate-limiter-ratelimiter-new-0-allows-no")
    );

    assert!(!limiter.try_acquire());
    assert!(!limiter.try_acquire());
    println!("  {}", tr("example-rate-limiter-try-acquire-always-false"));
    println!(
        "  {}",
        tr_args(
            "example-rate-limiter-dropped-count",
            &[("value", &(limiter.dropped_count()))]
        )
    );
    assert_eq!(limiter.dropped_count(), 2);
    println!(
        "  {}",
        tr("example-rate-limiter-rate-0-is-equivalent-to-full")
    );
}

/// 最佳实践建议
fn show_best_practices() {
    print_section(&tr("example-rate-limiter-7-best-practices"));

    println!("{}\n", tr("example-rate-limiter-ratelimiter-usage-advice"));

    println!("{}", tr("example-rate-limiter-1-rate-selection"));
    println!(
        "   {}",
        tr("example-rate-limiter-development-1000-5000-tokens")
    );
    println!(
        "   {}",
        tr("example-rate-limiter-production-100-1000-tokens-sec")
    );
    println!(
        "   {}",
        tr("example-rate-limiter-failure-scenarios-10-50-tokens")
    );

    println!(
        "\n{}",
        tr("example-rate-limiter-2-integrating-with-loggermanager")
    );
    println!("   let limiter = RateLimiter::new(1000);");
    println!("   // Check before writing logs:");
    println!("   if limiter.try_acquire() {{");
    println!("       sink.write(&record).await;");
    println!("   }}");

    println!(
        "\n{}",
        tr("example-rate-limiter-3-monitoring-dropped-count")
    );
    println!(
        "   {}",
        tr("example-rate-limiter-report-dropped-count-to-metrics")
    );
    println!(
        "   {}",
        tr("example-rate-limiter-continuously-growing-dropped")
    );
    println!(
        "   {}",
        tr("example-rate-limiter-dropped-count-0-means-the-limit")
    );

    println!(
        "\n{}",
        tr("example-rate-limiter-4-performance-characteristics")
    );
    println!("   {}", tr("example-rate-limiter-try-acquire-is-o-1"));
    println!(
        "   {}",
        tr("example-rate-limiter-uses-parking-lot-mutex-20ns")
    );
    println!(
        "   {}",
        tr("example-rate-limiter-suits-high-frequency-logging")
    );
}
