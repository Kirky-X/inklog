// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 速率限制器示例（Layer 0 零依赖）
//!
//! 演示 `inklog::RateLimiter` 令牌桶算法的使用：
//!
//! Example 1: creating the rate limiter（指定每秒令牌数）
//! 2. `try_acquire()` 获取令牌（允许/拒绝）
//! 3. `dropped_count()` 统计被限流的日志数
//! Example 4: automatic token refill（等待后恢复）
//! 5. 与日志写入结合的限流模式
//!
//! # 运行
//! ```bash
//! cargo run --bin rate_limiter
//! ```

use inklog::RateLimiter;
use inklog_examples::common::{print_section, print_separator};
use std::thread;
use std::time::Duration;

fn main() {
    print_separator("inklog rate limiter example");

    show_basic_creation();
    show_rate_limiting();
    show_dropped_count();
    show_token_refill();
    show_log_rate_limiting_pattern();
    show_zero_rate();
    show_best_practices();

    println!("\n✓ All rate limiter examples completed");
}

/// 展示限流器创建与基础属性
fn show_basic_creation() {
    print_section("Example 1: creating the rate limiter");

    let limiter = RateLimiter::new(1000);
    println!("RateLimiter::new(1000) — 1000 tokens per second");
    println!("  Initial state: bucket full (tokens = max_tokens = 1000)");
    println!("  Refill rate: 1000 tokens/sec");

    assert!(limiter.try_acquire());
    println!("  ✓ try_acquire() → true (bucket full, allowed)");
}

/// 展示速率限制行为
fn show_rate_limiting() {
    print_section("2. Rate limiting behavior");

    // 低速率便于演示：每秒 5 个令牌
    let limiter = RateLimiter::new(5);
    println!("RateLimiter::new(5) — 5 tokens per second\n");

    println!("Rapid consecutive try_acquire():");
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

    println!("\nResult: {} allowed, {} rejected", allowed, rejected);
    assert!(allowed > 0, "should allow a few through");
    assert!(rejected > 0, "should reject once the bucket is drained");
    println!("  ✓ Token bucket rate-limits correctly");
}

/// 展示 dropped_count 统计
fn show_dropped_count() {
    print_section("Example 3: dropped_count() statistics");

    let limiter = RateLimiter::new(2);
    println!("RateLimiter::new(2) — 2 tokens per second\n");

    assert!(limiter.try_acquire());
    assert!(limiter.try_acquire());
    println!(
        "After consuming 2 tokens: dropped_count = {}",
        limiter.dropped_count()
    );
    assert_eq!(limiter.dropped_count(), 0);

    // 后续请求被拒绝，dropped_count 递增
    assert!(!limiter.try_acquire());
    assert!(!limiter.try_acquire());
    assert!(!limiter.try_acquire());
    println!(
        "After 3 rejections: dropped_count = {}",
        limiter.dropped_count()
    );
    assert_eq!(limiter.dropped_count(), 3);

    println!("\n  ✓ dropped_count correctly counts rate-limited logs");
}

/// 展示令牌自动补充
fn show_token_refill() {
    print_section("Example 4: automatic token refill");

    let limiter = RateLimiter::new(100);
    println!("RateLimiter::new(100) — 100 tokens per second\n");

    for _ in 0..100 {
        limiter.try_acquire();
    }
    println!("After draining 100 tokens:");
    assert!(!limiter.try_acquire());
    println!("  try_acquire() → false (bucket empty)");

    // 等待 50ms，应补充约 5 个令牌（100 tokens/sec × 0.05s）
    thread::sleep(Duration::from_millis(50));
    println!("\nAfter waiting 50ms (~5 tokens refilled):");
    let mut refilled = 0;
    for _ in 0..10 {
        if limiter.try_acquire() {
            refilled += 1;
        }
    }
    println!("  Acquired again: {} allowed (expected ~5)", refilled);
    assert!(refilled > 0, "tokens should be available after waiting");
    println!("  ✓ Tokens refill automatically at refill_rate");
}

/// 展示日志限流的实际应用模式
fn show_log_rate_limiting_pattern() {
    print_section("Example 5: log rate-limiting patterns");

    let limiter = RateLimiter::new(10);
    println!("Scenario: cap log output at 10 logs/second\n");

    println!("Simulating a burst of logs (20):");
    let mut written = 0;
    let mut dropped = 0;
    for i in 1..=20 {
        if limiter.try_acquire() {
            println!("  [INFO] log #{} — written", i);
            written += 1;
        } else {
            dropped += 1;
        }
    }

    println!("\nStatistics:");
    println!("  Written: {}", written);
    println!("  Dropped: {}", dropped);
    println!("  Total dropped: {}", limiter.dropped_count());
    assert_eq!(written + dropped, 20);
    assert_eq!(limiter.dropped_count(), dropped as u64);
    println!("\n  ✓ Rate limiting works: protects downstream systems under bursts");
}

/// 展示零速率限流器
fn show_zero_rate() {
    print_section("Example 6: zero-rate limiter (rate=0)");

    let limiter = RateLimiter::new(0);
    println!("RateLimiter::new(0) — allows no logs at all\n");

    assert!(!limiter.try_acquire());
    assert!(!limiter.try_acquire());
    println!("  try_acquire() → always false");
    println!("  dropped_count = {}", limiter.dropped_count());
    assert_eq!(limiter.dropped_count(), 2);
    println!("  ✓ rate=0 is equivalent to full silence");
}

/// 最佳实践建议
fn show_best_practices() {
    print_section("7. Best practices");

    println!("RateLimiter usage advice:\n");

    println!("1. Rate selection:");
    println!("   - Development: 1000-5000 tokens/sec (loose)");
    println!("   - Production: 100-1000 tokens/sec (based on downstream capacity)");
    println!("   - Failure scenarios: 10-50 tokens/sec (prevents log storms)");

    println!("\n2. Integrating with LoggerManager:");
    println!("   let limiter = RateLimiter::new(1000);");
    println!("   // Check before writing logs:");
    println!("   if limiter.try_acquire() {{");
    println!("       sink.write(&record).await;");
    println!("   }}");

    println!("\n3. Monitoring dropped_count:");
    println!("   - Report dropped_count to Metrics periodically");
    println!("   - continuously growing dropped_count means the limit is too strict");
    println!("   - dropped_count = 0 means the limit is loose or there is no pressure");

    println!("\n4. Performance characteristics:");
    println!("   - try_acquire() is O(1)");
    println!("   - Uses parking_lot::Mutex (~20ns lock overhead)");
    println!("   - Suits high-frequency logging");
}
