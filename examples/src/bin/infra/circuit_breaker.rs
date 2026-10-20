// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 断路器示例（Layer 2 外部服务）
//!
//! 演示 CircuitBreaker 的故障隔离与自动恢复机制，覆盖完整的状态机：
//! Closed → Open → HalfOpen → Closed。
//!
//! # 功能演示
//!
//! - `CircuitBreaker::new` 创建断路器（指定失败阈值、超时、成功阈值）
//! - `CircuitBreaker::with_config` + `CircuitBreakerConfig` 配置式构建
//! - 失败计数触发 Closed → Open 转换
//! - 超时后通过 `can_execute` 触发 Open → HalfOpen 转换
//! - 半开状态下成功次数累积触发 HalfOpen → Closed 恢复
//! - 半开状态下失败立即回退 HalfOpen → Open
//! - `reset` 手动重置断路器
//!
//! # 状态机
//!
//! ```text
//!          失败次数 >= failure_threshold
//!   Closed ─────────────────────────────► Open
//!      ▲                                     │
//!      │                                     │ 超时后 can_execute
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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-circuit-breaker-inklog-circuit-breaker-example")
    );

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

    // 8. 真实场景模拟：用 can_execute 守护操作
    show_realistic_usage().await;

    println!(
        "\n{}",
        tr("example-circuit-breaker-all-circuit-breaker-examples")
    );
    Ok(())
}

/// 展示断路器创建与初始状态
fn show_creation() {
    print_separator(&tr(
        "example-circuit-breaker-1-breaker-creation-and-initial",
    ));

    print_section(&tr(
        "example-circuit-breaker-1-1-circuitbreaker-new-failure",
    ));
    let cb = CircuitBreaker::new(5, Duration::from_secs(30), 3);
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-threshold",
            &[("value", &(cb.config().failure_threshold))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-success-threshold",
            &[("value", &(cb.config().success_threshold))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-timeout",
            &[("value", &format!("{:?}", cb.config().timeout))]
        )
    );
    assert_eq!(cb.config().failure_threshold, 5);
    assert_eq!(cb.config().success_threshold, 3);

    print_section(&tr("example-circuit-breaker-1-2-initial-state-is-closed"));
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-count",
            &[("value", &(cb.failure_count()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-can-execute",
            &[("value", &(cb.can_execute()))]
        )
    );
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.failure_count(), 0);
    assert!(cb.can_execute());
}

/// 展示 Closed → Open：失败计数触发熔断
fn show_closed_to_open() {
    print_separator(&tr("example-circuit-breaker-2-closed-open-failure-count"));

    print_section(&tr(
        "example-circuit-breaker-2-1-config-failure-threshold-3",
    ));
    let cb = CircuitBreaker::new(3, Duration::from_secs(60), 2);
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-initial-state",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );

    print_section(&tr("example-circuit-breaker-2-2-failure-1-below-threshold"));
    cb.record_failure();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-count",
            &[("value", &(cb.failure_count()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-2",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-can-execute",
            &[("value", &(cb.can_execute()))]
        )
    );
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.can_execute());

    print_section(&tr("example-circuit-breaker-2-3-failure-2-below-threshold"));
    cb.record_failure();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-count",
            &[("value", &(cb.failure_count()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-2",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    assert_eq!(cb.state(), CircuitState::Closed);

    print_section(&tr(
        "example-circuit-breaker-2-4-failure-3-threshold-reached",
    ));
    cb.record_failure();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-count",
            &[("value", &(cb.failure_count()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-2",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-can-execute",
            &[("value", &(cb.can_execute()))]
        )
    );
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.can_execute(), "Open state should fail fast");

    print_section(&tr(
        "example-circuit-breaker-2-5-fast-fail-in-open-state-no",
    ));
    println!(
        "{}",
        tr("example-circuit-breaker-in-the-open-state-can-execute")
    );
    println!(
        "{}",
        tr("example-circuit-breaker-avoids-repeatedly-sending")
    );
}

/// 展示 Open → HalfOpen：超时后自动进入半开
fn show_open_to_half_open() {
    print_separator(&tr("example-circuit-breaker-3-open-halfopen-automatically"));

    print_section(&tr("example-circuit-breaker-3-1-config-timeout-100ms"));
    let cb = CircuitBreaker::new(2, Duration::from_millis(100), 3);
    // 触发熔断
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-after-tripping",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-can-execute-timeout-not-elapsed",
            &[("value", &(cb.can_execute()))]
        )
    );

    print_section(&tr("example-circuit-breaker-3-2-waiting-150ms-for-the"));
    std::thread::sleep(Duration::from_millis(150));
    println!("{}", tr("example-circuit-breaker-wait-elapsed-calling-can"));

    print_section(&tr("example-circuit-breaker-3-3-can-execute-triggers-open"));
    let allowed = cb.can_execute();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-can-execute-2",
            &[("allowed", &allowed)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-2",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    assert!(
        allowed,
        "Probe requests should be allowed after the timeout"
    );
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section(&tr("example-circuit-breaker-3-4-halfopen-allows-a-limited"));
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-2",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-can-execute",
            &[("value", &(cb.can_execute()))]
        )
    );
    assert!(cb.can_execute(), "HalfOpen allows probe requests through");
}

/// 展示 HalfOpen → Closed：累计成功恢复
fn show_half_open_to_closed() {
    print_separator(&tr("example-circuit-breaker-4-halfopen-closed-recovers"));

    print_section(&tr(
        "example-circuit-breaker-4-1-config-success-threshold-3",
    ));
    let cb = CircuitBreaker::new(2, Duration::from_millis(100), 3);
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    std::thread::sleep(Duration::from_millis(150));
    assert!(cb.can_execute());
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    println!("{}", tr("example-circuit-breaker-now-in-halfopen-success"));

    print_section(&tr("example-circuit-breaker-4-2-success-1-below-threshold"));
    cb.record_success();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-3",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section(&tr("example-circuit-breaker-4-3-success-2-below-threshold"));
    cb.record_success();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-3",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section(&tr(
        "example-circuit-breaker-4-4-success-3-threshold-reached",
    ));
    cb.record_success();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-2",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-count",
            &[("value", &(cb.failure_count()))]
        )
    );
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(
        cb.failure_count(),
        0,
        "Failure count should reset to zero after recovery"
    );
}

/// 展示 HalfOpen → Open：半开状态下失败立即回退
fn show_half_open_to_open() {
    print_separator(&tr("example-circuit-breaker-5-halfopen-open-a-failure-in"));

    print_section(&tr("example-circuit-breaker-5-1-entering-halfopen"));
    let cb = CircuitBreaker::new(2, Duration::from_millis(100), 3);
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    std::thread::sleep(Duration::from_millis(150));
    assert!(cb.can_execute());
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-3",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );

    print_section(&tr("example-circuit-breaker-5-2-one-success-recorded-in"));
    cb.record_success();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-3",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section(&tr("example-circuit-breaker-5-3-failure-in-half-open"));
    cb.record_failure();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-3",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
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
    print_separator(&tr("example-circuit-breaker-6-config-driven-construction"));

    print_section(&tr("example-circuit-breaker-6-1-customizing"));
    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        success_threshold: 5,
        timeout: Duration::from_secs(60),
    };
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-threshold-2",
            &[("failure_threshold", &config.failure_threshold)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-success-threshold-2",
            &[("success_threshold", &config.success_threshold)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-timeout-2",
            &[("timeout", &format!("{:?}", config.timeout))]
        )
    );

    print_section(&tr(
        "example-circuit-breaker-6-2-circuitbreaker-with-config",
    ));
    let cb = CircuitBreaker::with_config(config.clone());
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-config-failure-threshold",
            &[("value", &(cb.config().failure_threshold))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-config-success-threshold",
            &[("value", &(cb.config().success_threshold))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-config-timeout",
            &[("value", &format!("{:?}", cb.config().timeout))]
        )
    );
    assert_eq!(cb.config().failure_threshold, 10);
    assert_eq!(cb.config().success_threshold, 5);

    print_section(&tr(
        "example-circuit-breaker-6-3-circuitbreakerconfig-default",
    ));
    let default_config = CircuitBreakerConfig::default();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-default-failure-threshold",
            &[("failure_threshold", &default_config.failure_threshold)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-default-success-threshold",
            &[("success_threshold", &default_config.success_threshold)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-default-timeout",
            &[("timeout", &format!("{:?}", default_config.timeout))]
        )
    );
    assert_eq!(default_config.failure_threshold, 5);
    assert_eq!(default_config.success_threshold, 3);
    assert_eq!(default_config.timeout, Duration::from_secs(30));

    print_section(&tr("example-circuit-breaker-6-4-success-in-closed-state"));
    let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        timeout: Duration::from_secs(60),
    });
    cb.record_failure();
    cb.record_failure();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-count-after-2-failures",
            &[("value", &(cb.failure_count()))]
        )
    );
    cb.record_success();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-count-after-record",
            &[("value", &(cb.failure_count()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-3",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    assert_eq!(cb.failure_count(), 0);
    assert_eq!(cb.state(), CircuitState::Closed);
}

/// 展示 reset 手动重置
fn show_reset() {
    print_separator(&tr("example-circuit-breaker-7-manual-reset-with-reset"));

    print_section(&tr("example-circuit-breaker-7-1-tripping-the-breaker-into"));
    let cb = CircuitBreaker::new(2, Duration::from_secs(60), 3);
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-2",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-count",
            &[("value", &(cb.failure_count()))]
        )
    );

    print_section(&tr("example-circuit-breaker-7-2-reset-returns-to-the"));
    cb.reset();
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-state-2",
            &[("value", &format!("{:?}", cb.state()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-failure-count",
            &[("value", &(cb.failure_count()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-circuit-breaker-can-execute",
            &[("value", &(cb.can_execute()))]
        )
    );
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.failure_count(), 0);
    assert!(cb.can_execute());
}

/// 展示真实场景：用 can_execute 守护操作并记录结果
async fn show_realistic_usage() {
    print_separator(&tr(
        "example-circuit-breaker-8-realistic-simulation-guarding",
    ));

    print_section(&tr("example-circuit-breaker-8-1-simulated-sink-writes"));
    let cb = CircuitBreaker::new(3, Duration::from_millis(200), 2);

    // 定义一个模拟的写操作：根据传入参数返回成功或失败
    let write_op = |succeed: bool| -> Result<(), &'static str> {
        if succeed {
            Ok(())
        } else {
            Err("sink write failed")
        }
    };

    print_section(&tr("example-circuit-breaker-8-2-phase-1-3-consecutive"));
    for i in 1..=3 {
        if cb.can_execute() {
            match write_op(false) {
                Ok(_) => cb.record_success(),
                Err(e) => {
                    println!(
                        "  {}",
                        tr_args(
                            "example-circuit-breaker-write-failed",
                            &[("i", &i), ("e", &e)]
                        )
                    );
                    cb.record_failure();
                }
            }
        } else {
            println!(
                "  {}",
                tr_args(
                    "example-circuit-breaker-request-rejected-by-the-breaker",
                    &[("i", &i)]
                )
            );
        }
        println!(
            "    {}",
            tr_args(
                "example-circuit-breaker-state-failure-count",
                &[
                    ("value", &format!("{:?}", cb.state())),
                    ("value2", &(cb.failure_count()))
                ]
            )
        );
    }
    assert_eq!(cb.state(), CircuitState::Open);

    print_section(&tr("example-circuit-breaker-8-3-phase-2-fast-fail-in-open"));
    for i in 1..=2 {
        if cb.can_execute() {
            println!(
                "  {}",
                tr_args(
                    "example-circuit-breaker-request-allowed-to-execute",
                    &[("i", &i)]
                )
            );
        } else {
            println!(
                "  {}",
                tr_args(
                    "example-circuit-breaker-request-fast-rejected-by-the",
                    &[("i", &i)]
                )
            );
        }
    }
    assert_eq!(cb.state(), CircuitState::Open);

    print_section(&tr(
        "example-circuit-breaker-8-4-phase-3-waiting-250ms-past",
    ));
    tokio::time::sleep(Duration::from_millis(250)).await;
    if cb.can_execute() {
        println!(
            "  {}",
            tr_args(
                "example-circuit-breaker-probe-request-allowed-after",
                &[("value", &format!("{:?}", cb.state()))]
            )
        );
    }
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    print_section(&tr("example-circuit-breaker-8-5-phase-4-2-accumulated"));
    // success_threshold = 2
    for i in 1..=2 {
        match write_op(true) {
            Ok(_) => {
                cb.record_success();
                println!(
                    "  {}",
                    tr_args(
                        "example-circuit-breaker-probe-succeeded-state",
                        &[("i", &i), ("value", &format!("{:?}", cb.state()))]
                    )
                );
            }
            Err(_) => {
                cb.record_failure();
                println!(
                    "  {}",
                    tr_args(
                        "example-circuit-breaker-probe-failed-state",
                        &[("i", &i), ("value", &format!("{:?}", cb.state()))]
                    )
                );
            }
        }
    }
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.failure_count(), 0);

    print_section(&tr("example-circuit-breaker-8-6-full-state-machine"));
    println!(
        "{}",
        tr("example-circuit-breaker-closed-3-failures-open-timeout")
    );
    println!(
        "{}",
        tr("example-circuit-breaker-sink-recovered-breaker-back-to")
    );
}
