// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 断路器示例（Layer 2 外部服务）
//!
//! 演示 CircuitBreaker 的故障隔离与自动恢复机制，覆盖完整的状态机：
//! Closed → Open → HalfOpen → Closed。
//!
//! # 功能演示
//!
//! - `CircuitBreaker::new()` 创建断路器（指定失败阈值、超时、成功阈值）
//! - `CircuitBreaker::with_config()` + `CircuitBreakerConfig` 配置式构建
//! - 失败计数触发 Closed → Open 转换
//! - 超时后通过 `can_execute()` 触发 Open → HalfOpen 转换
//! - 半开状态下成功次数累积触发 HalfOpen → Closed 恢复
//! - 半开状态下失败立即回退 HalfOpen → Open
//! - `reset()` 手动重置断路器
//!
//! # 状态机
//!
//! ```text
//!          失败次数 >= failure_threshold
//!   Closed ─────────────────────────────► Open
//!      ▲                                     │
//!      │                                     │ 超时后 can_execute()
//!      │ HalfOpen 累计成功 >= success_threshold│
//!      │                                     ▼
//!      └──────────── Closed ◄── HalfOpen
//!                           失败立即回退 Open
//! ```
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin circuit_breaker
//! ```

use inklog::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use inklog_examples::common::{print_section, print_separator};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog circuit breaker example ===\n");

    show_creation();

    // 2. Closed → Open: failure count trips the breaker
    show_closed_to_open();

    // 3. Open → HalfOpen: automatically enters half-open after timeout
    show_open_to_half_open();

    // 4. HalfOpen → Closed: recovers after accumulated successes
    show_half_open_to_closed();

    // 5. HalfOpen → Open: a failure in half-open immediately reopens
    show_half_open_to_open();

    // 6. Config-driven construction with with_config + CircuitBreakerConfig
    show_with_config();

    show_reset();

    // 8. 真实场景模拟：用 can_execute() 守护操作
    show_realistic_usage().await;

    println!("\n✓ All circuit breaker examples completed");
    Ok(())
}

/// 展示断路器创建与初始状态
fn show_creation() {
    print_separator("1. Breaker creation and initial state");

    print_section("1.1 CircuitBreaker::new(failure_threshold, timeout, success_threshold)");
    let cb = CircuitBreaker::new(5, Duration::from_secs(30), 3);
    println!("failure_threshold = {}", cb.config().failure_threshold);
    println!("success_threshold = {}", cb.config().success_threshold);
    println!("timeout           = {:?}", cb.config().timeout);
    assert_eq!(cb.config().failure_threshold, 5);
    assert_eq!(cb.config().success_threshold, 3);

    print_section("1.2 Initial state is Closed; execution allowed");
    println!("state()       = {:?}", cb.state());
    println!("failure_count = {}", cb.failure_count());
    println!("can_execute() = {}", cb.can_execute());
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.failure_count(), 0);
    assert!(cb.can_execute());
}

/// 展示 Closed → Open：失败计数触发熔断
fn show_closed_to_open() {
    print_separator("2. Closed → Open: failure count trips the breaker");

    print_section("2.1 Config: failure_threshold = 3");
    let cb = CircuitBreaker::new(3, Duration::from_secs(60), 2);
    println!("initial state = {:?}", cb.state());

    print_section("2.2 Failure #1 (below threshold)");
    cb.record_failure();
    println!("failure_count = {}", cb.failure_count());
    println!("state         = {:?}", cb.state());
    println!("can_execute() = {}", cb.can_execute());
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.can_execute());

    print_section("2.3 Failure #2 (below threshold)");
    cb.record_failure();
    println!("failure_count = {}", cb.failure_count());
    println!("state         = {:?}", cb.state());
    assert_eq!(cb.state(), CircuitState::Closed);

    print_section("2.4 Failure #3 (threshold reached → Open)");
    cb.record_failure();
    println!("failure_count = {}", cb.failure_count());
    println!("state         = {:?}", cb.state());
    println!("can_execute() = {}", cb.can_execute());
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.can_execute(), "Open state should fail fast");

    print_section("2.5 Fast fail in Open state (no real operation runs)");
    println!("In the Open state, can_execute() returns false and requests are fast-rejected");
    println!("avoids repeatedly sending requests to a failed sink");
}

/// 展示 Open → HalfOpen：超时后自动进入半开
fn show_open_to_half_open() {
    print_separator("3. Open → HalfOpen: automatically enters half-open after timeout");

    print_section("3.1 Config: timeout = 100ms");
    let cb = CircuitBreaker::new(2, Duration::from_millis(100), 3);
    // 触发熔断
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    println!("state = {:?} after tripping", cb.state());
    println!(
        "can_execute()    = {} (timeout not elapsed)",
        cb.can_execute()
    );

    print_section("3.2 Waiting 150ms for the timeout");
    std::thread::sleep(Duration::from_millis(150));
    println!("Wait elapsed; calling can_execute() again triggers the transition");

    print_section("3.3 can_execute() triggers Open → HalfOpen");
    let allowed = cb.can_execute();
    println!("can_execute() = {}", allowed);
    println!("state         = {:?}", cb.state());
    assert!(
        allowed,
        "Probe requests should be allowed after the timeout"
    );
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section("3.4 HalfOpen allows a limited number of requests");
    println!("state         = {:?}", cb.state());
    println!("can_execute() = {}", cb.can_execute());
    assert!(cb.can_execute(), "HalfOpen allows probe requests through");
}

/// 展示 HalfOpen → Closed：累计成功恢复
fn show_half_open_to_closed() {
    print_separator("4. HalfOpen → Closed: recovers after accumulated successes");

    print_section("4.1 Config: success_threshold = 3");
    let cb = CircuitBreaker::new(2, Duration::from_millis(100), 3);
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    std::thread::sleep(Duration::from_millis(150));
    assert!(cb.can_execute());
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    println!("Now in HalfOpen, success_threshold = 3");

    print_section("4.2 Success #1 (below threshold, still HalfOpen)");
    cb.record_success();
    println!("state = {:?}", cb.state());
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section("4.3 Success #2 (below threshold, still HalfOpen)");
    cb.record_success();
    println!("state = {:?}", cb.state());
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section("4.4 Success #3 (threshold reached → Closed)");
    cb.record_success();
    println!("state         = {:?}", cb.state());
    println!("failure_count = {}", cb.failure_count());
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(
        cb.failure_count(),
        0,
        "Failure count should reset to zero after recovery"
    );
}

/// 展示 HalfOpen → Open：半开状态下失败立即回退
fn show_half_open_to_open() {
    print_separator("5. HalfOpen → Open: a failure in half-open immediately reopens");

    print_section("5.1 Entering HalfOpen");
    let cb = CircuitBreaker::new(2, Duration::from_millis(100), 3);
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    std::thread::sleep(Duration::from_millis(150));
    assert!(cb.can_execute());
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    println!("state = {:?}", cb.state());

    print_section("5.2 One success recorded in half-open (still HalfOpen)");
    cb.record_success();
    println!("state = {:?}", cb.state());
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section("5.3 Failure in half-open → immediately reopens");
    cb.record_failure();
    println!("state = {:?}", cb.state());
    assert_eq!(
        cb.state(),
        CircuitState::Open,
        "A failure in HalfOpen should reopen immediately"
    );
    assert!(
        !cb.can_execute(),
        "Requests should be rejected after reopening"
    );
}

/// 展示 with_config + CircuitBreakerConfig 配置式构建
fn show_with_config() {
    print_separator("6. Config-driven construction with with_config + CircuitBreakerConfig");

    print_section("6.1 Customizing CircuitBreakerConfig");
    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        success_threshold: 5,
        timeout: Duration::from_secs(60),
    };
    println!("failure_threshold = {}", config.failure_threshold);
    println!("success_threshold = {}", config.success_threshold);
    println!("timeout           = {:?}", config.timeout);

    print_section("6.2 CircuitBreaker::with_config(config)");
    let cb = CircuitBreaker::with_config(config.clone());
    println!(
        "config().failure_threshold = {}",
        cb.config().failure_threshold
    );
    println!(
        "config().success_threshold = {}",
        cb.config().success_threshold
    );
    println!("config().timeout           = {:?}", cb.config().timeout);
    assert_eq!(cb.config().failure_threshold, 10);
    assert_eq!(cb.config().success_threshold, 5);

    print_section("6.3 CircuitBreakerConfig::default()");
    let default_config = CircuitBreakerConfig::default();
    println!(
        "default failure_threshold = {}",
        default_config.failure_threshold
    );
    println!(
        "default success_threshold = {}",
        default_config.success_threshold
    );
    println!("default timeout           = {:?}", default_config.timeout);
    assert_eq!(default_config.failure_threshold, 5);
    assert_eq!(default_config.success_threshold, 3);
    assert_eq!(default_config.timeout, Duration::from_secs(30));

    print_section("6.4 Success in Closed state resets the failure count");
    let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        timeout: Duration::from_secs(60),
    });
    cb.record_failure();
    cb.record_failure();
    println!("failure_count = {} after 2 failures", cb.failure_count());
    cb.record_success();
    println!(
        "failure_count = {} after record_success()",
        cb.failure_count()
    );
    println!("state = {:?}", cb.state());
    assert_eq!(cb.failure_count(), 0);
    assert_eq!(cb.state(), CircuitState::Closed);
}

/// 展示 reset() 手动重置
fn show_reset() {
    print_separator("7. Manual reset with reset()");

    print_section("7.1 Tripping the breaker into Open");
    let cb = CircuitBreaker::new(2, Duration::from_secs(60), 3);
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    println!("state         = {:?}", cb.state());
    println!("failure_count = {}", cb.failure_count());

    print_section("7.2 reset() returns to the initial state");
    cb.reset();
    println!("state         = {:?}", cb.state());
    println!("failure_count = {}", cb.failure_count());
    println!("can_execute() = {}", cb.can_execute());
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.failure_count(), 0);
    assert!(cb.can_execute());
}

/// 展示真实场景：用 can_execute() 守护操作并记录结果
async fn show_realistic_usage() {
    print_separator("8. Realistic simulation: guarding operations with can_execute()");

    print_section("8.1 Simulated sink writes (failure threshold = 3, timeout = 200ms)");
    let cb = CircuitBreaker::new(3, Duration::from_millis(200), 2);

    // 定义一个模拟的写操作：根据传入参数返回成功或失败
    let write_op = |succeed: bool| -> Result<(), &'static str> {
        if succeed {
            Ok(())
        } else {
            Err("sink write failed")
        }
    };

    print_section("8.2 Phase 1: 3 consecutive failures → breaker trips");
    for i in 1..=3 {
        if cb.can_execute() {
            match write_op(false) {
                Ok(_) => cb.record_success(),
                Err(e) => {
                    println!("  write #{} failed: {}", i, e);
                    cb.record_failure();
                }
            }
        } else {
            println!("  request #{} rejected by the breaker", i);
        }
        println!(
            "    state = {:?}, failure_count = {}",
            cb.state(),
            cb.failure_count()
        );
    }
    assert_eq!(cb.state(), CircuitState::Open);

    print_section("8.3 Phase 2: fast fail in Open (write_op not called)");
    for i in 1..=2 {
        if cb.can_execute() {
            println!("  request #{} allowed to execute", i);
        } else {
            println!(
                "  request #{} fast-rejected by the breaker (protecting the failed sink)",
                i
            );
        }
    }
    assert_eq!(cb.state(), CircuitState::Open);

    print_section("8.4 Phase 3: waiting 250ms past the timeout → HalfOpen");
    tokio::time::sleep(Duration::from_millis(250)).await;
    if cb.can_execute() {
        println!(
            "  probe request allowed after timeout, state = {:?}",
            cb.state()
        );
    }
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section("8.5 Phase 4: 2 accumulated successes in HalfOpen → back to Closed");
    // success_threshold = 2
    for i in 1..=2 {
        match write_op(true) {
            Ok(_) => {
                cb.record_success();
                println!("  probe #{} succeeded, state = {:?}", i, cb.state());
            }
            Err(_) => {
                cb.record_failure();
                println!("  probe #{} failed, state = {:?}", i, cb.state());
            }
        }
    }
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.failure_count(), 0);

    print_section("8.6 Full state machine transition summary");
    println!("Closed ──3 failures──► Open ──timeout──► HalfOpen ──2 successes──► Closed");
    println!("✓ Sink recovered; breaker back to normal mode");
}
