// SPDX-License-Identifier: MIT
//! 速率限制器示例（Layer 0 零依赖）
//!
//! 演示 `inklog::RateLimiter` 令牌桶算法的使用：
//!
//! 1. 创建限流器（指定每秒令牌数）
//! 2. `try_acquire()` 获取令牌（允许/拒绝）
//! 3. `dropped_count()` 统计被限流的日志数
//! 4. 令牌自动补充（等待后恢复）
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
    print_separator("inklog 速率限制器示例");

    show_basic_creation();
    show_rate_limiting();
    show_dropped_count();
    show_token_refill();
    show_log_rate_limiting_pattern();
    show_zero_rate();
    show_best_practices();

    println!("\n✓ 所有速率限制器示例演示完成");
}

/// 展示限流器创建与基础属性
fn show_basic_creation() {
    print_section("1. 创建限流器");

    let limiter = RateLimiter::new(1000);
    println!("RateLimiter::new(1000) — 每秒 1000 个令牌");
    println!("  初始状态：桶已满（tokens = max_tokens = 1000）");
    println!("  补充速率：1000 tokens/sec");

    // 初始状态应允许获取
    assert!(limiter.try_acquire());
    println!("  ✓ try_acquire() → true（桶满，允许）");
}

/// 展示速率限制行为
fn show_rate_limiting() {
    print_section("2. 速率限制行为");

    // 低速率便于演示：每秒 5 个令牌
    let limiter = RateLimiter::new(5);
    println!("RateLimiter::new(5) — 每秒 5 个令牌\n");

    println!("快速连续 try_acquire()：");
    let mut allowed = 0;
    let mut rejected = 0;
    for i in 1..=10 {
        let ok = limiter.try_acquire();
        let status = if ok {
            allowed += 1;
            "✓ 允许"
        } else {
            rejected += 1;
            "✗ 拒绝"
        };
        println!("  #{:<3} {}", i, status);
    }

    println!("\n结果：允许 {} 次，拒绝 {} 次", allowed, rejected);
    assert!(allowed > 0, "应至少允许几次");
    assert!(rejected > 0, "桶耗尽后应拒绝");
    println!("  ✓ 令牌桶算法正确限流");
}

/// 展示 dropped_count 统计
fn show_dropped_count() {
    print_section("3. dropped_count() 统计");

    let limiter = RateLimiter::new(2);
    println!("RateLimiter::new(2) — 每秒 2 个令牌\n");

    // 消耗全部 2 个令牌
    assert!(limiter.try_acquire());
    assert!(limiter.try_acquire());
    println!(
        "消耗 2 个令牌后：dropped_count = {}",
        limiter.dropped_count()
    );
    assert_eq!(limiter.dropped_count(), 0);

    // 后续请求被拒绝，dropped_count 递增
    assert!(!limiter.try_acquire());
    assert!(!limiter.try_acquire());
    assert!(!limiter.try_acquire());
    println!("3 次被拒绝后：dropped_count = {}", limiter.dropped_count());
    assert_eq!(limiter.dropped_count(), 3);

    println!("\n  ✓ dropped_count 正确统计被限流的日志数");
}

/// 展示令牌自动补充
fn show_token_refill() {
    print_section("4. 令牌自动补充");

    let limiter = RateLimiter::new(100);
    println!("RateLimiter::new(100) — 每秒 100 个令牌\n");

    // 耗尽所有令牌
    for _ in 0..100 {
        limiter.try_acquire();
    }
    println!("耗尽 100 个令牌后：");
    assert!(!limiter.try_acquire());
    println!("  try_acquire() → false（桶空）");

    // 等待 50ms，应补充约 5 个令牌（100 tokens/sec × 0.05s）
    thread::sleep(Duration::from_millis(50));
    println!("\n等待 50ms 后（补充约 5 个令牌）：");
    let mut refilled = 0;
    for _ in 0..10 {
        if limiter.try_acquire() {
            refilled += 1;
        }
    }
    println!("  再次获取：{} 次允许（预期 ~5 次）", refilled);
    assert!(refilled > 0, "等待后应有令牌可用");
    println!("  ✓ 令牌按 refill_rate 自动补充");
}

/// 展示日志限流的实际应用模式
fn show_log_rate_limiting_pattern() {
    print_section("5. 日志限流应用模式");

    let limiter = RateLimiter::new(10);
    println!("场景：限制日志输出速率为 10 条/秒\n");

    println!("模拟突发日志（20 条）：");
    let mut written = 0;
    let mut dropped = 0;
    for i in 1..=20 {
        if limiter.try_acquire() {
            println!("  [INFO] 日志 #{} — 已写入", i);
            written += 1;
        } else {
            dropped += 1;
        }
    }

    println!("\n统计：");
    println!("  写入: {} 条", written);
    println!("  丢弃: {} 条", dropped);
    println!("  总计丢弃: {} 条", limiter.dropped_count());
    assert_eq!(written + dropped, 20);
    assert_eq!(limiter.dropped_count(), dropped as u64);
    println!("\n  ✓ 限流模式正确：突发流量下保护下游系统");
}

/// 展示零速率限流器
fn show_zero_rate() {
    print_section("6. 零速率限流器（rate=0）");

    let limiter = RateLimiter::new(0);
    println!("RateLimiter::new(0) — 不允许任何日志\n");

    assert!(!limiter.try_acquire());
    assert!(!limiter.try_acquire());
    println!("  try_acquire() → 始终 false");
    println!("  dropped_count = {}", limiter.dropped_count());
    assert_eq!(limiter.dropped_count(), 2);
    println!("  ✓ rate=0 等效于完全静默");
}

/// 最佳实践建议
fn show_best_practices() {
    print_section("7. 最佳实践");

    println!("RateLimiter 使用建议：\n");

    println!("1. 速率选择：");
    println!("   - 开发环境: 1000-5000 tokens/sec（宽松）");
    println!("   - 生产环境: 100-1000 tokens/sec（根据下游能力）");
    println!("   - 故障场景: 10-50 tokens/sec（防止日志风暴）");

    println!("\n2. 与 LoggerManager 集成：");
    println!("   let limiter = RateLimiter::new(1000);");
    println!("   // 在日志写入前检查:");
    println!("   if limiter.try_acquire() {{");
    println!("       sink.write(&record).await;");
    println!("   }}");

    println!("\n3. 监控 dropped_count：");
    println!("   - 定期上报 dropped_count 到 Metrics");
    println!("   - dropped_count 持续增长说明限流过严");
    println!("   - dropped_count = 0 说明限流过松或无压力");

    println!("\n4. 性能特点：");
    println!("   - try_acquire() 为 O(1) 操作");
    println!("   - 使用 parking_lot::Mutex（~20ns 锁开销）");
    println!("   - 适合高频日志场景");
}
