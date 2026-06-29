// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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

use inklog::support::io::sink::circuit_breaker::{
	CircuitBreaker, CircuitBreakerConfig, CircuitState,
};
use inklog_examples::common::{print_section, print_separator};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog 断路器示例 ===\n");

	// 1. 基础创建与初始状态
	show_creation();

	// 2. Closed → Open：失败计数触发熔断
	show_closed_to_open();

	// 3. Open → HalfOpen：超时后自动进入半开
	show_open_to_half_open();

	// 4. HalfOpen → Closed：累计成功恢复
	show_half_open_to_closed();

	// 5. HalfOpen → Open：半开状态下失败立即回退
	show_half_open_to_open();

	// 6. with_config + CircuitBreakerConfig 配置式构建
	show_with_config();

	// 7. reset() 手动重置
	show_reset();

	// 8. 真实场景模拟：用 can_execute() 守护操作
	show_realistic_usage().await;

	println!("\n✓ 所有断路器示例演示完成");
	Ok(())
}

/// 展示断路器创建与初始状态
fn show_creation() {
	print_separator("1. 断路器创建与初始状态");

	print_section("1.1 CircuitBreaker::new(failure_threshold, timeout, success_threshold)");
	let cb = CircuitBreaker::new(5, Duration::from_secs(30), 3);
	println!("failure_threshold = {}", cb.config().failure_threshold);
	println!("success_threshold = {}", cb.config().success_threshold);
	println!("timeout           = {:?}", cb.config().timeout);
	assert_eq!(cb.config().failure_threshold, 5);
	assert_eq!(cb.config().success_threshold, 3);

	print_section("1.2 初始状态为 Closed，可执行");
	println!("state()       = {:?}", cb.state());
	println!("failure_count = {}", cb.failure_count());
	println!("can_execute() = {}", cb.can_execute());
	assert_eq!(cb.state(), CircuitState::Closed);
	assert_eq!(cb.failure_count(), 0);
	assert!(cb.can_execute());
}

/// 展示 Closed → Open：失败计数触发熔断
fn show_closed_to_open() {
	print_separator("2. Closed → Open：失败计数触发熔断");

	print_section("2.1 配置 failure_threshold = 3");
	let mut cb = CircuitBreaker::new(3, Duration::from_secs(60), 2);
	println!("初始 state = {:?}", cb.state());

	print_section("2.2 第 1 次失败（未达阈值）");
	cb.record_failure();
	println!("failure_count = {}", cb.failure_count());
	println!("state         = {:?}", cb.state());
	println!("can_execute() = {}", cb.can_execute());
	assert_eq!(cb.state(), CircuitState::Closed);
	assert!(cb.can_execute());

	print_section("2.3 第 2 次失败（未达阈值）");
	cb.record_failure();
	println!("failure_count = {}", cb.failure_count());
	println!("state         = {:?}", cb.state());
	assert_eq!(cb.state(), CircuitState::Closed);

	print_section("2.4 第 3 次失败（达到阈值 → Open）");
	cb.record_failure();
	println!("failure_count = {}", cb.failure_count());
	println!("state         = {:?}", cb.state());
	println!("can_execute() = {}", cb.can_execute());
	assert_eq!(cb.state(), CircuitState::Open);
	assert!(!cb.can_execute(), "Open 状态下应快速失败");

	print_section("2.5 Open 状态快速失败（不执行实际操作）");
	println!("在 Open 状态下，can_execute() 返回 false，请求被快速拒绝");
	println!("避免持续向故障 Sink 发送请求");
}

/// 展示 Open → HalfOpen：超时后自动进入半开
fn show_open_to_half_open() {
	print_separator("3. Open → HalfOpen：超时后自动进入半开");

	print_section("3.1 配置 timeout = 100ms");
	let mut cb = CircuitBreaker::new(2, Duration::from_millis(100), 3);
	// 触发熔断
	cb.record_failure();
	cb.record_failure();
	assert_eq!(cb.state(), CircuitState::Open);
	println!("触发熔断后 state = {:?}", cb.state());
	println!("can_execute()    = {}（未超时）", cb.can_execute());

	print_section("3.2 等待 150ms 超时");
	std::thread::sleep(Duration::from_millis(150));
	println!("等待完成，再次调用 can_execute() 触发状态转换");

	print_section("3.3 can_execute() 触发 Open → HalfOpen");
	let allowed = cb.can_execute();
	println!("can_execute() = {}", allowed);
	println!("state         = {:?}", cb.state());
	assert!(allowed, "超时后应允许试探性请求");
	assert_eq!(cb.state(), CircuitState::HalfOpen);

	print_section("3.4 HalfOpen 状态允许有限请求");
	println!("state         = {:?}", cb.state());
	println!("can_execute() = {}", cb.can_execute());
	assert!(cb.can_execute(), "HalfOpen 允许试探请求通过");
}

/// 展示 HalfOpen → Closed：累计成功恢复
fn show_half_open_to_closed() {
	print_separator("4. HalfOpen → Closed：累计成功恢复");

	print_section("4.1 配置 success_threshold = 3");
	let mut cb = CircuitBreaker::new(2, Duration::from_millis(100), 3);
	cb.record_failure();
	cb.record_failure();
	assert_eq!(cb.state(), CircuitState::Open);
	std::thread::sleep(Duration::from_millis(150));
	assert!(cb.can_execute());
	assert_eq!(cb.state(), CircuitState::HalfOpen);
	println!("已进入 HalfOpen 状态，success_threshold = 3");

	print_section("4.2 第 1 次成功（未达阈值，仍 HalfOpen）");
	cb.record_success();
	println!("state = {:?}", cb.state());
	assert_eq!(cb.state(), CircuitState::HalfOpen);

	print_section("4.3 第 2 次成功（未达阈值，仍 HalfOpen）");
	cb.record_success();
	println!("state = {:?}", cb.state());
	assert_eq!(cb.state(), CircuitState::HalfOpen);

	print_section("4.4 第 3 次成功（达到阈值 → Closed）");
	cb.record_success();
	println!("state         = {:?}", cb.state());
	println!("failure_count = {}", cb.failure_count());
	assert_eq!(cb.state(), CircuitState::Closed);
	assert_eq!(cb.failure_count(), 0, "恢复后失败计数应清零");
}

/// 展示 HalfOpen → Open：半开状态下失败立即回退
fn show_half_open_to_open() {
	print_separator("5. HalfOpen → Open：半开状态下失败立即回退");

	print_section("5.1 进入 HalfOpen 状态");
	let mut cb = CircuitBreaker::new(2, Duration::from_millis(100), 3);
	cb.record_failure();
	cb.record_failure();
	assert_eq!(cb.state(), CircuitState::Open);
	std::thread::sleep(Duration::from_millis(150));
	assert!(cb.can_execute());
	assert_eq!(cb.state(), CircuitState::HalfOpen);
	println!("state = {:?}", cb.state());

	print_section("5.2 半开状态下记录 1 次成功（仍 HalfOpen）");
	cb.record_success();
	println!("state = {:?}", cb.state());
	assert_eq!(cb.state(), CircuitState::HalfOpen);

	print_section("5.3 半开状态下失败 → 立即回退 Open");
	cb.record_failure();
	println!("state = {:?}", cb.state());
	assert_eq!(
		cb.state(),
		CircuitState::Open,
		"HalfOpen 失败应立即回退到 Open"
	);
	assert!(!cb.can_execute(), "回退后请求应被拒绝");
}

/// 展示 with_config + CircuitBreakerConfig 配置式构建
fn show_with_config() {
	print_separator("6. with_config + CircuitBreakerConfig 配置式构建");

	print_section("6.1 CircuitBreakerConfig 自定义");
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
	println!("config().failure_threshold = {}", cb.config().failure_threshold);
	println!("config().success_threshold = {}", cb.config().success_threshold);
	println!("config().timeout           = {:?}", cb.config().timeout);
	assert_eq!(cb.config().failure_threshold, 10);
	assert_eq!(cb.config().success_threshold, 5);

	print_section("6.3 CircuitBreakerConfig::default()");
	let default_config = CircuitBreakerConfig::default();
	println!("default failure_threshold = {}", default_config.failure_threshold);
	println!("default success_threshold = {}", default_config.success_threshold);
	println!("default timeout           = {:?}", default_config.timeout);
	assert_eq!(default_config.failure_threshold, 5);
	assert_eq!(default_config.success_threshold, 3);
	assert_eq!(default_config.timeout, Duration::from_secs(30));

	print_section("6.4 Closed 状态下成功重置失败计数");
	let mut cb = CircuitBreaker::with_config(CircuitBreakerConfig {
		failure_threshold: 3,
		success_threshold: 2,
		timeout: Duration::from_secs(60),
	});
	cb.record_failure();
	cb.record_failure();
	println!("2 次失败后 failure_count = {}", cb.failure_count());
	cb.record_success();
	println!("record_success() 后 failure_count = {}", cb.failure_count());
	println!("state = {:?}", cb.state());
	assert_eq!(cb.failure_count(), 0);
	assert_eq!(cb.state(), CircuitState::Closed);
}

/// 展示 reset() 手动重置
fn show_reset() {
	print_separator("7. reset() 手动重置");

	print_section("7.1 触发熔断进入 Open");
	let mut cb = CircuitBreaker::new(2, Duration::from_secs(60), 3);
	cb.record_failure();
	cb.record_failure();
	assert_eq!(cb.state(), CircuitState::Open);
	println!("state         = {:?}", cb.state());
	println!("failure_count = {}", cb.failure_count());

	print_section("7.2 reset() 重置到初始状态");
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
	print_separator("8. 真实场景模拟：can_execute() 守护操作");

	print_section("8.1 模拟 Sink 写入（失败阈值 = 3，超时 = 200ms）");
	let mut cb = CircuitBreaker::new(3, Duration::from_millis(200), 2);

	// 定义一个模拟的写操作：根据传入参数返回成功或失败
	let write_op = |succeed: bool| -> Result<(), &'static str> {
		if succeed {
			Ok(())
		} else {
			Err("sink write failed")
		}
	};

	print_section("8.2 阶段一：连续 3 次失败 → 触发熔断");
	for i in 1..=3 {
		if cb.can_execute() {
			match write_op(false) {
				Ok(_) => cb.record_success(),
				Err(e) => {
					println!("  第 {} 次写入失败：{}", i, e);
					cb.record_failure();
				}
			}
		} else {
			println!("  第 {} 次请求被断路器拒绝", i);
		}
		println!("    state = {:?}, failure_count = {}", cb.state(), cb.failure_count());
	}
	assert_eq!(cb.state(), CircuitState::Open);

	print_section("8.3 阶段二：Open 状态快速失败（不调用 write_op）");
	for i in 1..=2 {
		if cb.can_execute() {
			println!("  第 {} 次请求被允许执行", i);
		} else {
			println!("  第 {} 次请求被断路器快速拒绝（保护故障 Sink）", i);
		}
	}
	assert_eq!(cb.state(), CircuitState::Open);

	print_section("8.4 阶段三：等待 250ms 超时 → 进入 HalfOpen");
	tokio::time::sleep(Duration::from_millis(250)).await;
	if cb.can_execute() {
		println!("  超时后试探请求被允许，state = {:?}", cb.state());
	}
	assert_eq!(cb.state(), CircuitState::HalfOpen);

	print_section("8.5 阶段四：HalfOpen 累计 2 次成功 → 恢复 Closed");
	// success_threshold = 2
	for i in 1..=2 {
		match write_op(true) {
			Ok(_) => {
				cb.record_success();
				println!("  第 {} 次试探成功，state = {:?}", i, cb.state());
			}
			Err(_) => {
				cb.record_failure();
				println!("  第 {} 次试探失败，state = {:?}", i, cb.state());
			}
		}
	}
	assert_eq!(cb.state(), CircuitState::Closed);
	assert_eq!(cb.failure_count(), 0);

	print_section("8.6 状态机完整流转汇总");
	println!("Closed ──3次失败──► Open ──超时──► HalfOpen ──2次成功──► Closed");
	println!("✓ Sink 已恢复，断路器回到正常模式");
}
